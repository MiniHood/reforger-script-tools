# Researcher Contract Acceptance

Run these bounded scenarios after changing `/researcher` source-mode, roster,
evidence-package, partial-coverage, or synthesis rules. Record the request,
base revision, selected/displaced/ineligible personas, actual source classes,
coverage result, and observed outcome. These checks validate workflow
semantics, not the truth of a researched topic.

| Scenario | Request | Expected result |
|---|---|---|
| Local auto | `sources:local` on a narrow repository question | Codebase and Architecture may form the complete two-persona roster; no web research or Online Research persona runs. |
| Online auto | `sources:online` on an external-practice question | Online Research starts; local code/game-data evidence is excluded except policy/status control-plane reading. |
| Both auto | `sources:both` on a cross-layer question | Source-compatible core and only material specialists run; the final brief labels local and online evidence separately. |
| Incompatible explicit persona | `sources:local personas-only:online-research` | Coordinator rejects the request and asks for a compatible mode or persona; it does not browse. |
| Explicit overflow | `sources:both personas:game-api-examples,online-research,language-semantics` | Coordinator requests a narrowed roster or named follow-up pass; it does not silently omit a lens. |
| Narrow relevance | A question with only two material local lenses | Coordinator records every materially considered specialist and runs only those two. |
| Unavailable persona | A selected persona is interrupted, malformed, or has no journal/final report after two waits | Its reason and journal status are retained; the synthesized brief is explicitly partial. |
| Evidence snapshot | A repository changes during a research run | Final brief records clean revisions or dirty/untracked file fingerprints; changed/later reads are labelled post-snapshot rather than merged states. |
| Auto depth | `depth:auto` on a bounded question | Smallest sufficient roster and search run; final brief states its saturation rationale. |
| Full depth | `depth:full` on the same question | Every material source-compatible lens up to four is considered; competing directions/counterexamples and exclusions are recorded. |
| Curiosity stop | A tempting first fix appears early | Research continues through a credible challenge and reports why it was accepted or rejected before synthesis. |
| Durable option | Evidence supports one complete durable design and one temporary mitigation | Durable target ranks first; each option has favorability, evidence IDs, ranking rationale, and mitigation removal condition. |
| One viable option | Evidence rejects all alternatives | Brief presents one viable option with favorability and evidence IDs, and explains why alternatives were unsupported. |
