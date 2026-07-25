# Use progressive retrieval for MCP game-data evidence

The first MCP capability exposes a status, search, inspect, and bounded-source
read loop. Search remains compact for reliable AI discovery, inspection
projects the rich semantic facts already stored by the language index, and
source reading supplies authoritative code context.

After the language engine gained the required semantic facts, this loop was
extended with separate bounded example discovery, paginated direct-member
discovery, and named inheritance/reference/caller queries. These operations
retain the same immutable catalogue revision and return copy-ready inspection
or source-read handoffs. The server still does not dump the complete index,
present text matches as references, guess a call graph, or expose an unbounded
graph-query language.
