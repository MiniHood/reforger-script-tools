# Roster Acceptance

Use these deterministic scenarios after modifying `/review` roster behavior.
For each, record the requested mode, selected/skipped lenses, and whether the
observed action matched the expected result.

| Scenario | Request and evidence surface | Expected result |
|---|---|---|
| Language auto | `depth:auto`; parser/completion/Workbench behavior | Correctness, Architecture, and Language Fidelity are selected; unrelated specialists are explicitly skipped. |
| Evidence auto | `depth:auto`; defect with tests, diagnostics, or reproduction evidence | Correctness, Architecture, and Verification & Observability are selected; unrelated specialists are explicitly skipped. |
| Full bounded | `depth:full`; language defect with diagnostics | Correctness and Architecture plus Language Fidelity and Verification & Observability are selected; no fifth reviewer starts. |
| Explicit specialist pair | `personas:language-fidelity,verification-observability` | Correctness and Architecture are retained and the two named specialists are selected. |
| Persona-only request | `personas-only:language-fidelity` | Only Language Fidelity is selected; the coverage report records the intentionally narrow roster. |
| Explicit overflow | More than four resulting personas | The coordinator requests a narrower roster or follow-up review; it does not silently omit a lens. |
| Invalid request | Duplicate, malformed, or unknown depth/persona token | The coordinator requests clarification before fan-out. |
| Specialist tie-break | Three relevant specialist surfaces | The two highest-ranked specialists are selected using scope ownership, user concern, then risk; the displaced lens and reason are recorded. |
| Unavailable reviewer | A selected reviewer fails, is interrupted, or returns no conforming report | Its partial journal is retained, coverage names it unavailable, and the report is labelled partial. |
| Unresponsive reviewer | A selected reviewer has no report or journal progress after two coordinator wait intervals | It is marked unavailable, its journal is retained, and the report is labelled partial. |
| Isolation and result barrier | Any multi-persona review | Reviewers receive only their own contract, common contract, and immutable package; peer journals are not read; interim findings are withheld; only journals change before final synthesis. |

Do not claim the scenarios passed from intent alone. Record the concrete
request, observed roster, and partial-coverage wording in the validation
handoff or generated review evidence.
