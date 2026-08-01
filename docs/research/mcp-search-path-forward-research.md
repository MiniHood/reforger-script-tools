# MCP search path-forward research

Research date: 2026-08-01.

This note turns the review of Game Data and Official Wiki search into a
concrete implementation path. The Phase 0 and Phase 1 work described below is
now implemented in the MCP contracts and Rust language layer. The remaining
text-search phase is intentionally a measured follow-up, not an unverified
feature commitment. The evidence used here is the current Rust implementation,
generated MCP contracts, and the repository's existing search research.

## Baseline

The current search boundary is fundamentally sound:

- Game Data symbol search is semantic. It ranks exact names before prefixes,
  qualified names, substrings, signatures, and type matches, then returns
  revision-bound symbol references and source-read handoffs. See
  [`game_data_search.rs`](../../server/src/game_data_search.rs) and the
  generated [`search_game_data_symbols` contract](../mcp-api/tools/search_game_data_symbols.md).
- Game Data inspection, member listing, relationship queries, example search,
  and bounded source reads form a progressive retrieval path. The MCP adapter
  projects facts from the parser-owned catalogue; it does not parse or search
  source files independently. See
  [ADR 0010](../adr/0010-progressive-mcp-game-data-retrieval.md).
- Official Wiki search is lexical and section-local. It validates the copied
  corpus, preserves canonical URLs and exact line ranges, and returns a
  revision-bound read handoff. See [`official_wiki.rs`](../../server/src/official_wiki.rs),
  [`search_official_wiki`](../mcp-api/tools/search_official_wiki.md), and the
  [corpus report](official-wiki-corpus-report.md).
- Keeping the two public searches separate is intentional. They have different
  authorities, matching semantics, ranking, and result shapes. See
  [ADR 0009](../adr/0009-separate-mcp-search-interfaces.md).

The main deficiency is not an excess of filters. The AI lacks an authoritative
search path for the project code it is editing, and the Wiki/example surfaces
need better discovery evidence.

## Recommended path

### Phase 0: improve the existing contracts — complete

Make the small, low-risk changes first:

1. Correct the generated Official Wiki example from `Guides/` to the actual
   corpus prefix `Modding/`.
2. Change the Wiki contract wording so it precisely describes the current
   matching rule: every term must occur in the same page's logical path, title,
   or one heading section (heading/body); it is not a general natural-language
   search.
3. Add match evidence to Wiki hits. A `matchLine`, bounded matched excerpt, or
   equivalent output should let the AI see why a body hit matched. The current
   implementation always builds the excerpt from the beginning of the section,
   so a long section can match while its returned excerpt contains no useful
   evidence.
4. Make supported example topics explicit, including root topics and
   subtopics. The current implementation accepts root topics such as
   `replication`, but its helper for displaying supported topics only renders
   topic/subtopic pairs. See
   [`supported_example_topics`](../../server/src/game_data_research.rs).
5. Add an intent-routing paragraph to the MCP server instructions and generated
   API guide. Prefer routing guidance over another dispatcher tool.

These changes improve AI success without changing authority or adding a new
search index.

### Phase 1: workspace semantic search — complete

Build the missing project-code path in the Rust language engine, then expose a
small MCP projection. The engine already owns separate immutable workspace and
Game Data layers and publishes generations for requests to capture. See
[`language-engine.md`](../language-engine.md) and the existing
[base-game search research](base-game-search-research.md).

The first slice should be:

```text
Rust shared symbol query
  -> search_workspace_symbols MCP tool
  -> revision-bound symbol and source handoffs
```

The query should support only the inputs that already have a clear semantic
meaning:

- query text;
- symbol-kind filters;
- bounded limit and opaque generation-bound cursor.

Do not initially add owner, source-category, fuzzy-mode, or dependency toggles
unless tests show that workspace callers need them. Scope is implicit in the
tool name, which keeps the AI from choosing between a matrix of scope flags.

The result should reuse the Game Data search vocabulary where it is genuinely
shared: name, kind, qualified name, containing symbol, signature or summary,
logical source identity, selection range, source generation, and a copy-ready
inspection/read handoff. It must not expose physical cache paths or dense
runtime symbol IDs.

Add inspection and relationship queries only after the initial search result
can be opened and verified. They should reuse the existing progressive pattern,
not become a second workspace analysis implementation.

### Phase 2: bounded source-text search — deferred pending measurement

Add a separate exact text-search capability only after measuring the need and
the cost. It answers questions semantic search cannot answer, such as:

- where a literal or comment occurs;
- where an unresolved identifier is used;
- where a particular call spelling or configuration fragment appears.

The result should contain a logical path, exact line/range, bounded excerpt,
source generation, and a copy-ready read handoff. It must be labelled as text
evidence and must not return symbol references or imply semantic resolution.

The initial public surface should remain scoped to one authoritative corpus,
for example `search_game_data_text`. Do not introduce a federated
`search_everything` or `search_reference` facade while the current separate
authority model is still being evaluated. A future shared lexical contract is
possible only if relevance, availability, provenance, and result shape are
shown to be materially compatible. This repository does not currently contain
a representative extracted Game Data text corpus and benchmark fixture for
that decision, so no text-search tool is added in this ticket.

### Phase 3: evaluate freshness and retrieval quality

The packaged Wiki corpus is useful and reproducible, but it is a snapshot, not
proof of the current public Wiki or Workbench behavior. Continue to report its
revision and coverage through status. Treat freshness/acquisition as a corpus
problem, not a search-parameter problem. The existing corpus report identifies
freshness and coverage as the more valuable future enhancement than a major
search redesign.

## Public-interface decisions

Keep the current parameters unless a measured use case proves otherwise:

| Surface | Keep | Do not add yet |
| --- | --- | --- |
| Game Data symbols | `query`, `kinds`, `owner`, `sourceCategories`, `limit`, `cursor` | fuzzy mode, arbitrary field flags, inherited-member switches |
| Official Wiki | `query`, `pathPrefix`, `limit`, `cursor` | fuzzy mode, many boolean field filters, semantic ranking |
| Examples | `topic`, optional `subtopic`, optional source filters, paging | a second free-form example search language |
| New workspace symbols | query, kinds, limit, cursor | scope matrices and dependency toggles |
| New text search | query, bounded scope, limit, cursor | semantic symbol fields or embedding-only ranking |

Optional filters do not create much choice burden when their defaults are
correct. The greater source of AI confusion is unclear intent routing and
incomplete follow-up evidence, not the existence of `limit` or `cursor`.

## AI routing contract

The MCP server instructions should state this directly:

| Intent | Tool path |
| --- | --- |
| Find an exact class, method, field, enum, or function | Game Data or workspace symbol search |
| Inspect declaration details or direct members | Symbol inspection, then member listing if truncated |
| Find callers, references, inheritance, or overrides | Symbol inspection, then relationship query |
| Find a source-backed implementation pattern | Game Data example search |
| Read exact source context | The returned source-read handoff |
| Find a documented workflow or Workbench procedure | Official Wiki search, then Wiki read |
| Find arbitrary literals, comments, or unresolved text | Corpus-specific text search once Phase 2 exists |

This is a documentation and prompt concern, not justification for a generic
MCP routing tool.

## Acceptance criteria

### Phase 0

- Generated API examples use real corpus prefixes and supported example topics.
- A Wiki body match returns evidence that visibly contains or points to the
  matching line.
- Existing revision, pagination, provenance, and bounded-output tests remain
  green.
- No new authority, index, or public dispatcher is introduced.

### Phase 1

- Equivalent workspace symbol facts are produced by the same Rust query owner
  used by LSP-facing symbol features and MCP.
- Results are bound to one immutable workspace generation and stale handoffs
  fail explicitly.
- Project source identities are logical and revision-correct; physical cache
  paths never cross the MCP seam.
- Search is cancellable, bounded, deterministic, and tested against changed
  workspace snapshots.
- A returned hit can be inspected or read without asking the AI to rediscover
  a path or reconstruct an internal ID.

### Phase 2

- Text hits are clearly distinguished from semantic declarations.
- Exact ranges and excerpts are stable for one source generation.
- The search is measured against a representative corpus before introducing a
  persistent text index or broader federation.

## Non-goals

- No generic `call_tool`, raw filesystem proxy, shell access, or arbitrary
  source execution.
- No cross-source ranking that makes Game Data and Wiki authority appear
  interchangeable.
- No replacement of native VS Code project search for ordinary workspace files.
- No embeddings or fuzzy matching in the authoritative exact-search path.
- No new manager, registry, or evidence-provider abstraction without two real
  adapters requiring it.

## Open questions before implementation

1. Should workspace MCP search expose only the project layer, or the project
   layer plus selected loaded dependencies? The default should be project-only
   until dependency noise and identity rules are measured.
2. Does workspace source reading already have a stable logical MCP handoff, or
   must the first workspace slice include one? Search is not complete if its
   result cannot be opened.
3. Which Wiki match evidence is most useful in practice: a single matched line,
   a centered excerpt, or multiple bounded match ranges?
4. Is a full-text scan within the five-second ready-operation budget on the
   representative Game Data corpus? If not, measure an inverted/trigram index
   before choosing a storage design.
5. Which example topics should be promoted first, based on real AI workflows,
   rather than expanding the hard-coded catalogue indiscriminately?

## Decision summary

Proceed in this order:

1. Fix Phase 0 contract and evidence issues.
2. Add workspace semantic search through the shared Rust language layer.
3. Measure and add corpus-specific text search if the evidence gap remains.
4. Treat Wiki freshness as a separate acquisition/revision project.

This preserves the repository's central invariant: one authoritative answer per
fact, with small MCP interfaces and the complexity hidden behind the Rust
language/evidence modules.
