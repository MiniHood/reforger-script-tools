# Workbench compiler-validation research

Research date: 2026-07-23. This journal records the evidence and proposed
architecture for a first Workbench NET API feature: compiler-backed script
validation in the VS Code extension. It is a design/research record, not an
implementation commitment or proof from a live Workbench session.

## Scope and evidence standard

The feature is named **Workbench script validation** in the UI and public
contract. The built-in endpoint is named `ValidateScripts`, but the documented
operation validates an explicit script *configuration*; it is not documented as
"compile this named file".

Confirmed protocol facts come from the engine-owned extracted source at:

- `C:\Users\Gray\AppData\Roaming\Code\User\globalStorage\undefined_publisher.reforger-sript-tools\game-data\scripts\WorkbenchGameCommon\NetApiDocs.c:11-220`; and
- `C:\Users\Gray\AppData\Roaming\Code\User\globalStorage\undefined_publisher.reforger-sript-tools\game-data\scripts\WorkbenchGameCommon\generated\NetApi\NetApiHandler.c:15-35`.

The current official [Workbench NET API reference](https://community.bistudio.com/wikidata/external-data/arma-reforger/EnfusionScriptAPIPublic/Page_NetApi.html)
corroborates the request, response, and built-in endpoint contract. The
official [NetApiHandler reference](https://community.bistudio.com/wikidata/external-data/arma-reforger/EnfusionScriptAPIPublic/interfaceNetApiHandler.html)
corroborates custom-handler dispatch. Live acceptance began against Workbench
`1.7.0.54` on 2026-07-23; only the observations recorded below are promoted
from the **Live-validation gaps** backlog.

## Confirmed built-in contract

### Transport and readiness

NET API is a client-initiated protocol to a running Workbench. Each transaction
uses a new TCP/IP connection and receives one response; requests contain
protocol version, client ID, content type, and a UTF-8 JSON payload
(`NetApiDocs.c:18-65`). The public MCP server owns this private transport
adapter; Workbench is external to the MCP server.

The documented readiness probe is:

```json
{ "APIFunc": "IsWorkbenchRunning" }
```

Its response has both `IsRunning` and `ScriptsCompiled` booleans, where the
latter is documented as whether scripts compiled successfully
(`NetApiDocs.c:84-97`). A reachable socket alone is therefore not enough to
claim that compiler-backed validation is ready. The documented
`IsWorldEditorRunning` response has the same two fields, but World Editor is
not required for `ValidateScripts` (`NetApiDocs.c:98-110`).

### Request shape and scope

The entire documented request is:

```json
{
  "APIFunc": "ValidateScripts",
  "Configuration": "WORKBENCH"
}
```

`Configuration` is required and is described as a script configuration from
project settings, with examples `WORKBENCH`, `PC`, `PLAYSTATION`, and `XBOX`
(`NetApiDocs.c:112-119`). There is no documented file path, resource ID,
document URI, source text, editor-buffer revision, target class, addon filter,
or cancellation parameter.

Therefore these statements are confirmed:

- The built-in operation validates a selected configuration, not a named file.
- One request can report diagnostics for multiple files in that configuration.
- The host must require an explicit configuration value; it must not silently
  invent a target configuration.

It does **not** follow that every file in every configuration is compiled, or
that a successful result means a particular active VS Code document was
included. Those are live-validation questions.

### Response shape

The endpoint returns a JSON payload with `Errors`, `Warnings`, and `Success`
(`NetApiDocs.c:120-153`). Each documented diagnostic object has:

| Field | Confirmed meaning |
| --- | --- |
| `error` | Human-readable compiler error or warning text. |
| `file` | Resource-relative source path. |
| `fileAbs` | Optional absolute path, documented as present for unpacked scripts. |
| `addon` | Optional addon name, documented as present for unpacked scripts. |
| `line` | Source line location. The documentation does not state whether numbering is zero- or one-based. |

NET API also frames every response with an error-code string separate from the
endpoint-specific JSON payload (`NetApiDocs.c:156-174`). The MCP host must
treat a transport/protocol error and a completed validation containing compiler
errors as different outcomes. `Success` is documented as a field, but its exact
relationship to errors and warnings needs a live fixture; do not infer it from
the single example response.

### Initial live response-envelope evidence

Workbench `1.7.0.54` listened on TCP port `5775` and accepted the documented
transaction through the configured `127.0.0.1` loopback endpoint. A byte-level
`IsWorkbenchRunning` request returned the response error-code string `Ok` and
the payload:

```json
{"IsRunning":true,"ScriptsCompiled":false}
```

This establishes `Ok` as the successful response-envelope code for the
supported live Workbench version. An empty error-code string is not the
successful sentinel. The payload also confirms that a reachable, running
Workbench can report scripts as not compiled. The following validation proved
that `ScriptsCompiled: false` also represents a completed compiler failure, so
it must not be used to present the Workbench API as unavailable or still
launching.

A live `ValidateScripts` request for `WORKBENCH` used the same `Ok` envelope
code and returned four compiler errors, an empty `Warnings` array, and
`Success: false`. Two unpacked-addon diagnostics included `file`, `fileAbs`,
`addon`, and line values `12` and `138`; both values identified the exact
declaration on the corresponding one-based source line. This establishes the
one-based line mapping and confirms that compiler findings are a successful
NET API transaction even when validation `Success` is false. Clean validation,
warning behavior, packed-resource locations, and cross-addon projection still
require live fixtures.

### Live acceptance refinement

Once a status transaction succeeds, the UI presents the Workbench API as
connected and keeps validation available. It presents `ScriptsCompiled`
separately as the last Workbench-reported compiler state. Compiler findings are
rendered in Problems and in a dedicated latest-result output with clickable
project-contained source locations and explanatory messages.

## What this cannot validate

The built-in request cannot carry the unsaved text of a VS Code document. The
complete documented parameter list contains only `APIFunc` and `Configuration`.
Consequently, an external extension cannot ask this built-in endpoint to
compile an arbitrary unsaved VS Code buffer. It can only receive validation of
the source state available to Workbench at the time of the request.

For a normal file-backed workspace, that means a dirty VS Code buffer must be
saved before its changes can be expected to affect a Workbench validation. This
is an inference from the missing buffer-content/revision parameter and the
separate-process architecture, not a claim that Workbench never has its own
unsaved editor buffers. A custom handler could be designed to accept text, but
it would not make the built-in compiler validate that text unless the handler
has a proven engine-supported in-memory compilation route. No such route was
found in the reviewed source or official NET API reference.

Likewise, no documented request cancellation, cancellation token, progress
event, queue status, or request correlation field exists. Closing an external
TCP connection is not a documented cancellation mechanism and must not be used
as one until a live experiment establishes the outcome.

## Proposed extension behavior

The goal is continuous *authoritative* compiler feedback without presenting
diagnostics for an older saved snapshot as though they describe current text.
This complements, rather than replaces, immediate Rust language-engine
diagnostics for dirty buffers.

### Status-bar lifecycle

After game-data acquisition and the language engine's initial external index
are ready, activate an optional Workbench connection controller. A persistent
lower-right status-bar item communicates availability; a progress notification
is reserved for bounded work.

| State | Suggested text | Compiler validation availability |
| --- | --- | --- |
| Index not ready | No Workbench state shown yet. | Not started. |
| Discovering | `$(sync~spin) Looking for Reforger Workbench...` | Unavailable. |
| Connected | `$(plug) Workbench API connected` | `ValidateScripts` available; compilation state is shown separately. |
| Validating | `$(sync~spin) Validating scripts in Workbench...` | One validation in flight. |
| Lost | `$(circle-slash) Reforger Workbench unavailable - retrying` | Unavailable; file and Rust features remain available. |

Probe immediately once the index-ready signal arrives, then every second while
unavailable. On a healthy connection, use a five-second `IsWorkbenchRunning`
heartbeat. A successfully decoded status response establishes that the
configured API is connected. `ScriptsCompiled` remains visible compiler state,
not an availability gate. The first compiler feature is built-in and does not
depend on the future custom plugin or its `capabilities` manifest.

### Commands and automatic modes

Expose a manual command, **Reforger: Validate Scripts in Workbench**, with an
optional keybinding. It requests an explicitly selected configuration and is
always available when the connection is ready.

Automatic validation should be a user-controlled policy, separate from
connection health. A future setting may use these modes:

| Mode | Trigger | Required behavior |
| --- | --- | --- |
| `manual` | Command/keybinding only. | Never starts validation automatically. |
| `onSave` | A supported Enforce Script document is saved. | Debounce/coalesce then validate the chosen configuration. |
| `onTypeIdle` | A supported dirty document remains idle for the configured interval. | Explicitly saves the document first; validate only after a successful save. This mode must make its save behavior clear to the user. |

`onTypeIdle` is the appropriate answer to a user who wants feedback after each
meaningful edit burst rather than only at an unpredictable manual save. It is
not "compile on every line": a line/cursor event has no new disk state by
itself, and it produces avoidable transient diagnostics. A default idle delay
of 750 ms is a starting design value, not an API fact; make it configurable and
validate its cost in live Workbench sessions. Users who already use VS Code
auto-save can use `onSave` and get the same saved-idle behavior without the
extension initiating saves.

For either automatic mode, the scheduler must be single-flight:

```text
saved eligible document
  -> reset short debounce timer
  -> if no validation runs, validate selected configuration
  -> if validation runs, retain only one latest pending run
  -> after response, run the retained latest request if needed
```

No save or validation should block the VS Code save operation. Do not validate
on extension activation, every successful health probe/reconnect, every cursor
move, or every text change before a save.

### Diagnostics and stale-result policy

Map completed Workbench diagnostics into a dedicated VS Code diagnostic
collection labelled `Workbench`; never merge them silently with the Rust
language-engine collection. Preserve diagnostic severity, `file`, optional
`fileAbs`, optional `addon`, line, selected configuration, request time, and
the source authority (`workbench-compiler`) in the host result model.

Resolve locations in this order only after proving the mapping against fixture
projects: validated canonical `fileAbs`; project/addon-aware resolution of
`file`; otherwise an unlocated diagnostic associated with the validation
result. Do not guess a workspace path from a relative string. The line base,
column availability, packed-resource path mapping, and multi-root/addon
selection are all live-validation gaps.

Capture the VS Code document version and/or saved file fingerprint when a
validation is scheduled. If an affected document has changed or become dirty
before the response is applied, mark the result stale and schedule the one
latest validation rather than claiming it describes the new text. Because
`ValidateScripts` is configuration-wide and has no source-revision token, this
is a host-side best-effort freshness rule, not a server guarantee.

## Proposed host boundary

```text
VS Code save or explicit command
  -> compiler-validation scheduler (mode, debounce, coalescing, freshness)
  -> Workbench connection controller (ready state)
  -> private NET API adapter
  -> ValidateScripts { Configuration }
  -> Workbench diagnostic normalizer
  -> dedicated Workbench VS Code diagnostic collection
```

The extension host owns scheduling, user settings, status-bar presentation,
timeouts, connection retries, stale-result suppression, path mapping, and VS
Code diagnostics. The NET API adapter owns only framing and typed requests.
Workbench owns the compiler and its source/configuration truth. The Rust
language engine remains the immediate source of unsaved-buffer diagnostics.

Do not add a generic `call_workbench_api` path: `ValidateScripts` is a named,
typed built-in operation with a dedicated request and result mapper.

## Live-validation gaps and acceptance experiments

Before implementation is considered complete, run and record these experiments
against every supported Workbench/Reforger version:

1. Enable NET API through the documented Workbench control and make an
   `IsWorkbenchRunning` byte-level probe. Record endpoint discovery method,
   healthy latency, `IsRunning`/`ScriptsCompiled` transitions, connection-close
   behavior, and sanitized errors.
2. Call `ValidateScripts` for each relevant configuration on a clean fixture,
   then add a deliberate error and warning. Record the response envelope,
   `Success` semantics, exact `Errors`/`Warnings` shape, line-number base, and
   whether both arrays may be absent or empty.
3. Put errors in different addons and packed/unpacked locations. Verify
   `file`, `fileAbs`, `addon`, duplicate path handling, and conversion to
   VS Code URIs.
4. Edit a file in VS Code without saving, invoke validation, then save and
   invoke it again. Confirm precisely which source state Workbench sees. Also
   test a document with VS Code auto-save and, separately, an extension-initiated
   save if `onTypeIdle` is implemented.
5. Issue overlapping validations and disconnect the client while one is in
   flight. Measure serialization/concurrency, timeout behavior, whether
   closing the connection cancels or merely abandons the response, and any
   Workbench-side side effects. Do not ship cancellation semantics before this
   experiment.
6. Request validation while `ScriptsCompiled` is false and during Workbench
   startup. Establish whether the readiness gate is necessary, sufficient, or
   needs a user-visible retry state.
7. Measure normal and large-project latency and resource cost. Choose default
   idle delay, timeout, and automatic-mode defaults from those measurements,
   rather than assuming a whole-configuration validation is cheap.

## Decision record

- Build the first NET API feature around the stock `ValidateScripts` endpoint;
  no custom plugin is required for its initial transport contract.
- Publicly call it Workbench script validation, because the only proven target
  is a selected configuration, not a file-level compiler invocation.
- Support continuous feedback through saved-idle validation and immediate Rust
  feedback for unsaved text; never claim external Workbench validation covers a
  dirty buffer.
- Treat availability, validation execution, compiler failure, stale results,
  and transport failure as distinct states with distinct UI/diagnostic behavior.
- Keep all unproven behavior - endpoint discovery, diagnostics conventions,
  performance, concurrency, cancellation, and exact source snapshot - behind
  the live-validation backlog.

## Accepted initial implementation contract (2026-07-23)

This section supersedes the exploratory automatic-mode, endpoint-discovery,
and stale-result proposals above. It records the agreed design; the
live-validation gaps remain acceptance requirements rather than established
Workbench facts.

### Boundary and initial capabilities

`src/workbenchGateway/` is a host-neutral TypeScript module with no `vscode`
imports. It owns the NET API codec, one-transaction transport, typed outcomes,
per-capability internal deadlines, Workbench Availability State, and exactly
two initial named capabilities: `getStatus()` and
`validateScripts(profile)`. It exposes no generic endpoint or handler
invocation.

`src/workbenchCompiler/` is the VS Code adapter. It owns settings, the status
item, document saves, Continuous Compiler Validation, diagnostic location
projection, and the Workbench Compiler Diagnostic Collection. A future MCP
host consumes the Gateway rather than reimplementing NET API.

### Endpoint and workspace contract

The Gateway contacts only the extension-owned configured loopback endpoint.
`reforgerScriptTools.workbenchNetApi.enabled` defaults to `true` and disables
all NET API traffic when false. The configurable host is loopback-only and
defaults to `127.0.0.1`; the configurable port defaults to `5775`. The
extension does not discover, scan, change, or repair this endpoint. Status and
validation requests contact only that configured endpoint.

The initial supported workspace is one Addon Workspace: the Reforger addon
project folder opened in VS Code. Multi-root selection and files outside that
folder are unsupported. The stock built-ins do not establish the identity of
the active Workbench project, so the UI must state that Workbench validates its
currently open project rather than claiming that linkage was independently
verified.

### Validation controls and scheduling

The extension exposes these user-facing validation controls:

| Setting | Default | Contract |
| --- | --- | --- |
| `reforgerScriptTools.workbenchCompilerValidationDelay` | `3` | A positive whole-second idle delay enables Continuous Compiler Validation; `0` is manual-only. |
| `reforgerScriptTools.workbenchCompilerValidationProfile` | `WORKBENCH` | A constrained profile setting. `WORKBENCH` is the only initially verified allowed value. |

An explicit **Reforger: Validate Scripts in Workbench** command remains
available when the Gateway is ready. With a positive delay, a save or an idle
pause on the active Enforce Script document schedules one validation. Idle
validation saves only that active document; it never saves every dirty tab.
If saving fails, it does not validate, preserves prior Workbench evidence as
stale, and reports a `save-failed` outcome. One validation may run at a time;
later triggers coalesce into one follow-up validation after the latest delay.
Changing any Gateway or validation setting applies immediately and supersedes
queued work without a reload.

### Diagnostics, status, and observability

Workbench Compiler Diagnostics are separate from Provisional Parser
Diagnostics. The extension atomically replaces the complete selected-profile
Workbench Compiler Diagnostic Collection after a successful validation,
including clearing it after a clean result. A failed or unavailable validation
does not clear the preceding set.

If source changes after a validation is scheduled, or Workbench becomes
unavailable, the preceding Workbench diagnostics remain visible as **stale**
evidence. Their source/message and the Workbench Status Item identify the
prior-snapshot or unavailable state; the next fresh result replaces them.
The Gateway remains host-neutral by returning Workbench resource identities and
paths. Only the extension maps a location to a VS Code URI after proving it is
inside the Addon Workspace; unresolvable locations are structured result/log
evidence, never guessed workspace files.

The dedicated latest-result output summarizes the validation and renders
project-contained locations as clickable `path:line:column` entries with
severity and compiler messages. Unmapped findings remain visible with an
explicit mapping explanation.

One lower-right Workbench Status Item reports disabled, connecting, API
connected, validating, or unavailable/retrying. It exposes the endpoint,
profile, Workbench-reported compilation state, last validation outcome/time,
and sanitized failure category in its tooltip; clicking it runs the explicit
validation command and reveals the result output. There are no recurring
connection-loss notifications.

Gateway outcomes use stable typed categories such as `unavailable`, `timeout`,
`protocol`, `unsupported`, and `workbench-error`, each with a recovery hint.
The existing centralized extension diagnostics log records Gateway state
transitions and outcomes with category and elapsed time, but never payloads,
source text, endpoint addresses, or raw transport errors.

### Required live acceptance

Before this integration is complete, verify the configured `127.0.0.1:5775`
endpoint with NET API enabled in a live Workbench session: `getStatus` framing
and readiness; clean validation clearing the collection; a deliberate saved
compiler error at the correct VS Code file/line; a stale result followed by a
fresh replacement; and disabled NET API, wrong-endpoint, and save-failure
outcomes that preserve useful evidence. Unit tests prove the Gateway and VS
Code adapter contracts but do not replace this acceptance evidence.
