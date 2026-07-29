# Workbench script-reload surface research

This journal records evidence for a background-safe, in-process trigger of
Workbench's script reload. It is not an implementation decision: a public
API declaration does not prove either the required menu path or background
behaviour in a live Workbench session.

## Scope and evidence revision

The investigation was run on 2026-07-28 against the live installed
`ArmaReforgerWorkbenchSteamDiag.exe`, file/product version `1.7.0.54`, at:

```text
C:\Program Files (x86)\Steam\steamapps\common\Arma Reforger Tools\Workbench\ArmaReforgerWorkbenchSteamDiag.exe
```

The extracted API source is revision
`2735631ce1400eaf9f1761c66cdee10c46921d37` (`1.7.0.54`), recorded in:

```text
C:\Users\Gray\AppData\Roaming\Code\User\globalStorage\burn0ut7.reforger-script-tools\game-data\metadata.json
```

The official generated API documentation corroborates the extracted
declarations: [ScriptEditor interface reference](https://community.bistudio.com/wikidata/external-data/arma-reforger/EnfusionScriptAPIPublic/interfaceScriptEditor.html).

## Verified surfaces

| Candidate | Evidence | Result |
| --- | --- | --- |
| Named public reload/compile API on `Workbench` | `GameLib/generated/WorkbenchAPI/Workbench.c` was read in full. It exposes modules, resources, dialogs, processes, paths, and script-module getters only. | Not found. |
| Named public reload/compile API on `WorkbenchPlugin` / `ResourceManagerPlugin` | `Plugins/WorkbenchPlugin.c` and `Plugins/ResourceManagerPlugin.c` were read in full. They expose lifecycle hooks and resource-build hooks only. | Not found. |
| Named public reload/compile API on `ScriptEditor` / `ResourceManager` | `Modules/ScriptEditor.c` and `Modules/ResourceManager.c` were read in full. Script Editor exposes current-file and line editing; Resource Manager exposes register/rebuild resource operations. | Not found. |
| Generic in-process action dispatch | `Modules/WBModuleDef.c` declares `proto external bool ExecuteAction(notnull array<string> menuPath, bool bKeepFocus = true)`. `ScriptEditor` inherits it, as does `ResourceManager`. | Public API exists, but its exact action-path grammar and completion semantics are undocumented. |
| Actual reload menu item and accelerator | ASCII-string extraction from the installed executable finds `&Reload WB Scripts`, immediately followed by `Shift+Ctrl+R`; it also contains `ScriptPluginsMenu`. | The visible action is real in this installed version. |
| Reload completion markers | The executable contains `Reloading game scripts`, `Script validation`, `Compiling GameLib scripts`, `Compiling Game scripts`, and `Game Scripts reloaded.` The current server's verification additionally observes module load. | Strong postcondition; verify a newly appended complete sequence. |

The extracted sample source also states that scripts become available after
using **Compile and Reload Scripts** in Script Editor:
`GameLib/replication/RplDocs.c:178-179`. That confirms the editor operation's
role, though not its callable action path.

## Current conclusion

There is **no supported named script-reload method** on `Workbench`,
`WorkbenchPlugin`, `ResourceManager`, or `ScriptEditor` in the examined
`1.7.0.54` public API. The only supported in-process candidate is
`ScriptEditor.ExecuteAction(menuPath, bKeepFocus)` inherited from
`WBModuleDef`.

It is credible precisely because it runs inside Workbench's script runtime;
it does not require Windows UI Automation or keyboard delivery. It is not yet
safe to expose: the exact `menuPath` for `Reload WB Scripts` has not been
verified, and the API's `bKeepFocus` name/default do not document a guarantee
that the action leaves the Workbench window unfocused or unactivated.

Do not infer the path from `ScriptPluginsMenu`, the visible label, or generic
menu names. That would recreate the rejected stringly menu-dispatch path.

## Decisive acceptance experiment

Perform this only through an **already loaded**, version-pinned custom
`NetApiHandler`; it cannot bootstrap a handler that Workbench has not yet
loaded. Implement a temporary handler with one fixed, no-input operation (not
an arbitrary action proxy) and structured output containing:

- `workbenchProcessId`, `foregroundWindowHandleBefore`, and
  `foregroundProcessIdBefore` captured by the external host immediately before
  the NET request;
- the fixed candidate path, the `bKeepFocus` value, and the direct boolean
  returned by `ScriptEditor.ExecuteAction`;
- a new console-log cursor captured before dispatch and the exact newly
  observed reload-marker lines afterward;
- `foregroundWindowHandleAfter` and `foregroundProcessIdAfter` captured by
  the host after the request; and
- a monotonic request ID shared by the host log and handler response.

Run a small, explicit candidate list one at a time, never a fallback loop.
For each candidate, keep a non-Workbench process foreground and record:

1. `GetForegroundWindow` / process ID before the NET request.
2. The handler's API return value and elapsed time.
3. The foreground window/process after the request.
4. A full *new* console sequence: reload started, validation, GameLib
   compilation, Game compilation, and WorkbenchGame module-loaded marker.

Accept a candidate only if all of the following are true in the same run:

- the handler returns `true`;
- the foreground process/window is unchanged (Workbench was neither focused
  nor activated);
- the complete reload sequence begins after the pre-dispatch cursor; and
- the terminal marker reports successful game-script reload/module load.

Reject it on a false return, foreground change, missing/incomplete log
sequence, timeout, or any modal/error dialog. Keep the existing explicit-focus
keyboard route as the only production path until this test passes for each
supported Workbench version.

## Exact read-only commands used

```powershell
$gd = 'C:\Users\Gray\AppData\Roaming\Code\User\globalStorage\burn0ut7.reforger-script-tools\game-data\scripts'
rg -n -i -C 4 'Reload WB Scripts|ScriptPluginsMenu|Reloading game scripts|Reload.*Scripts|ReloadScripts' $gd --glob '*.c'

$exe = 'C:\Program Files (x86)\Steam\steamapps\common\Arma Reforger Tools\Workbench\ArmaReforgerWorkbenchSteamDiag.exe'
$bytes = [IO.File]::ReadAllBytes($exe)
$text = [Text.Encoding]::ASCII.GetString($bytes)
$text.IndexOf('&Reload WB Scripts')
$text.IndexOf('ScriptPluginsMenu')
```

The first command confirms no matching reload implementation appears in the
extracted `.c` sources (apart from the documentation wording). The second
inspectable binary evidence finds the installed action label, shortcut, and
menu-class name. Binary strings establish version-pinned UI existence, not an
API contract.
