# Design: Implement EX12 Shambala + Virus Busters slices

## Context

EX12's data layer is fully landed (commit `e53df0d8e`): all 77 cards in `data/cards.json` (overrides re-applied), per-card JSONs in `code/digimon-engine/cards/ex12/`, card scans mirrored to the lookup-skill cache, lexicons refreshed, and the three DUAL cards' Option-face colors face-verified. The two target slices — Shambala (33) and Virus Busters (21) — have zero implementations.

This is the first set authored **without DCGO** (no community C# for EX12) and **without official-DB bundles** (`world.digimoncard.com` mid-restructure). The keyword gate already found two new printed keywords (＜Guard＞, ＜Engage＞) and at least one new token species ([Paishu]). An 8-batch assessment workflow was authored (`workflows/scripts/ex12-shambala-vb-assess-wf_6f7700f2-6c5.js`, run `wf_6f7700f2-6c5`) but died on usage credits before any batch ran.

The proven pipeline from the store-champs June-2026 campaign applies: assess → close gaps (rule 28) → TDD implementation waves with per-card Opus review → verdicts → interaction-test capstone. Its hard-won process lessons are recorded in memory (`reference_card_impl_workflow_lessons`): reviewers judge embedded bodies, honesty contracts with pasted `test result:` lines, HTML-entity unescape at merge, 3-way patch merges, worktree-sync preambles against stale bases, and per-card scan-grounded review as the omission catcher.

## Goals / Non-Goals

**Goals:**
- Every printed clause of all 54 cards faithfully implemented (no-approximations, §17) with per-card behavioral suites green and verdicts recorded.
- ＜Guard＞ and ＜Engage＞ as first-class engine keywords: printed parse, faithful behavior, RL action-space exposure of every choice, clone-safe.
- Token registry entries for the new species with their printed keywords carried.
- Substrate gaps surfaced by assessment closed properly (widen, never route around), each with tests and tracker resolutions.
- Interaction-test capstone for both slices.

**Non-Goals:**
- The other EX12 slices (DS/ME/NSo/NSp/WG), including the out-of-slice DUALs EX12-033/052 and Engage-consumer EX12-060 (the *keyword machinery* covers them; their cards are not authored here).
- EX12 deck-library/meta ingestion, frontend work, and re-scraping the official DB (revisit bundles when the site stabilizes).
- Backfilling DCGO parity for EX12 (no oracle exists; parity re-checks happen if/when DCGO ships the set).

## Decisions

**D1 — Keywords first, as engine substrate, before any card wave.**
Guard and Engage gate 5+ cards and tokens; implementing them per-card would fork semantics. Guard auto-emits a protect-others leave replacement (the round-3 `protect_others` substrate: cost `delete_self`, outcome `prevent`) with the cause scope narrowed to *opponent effects* (the printed reminder text's "by your opponent's effects" — narrower than the generic substrate default; the replacement-cause filter already models this via `replacement_cause` predicates). Engage adds an end-of-turn optional attack window; it shares Vortex's EOT machinery but is a distinct `Keyword` variant — the reminder text lacks Vortex's played-this-turn allowance and its Digimon-target clause, so semantics are confirmed against wiki rulings/Q&A first and any unresolved ambiguity is implemented conservatively with the ambiguity documented in the keyword test file. Alternative considered: express both purely in per-card YAML `grant_keyword`+clauses — rejected because printed keywords must parse from card text (tokens carry them; future sets reuse them).

**D2 — Scan-first authoring discipline (no DCGO).**
Authority order for this change: card scan (`.claude/skills/digimon-card-lookup/.cache/EX12-*.webp`) > official rulings (wiki card pages, fetched via browser when bot-blocked) > `general_rule.pdf` + `docs/digimon-rules/` > per-card JSON (digimoncard.io text is paraphrased for this set — treat as hint only). Every implementer and reviewer prompt embeds this order and requires reading the scan. Where printed text is ambiguous and no ruling exists, the card ships PARTIAL with a named tracker entry rather than a guessed behavior. Alternative: wait for DCGO to ship EX12 — rejected; indefinite delay, and the store-champs campaign already proved scan-grounded review catches omissions (P-180 shipped DCGO-less and clean).

**D3 — Resume the existing assessment workflow, don't redesign it.**
The 8-batch audit script is already authored with the right constraints (scan authority, pre-named gap ids G-KEYWORD-GUARD/G-KEYWORD-ENGAGE, consolidation stage). Re-run via `Workflow({scriptPath, resumeFromRunId: "wf_6f7700f2-6c5"})` once credits allow — all batches failed, so everything runs live; the resume path just reuses the persisted script + args.

**D4 — Wave structure mirrors store-champs, sized to the slices.**
Waves of ~8–16 cards grouped by sub-engine (Shambala: eggs+Lv3, Lv4–5 SW, Lv4–5 TB, Tentei Hachibushu Lv6s + Susanoomon + Options; VB: Gammamon line + DUAL, Agu/Gabu lines, Omnimon/support/Tamer). Each card: Sonnet-class implementer in an isolated worktree with a worktree-sync preamble, returning FULL yaml/test bodies via structured output under the honesty contract; Opus reviewer judging the embedded bodies against the scan (file-not-on-disk is never a rejection). Orchestrator merges via entity-unescape + 3-way patches, runs scoped suites, records verdicts, commits per wave. The DUAL Siriusmon follows the shipped dual-YAML shape (ST23-09/ST24-07/BT25-043/057; BT25-085 if merged by then).

**D5 — Gap closure is batched by subsystem, not by card.**
Assessment output is consolidated into capability-centric entries; closure rounds group by subsystem (keyword machinery, any new DSL leaves/steps, token registry) exactly as the store-champs rounds did, each round TDD with DSL-level tests, clone-safety (resume frames, no closures), schema regen + vocab-doc drift gate, and tracker RESOLVED marks.

## Risks / Trade-offs

- **[No DCGO oracle — behavioral misreads ship undetected by parity tools]** → scan-grounded per-card Opus review (the store-champs omission catcher), conservative PARTIAL-over-guess policy, rulings lookups per card, and a re-audit pass tagged for when DCGO ships EX12.
- **[digimoncard.io text is paraphrased for new sets]** → per-card JSON demoted to hint; scans mandatory in every prompt; reviewers must flag any JSON-vs-scan divergence for `card_overrides.json` reconciliation.
- **[Engage semantics under-specified by reminder text]** → rulings check first; if unresolved, implement the literal reminder text and pin the open question in the keyword test file + tracker entry (never silently extrapolate from Vortex).
- **[Usage credits gate every agent phase]** → phases are independently resumable: the assessment workflow resumes by runId; waves are commit-gated so a credit outage never strands unmerged work.
- **[Guard on tokens]** → token registry entries must carry printed keywords through the same parse path as real cards; the Paishu test must exercise Guard from a token instance, not only from a printed card.
- **[Machine saturation during waves]** → cap concurrent implementers per wave (learned limit ~14 live agents); full-suite seals run between waves, not inside them.

## Migration Plan

No deployment/migration surface: additive card + engine work behind the existing embedded registry. Rollback = revert the wave commits (each wave is an atomic commit with green scoped suites). The final seal is a full `cards_behavioral` + `dsl` run before the change is archived.

## Open Questions

- ＜Engage＞: may the Engage attack target unsuspended opponent Digimon (Vortex-style) or players only? Does it work the turn the Digimon entered play? (Resolve from official Q&A/rulings during the keyword phase; the assessment batch notes may already answer this from the scan reminder text on EX12-019/060.)
- [Kotenken] token stats/keywords (EX12-034 Erlangmon) — confirm from the scan during assessment.
- Whether assessment surfaces DSL-vocabulary gaps beyond the two keywords (e.g. Shambala's SW/TB cross-references or Susanoomon's Lv7 clauses) — sizing of the closure rounds depends on this.
