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

**Auto-Block Control Header**:
A newly authored `for`, `foreach`, `while`, or `switch` header whose typing
assist creates a braced body when Enter is pressed from inside the parentheses
or immediately after them, regardless of whether the header contents are
complete. `if` and `else if` do not create blocks automatically because their
bodies do not have one agreed automatic shape; `else` is not a header.
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
