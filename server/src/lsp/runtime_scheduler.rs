use super::*;

pub(super) struct OpenDocumentAnalysisJob {
    pub(super) task: AnalysisTask,
    pub(super) scheduled_at: Instant,
}

pub(super) struct ForegroundDocumentJob {
    pub(super) task: AnalysisTask,
    pub(super) scheduled_at: Instant,
}

#[derive(Clone)]
pub(super) struct RuntimeWorkExecutor {
    pub(super) state: Arc<(
        Mutex<BTreeMap<(TaskClass, String), RuntimeWorkJob>>,
        Condvar,
    )>,
    pub(super) sender: mpsc::Sender<ServerEvent>,
    #[cfg(test)]
    pub(super) test_before_execute: Option<RuntimeWorkTestHook>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeWorkerLane {
    Foreground,
    /// On a single logical CPU, the sole foreground worker may advance
    /// background convergence only when no foreground work is runnable. This
    /// avoids oversubscribing the CPU while preserving eventual convergence
    /// during an idle period.
    ForegroundWithIdleBackground,
    Background,
}

impl RuntimeWorkerLane {
    pub(super) fn accepts(self, class: TaskClass) -> bool {
        match self {
            // This reservation means an edit can build its current lexical and
            // syntax snapshot even when a whole-file background job is busy.
            Self::Foreground | Self::ForegroundWithIdleBackground => class == TaskClass::Foreground,
            Self::Background => class != TaskClass::Foreground,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RuntimeWorkCapacity {
    pub(super) foreground_workers: usize,
    pub(super) background_workers: usize,
}

impl RuntimeWorkCapacity {
    pub(super) fn for_logical_cpus(logical_cpus: usize) -> Self {
        // A process may fail to discover CPU capacity; treating that as one
        // CPU is the safe choice because it never starts competing background
        // CPU work beside foreground edits.
        let logical_cpus = logical_cpus.max(1);
        Self {
            foreground_workers: FOREGROUND_RUNTIME_WORKERS,
            background_workers: logical_cpus
                .saturating_sub(FOREGROUND_RUNTIME_WORKERS)
                .min(MAX_BACKGROUND_RUNTIME_WORKERS),
        }
    }

    pub(super) fn available() -> Self {
        let logical_cpus = thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1);
        Self::for_logical_cpus(logical_cpus)
    }

    pub(super) fn foreground_lane(self) -> RuntimeWorkerLane {
        if self.background_workers == 0 {
            RuntimeWorkerLane::ForegroundWithIdleBackground
        } else {
            RuntimeWorkerLane::Foreground
        }
    }
}

#[cfg(test)]
pub(super) type RuntimeWorkTestHook = Arc<dyn Fn(TaskClass) + Send + Sync>;

pub(super) enum RuntimeWorkJob {
    Foreground(ForegroundDocumentJob),
    Semantic(OpenDocumentAnalysisJob),
    Rich(RichSemanticTokensJob),
    Debug(DebugRequestJob),
}

impl RuntimeWorkJob {
    pub(super) fn task(&self) -> &AnalysisTask {
        match self {
            Self::Foreground(job) => &job.task,
            Self::Semantic(job) => &job.task,
            Self::Rich(job) => &job.task,
            Self::Debug(job) => job.task(),
        }
    }

    pub(super) fn scheduled_at(&self) -> Instant {
        match self {
            Self::Foreground(job) => job.scheduled_at,
            Self::Semantic(job) => job.scheduled_at,
            Self::Rich(job) => job.scheduled_at,
            Self::Debug(job) => job.scheduled_at(),
        }
    }

    pub(super) fn due_at(&self) -> Instant {
        // Every job is immediately runnable once admitted. Foreground work
        // still wins through task-class priority and reserved capacity, while
        // latest-wins cancellation suppresses obsolete background results.
        self.scheduled_at()
    }
}

impl RuntimeWorkExecutor {
    pub(super) fn start(sender: mpsc::Sender<ServerEvent>) -> Self {
        Self::start_with_capacity(sender, RuntimeWorkCapacity::available())
    }

    pub(super) fn start_with_capacity(
        sender: mpsc::Sender<ServerEvent>,
        capacity: RuntimeWorkCapacity,
    ) -> Self {
        let scheduler = Self {
            state: Arc::new((Mutex::new(BTreeMap::new()), Condvar::new())),
            sender,
            #[cfg(test)]
            test_before_execute: None,
        };
        for _ in 0..capacity.foreground_workers {
            let worker = scheduler.clone();
            let lane = capacity.foreground_lane();
            thread::spawn(move || worker.run(lane));
        }
        for _ in 0..capacity.background_workers {
            let worker = scheduler.clone();
            thread::spawn(move || worker.run(RuntimeWorkerLane::Background));
        }
        scheduler
    }

    #[cfg(test)]
    pub(super) fn start_with_capacity_and_test_hook(
        sender: mpsc::Sender<ServerEvent>,
        capacity: RuntimeWorkCapacity,
        test_before_execute: RuntimeWorkTestHook,
    ) -> Self {
        let scheduler = Self {
            state: Arc::new((Mutex::new(BTreeMap::new()), Condvar::new())),
            sender,
            test_before_execute: Some(test_before_execute),
        };
        for _ in 0..capacity.foreground_workers {
            let worker = scheduler.clone();
            let lane = capacity.foreground_lane();
            thread::spawn(move || worker.run(lane));
        }
        for _ in 0..capacity.background_workers {
            let worker = scheduler.clone();
            thread::spawn(move || worker.run(RuntimeWorkerLane::Background));
        }
        scheduler
    }

    pub(super) fn schedule(&self, job: OpenDocumentAnalysisJob) {
        self.schedule_work(RuntimeWorkJob::Semantic(job));
    }

    pub(super) fn schedule_foreground(&self, job: ForegroundDocumentJob) {
        self.schedule_work(RuntimeWorkJob::Foreground(job));
    }

    pub(super) fn schedule_rich(&self, job: RichSemanticTokensJob) {
        self.schedule_work(RuntimeWorkJob::Rich(job));
    }

    pub(super) fn schedule_debug(&self, job: DebugRequestJob) {
        self.schedule_work(RuntimeWorkJob::Debug(job));
    }

    pub(super) fn schedule_work(&self, job: RuntimeWorkJob) {
        let (lock, wake) = &*self.state;
        let mut pending = lock.lock().unwrap();
        let key = (
            job.task().identity().class(),
            job.task().identity().uri().to_string(),
        );
        if !pending.contains_key(&key) && pending.len() >= MAX_PENDING_DOCUMENT_ANALYSIS_JOBS {
            // A higher-priority incoming job may displace only equal- or
            // lower-priority queued work. Foreground edits therefore retain a
            // path to their reserved worker while rich/debug work remains
            // best-effort.
            let eviction = pending
                .iter()
                .filter(|((class, _), _)| *class >= key.0)
                .min_by_key(|((class, uri), job)| {
                    (Reverse(*class), job.scheduled_at(), uri.as_str())
                })
                .map(|(key, _)| key.clone());
            if let Some(evicted_key) = eviction {
                let evicted = pending
                    .remove(&evicted_key)
                    .expect("selected pending job exists");
                evicted.task().cancel();
                self.send_skipped(evicted, "scheduler-capacity-evicted");
            } else {
                let reason = match key.0 {
                    TaskClass::Foreground => "scheduler-capacity-dropped-foreground",
                    TaskClass::Semantic => "scheduler-capacity-dropped-semantic",
                    TaskClass::Rich => "scheduler-capacity-dropped-rich",
                };
                self.send_skipped(job, reason);
                return;
            }
        }
        if let Some(previous) = pending.insert(key, job) {
            previous.task().cancel();
            self.send_skipped(previous, "superseded-before-dispatch");
        }
        // Workers serve disjoint lanes. A single wake-up can choose the wrong
        // idle lane, so every newly admitted job wakes both fixed workers.
        wake.notify_all();
    }

    pub(super) fn run(self, lane: RuntimeWorkerLane) {
        let (lock, wake) = &*self.state;
        loop {
            let mut pending = lock.lock().unwrap();
            let key = loop {
                let now = Instant::now();
                if let Some(key) = next_runnable_work_key_for_lane(&pending, now, lane) {
                    break key;
                }
                pending = wake.wait(pending).unwrap();
            };
            let due_at = pending[&key].due_at();
            let now = Instant::now();
            if now < due_at {
                let (pending_after_wait, _) = wake.wait_timeout(pending, due_at - now).unwrap();
                pending = pending_after_wait;
                continue;
            }
            let Some(job) = pending.remove(&key) else {
                continue;
            };
            drop(pending);
            if job.task().is_cancelled() {
                self.send_skipped(job, "cancelled-before-dispatch");
                continue;
            }
            self.execute(job);
        }
    }

    pub(super) fn execute(&self, job: RuntimeWorkJob) {
        #[cfg(test)]
        if let Some(test_before_execute) = &self.test_before_execute {
            test_before_execute(job.task().identity().class());
        }
        match job {
            RuntimeWorkJob::Foreground(job) => {
                let cancelled = || job.task.is_cancelled();
                let Some(positions) =
                    PositionIndex::new_cancellable(job.task.snapshot().text(), Some(&cancelled))
                else {
                    self.send_skipped(
                        RuntimeWorkJob::Foreground(job),
                        "cancelled-during-position-index",
                    );
                    return;
                };
                if job.task.is_cancelled() {
                    self.send_skipped(RuntimeWorkJob::Foreground(job), "cancelled-before-lexical");
                    return;
                }
                let lexer_tokens = lex(job.task.snapshot().text());
                if job.task.is_cancelled() {
                    self.send_skipped(RuntimeWorkJob::Foreground(job), "cancelled-before-syntax");
                    return;
                }
                let syntax = parse_source(job.task.snapshot().text());
                let event = if job.task.is_cancelled() {
                    ServerEvent::ForegroundDocumentSkipped {
                        task: job.task.identity().clone(),
                        reason: "cancelled-during-syntax".to_string(),
                        elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                    }
                } else {
                    ServerEvent::ForegroundDocumentReady {
                        task: job.task.identity().clone(),
                        positions,
                        lexer_tokens,
                        syntax,
                        elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                    }
                };
                let _ = self.sender.send(event);
            }
            RuntimeWorkJob::Semantic(job) => {
                let (analysis, timings) =
                    file_index_for_source_with_timings(job.task.snapshot().text());
                let event = if job.task.is_cancelled() {
                    ServerEvent::DocumentAnalysisSkipped {
                        task: job.task.identity().clone(),
                        reason: "superseded-during-analysis".to_string(),
                        elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                    }
                } else {
                    ServerEvent::DocumentAnalysisReady {
                        task: job.task.identity().clone(),
                        analysis,
                        timings,
                        elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                    }
                };
                let _ = self.sender.send(event);
            }
            RuntimeWorkJob::Rich(job) => {
                let projection =
                    semantic_tokens_for_cached_analysis_with_external_indexes_cancelled(
                        job.task.snapshot().text(),
                        &job.analysis,
                        job.external_snapshot.workspace.as_deref(),
                        job.external_snapshot.game_data.as_deref(),
                        &|| job.task.is_cancelled(),
                    );
                let event = match projection {
                    Some(projection) => ServerEvent::RichSemanticTokensReady {
                        task: job.task.identity().clone(),
                        uri: job.uri,
                        revision: job.revision,
                        external_generation: job.external_generation,
                        external_status: job.external_snapshot.status,
                        projection,
                        elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                    },
                    None => ServerEvent::RichSemanticTokensSkipped {
                        task: job.task.identity().clone(),
                        uri: job.uri,
                        revision: job.revision,
                        external_generation: job.external_generation,
                        reason: "cancelled-during-work".to_string(),
                        elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                    },
                };
                let _ = self.sender.send(event);
            }
            RuntimeWorkJob::Debug(job) => {
                if job.task().is_cancelled() {
                    self.send_skipped(RuntimeWorkJob::Debug(job), "cancelled-before-work");
                    return;
                }
                let event = job.execute();
                let _ = self.sender.send(event);
            }
        }
    }

    pub(super) fn send_skipped(&self, job: RuntimeWorkJob, reason: &str) {
        let event = match job {
            RuntimeWorkJob::Foreground(job) => ServerEvent::ForegroundDocumentSkipped {
                task: job.task.identity().clone(),
                reason: reason.to_string(),
                elapsed_ms: job.scheduled_at.elapsed().as_millis(),
            },
            RuntimeWorkJob::Semantic(job) => ServerEvent::DocumentAnalysisSkipped {
                task: job.task.identity().clone(),
                reason: reason.to_string(),
                elapsed_ms: job.scheduled_at.elapsed().as_millis(),
            },
            RuntimeWorkJob::Rich(job) => ServerEvent::RichSemanticTokensSkipped {
                task: job.task.identity().clone(),
                uri: job.uri,
                revision: job.revision,
                external_generation: job.external_generation,
                reason: reason.to_string(),
                elapsed_ms: job.scheduled_at.elapsed().as_millis(),
            },
            RuntimeWorkJob::Debug(job) => match job {
                DebugRequestJob::Hover(job) => ServerEvent::DebugRequestReady {
                    task: job.task.identity().clone(),
                    id: job.id,
                    method: DEBUG_HOVER_METHOD,
                    uri: job.uri,
                    revision: job.revision,
                    details: format!("skipped reason={reason}"),
                    result: Value::Null,
                    elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                },
                DebugRequestJob::Completion(job) => ServerEvent::DebugRequestReady {
                    task: job.task.identity().clone(),
                    id: job.id,
                    method: DEBUG_COMPLETION_METHOD,
                    uri: job.uri,
                    revision: job.revision,
                    details: format!("skipped reason={reason}"),
                    result: Value::Null,
                    elapsed_ms: job.scheduled_at.elapsed().as_millis(),
                },
            },
        };
        let _ = self.sender.send(event);
    }
}

// Compatibility name for focused scheduler tests. Production only constructs
// `RuntimeWorkExecutor`; the former semantic-only worker no longer exists.
#[cfg(test)]
pub(super) type OpenDocumentAnalysisScheduler = RuntimeWorkExecutor;

#[cfg(test)]
pub(super) fn next_runnable_work_key(
    pending: &BTreeMap<(TaskClass, String), RuntimeWorkJob>,
    now: Instant,
) -> Option<(TaskClass, String)> {
    next_runnable_work_key_for_lane(pending, now, RuntimeWorkerLane::Background)
}

pub(super) fn next_runnable_work_key_for_lane(
    pending: &BTreeMap<(TaskClass, String), RuntimeWorkJob>,
    now: Instant,
    lane: RuntimeWorkerLane,
) -> Option<(TaskClass, String)> {
    let idle_background = lane == RuntimeWorkerLane::ForegroundWithIdleBackground
        && !pending
            .iter()
            .any(|((class, _), job)| *class == TaskClass::Foreground && job.due_at() <= now);
    pending
        .iter()
        .filter(|((class, _), _)| {
            lane.accepts(*class) || (idle_background && *class != TaskClass::Foreground)
        })
        .min_by_key(|((class, uri), job)| {
            // A ready higher-priority class always runs first. Until it is
            // ready, an older lower-priority job may use the idle worker.
            (job.due_at() > now, *class, job.due_at(), uri.as_str())
        })
        .map(|(key, _)| key.clone())
}
