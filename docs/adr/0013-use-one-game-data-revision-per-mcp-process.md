# Use one immutable game-data revision per MCP process

The first game-data call lazily loads or rebuilds one validated semantic
catalogue, and concurrent calls join that initialization. The MCP process then
serves one immutable Game Data Catalogue Revision for its lifetime, making
multi-call symbol references and source ranges deterministic; an installation
change requires process restart rather than hidden hot reload, mutable
snapshots, or duplicated in-flight builds.
