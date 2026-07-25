# Use revision-bound logical references for MCP symbols

Game Data Symbol Search returns a human-readable identity plus an opaque
`symbolRef` that Game Data Symbol Inspection accepts directly. The reference
is bound to the game-data catalogue revision and contains no physical path or
raw in-memory `GlobalSymbolId`; a changed catalogue produces an actionable
stale-reference error so the agent searches again instead of inspecting a
different declaration.
