# Reforger Script Tools

This glossary records the durable language used for the language engine's LSP
runtime boundaries.

## LSP Runtime

**Document Runtime**:
The owner of open-document snapshots, their analysis lifecycle, and the
admission of document-backed LSP queries.
_Avoid_: document coordinator, document state

**Document Query**:
A request-local view that captures an open-document snapshot together with the
external-index snapshot that the request may use.
_Avoid_: request context, analysis context

**External Index Snapshot**:
The immutable workspace and game-data index generation captured for one
document query or scheduled analysis job.
_Avoid_: current index, global index

**Runtime Effect**:
An observable action emitted by the Document Runtime for the LSP composition
root to deliver, such as a response, notification, refresh request, or log.
_Avoid_: callback, side effect

## Enfusion Preprocessing

**Preprocessor Directive**:
One of the supported `#`-initiated source forms: `#define`, `#ifdef`,
`#ifndef`, `#else`, or `#endif`. Its `#` is the first non-whitespace source
token on a physical line, at any indentation depth; completion offers every
supported directive without structural filtering.
_Avoid_: hashtag command, preprocessor keyword

**Macro**:
The identifier declared by `#define` and consumed as the condition operand of
`#ifdef` or `#ifndef`. Macro completion draws from every indexed current,
workspace, and game-data macro; commented-out directive text is excluded.
_Avoid_: preprocessor variable, define variable

**Experimental Auto Formatting**:
The default-on user control for every automatic source edit made by the
extension, including typing assists and directive separators. A setting change
applies to the next automatic edit without restarting the language server. It
is exposed as `reforgerScriptTools.experimentalAutoFormatting`.
_Avoid_: preprocessor formatting, individual auto-edit switches

**Pre-Native Input Route**:
A route for an editor operation, such as inserting a newline, text, or an
indent, that decides its outcome before VS Code performs the native edit. A
keybinding, command, completion acceptance, or another editor action may
request the same operation. The route either applies one atomic editor action, such as
creating a braced control body and placing the caret inside it, or delegates
unchanged to the native behavior. It must never simulate an auto-format action
by allowing a visible native edit and subsequently undoing or correcting it.
Every gesture remains native unless a declared feature claims its exact
situation; every feature uses this same contract regardless of the gesture it
handles.
_Avoid_: post-input formatter, keystroke interceptor

**Input Feature Owner**:
The one input feature selected to handle an editor operation in its current
document and selection state. Ownership is exclusive: it returns one atomic
result or declines, after which native behavior is used. Input features do not
compose or apply sequential corrections to the same operation.
When multiple features could claim an operation, Rust selects an explicit,
fixed priority; tests must reject an unresolved tie.
_Avoid_: input pipeline stage, formatter chain

**Input Routing Decision**:
Rust's versioned decision for one editor operation, using the current document
and editor context. It returns the Input Feature Owner's complete atomic edit
and resulting selection, or declines so the TypeScript editor shell performs
the native operation unchanged.
_Avoid_: TypeScript typing rule, client-side formatter decision

**Atomic Input Result**:
The initial handled result of an Input Routing Decision: one set of document
text edits and the final selections, applied together as one undoable editor
operation. A Rust-authored snippet replacement is also permitted when VS Code
must retain a blank structural line and its caret as one operation. It may
declaratively request that the suggestion UI opens afterwards. The protocol
permits no other arbitrary VS Code command, delayed follow-up action, or edit
sequence.
_Avoid_: command result, chained editor mutation

**Atomic Switch Arm**:
The `switch` control-header result embeds `default:` in its primary atomic text
edit and returns a final selection range covering `default`. It may request the
suggestion UI, but it must not insert a second snippet after applying the
primary edit.
_Avoid_: follow-up switch snippet, default post-edit

**Single-Caret Eligibility**:
The initial eligibility boundary for an Input Feature Owner: exactly one empty
caret. Requests still represent all selections, but a non-empty selection or
multiple carets must decline to native editor behavior until a feature defines
its correct multi-selection semantics.
_Avoid_: partial multi-cursor support, best-effort selection edit

**Operation Entry Command**:
The one extension command for a supported semantic editor operation. Its
keybinding, completion flow, and any future extension action all invoke this
same command, which obtains an Input Routing Decision and atomically applies
it or delegates to native VS Code behavior.
_Avoid_: key-specific formatter command, duplicated input path

**Operation Vocabulary**:
The closed, versioned set of semantic editor operations the router supports,
beginning with `insertNewline` and growing through explicit entries such as
`insertText`, `indent`, or `acceptSuggestion`. Adding one requires its
canonical Native Fallback and tests; Rust may not request a free-form VS Code
command name.
_Avoid_: raw command protocol, arbitrary operation string

**Keybinding Ownership Boundary**:
VS Code's keybinding resolution remains authoritative. The router owns a
semantic operation only when an Operation Entry Command is invoked; user
remappings, platform-reserved input, and IME composition may deliberately
bypass it.
_Avoid_: universal key capture, router key precedence

**Native Fallback**:
The canonical VS Code operation directly invoked when an Input Routing Decision
declines. It is not a redispatch of the originating keybinding, so fallback
cannot re-enter the router or duplicate the input.
_Avoid_: synthetic keypress, routed fallback

**Decision Availability Rule**:
A plausible input route waits for Rust's decision without an arbitrary response
deadline. If the request fails or the language server is unavailable, the
Operation Entry Command immediately uses Native Fallback and ignores any late
result.
_Avoid_: formatting timeout, delayed correction

**Native Editing Mode**:
An editor state with its own input semantics, initially including an open
completion list, an active snippet placeholder, IME or composition input, and
a read-only editor. Enter with an open completion list is the one current
exception: the entry command hides the list without accepting its selection,
then routes the newline normally. The router uses Native Fallback in the other
modes unless a future Input Feature Owner explicitly defines compatible
behavior.
_Avoid_: generally safe typing context, route exception

**Stale Input Decision**:
An Input Routing Decision whose document version or selection snapshot no
longer matches the editor when its response arrives. It is discarded without
editing; the operation instead uses Native Fallback at the current editor
state.
_Avoid_: late auto-format result, best-effort route response

**Auto-Formatting Gate**:
The application of `Experimental Auto Formatting` to input routing. When the
setting is disabled, every auto-format Input Feature Owner declines and the
Operation Entry Command uses Native Fallback. Routing remains available for
future non-formatting features.
_Avoid_: per-feature formatting setting, router off switch

**Input Route Trace**:
An optional, centralized diagnostic record for an input operation. It contains
only the operation kind, eligibility or decline reason, selected Input Feature
Owner, document-version match, outcome, and elapsed time; it never contains
source text or identifiers. It is disabled in ordinary use.
_Avoid_: source logging, per-feature console log

**Input Route Migration Boundary**:
The initial router migrates established automatic edits: control-header Enter
auto-blocks and the narrow automatic-semicolon case run as pre-native atomic
edits. Block-comment expansion is a future text-insert feature. Legacy
post-edit incomplete-`if` repair is retired rather than preserved as a silent
default. Completion and snippet transactions and native language configuration
remain outside the router.
_Avoid_: migrate every existing typing assist, input-router completion layer

**Single Input Mutation Path**:
After an automatic source edit is migrated to an Input Feature Owner, no
`onDidChangeTextDocument` listener may mutate the document to reproduce that
behavior. Observers may trace or clean up state only. This rule prevents a
native edit followed by a visible correction from reappearing.
_Avoid_: post-input repair, duplicate typing assist

**Input Route Acceptance Checks**:
Required evidence for an input-route delivery: Rust decision tests for supported
and declined source shapes; TypeScript fallback, failure, stale-state,
special-mode, selection, and presentation tests; an extension-host test proving
the legacy post-edit path does not run; and a manual VS Code check showing no
visible correction or cursor flash.
_Avoid_: unit-tests-only verification, visual polish assumption

**Auto-Block Control Header**:
A newly authored `for`, `foreach`, `while`, or `switch` header whose typing
assist creates a braced body when Enter is pressed from inside the parentheses
or immediately after them, regardless of whether the header contents are
complete. `if` and `else if` never receive automatic braces; Enter inside a
completed paired condition preserves the header and places one indented,
unbraced body line below it. `else` is not a header.
The assist preserves the header exactly and only adds the body below it.
_Avoid_: control-flow snippet, automatic braces

**Control Header Completion**:
The accepted keyword completion that creates the paired parentheses of an
`if`, `for`, `foreach`, `while`, or `switch` header and places the caret
inside. An `if` following `else` is the same `if` completion, not a separate
`else if` form. `for` completion does not add clause separators or
placeholders.
_Avoid_: control-flow snippet, expression auto-completion

**Switch Arm Placeholder**:
The initially selected `default` label in an auto-created switch body. Tab
keeps the default arm; typing replaces it with a complete alternative such as
`case value` while preserving the colon and body indentation. Selecting its
`case` completion creates a value slot and opens value completion there.
_Avoid_: default snippet, case placeholder
