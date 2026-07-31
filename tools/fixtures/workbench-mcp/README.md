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
  "useProfile": true,
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
fixture unless `profileRootOutsideFixture` is explicitly set. With
`useProfile:true`, the runner starts its MCP server with the manifest's profile root,
then calls the public `workbench_launch` operation with the exact fixture
`.gproj`; it never constructs a Workbench process command line. The launch
implementation always supplies the discovered base-game add-on directory and
the dedicated profile. The manifest's expected add-on list must include
`ArmaReforger`, and the project must make that base-game add-on available. The
runner waits for `workbench_status.isRunning`, discovers the World Editor
through `workbench_list_editors`, opens it through `workbench_open_editor`,
opens the canonical fixture world through `workbench_open_resource`, and
cleans up only a process that the MCP launch operation reported as newly
started. A reused process is permitted only for an explicitly marked smoke
manifest; it is not disposable isolation.

`useProfile:false` is an explicit escape hatch for a pre-provisioned local
profile whose managed bridge and base-game setup are already installed. It
does not permit process reuse: the fixture must still launch with
`allowExistingProcess:false`, and ownership is verified from the public launch
result. Use this only when first-install bridge consent cannot be supplied to
the disposable profile by the public MCP contract.

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
`$fixture.projectPath` and `$fixture.worldResource` references for lifecycle and
world-identity checks; there is no arbitrary expression or command expansion.

The committed `scenarios/test-bullshit-all-apis.json` is the complete live
scenario for the current 63-tool Workbench catalogue. It uses the fixture's
stable world, spline, and component identities, creates disposable scene
entities, confirms every preview token in the same MCP session, and restores or
deletes each scene mutation. It also records structured expected failures for
capabilities that the target Workbench cannot complete in the current session.
An environment-dependent step may set `expect.allowError` when both a success
result and a structured unavailable result are valid outcomes; it must also
provide an explicit `expect.error` code/phase oracle. This is reserved for
window capture and any reload run whose replacement generation cannot be
observed; such a result remains visible in the
report's `expectedErrorCount`. Steps marked `expect.completion: false` remain
live evidence for inventory coverage, but appear in `liveCoverage.incomplete`
and increment `expectedUnavailableCount`, so an unavailable or unsupported
operation cannot be mistaken for completed editor behavior.

Each JSON report also contains `endpointCorpus`. It has one record for every
published endpoint, with `approved`, `failed`, `incomplete`, or `not-tested`
status, the scenario observations that support the status, timing, structured
errors, and assertion reasons. An expected structured error is approved when
it matches its explicit oracle; an endpoint is incomplete when its scenario
marks the capability as unavailable.

For a disposable manifest with `allowExistingProcess` omitted or false, the
runner verifies the owned lifecycle after the scenario: it restarts the exact
process returned by launch, adopts the returned replacement process ID, and
stops that replacement. The result is recorded under `ownedLifecycle`.

Run a live scenario only with both explicit inputs:

```powershell
node tools/workbench-mcp-conformance.mjs `
  --server server/target/debug/reforger_language_server.exe `
  --fixture C:/path/to/fixture.manifest.json `
  --scenario C:/path/to/scenario.json `
  --require-live-coverage `
  --out .cache/reports/workbench-mcp-live.json
```

The runner owns fixture process cleanup, but scenario authors still need to
make mutations reversible or rely on disposable-project teardown. A scenario
must verify mutations with a read-after step; a successful MCP transaction is
not a sufficient oracle.
