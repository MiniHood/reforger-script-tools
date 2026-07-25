# Generate the MCP API reference from the live tool catalogue

The Rust tool catalogue and its standard `tools/list` response are the
authoritative MCP interface. A committed `docs/mcp-api.md` is generated from
those same descriptors and verified for drift, giving maintainers and offline
agents exact schemas, workflows, limits, errors, and examples without adding a
redundant API-index tool or a separately maintained contract.
