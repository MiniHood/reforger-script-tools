# Enfusion `new` editing research

Research date: 2026-07-23. This report determines the safe automatic-editing
boundary after the Enfusion `new` keyword. It is evidence for later work, not
an implementation plan.

## Evidence

The primary corpus is verified extracted Reforger 1.7.0.54 source at
`C:\\Users\\Gray\\Documents\\VS\\Reforger-Codex-Agent-Skill\\raw\\game-data\\scripts`.
It has 5,715 `new` matching lines across 1,619 script files (comments included).
Approximate overlapping source categories are 3,349 parenthesized assignments,
643 assignments without parentheses, 168 `return new` expressions, 547 argument
occurrences, and 21 array-literal occurrences.

Official Reforger documentation says `new` creates an instance of the
*provided class* and constructor-defined arguments are required
([Keywords: `new`](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Keywords#new)).
It documents `{}` as the direct initializer for dynamic arrays, but uses
`new set<T>()` and `new map<K, V>()` in its set and map examples
([Enforce Script Syntax](https://community.bistudio.com/wiki/Arma_Reforger:Enforce_Script_Syntax)).
The operator table classifies `new` as dynamic-memory allocation
([Operators](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Operators)).

## Corpus examples

| Form | Verified examples | Formatting implication |
| --- | --- | --- |
| Empty constructor with `()` | `ref ScriptInvoker m_OnChanged = new ScriptInvoker();` ([Workshop](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Game/Workshop/SCR_WorkshopItem.c:32)); `TraceParam paramGround = new TraceParam();` ([recoil](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/GameCode/Weapon/SCR_RecoilForceAimModifier.c:55)). | Common, but only one of several valid forms. |
| Empty construction **without** `()` | `array<ref SCR_WorkshopItem> dependencies = new array<ref SCR_WorkshopItem>;` ([Workshop](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Game/Workshop/SCR_WorkshopItem.c:240)); `TStringArray arr1 = new TStringArray;` ([Core](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Core/proto/Types.c:227)). | Adding `()` is not a neutral automatic format. |
| Generic / nested collection | `new map<typename, ref array<string>>();` ([game-data fixture](C:/Users/Gray/Documents/VS/reforger-script-tools/tools/fixtures/parser/modded_game_mode_members.c:14)); `new array<ref SCR_WorkshopItem>` ([Workshop](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Game/Workshop/SCR_WorkshopItem.c:240)). | Copying a declaration type after `new` requires parsed type facts, including `ref` and nested generics. |
| Assignment and return | `m_CallbackRequestFavourite = new BackendCallback();` ([Workshop](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Game/Workshop/SCR_WorkshopItem.c:391)); `return new MaterialValidatorRequest();` ([validator](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGameCommon/ValidateMaterialPlugin.c:334)). | Expected type can come from a field or return type; it still does not uniquely select a constructible class. |
| Nested call/literal expression | `properties.Insert(new SCR_GeoProperty(...));` ([exporter](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/WorldEditor/SCR_ExportGeoDataPlugin.c:449)); `pointData = { new SCR_GeoPointData(...) };` ([exporter](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/WorldEditor/SCR_ExportGeoDataPlugin.c:519)). | Argument/literal nesting gives no single expansion. |
| Nonempty and named constructors | `new WBProgressDialog("Processing", worldEditor)` ([exporter](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/WorldEditor/SCR_ExportGeoDataPlugin.c:407)); `new SCR_WorkshopItemActionDownload(this, latestVersion: latestRevision, targetRevision)` ([Workshop](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/Game/Workshop/SCR_WorkshopItem.c:974)). | Argument count, ordering and labels are author intent, not formatter trivia. |

The extracted Workbench autocomplete plugin explicitly presents
`array<string> instance = new array<string>();` to `array<string> instance = {};`
as a transform ([plugin](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_AutocompletePlugin.c:52)).
The extracted Basic Code Formatter only reports a ` = new ` construction with
no `(` as a finding, and recommends `{}` over `new array<>`; it does not infer
or rewrite constructor arguments ([formatter](C:/Users/Gray/Documents/VS/Reforger-Codex-Agent-Skill/raw/game-data/scripts/WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c:640)).
This is Workbench style evidence, not a grammar rule.

## Decision: bare `new` must remain native

Given:

```c
array<Managed> stateComponents = new|
```

the extension must not automatically append a type, generic arguments, `()`,
an argument, literal, or semicolon on typing `new`, Space, or Enter. Plausible
next forms include:

```c
new array<Managed>;
new array<Managed>();
new SomeManagedSubclass(...);
```

The user can alternatively replace `new` with `{}` for an array. Even a
resolved expected type does not prove the concrete type or constructor, and
the corpus proves both parentheses choices occur. Therefore automatically
doing “all the rest” would be a semantic guess rather than auto-formatting.

The safe automatic behavior is preserving ordinary whitespace: a following
space remains one space and Enter uses native newline/indentation. It must
decline in comments, strings, directives, recovery syntax, non-empty
selections, and multiple-caret edits.

## Safe future interaction

| User action | Safe result |
| --- | --- |
| Explicitly accept a resolved constructible-type completion | Insert `Type($0)` as a snippet, or expose distinct `Type` / `Type()` candidates so the author selects the observed style. |
| Explicitly accept a collection completion | Insert generic placeholders; offer array literal initialization separately rather than turning a bare `new` into one. |
| Type `(` after a resolved type | Provide signature help, including optional named-argument labels. |
| Explicit document/range formatting | Normalize only existing trivia (for example `new Type (` to `new Type(` if adopted); never add/remove `new`, type text, parentheses, or arguments. |

This agrees with the repository's existing collection-tail decision: arrays
offer `= {};` before explicit `new array<T>`, while sets/maps use constructor
forms ([key-input routing](../key-input-routing.md#collection-declaration-tail)).
It also preserves the existing separation: constructor selection belongs to
completion, not Enter ([structural research](enfusion-structural-formatting-research.md#c-completion-snippet-and-code-action-opportunities)).

## Acceptance evidence for a later implementation

- A bare `new` never creates an on-type edit containing type text, `()`, an
  argument, generic argument, literal, or semicolon.
- Completion coverage includes fields, locals, assignments, returns, arguments,
  array literals, nested `ref` generics, named arguments, both parentheses
  styles, and ambiguous/unavailable type facts.
- Formatter coverage proves idempotence and byte-for-byte preservation of
  comments, strings, and preprocessor lines containing `new`.
