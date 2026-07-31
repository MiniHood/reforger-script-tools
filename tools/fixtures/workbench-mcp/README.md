# Workbench MCP live fixture

This directory is the developer-only home for the disposable Workbench MCP
fixture contract. It is intentionally outside `dist/` and is never included
in the extension package.

The live runner takes a manifest owned by the test invocation:

```json
{
  "name": "mcp-conformance",
  "revision": "2026-07-31",
  "fixtureRoot": "C:/path/to/disposable-project",
  "profileRoot": "profile",
  "project": {
    "gproj": "mcp-conformance/mcp-conformance.gproj",
    "addonsDir": "C:/path/to/ArmaReforger/addons"
  },
  "expected": {
    "worldResource": "McpFixture/Worlds/Conformance.ent",
    "loadedAddonIds": ["ArmaReforger", "McpFixture"]
  },
  "readiness": {
    "timeoutMs": 120000,
    "intervalMs": 1000
  }
}
```

`gproj` and `profileRoot` must be inside `fixtureRoot` for a disposable
fixture. The runner uses the MCP `workbench_launch` operation to launch or
reuse the Workbench process configured for the MCP server; it never constructs
a Workbench process command line. The manifest's expected add-on list must
include `ArmaReforger`, and the configured project must make that base-game
add-on available. The runner waits for `workbench_status.isRunning`, discovers
the World Editor through `workbench_list_editors`, opens it through
`workbench_open_editor`, opens the canonical fixture world through
`workbench_open_resource`, and cleans up only a process that the MCP launch
operation reported as newly started. A reused process is an explicit smoke-test
mode, not disposable isolation.

The fixture itself must provide the stable project, world, resource, entity,
component, layer, prefab, shape, and terrain identities used by the scenario
files. Those assets are machine- and game-install-specific, so they are
provisioned outside the repository and identified by the manifest rather than
checked into the extension source tree.

The first scenario steps should call `workbench_status` and `workbench_state`,
then assert `/result/structuredContent/activeWorldPath` equals
`expected.worldResource`. Later world mutations must use read-before/write/
read-after steps and stable entity identities. Scenario arguments and pointer
oracles may use the explicit `$fixture.processId` and
`$fixture.worldResource` references for lifecycle and world-identity checks;
there is no arbitrary expression or command expansion.

Run a live scenario only with both explicit inputs:

```powershell
node tools/workbench-mcp-conformance.mjs `
  --server server/target/debug/reforger_language_server.exe `
  --fixture C:/path/to/fixture.manifest.json `
  --scenario C:/path/to/scenario.json `
  --out .cache/reports/workbench-mcp-live.json
```

The runner owns fixture process cleanup, but scenario authors still need to
make mutations reversible or rely on disposable-project teardown. A scenario
must verify mutations with a read-after step; a successful MCP transaction is
not a sufficient oracle.
