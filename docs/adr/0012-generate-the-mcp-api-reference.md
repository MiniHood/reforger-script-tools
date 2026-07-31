# Generate the MCP API reference from the live tool catalogue

The Rust tool catalogue and its standard `tools/list` response are the
authoritative MCP interface. A committed `docs/mcp-api.md` is generated from
those same descriptors as the compact AI usage guide and categorized router.
Every router entry links to one self-contained generated contract under
`docs/mcp-api/tools/` containing that tool's exact schemas, workflows, limits,
errors, and examples.

The generator writes and drift-checks the guide and complete contract set
together. It rejects missing, changed, duplicate, and stale contract files.
This keeps exact offline inspection available without requiring an AI to load
the entire catalogue or introducing a separately maintained contract.
