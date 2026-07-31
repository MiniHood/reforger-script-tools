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
  "workbench": {
    "executable": "C:/path/to/Workbench.exe",
    "workingDirectory": "C:/path/to/workbench"
  },
  "expected": {
    "worldResource": "McpFixture/Worlds/Conformance.ent",
    "loadedAddonIds": ["McpFixture"]
  },
  "readiness": {
    "timeoutMs": 120000,
    "intervalMs": 1000
  }
}
```

`gproj` and `profileRoot` must be inside `fixtureRoot`. The runner passes the
resolved paths as separate process arguments (`-profile`, `-gproj`, and
`-addonsDir`) so spaces are preserved, starts World Editor with the canonical
fixture world through Workbench's typed `-wbModule WorldEditor -run -load`
startup parameters, waits for `workbench_status.isRunning`, and cleans up only
the process it started.

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
