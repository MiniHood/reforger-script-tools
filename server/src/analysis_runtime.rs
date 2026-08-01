//! Revision-safe, compiler-owned document state.
//!
//! This module is deliberately below the LSP boundary.  Protocol handlers may
//! translate their document notifications into these operations, but snapshots
//! and UTF-16/byte conversion remain Rust language-engine concerns.

use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
};

/// A zero-based LSP-compatible position.  `character` is counted in UTF-16
/// code units, never Unicode scalar values or bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Immutable conversion table for a single source snapshot.
///
/// Every byte inside a UTF-8 code point maps to that code point's starting
/// position.  This makes malformed or non-boundary offsets deterministic while
/// callers that need source slicing can still validate a returned byte offset.
#[derive(Debug, Clone)]
pub struct PositionIndex {
    positions: Vec<Position>,
}

impl PositionIndex {
    pub fn new(source: &str) -> Self {
        Self::new_cancellable(source, None).expect("unconditional position index build")
    }

    /// Builds the immutable table while allowing a caller-owned scheduler to
    /// abandon obsolete work.  Cancellation is checked between bounded groups
    /// of source characters, never while a partially-built table is exposed.
    pub fn new_cancellable(source: &str, should_cancel: Option<&dyn Fn() -> bool>) -> Option<Self> {
        let mut positions = vec![
            Position {
                line: 0,
                character: 0
            };
            source.len() + 1
        ];
        let mut position = Position {
            line: 0,
            character: 0,
        };

        for (character_index, (offset, character)) in source.char_indices().enumerate() {
            if character_index % 64 == 0
                && should_cancel.is_some_and(|should_cancel| should_cancel())
            {
                return None;
            }
            let next_offset = offset + character.len_utf8();
            positions[offset..next_offset].fill(position);
            match character {
                '\r' => {
                    position.line = position.line.saturating_add(1);
                    position.character = 0;
                }
                '\n' if offset == 0 || source.as_bytes()[offset - 1] != b'\r' => {
                    position.line = position.line.saturating_add(1);
                    position.character = 0;
                }
                '\n' => {}
                _ => {
                    position.character = position
                        .character
                        .saturating_add(character.len_utf16() as u32)
                }
            }
        }
        positions[source.len()] = position;
        Some(Self { positions })
    }

    pub fn position_for_offset(&self, offset: usize) -> Position {
        self.positions
            .get(offset.min(self.positions.len().saturating_sub(1)))
            .copied()
            .unwrap_or(Position {
                line: 0,
                character: 0,
            })
    }

    /// Returns the earliest byte offset for an exact UTF-16 boundary.
    /// Positions in the middle of a surrogate pair are deliberately rejected.
    pub fn offset_for_position(&self, wanted: Position) -> Option<usize> {
        let mut run_start = 0;
        let mut previous = self.positions.first().copied()?;
        for (offset, position) in self.positions.iter().copied().enumerate() {
            if position != previous {
                run_start = offset;
                previous = position;
            }
            if position == wanted {
                return Some(run_start);
            }
        }
        None
    }

    /// Resolves positions inside a UTF-16 character to that character's byte
    /// start. This is retained only for the protocol's tolerant source-text
    /// fallback; foreground snapshot coordinates use the strict method above.
    pub fn offset_for_position_recovering(&self, wanted: Position) -> Option<usize> {
        let mut run_start = 0;
        let mut previous = self.positions.first().copied()?;
        for (offset, position) in self.positions.iter().copied().enumerate() {
            if position != previous {
                if previous.line == wanted.line
                    && position.line == wanted.line
                    && previous.character < wanted.character
                    && wanted.character < position.character
                {
                    return Some(run_start);
                }
                run_start = offset;
                previous = position;
            }
            if position == wanted {
                return Some(run_start);
            }
        }
        None
    }
}

/// Immutable text and coordinate state for exactly one accepted document edit.
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    uri: Arc<str>,
    version: i32,
    revision: u64,
    text: Arc<str>,
    // Position conversion is foreground work.  The cell is shared by every
    // clone of this revision so a worker installs one immutable table before
    // any protocol projection can consume it.
    positions: Arc<OnceLock<Arc<PositionIndex>>>,
}

impl DocumentSnapshot {
    fn new(uri: Arc<str>, version: i32, revision: u64, text: Arc<str>) -> Self {
        Self {
            uri,
            version,
            revision,
            text,
            positions: Arc::new(OnceLock::new()),
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn version(&self) -> i32 {
        self.version
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn text_arc(&self) -> Arc<str> {
        self.text.clone()
    }
    pub fn positions(&self) -> Option<Arc<PositionIndex>> {
        self.positions.get().cloned()
    }

    /// Installs the foreground-built coordinate table.  A stale worker may
    /// race a newer revision, but it can only fill its own revision's cell.
    pub fn install_positions(&self, positions: PositionIndex) -> bool {
        self.positions.set(Arc::new(positions)).is_ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    Accepted,
    Stale,
}

/// Declares how completely a feature result represents its captured current
/// snapshot.  Callers must never silently substitute an older semantic result
/// when an exact one is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryQuality {
    Exact,
    Unavailable,
}

impl QueryQuality {
    pub const fn permits_local_facts(self) -> bool {
        matches!(self, Self::Exact)
    }
}

/// Latest-wins owner for open document snapshots.
///
/// A snapshot is replaced atomically at the map entry level; readers retain an
/// owned clone and can therefore never observe new text paired with old ranges.
#[derive(Debug, Default)]
pub struct DocumentStore {
    documents: HashMap<Arc<str>, DocumentSnapshot>,
    next_revision: u64,
}

/// Compiler-owned runtime boundary for document identity and admitted work.
/// LSP adaption may submit and publish through this owner, but it must not own
/// an independent document-revision table.
#[derive(Debug)]
pub struct AnalysisRuntime {
    documents: DocumentStore,
    admission: TaskAdmission,
}

impl AnalysisRuntime {
    pub fn new(limits: AdmissionLimits) -> Self {
        Self {
            documents: DocumentStore::new(),
            admission: TaskAdmission::new(limits),
        }
    }

    pub fn upsert(
        &mut self,
        uri: impl Into<Arc<str>>,
        version: i32,
        text: impl Into<Arc<str>>,
    ) -> UpsertOutcome {
        let uri = uri.into();
        let outcome = self.documents.upsert(uri.clone(), version, text);
        // A newly accepted revision makes every retained task for its older
        // snapshot ineligible to publish.  This is deliberately runtime
        // ownership rather than a protocol-layer cancellation convention.
        if outcome == UpsertOutcome::Accepted {
            self.admission.cancel_uri(&uri);
        }
        outcome
    }

    pub fn latest(&self, uri: &str) -> Option<DocumentSnapshot> {
        self.documents.latest(uri)
    }

    pub fn close(&mut self, uri: &str, revision: u64) -> Option<DocumentSnapshot> {
        self.admission.cancel_uri(uri);
        self.documents.close(uri, revision)
    }

    pub fn admit(
        &mut self,
        class: TaskClass,
        snapshot: DocumentSnapshot,
        request_id: u64,
    ) -> AdmissionDisposition {
        self.admission.admit(class, snapshot, request_id)
    }

    pub fn take_next(&mut self) -> Option<AnalysisTask> {
        self.admission.take_next()
    }

    pub fn complete(&mut self, identity: &TaskIdentity) -> bool {
        self.admission.complete(identity)
    }
}

impl DocumentStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accepts only a strictly newer client version for an already-open URI.
    pub fn upsert(
        &mut self,
        uri: impl Into<Arc<str>>,
        version: i32,
        text: impl Into<Arc<str>>,
    ) -> UpsertOutcome {
        let uri = uri.into();
        if self
            .documents
            .get(uri.as_ref())
            .is_some_and(|current| version <= current.version)
        {
            return UpsertOutcome::Stale;
        }
        self.next_revision = self.next_revision.saturating_add(1);
        let snapshot = DocumentSnapshot::new(uri.clone(), version, self.next_revision, text.into());
        self.documents.insert(uri, snapshot);
        UpsertOutcome::Accepted
    }

    pub fn latest(&self, uri: &str) -> Option<DocumentSnapshot> {
        self.documents.get(uri).cloned()
    }

    /// Closes only the snapshot the caller observed.  A delayed close must not
    /// erase a newer document version that arrived while it was in flight.
    pub fn close(&mut self, uri: &str, revision: u64) -> Option<DocumentSnapshot> {
        if self
            .documents
            .get(uri)
            .is_some_and(|current| current.revision == revision)
        {
            self.documents.remove(uri)
        } else {
            None
        }
    }
}

/// Runtime lane for CPU-bearing work. The order is also the deterministic
/// admission priority: foreground work is always dispatched before semantic
/// convergence, which is dispatched before best-effort rich/debug work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskClass {
    Foreground,
    Semantic,
    Rich,
}

impl TaskClass {
    const PRIORITY: [Self; 3] = [Self::Foreground, Self::Semantic, Self::Rich];
}

/// Bounded retained-work limits. Both queued and running tasks count because
/// either one retains a document snapshot until it is cancelled or completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionLimits {
    pub max_retained_jobs: usize,
    pub max_retained_bytes: usize,
}

impl AdmissionLimits {
    pub const fn new(max_retained_jobs: usize, max_retained_bytes: usize) -> Self {
        Self {
            max_retained_jobs,
            max_retained_bytes,
        }
    }
}

/// Identity used for cancellation and publication. A task may publish only
/// while this identity remains current for its URI and lane.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskIdentity {
    uri: Arc<str>,
    class: TaskClass,
    revision: u64,
    request_id: u64,
    sequence: u64,
}

impl TaskIdentity {
    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn class(&self) -> TaskClass {
        self.class
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn request_id(&self) -> u64 {
        self.request_id
    }
}

/// A pending or running unit of runtime work. The cancellation flag is shared
/// with the admission table so long-running code can cooperatively abandon an
/// obsolete snapshot at its own bounded checkpoints.
#[derive(Debug, Clone)]
pub struct AnalysisTask {
    identity: TaskIdentity,
    snapshot: DocumentSnapshot,
    cancelled: Arc<AtomicBool>,
}

impl AnalysisTask {
    pub fn identity(&self) -> &TaskIdentity {
        &self.identity
    }

    pub fn snapshot(&self) -> &DocumentSnapshot {
        &self.snapshot
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Shares runtime-owned cancellation with a bounded background projection.
    /// Consumers may observe or cooperatively poll this flag, but only the
    /// runtime decides whether the task remains eligible to publish.
    pub fn cancellation_token(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }

}

/// Deterministic result of trying to retain a task. A replacement is accepted
/// even when the table is otherwise full because it does not increase the
/// number of retained URI/lane entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionDisposition {
    Enqueued {
        task: TaskIdentity,
        replaced: Option<TaskIdentity>,
    },
    DroppedOverload {
        class: TaskClass,
        retained_jobs: usize,
        retained_bytes: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TaskKey {
    uri: Arc<str>,
    class: TaskClass,
}

#[derive(Debug)]
struct RetainedTask {
    task: AnalysisTask,
    state: RetainedTaskState,
    bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetainedTaskState {
    Queued,
    InFlight,
}

/// Fixed-capacity, latest-wins runtime admission table.
///
/// This type deliberately owns no threads. The future executor asks for the
/// next admitted task, performs bounded work, and calls [`Self::complete`] to
/// obtain the only valid publication decision. Keeping the state here lets
/// protocol adaptation remain a thin submit/publish boundary.
#[derive(Debug)]
pub struct TaskAdmission {
    limits: AdmissionLimits,
    retained: HashMap<TaskKey, RetainedTask>,
    pending: HashMap<TaskClass, VecDeque<TaskIdentity>>,
    retained_bytes: usize,
    next_sequence: u64,
}

impl TaskAdmission {
    pub fn new(limits: AdmissionLimits) -> Self {
        let pending = TaskClass::PRIORITY
            .into_iter()
            .map(|class| (class, VecDeque::new()))
            .collect();
        Self {
            limits,
            retained: HashMap::new(),
            pending,
            retained_bytes: 0,
            next_sequence: 0,
        }
    }

    /// Retains only the newest task for one URI/lane. A new revision cancels
    /// the old one before capacity is evaluated, preventing an obsolete task
    /// from publishing when the replacement is too large to retain.
    pub fn admit(
        &mut self,
        class: TaskClass,
        snapshot: DocumentSnapshot,
        request_id: u64,
    ) -> AdmissionDisposition {
        let key = TaskKey {
            uri: Arc::from(snapshot.uri()),
            class,
        };
        let replaced = self.remove(&key).map(|task| task.identity().clone());
        let bytes = snapshot.text().len();
        if self.retained.len() >= self.limits.max_retained_jobs
            || bytes
                > self
                    .limits
                    .max_retained_bytes
                    .saturating_sub(self.retained_bytes)
        {
            return AdmissionDisposition::DroppedOverload {
                class,
                retained_jobs: self.retained.len(),
                retained_bytes: self.retained_bytes,
            };
        }

        self.next_sequence = self.next_sequence.saturating_add(1);
        let identity = TaskIdentity {
            uri: key.uri.clone(),
            class,
            revision: snapshot.revision(),
            request_id,
            sequence: self.next_sequence,
        };
        let task = AnalysisTask {
            identity: identity.clone(),
            snapshot,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        self.retained_bytes = self.retained_bytes.saturating_add(bytes);
        self.retained.insert(
            key,
            RetainedTask {
                task,
                state: RetainedTaskState::Queued,
                bytes,
            },
        );
        self.pending
            .get_mut(&class)
            .expect("every task class has a queue")
            .push_back(identity.clone());
        AdmissionDisposition::Enqueued {
            task: identity,
            replaced,
        }
    }

    /// Takes the highest-priority queued task. Replaced identities are removed
    /// from their queues eagerly, but this loop also tolerates a stale entry as
    /// a defensive invariant check rather than ever dispatching it.
    pub fn take_next(&mut self) -> Option<AnalysisTask> {
        for class in TaskClass::PRIORITY {
            let queue = self
                .pending
                .get_mut(&class)
                .expect("every task class has a queue");
            while let Some(identity) = queue.pop_front() {
                let key = TaskKey {
                    uri: identity.uri.clone(),
                    class: identity.class,
                };
                let Some(retained) = self.retained.get_mut(&key) else {
                    continue;
                };
                if retained.task.identity != identity || retained.state != RetainedTaskState::Queued
                {
                    continue;
                }
                retained.state = RetainedTaskState::InFlight;
                return Some(retained.task.clone());
            }
        }
        None
    }

    /// Returns true only if this exact task remains current and may publish.
    /// Completion releases retained bytes regardless of whether the task chose
    /// to publish; a replaced task cannot remove its replacement.
    pub fn complete(&mut self, identity: &TaskIdentity) -> bool {
        let key = TaskKey {
            uri: identity.uri.clone(),
            class: identity.class,
        };
        let is_current = self
            .retained
            .get(&key)
            .is_some_and(|retained| retained.task.identity == *identity);
        if is_current {
            self.remove(&key);
        }
        is_current
    }

    /// Cancels all retained work for one URI, for close and replacement
    /// events. Existing workers observe cancellation through their task flag.
    pub fn cancel_uri(&mut self, uri: &str) {
        let keys: Vec<_> = self
            .retained
            .keys()
            .filter(|key| key.uri.as_ref() == uri)
            .cloned()
            .collect();
        for key in keys {
            self.remove(&key);
        }
    }

    pub fn retained_jobs(&self) -> usize {
        self.retained.len()
    }

    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    fn remove(&mut self, key: &TaskKey) -> Option<AnalysisTask> {
        let retained = self.retained.remove(key)?;
        retained.task.cancelled.store(true, Ordering::Release);
        self.retained_bytes = self.retained_bytes.saturating_sub(retained.bytes);
        if retained.state == RetainedTaskState::Queued {
            self.pending
                .get_mut(&key.class)
                .expect("every task class has a queue")
                .retain(|identity| identity != &retained.task.identity);
        }
        Some(retained.task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_quality_never_treats_unavailable_as_current_local_facts() {
        assert!(QueryQuality::Exact.permits_local_facts());
        assert!(!QueryQuality::Unavailable.permits_local_facts());
    }

    #[test]
    fn runtime_close_cancels_admitted_work_for_the_closed_snapshot() {
        let mut runtime = AnalysisRuntime::new(AdmissionLimits::new(2, 32));
        assert_eq!(
            runtime.upsert("file:///a.c", 1, "class A {}"),
            UpsertOutcome::Accepted
        );
        let snapshot = runtime.latest("file:///a.c").unwrap();
        let identity = match runtime.admit(TaskClass::Semantic, snapshot.clone(), 1) {
            AdmissionDisposition::Enqueued { task, .. } => task,
            other => panic!("unexpected admission disposition: {other:?}"),
        };
        let task = runtime.take_next().unwrap();
        assert_eq!(task.identity(), &identity);
        assert!(runtime.close("file:///a.c", snapshot.revision()).is_some());
        assert!(task.is_cancelled());
        assert!(!runtime.complete(&identity));
    }

    #[test]
    fn accepting_a_new_revision_cancels_the_previous_runtime_task() {
        let mut runtime = AnalysisRuntime::new(AdmissionLimits::new(2, 64));
        assert_eq!(
            runtime.upsert("file:///a.c", 1, "class A {}"),
            UpsertOutcome::Accepted
        );
        let old = match runtime.admit(
            TaskClass::Semantic,
            runtime.latest("file:///a.c").unwrap(),
            1,
        ) {
            AdmissionDisposition::Enqueued { .. } => runtime.take_next().unwrap(),
            other => panic!("unexpected admission disposition: {other:?}"),
        };

        assert_eq!(
            runtime.upsert("file:///a.c", 2, "class B {}"),
            UpsertOutcome::Accepted
        );
        assert!(old.is_cancelled());
        assert!(!runtime.complete(old.identity()));
    }

    #[test]
    fn position_index_uses_utf16_and_crlf_boundaries() {
        let index = PositionIndex::new("a😀\r\nb\n");
        assert_eq!(
            index.position_for_offset(0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            index.position_for_offset(1),
            Position {
                line: 0,
                character: 1
            }
        );
        assert_eq!(
            index.position_for_offset(5),
            Position {
                line: 0,
                character: 3
            }
        );
        assert_eq!(
            index.position_for_offset(7),
            Position {
                line: 1,
                character: 0
            }
        );
        assert_eq!(
            index.offset_for_position(Position {
                line: 0,
                character: 2
            }),
            None
        );
        assert_eq!(
            index.offset_for_position(Position {
                line: 1,
                character: 0
            }),
            Some(6)
        );
    }

    #[test]
    fn accepted_snapshot_has_no_positions_until_foreground_installs_them() {
        let mut runtime = AnalysisRuntime::new(AdmissionLimits::new(2, 128));
        assert_eq!(
            runtime.upsert("file:///foreground.c", 1, "a😀\r\nb"),
            UpsertOutcome::Accepted
        );
        let snapshot = runtime.latest("file:///foreground.c").unwrap();
        assert!(snapshot.positions().is_none());

        assert!(snapshot.install_positions(PositionIndex::new(snapshot.text())));
        assert_eq!(
            runtime
                .latest("file:///foreground.c")
                .unwrap()
                .positions()
                .unwrap()
                .offset_for_position(Position {
                    line: 1,
                    character: 1
                }),
            Some("a😀\r\nb".len())
        );
    }

    #[test]
    fn store_rejects_stale_edits_and_preserves_owned_snapshots() {
        let mut store = DocumentStore::new();
        assert_eq!(
            store.upsert("file:///a.c", 1, "old"),
            UpsertOutcome::Accepted
        );
        let old = store.latest("file:///a.c").unwrap();
        assert_eq!(
            store.upsert("file:///a.c", 2, "new"),
            UpsertOutcome::Accepted
        );
        assert_eq!(
            store.upsert("file:///a.c", 1, "stale"),
            UpsertOutcome::Stale
        );
        let current = store.latest("file:///a.c").unwrap();
        assert_eq!(old.text(), "old");
        assert_eq!(current.text(), "new");
        assert!(current.revision() > old.revision());
    }

    #[test]
    fn delayed_close_cannot_remove_a_newer_snapshot() {
        let mut store = DocumentStore::new();
        store.upsert("file:///a.c", 1, "first");
        let first = store.latest("file:///a.c").unwrap();
        store.upsert("file:///a.c", 2, "second");
        assert!(store.close("file:///a.c", first.revision()).is_none());
        let second = store.latest("file:///a.c").unwrap();
        assert_eq!(second.text(), "second");
        assert_eq!(
            store
                .close("file:///a.c", second.revision())
                .unwrap()
                .text(),
            "second"
        );
        assert!(store.latest("file:///a.c").is_none());
    }

    fn snapshot(uri: &str, version: i32, text: &str) -> DocumentSnapshot {
        let mut store = DocumentStore::new();
        assert_eq!(store.upsert(uri, version, text), UpsertOutcome::Accepted);
        store.latest(uri).unwrap()
    }

    #[test]
    fn admission_is_latest_wins_and_cancels_an_inflight_replacement() {
        let mut admission = TaskAdmission::new(AdmissionLimits::new(2, 64));
        let first = snapshot("file:///a.c", 1, "old");
        let second = snapshot("file:///a.c", 2, "new");

        let first_identity = match admission.admit(TaskClass::Semantic, first, 10) {
            AdmissionDisposition::Enqueued { task, replaced } => {
                assert!(replaced.is_none());
                task
            }
            other => panic!("unexpected disposition: {other:?}"),
        };
        let running = admission.take_next().unwrap();
        assert_eq!(running.identity(), &first_identity);

        let second_identity = match admission.admit(TaskClass::Semantic, second, 11) {
            AdmissionDisposition::Enqueued { task, replaced } => {
                assert_eq!(replaced.as_ref(), Some(&first_identity));
                task
            }
            other => panic!("unexpected disposition: {other:?}"),
        };
        assert!(running.is_cancelled());
        assert!(!admission.complete(running.identity()));

        let replacement = admission.take_next().unwrap();
        assert_eq!(replacement.identity(), &second_identity);
        assert!(admission.complete(replacement.identity()));
        assert_eq!(admission.retained_jobs(), 0);
        assert_eq!(admission.retained_bytes(), 0);
    }

    #[test]
    fn admission_prioritizes_foreground_and_bounds_retained_snapshots() {
        let mut admission = TaskAdmission::new(AdmissionLimits::new(2, 6));
        let semantic = snapshot("file:///semantic.c", 1, "abc");
        let foreground = snapshot("file:///foreground.c", 1, "def");
        let overload = snapshot("file:///rich.c", 1, "g");

        assert!(matches!(
            admission.admit(TaskClass::Semantic, semantic, 1),
            AdmissionDisposition::Enqueued { .. }
        ));
        assert!(matches!(
            admission.admit(TaskClass::Foreground, foreground, 2),
            AdmissionDisposition::Enqueued { .. }
        ));
        assert_eq!(admission.retained_jobs(), 2);
        assert_eq!(admission.retained_bytes(), 6);
        assert_eq!(
            admission.admit(TaskClass::Rich, overload, 3),
            AdmissionDisposition::DroppedOverload {
                class: TaskClass::Rich,
                retained_jobs: 2,
                retained_bytes: 6,
            }
        );

        let first = admission.take_next().unwrap();
        assert_eq!(first.identity().class(), TaskClass::Foreground);
        assert!(admission.complete(first.identity()));
        let second = admission.take_next().unwrap();
        assert_eq!(second.identity().class(), TaskClass::Semantic);
        assert!(admission.complete(second.identity()));
    }

    #[test]
    fn overload_cancels_an_obsolete_task_and_close_releases_queued_bytes() {
        let mut admission = TaskAdmission::new(AdmissionLimits::new(1, 3));
        let old = snapshot("file:///a.c", 1, "abc");
        let too_large = snapshot("file:///a.c", 2, "abcd");
        let old_identity = match admission.admit(TaskClass::Semantic, old, 1) {
            AdmissionDisposition::Enqueued { task, .. } => task,
            other => panic!("unexpected disposition: {other:?}"),
        };
        let old_task = admission.take_next().unwrap();

        assert_eq!(
            admission.admit(TaskClass::Semantic, too_large, 2),
            AdmissionDisposition::DroppedOverload {
                class: TaskClass::Semantic,
                retained_jobs: 0,
                retained_bytes: 0,
            }
        );
        assert!(old_task.is_cancelled());
        assert!(!admission.complete(&old_identity));
        assert_eq!(admission.retained_jobs(), 0);

        let queued = snapshot("file:///a.c", 3, "abc");
        assert!(matches!(
            admission.admit(TaskClass::Rich, queued, 3),
            AdmissionDisposition::Enqueued { .. }
        ));
        admission.cancel_uri("file:///a.c");
        assert_eq!(admission.retained_jobs(), 0);
        assert_eq!(admission.retained_bytes(), 0);
        assert!(admission.take_next().is_none());
    }
}
