# MCP server research and implementation design

Research date: 2026-07-25.

This is the build guide for the local Reforger MCP server. It records durable
decisions, authority boundaries, and the next verified slices. Public tool
schemas belong in [MCP API Reference](../mcp-api.md).

## Status and decisions

The current MCP runtime is a packaged Rust process running over `stdio`. Its
public surface includes static Game Data and Official Wiki retrieval plus a
bounded Workbench lifecycle. See `mcp-api.md` for the generated descriptors,
exact limits, and effect annotations.

NET API is the one live-editor integration path. It is private between the
local MCP host and a running Workbench; it is never exposed as an MCP endpoint
or generic proxy. Rust now owns the codec. The TypeScript compiler integration
uses the bundled executable's private `workbench-api` mode and does not retain
a parallel implementation.

| Decision | Keep |
| --- | --- |
| Semantic authority | The existing Rust language engine and its game-data index own Enforce symbols, relationships, source facts, and workspace semantics. |
| Documentation authority | Packaged Official Wiki Markdown owns documentation search and reads. |
| Live authority | Workbench owns active project/resource resolution, compiler state, editor/world state, imported-resource state, Undo, and visual/editor facts. |
| Static file authority | Bounded project-file access supports orientation, indexing, and version-checked edits. It must not imitate live Workbench facts. |
| Workbench transport | A single typed NET API Gateway owns framing, configured endpoint access, timeouts, retries, and outcome mapping. |
| Workbench extension | One optional, versioned project-side handler package owns engine calls and exposes named DTOs. |
| Public MCP surface | Named tools with typed schemas and structured results. No `call_tool`, raw NET API/handler dispatch, arbitrary console command, shell, or filesystem proxy. |

This is one authority per fact, not two competing systems. Offline tools work
without Workbench. Live tools report typed unavailability when Workbench is
closed, in an unsuitable mode, on another project, or has no compatible plugin.

## Architecture

```text
MCP client
  -> Rust MCP runtime: protocol, schemas, bounds, result mapping
       -> Rust language engine + bounded project files
       -> private Workbench Gateway
            -> configured loopback NET API
                 -> versioned Workbench handler package
                      -> Workbench resource/compiler/world/editor APIs
```

The MCP runtime owns client-facing contracts only. The language engine owns
semantic interpretation. Workbench handlers own live engine calls and editor
transactions. The Gateway owns NET API wire details; no other module recreates
them.

The existing proven Gateway is TypeScript under `src/workbenchNetApi/gateway/`.
Before a Rust MCP tool calls Workbench, choose one reusable owner: move the
host-neutral Gateway below the Rust boundary or give Rust a narrow private
process boundary to that Gateway. Do not implement a second codec.

## Current baseline

The shipped slices are intentionally narrow:

- Game Data: status, symbol search, symbol inspection, and bounded extracted
  source reads.
- Official Wiki: status, deterministic search, and bounded Markdown reads.
- Runtime: `stdio`, typed request/result models, structured content, bounded
  work, cancellation, sanitized errors, and packaged-layout acceptance.
- Workbench: path/process/status diagnosis, consented managed-handler
  maintenance after the extension-owned first-install prompt, native compiler
  validation, verified hot
  activation, handler state, bounded support-log reads, and approval-bearing
  graceful stop/restart.

The exact tool names, schemas, annotations, stable error codes, limits, and
recovery guidance are generated into [MCP API Reference](../mcp-api.md). Do
not duplicate them here or add a competing descriptor catalogue.

The current compatibility and packaging evidence remains useful only as a
record. Recheck the pinned SDK, protocol compatibility, Inspector, Codex, and
the packaged binary before changing transport or runtime dependencies.

## NET API capability plan

Workbench capabilities must come from documented built-ins or versioned custom
handlers, never guessed UI paths or scraped editor state. The authoritative
protocol evidence and current validation backlog are in
[Workbench NET API research](workbench-net-api-research.md).

The documented built-ins already justify small named operations for Workbench
status, World Editor status, resource/module focus, and compiler validation.
They do not establish general editor automation, entity editing, project
discovery, resource indexing, or arbitrary command execution.

The handler package begins with a `capabilities` operation. It returns plugin
and DTO versions, loaded-project identity, capability revision, named groups,
limits, effects, and unavailable reasons. The MCP host caches no capability
fact beyond its declared revision.

| Group | First useful operations | Required identity and result facts |
| --- | --- | --- |
| `project` | `project_context`, content roots, loaded addon identity | Canonical project/addon identity and containment facts. |
| `resource` | Resolve/list/inspect a bounded resource set; prefab/container inspection | Resource kind, canonical path or GUID, cursor/limit, provenance, and affected resources. |
| `world` | Current selection summary; typed entity/hierarchy inspection | Stable editor entity identity, not display-name matching; explicit editor mode. |
| `compiler` | Readiness and `validate_scripts` | Invocation configuration, normalized diagnostics, locations, and artifacts. |
| `editor` | Open/focus supported resources or modules | User-visible effect classification and resulting context. |
| `visual` | Later screenshot, thumbnail, or preview capture | Proven image-transfer path, bounded bytes/dimensions, source and timestamp. |
| `workflow` | Later project-specific composite operations | Built only from approved domain operations; never arbitrary script execution. |

Start with read operations. Add a mutating command only after the corresponding
read/observe/verify loop is live and accepted in a supported Workbench version.

The consolidated external MCP-host and Workbench-handler catalogue, including
each handler's conservative disposition, is in
[Workbench NET API research](workbench-net-api-research.md). MCP tools are
selected only from the proven, versioned capabilities recorded in this plan and
the NET API evidence journal.

## Mutation and safety contract

Every mutation needs a stable intent-level name and typed DTO. It must target a
selected project, expose affected entity/resource identities, and state its
effect before execution.

Where meaningful, a mutation provides a preview or dry run, expected versions
or content hashes, clear idempotency/partial-failure rules, and post-action
verification. World operations use one named native Undo transaction and close
that transaction on every outcome.

Handlers return typed `ok` or typed error codes, not agent-directed prose. The
Gateway converts that outcome into an MCP success or `isError` result. A TCP
success never proves the requested operation succeeded.

The endpoint remains user-configured loopback only. Do not scan ports, start a
second listener, accept arbitrary script text, or allow arbitrary filesystem
paths. The lifecycle tools may resolve known Reforger installations, launch the
known Workbench executable, and gracefully close an exact reported Workbench
PID. They never force-kill a process, close an unreported PID, or infer consent
to install the handler package.

## Evidence and compatibility rules

Use this hierarchy for each new Workbench operation:

1. Live Workbench behavior on every supported version.
2. Official Workbench and NET API documentation.
3. Verified extracted game data and source examples.
4. Third-party source review, clearly labelled as non-authoritative.

An operation becomes MCP-visible only after its handler, DTO version behavior,
unavailable states, size/paging bounds, and live acceptance have been proven.
Plugin absence or version mismatch disables only the affected group.

The external handler review in
[Workbench NET API research](workbench-net-api-research.md) records failure
patterns and candidate groups. It is a design warning and catalogue, not a
contract to implement.

The [s&box source review](sbox-mcp-research.md) is an optional example bank for
AI-facing tool design. It does not define Reforger architecture, transport,
dynamic discovery, or public API requirements.

## Delivery sequence

1. Preserve and test the existing status/compiler Gateway.
2. Resolve the one-Gateway integration seam between Rust MCP and TypeScript.
3. Add `capabilities`, then `project_context`; prove missing, stale, malformed,
   oversized, and incompatible requests.
4. Add bounded resource resolution and inspection with canonical identities.
5. Add typed selection/world inspection with stable editor identities.
6. Add one narrow mutation, prove one Undo restores it, then verify the result.
7. Add compiler/build/test and visual operations only after their source and
   transfer contracts are independently proven.

Each slice updates `mcp-api.md`, adds focused tests, and is accepted from a
packaged installation with Workbench running and unavailable. The static
Game Data and Official Wiki tools remain usable throughout.

## Non-goals

The server does not need remote transport, multi-user sessions, dynamic MCP
tool registration, a generic invocation layer, arbitrary file/shell tools, a
second parser/index, or a parallel live-editor model.

Do not add indexes, caches, registries, settings, or abstractions merely in
anticipation of future tools. Add each mechanism only after a measured or
proven capability requires it.

## Related records

- [MCP API Reference](../mcp-api.md): generated public descriptors and limits.
- [Workbench NET API research](workbench-net-api-research.md): protocol,
  official/extracted evidence, Gateway contract, and validation backlog.
- [Workbench compiler-validation research](workbench-compiler-validation-research.md): compiler outcomes and diagnostic projection.
- [Base-game source search research](base-game-search-research.md): shared
  language-index ownership and MCP adapter guidance.
- [s&box MCP source review](sbox-mcp-research.md): optional external examples.
