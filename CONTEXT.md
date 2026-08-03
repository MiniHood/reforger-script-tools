# Reforger Script Tools

This glossary records the durable language used for the language engine's LSP
runtime boundaries.

## MCP Evidence

**Official Wiki Corpus**:
The local packaged copy of official Arma Reforger documentation used as
documentary evidence by Reforger Script Tools. It is distinct from extracted
game source and never refers to Wikidata.org.
_Avoid_: Wikidata, wiki data, live wiki

**MCP Runtime**:
The independently launched Rust process that serves one MCP client connection
while reusing the language engine's modules and validated game-data cache. It
does not depend on or attach to the editor-owned LSP process.
_Avoid_: LSP proxy, extension-hosted MCP, shared LSP session

**MCP API Reference**:
The exact human-readable projection of the MCP Runtime's public tool catalogue,
including schemas, effects, limits, errors, examples, and intended tool
handoffs.
_Avoid_: manually maintained tool index, API-index tool, approximate tool list

**Game Data Symbol Search**:
A semantic query over extracted Reforger game-data declarations and their
language identities, kinds, signatures, relationships, and source locations.
_Avoid_: game-data text search, file grep, wiki search

**Game Data Catalogue Revision**:
The immutable fingerprint identifying the exact extracted game-data source and
semantic catalogue used by one MCP Runtime. It binds searches, Symbol
References, inspections, and source reads to the same evidence generation.
_Avoid_: current index, live game version, mutable catalogue

**Game Data Symbol Inspection**:
A focused structured view of one Game Data Symbol Search result, exposing the
semantic facts already owned by the language index without expanding every
search hit into a full symbol record.
_Avoid_: full index dump, verbose search result, source-text inference

**Game Data Example Search**:
A topic-oriented query over bounded extracted Game Data examples. It keeps
generated declarations and handwritten implementation evidence explicitly
classified, reports matching symbols and terms, and hands an exact logical
source range to Game Data Source Read.
_Avoid_: fuzzy symbol search, unbounded source search, example as runtime proof

**Game Data Member Discovery**:
The paginated direct-member view of one revision-bound Symbol Reference. It
completes inspection when the compact preview is truncated and supports
semantic-kind filters without requiring the caller to know a member name.
_Avoid_: owner text search, inherited completion list, full type dump

**Game Data Relationship Query**:
A bounded query for language-engine-proven inheritance, override,
implementation, declaration-reference, and caller relationships around one
revision-bound Symbol Reference. Textual coincidences and unresolved calls are
not relationships.
_Avoid_: grep references, guessed call graph, generic graph query

**Symbol Reference**:
An opaque, revision-bound logical identity returned by Game Data Symbol Search
and accepted by Game Data Symbol Inspection. It identifies one declaration
without exposing a physical path or unstable in-memory symbol ID.
_Avoid_: global symbol ID, physical symbol path, permanent symbol ID

**Game Data Source Read**:
A bounded verbatim passage from extracted game source identified by a logical
path, exact line range, and game-data catalogue revision. It does not require a
Symbol Reference, allowing an agent to continue through a known source file.
_Avoid_: arbitrary file read, physical storage path, unbounded source dump

**Official Wiki Search**:
A lexical query over titles, headings, paths, and passages in the Official Wiki
Corpus, retaining exact document ranges and canonical source URLs.
_Avoid_: symbol search, federated search, Wikidata search

**Official Wiki Corpus Revision**:
The deterministic fingerprint of the packaged Markdown pages used to bind an
Official Wiki Search cursor or source read to the exact corpus generation that
produced it. It is derived from the authoritative files, not a manifest or
index.
_Avoid_: wiki index version, live wiki revision, mutable corpus

## Workbench Integration

**Addon Workspace**:
The single Reforger addon-project folder opened as the supported VS Code
workspace for Workbench compiler validation. General file workspaces and
multi-root project selection are outside the initial validation contract.
_Avoid_: arbitrary workspace, project picker, multi-root target

**Configured Workbench Endpoint**:
The explicit NET API loopback endpoint selected in extension configuration,
with host defaulting to `127.0.0.1` and port to `5775`. The host is restricted
to loopback values; the Gateway contacts only this endpoint and never
discovers, scans, or changes it.
_Avoid_: endpoint discovery, port scan, self-healing connection

**Workbench NET API Enablement**:
The extension-owned master control and sole durable approval for Workbench
integration. When enabled, it authorizes managed bridge installation and all
Gateway status and capability calls. When disabled, it authorizes no bridge
write, performs no NET API traffic, and any retained Workbench Compiler
Diagnostics are stale evidence.
_Avoid_: compiler-delay opt-out, Workbench option, connection preference

**Workbench Status Item**:
The single extension status-bar item that reports Workbench Availability State,
current validation activity, Workbench-reported compilation state, and
Workbench Compiler Diagnostic freshness. Its tooltip exposes configuration and
the last sanitized outcome. Selecting it while disabled requests the existing
Workbench Integration enablement prompt; selecting it in every enabled state
requests compiler validation. It does not use recurring connection-loss
notifications.
_Avoid_: connection popup, compiler progress notification, NET API console

**Live Gateway Configuration**:
The immediate application of Workbench NET API enablement and endpoint setting
changes. A changed setting supersedes queued work;
existing compiler evidence remains stale until a result under the new
configuration succeeds.
_Avoid_: reload-only setting, deferred reconfiguration, mixed configuration

**Workbench Gateway**:
The host-neutral, authoritative boundary for the private Workbench NET API. It
offers only named, typed Workbench operations. Rust owns its codec; MCP calls it
directly and the TypeScript compiler integration reaches it through the
packaged executable's private process mode.
_Avoid_: NET API client, Workbench bridge, MCP transport

**Managed Workbench Handler Package**:
The versioned Reforger Script Tools scripts and ownership manifest under the
current user's Workbench profile. First installation requires the one-time
VS Code extension prompt after a successful Workbench connection. The manifest
authorizes later MCP or connection-time repair or upgrade of only its listed
files; unknown files and newer package versions are preserved.
_Avoid_: project plugin, arbitrary profile scripts, silent first install

**Workbench Lifecycle Support Log**:
The bounded, always-on local integration log containing sanitized Workbench
operation and outcome records. It excludes source and NET API payloads and is
read through the bounded Workbench log tool when diagnosing a report.
_Avoid_: payload trace, compiler output, console dump

**Workbench Capability**:
A named, versioned Workbench operation with a typed request and result,
availability state, and declared effect classification. It is not a raw NET
API endpoint exposed for arbitrary dispatch.
_Avoid_: API function, handler name, generic command

**Workbench Availability State**:
The durable, observable Gateway assessment of whether the configured Workbench
API is disabled, unavailable, or connected. It is derived from short NET API
transactions and is not a claim that a TCP connection remains open.
_Avoid_: socket state, permanent connection, client session

**Workbench Gateway Diagnostic Record**:
A centralized, sanitized extension diagnostic-log record for a Gateway state
transition or named capability outcome. It carries stable categories and timing
metadata but never NET API payloads, source text, or raw transport errors.
_Avoid_: payload log, socket dump, per-retry console output

**Workbench Compiler Diagnostic**:
A diagnostic reported by a completed Workbench script validation for its saved
configuration snapshot. It is compiler truth for that snapshot and remains
distinct from the extension's provisional parser analysis.
_Avoid_: parser error, live-buffer compiler error, extension diagnostic

**Workbench Compiler Diagnostic Collection**:
The extension-owned VS Code diagnostic collection that renders Workbench
Compiler Diagnostics. Workbench supplies a source line but no column. The
extension underlines a uniquely named subject from the compiler message when
that exact subject occurs on the saved source line; otherwise it underlines
the line's non-whitespace content. A missing-semicolon broken-expression
diagnostic underlines only the reported line's non-whitespace content. The
nearest preceding non-blank source line is attached as separate related
information, preserving recovery context without underlining intervening
indentation, blank lines, or newlines. It is independent of the language
server's provisional diagnostic publication.
_Avoid_: LSP compiler diagnostics, shared diagnostic collection, Rust output

**Workbench Compiler Output**:
The user-facing current-operation or latest-result output. Once a validation
request has been dispatched, a timestamped one-line waiting state replaces the
previous result until Workbench returns; a failed request replaces that state
with a terminal no-result message. A completed result's compact first line
begins with a bracketed local 24-hour completion timestamp, then reports
Workbench request duration, project error/warning counts, and the count of
hidden non-project findings. The next line explicitly reports Workbench's
successful or failed validation outcome. It then lists only project-contained
findings as clickable
source locations with severity and compiler messages. The displayed location
is workspace-relative, never absolute, and the complete location-plus-message
after the severity is one link to the projected diagnostic range. Activating
that link opens the source in preview, selects the same range used by the
Workbench Compiler Diagnostic, places the active cursor at the range start,
and reveals it in the editor. Detailed timing remains in the sanitized
extension diagnostic log rather than this user-focused output. Unmapped
finding details stay out of the output.
_Avoid_: payload log, compiler history, raw NET API response

**Provisional Parser Diagnostic**:
A Rust language-engine diagnostic derived from the editor's current document
snapshot. It remains useful for unsaved editing but is not a claim of
Workbench compiler truth.
_Avoid_: compiler diagnostic, validation error

**Continuous Compiler Validation**:
The default-on scheduling of Workbench script validation once after the first
successful Workbench connection in an extension session, immediately after a
save, or after a fixed three-second idle pause that first saves the changed
script. It is separate from the
language engine's continuous parsing and is single-flight: later edits coalesce
into one follow-up run rather than overlapping compiler requests.
_Avoid_: auto-parsing, live compiler, background build

**Validation Save Target**:
The active Enforce Script document whose idle edit triggers Continuous Compiler
Validation. Other dirty script documents are not implicitly saved and retain
only their Provisional Parser Diagnostics until separately saved or visited.
_Avoid_: workspace auto-save, save-all validation, dirty-project snapshot

**Validation Profile**:
The internal Workbench compilation configuration used for every Workbench
Compiler Validation request. It is fixed to the only supported, verified value:
`WORKBENCH`.
_Avoid_: user-selectable compiler mode, target guess, endpoint parameter

**Stale Workbench Compiler Diagnostic**:
A Workbench Compiler Diagnostic from a prior saved snapshot when a newer
relevant script change has been made, or when Workbench becomes unavailable.
It remains visible as useful compiler evidence but must identify its stale
status until replaced by a fresh result.
_Avoid_: current compiler error, discarded validation, parser diagnostic

**Workbench Compiler Diagnostic Set**:
The complete compiler-diagnostic result of one successful Validation Profile
run. Records with the same message and source identity/location normalize to
one diagnostic before publication even when Workbench returns both error and
warning copies; error severity wins. Different messages at that location
remain distinct. The extension replaces the profile's displayed set atomically;
an unsuccessful run retains the preceding set only as stale evidence.
_Avoid_: incremental compiler output, merged run history, partial refresh

## LSP Runtime

**Reforger Semantic Palette**:
The default Enforce-only semantic-token foreground palette. It overlays the
user's selected VS Code theme for Enforce source without selecting, replacing,
or otherwise changing that theme. A user may opt out or override any palette
entry through VS Code's native semantic-token color customization. Semantic
highlighting is enabled for Enforce by default, while an explicit global or
Enforce-specific VS Code preference to disable it takes precedence. Its
definition is authoritative; no selectable full Reforger color theme exists.
The authoritative foreground rules are one native VS Code semantic-token
customization block; the language engine owns classifications but no colors.
VS Code does not apply editor semantic-token styling inside Markdown hovers, so
the hover bridge resolves that same effective customization block and applies
its foregrounds to Rust-authored semantic-role markers. The bridge never owns
a second palette. The hover debug report records both the resolved client
foregrounds and the server classifications/markup.
The shipped colors preserve the established dark-oriented palette. A light
theme may select theme-specific Reforger Semantic Palette Overrides; an
official light palette requires its own deliberate design and contrast
validation. Palette entries set foreground color only; bold, italic,
underline, strikethrough, and other font presentation remain owned by the
selected theme and user overrides.
_Avoid_: Reforger color theme, enforced workbench theme, editor decoration,
duplicate palette

**Reforger Semantic Palette Override**:
A user-authored Enforce-qualified semantic-token rule in VS Code's native
`editor.semanticTokenColorCustomizations` setting. It replaces or augments one
Reforger Semantic Palette entry without introducing extension-owned color
settings or affecting another language.
_Avoid_: Reforger color preference, palette-settings UI, decoration override

**Function Palette Role**:
The single Reforger Semantic Palette role for both global functions and class
methods. `function:enforce` controls their shared foreground color. The
language model may retain the structural distinction required for membership,
inheritance, overriding, lookup, and other language behavior; that distinction
does not create a second color role.
_Avoid_: method color, `method:enforce`, callable palette split

**Reforger Semantic Token Type**:
A semantic classification emitted for Enforce source that has no adequate VS
Code standard token type, or where the standard type would replace established
Reforger vocabulary at the public customization boundary. Each is namespaced,
formally registered with a standard semantic supertype, and independently
addressable by a Reforger Semantic Palette Override. The initial types are
fields, punctuation, and preprocessor text. A field retains that Reforger name
while inheriting standard property styling as its fallback. Their public
selectors are `reforgerField:enforce`, `reforgerPunctuation:enforce`, and
`reforgerPreprocessor:enforce`; published selector names are compatibility
contracts.
_Avoid_: undeclared token type, generic variable, generic operator

**Semantic Token Boundary Guard**:
The Enforce-only editor presentation that places invisible, zero-width
default-foreground ranges at both edges of every settled semantic token.
The ranges expand only over text inserted at that exact boundary, preventing
VS Code's retained semantic range from briefly lending its old foreground to
the new text. A current semantic-token response atomically replaces the guard
positions; it does not change Rust classifications or define another palette.
_Avoid_: syntax decoration, client-side token classification, fallback palette

**Scope Delimiter**:
A matched Enfusion curly-brace, parenthesis, square-bracket, or parser-proven generic-angle-bracket pair whose presentation is classified by its immediate semantic anchor when one exists, otherwise by the syntactic scope that contains it.
Calls and index expressions use their callable or indexed symbol as that anchor; a block uses the declaration or control header that introduces it.
Generic angle brackets use their nearest generic/type owner as that anchor.
Nested Scope Delimiters retain their own anchors; a containing scope never recolors an anchored inner pair.
Initializer braces retain ordinary theme punctuation coloring while remaining
Scope Delimiters for active innermost-pair matching.
Attribute square brackets and attribute-call parentheses use the attribute/decorator as their anchor.
Constructor-call parentheses use the constructed type as their anchor.
Angle brackets in comparison expressions are not Scope Delimiters.
Delimiter characters in comments inherit comment coloring and are never active Scope Delimiters.
Delimiter characters in strings inherit string coloring and are never active Scope Delimiters.
Preprocessor directive text does not participate in delimiter classification or matching.
When an editing snapshot cannot prove a code delimiter's owner, it retains ordinary punctuation coloring until analysis can classify it.
After an edit, the current lexical token projection remains an internal input
and is never published as an interim full response. The editor keeps its
settled semantic presentation until the current rich projection replaces it;
the Semantic Token Boundary Guard keeps inserted text in the default
foreground during that interval.
The active Scope Delimiter is the innermost matched pair that contains the caret.
Its active range includes the caret immediately after its opener or immediately before its closer.
Its active-pair decoration remains settled while a current projection or
foreground retry is pending, then a current terminal response atomically
replaces or clears it. Stale responses never change the visible pair.
_Avoid_: bracket formatting, rainbow bracket

**Bracket Coloring Mode**:
The single application-scoped user preference that selects Scope Delimiter
presentation consistently across VS Code windows.
`semantic` is the default and uses each delimiter's semantic owner;
`punctuation` uses the Reforger Semantic Palette's punctuation color for every
code delimiter; `vscode` delegates delimiter foreground and matching
presentation to VS Code. The first two modes retain custom innermost-pair
highlighting and layer their Enforce-only foregrounds over the selected theme.
No mode defines a hard-coded server color.

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

**Constructor Completion**:
An accepted completion in a `new` expression that inserts a resolved
constructible type with its parenthesized constructor call and argument
placeholders. Its list can begin on the exact keyword prefixes `n` or `ne`,
remains available on a bare typed `new`, and also opens after a proven `new`
operand space. Before that space, the item previews the whole expression and
atomically replaces the typed prefix. Only required parameters receive
placeholders; array-literal initialization is a separate collection choice. A
constructible type without an indexed constructor signature is rendered as a
zero-argument call. It never inserts statement punctuation.
_Avoid_: new formatter, automatic constructor expansion

**Contextual Construction Type**:
The uniquely resolved type expected for a `new` expression by its surrounding
declaration or expression context. It bounds the constructor-completion list.
_Avoid_: copied declaration text, guessed constructor type

**Selected Constructor Suggestion**:
The contextually preferred Constructor Completion shown as the active
suggestion after `new`, before the author explicitly accepts it.
_Avoid_: automatic source edit, implicit constructor insertion

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

**Dynamic Collection Type**:
One of Enfusion's built-in generic collection types: `array<T>`, `set<T>`, or
`map<TKey, TValue>`. Collection completion and declaration initialization apply
to this closed set, not static-array syntax or arbitrary user-defined generics.
The base-game `Tuple1` through `Tuple6` classes receive their own arity-based
generic completion, but never collection initialization or declaration-tail
behavior.
_Avoid_: generic type, container type

**Collection Type Completion**:
The completion skeleton for a Dynamic Collection Type: one editable element
type for `array` and `set`; editable key then value types for `map`.
It is available at every parser-confirmed type position, while its declaration
initializer remains governed by Collection Scaffold Eligibility. Entering a
type placeholder opens type completion immediately; standard Enfusion types
rank before indexed classes. Prefix-match quality takes precedence over this
category ranking; `ref` ranks after standard types and before indexed enums and
classes; `void` is excluded.
_Avoid_: empty generic insertion, collection template

**Empty Collection Initializer**:
The idiomatic empty declaration form for a Dynamic Collection Type: `= {}` for
an `array`; `= new set<T>` for a `set`; and `= new map<TKey, TValue>` for a
`map`. An array declaration-tail prompt presents its literal form before the
also-valid explicit `new array<T>` form.
_Avoid_: universal collection constructor, empty collection expression

**Collection Declaration Scaffold**:
A completed empty Dynamic Collection Type declaration, including its
initializer and terminating semicolon. It is `array<T> name = {};`, `set<T>
name = new set<T>;`, or `map<TKey, TValue> name = new map<TKey, TValue>;`.
_Avoid_: partial collection initializer, unterminated collection declaration

**Collection Scaffold Eligibility**:
The declaration boundary at which a Collection Declaration Scaffold may be
created: an otherwise uninitialized, single-declarator local or class field.
Parameters, return types, loop headers, multi-declarators, and selections are
ineligible.
_Avoid_: declaration completion, any collection type occurrence

**Collection Declaration-Tail Prompt**:
The small completion list opened by Space at Collection Scaffold Eligibility.
It leaves the typed space intact until the user accepts either the default Empty
Collection Initializer or a declaration terminator; dismissing it preserves
native editing for a custom initializer. The Auto-Formatting Gate suppresses
the prompt entirely, leaving Space native. Tab, click, or a user completion
binding accepts a choice; Enter dismisses the prompt and remains native. It
declines in Native Editing Mode and for multi-caret or non-empty selections.
_Avoid_: automatic collection rewrite, mandatory initializer

**Custom Collection Initializer**:
The declaration-tail choice that inserts `= `, then opens ordinary expression
completion in the resulting parsed expression context. It provides values,
members, and calls without pretending their open-ended set is a collection
scaffold list.
_Avoid_: fixed initializer catalog, synthetic expression candidates

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
