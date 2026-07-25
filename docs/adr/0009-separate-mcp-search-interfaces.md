# Keep symbol search separate from official-wiki search

The MCP interface exposes Game Data Symbol Search separately from Official Wiki
Search because they have different authorities, query semantics, ranking, and
result shapes. The server will not introduce a federated search tool,
cross-source ranking, or a generic evidence-provider framework merely to make
the two operations look uniform; they share only protocol-level conventions
such as bounded typed results, provenance, cancellation, and errors.

The initial stable inventory is `game_data_status`,
`search_game_data_symbols`, `inspect_game_data_symbol`, and
`read_game_data_source`, followed by `official_wiki_status`,
`search_official_wiki`, and `read_official_wiki`.
