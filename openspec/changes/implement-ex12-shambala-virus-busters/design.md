# Design: Implement EX12 Shambala + Virus Busters slices

## Context

EX12's data layer is fully landed (commit `e53df0d8e`): all 77 cards in `data/cards.json` (overrides re-applied), per-card JSONs in `code/digimon-engine/cards/ex12/`, card scans mirrored to the lookup-skill cache, lexicons refreshed, and the three DUAL cards' Option-face colors face-verified. The two target slices — Shambala (33) and Virus Busters (21) — have zero implementations.

**Update 2026-07-07 — DCGO now covers EX12.** The change was originally drafted as the first set authored without the DCGO oracle; that premise no longer holds. The DCGO submodule was bumped to upstream `a5e66480b` (Beta 1.16.9+), which implements **all 77 EX12 cards** (`$BASE_DCGO/Assets/Scripts/CardEffect/EX12/<Color>/EX12_*.cs`, rule-29 base-repo resolution) and ships first-class ＜Guard＞ and ＜Engage＞ keyword machinery (`Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/{Engage,…}.cs`, `CardEffectFactory/KeyWordEffects/{Guard,Engage}.cs`). Official-DB bundles remain unavailable (`world.digimoncard.com` mid-restructure). The keyword gate already found the two new printed keywords (＜Guard＞, ＜Engage＞) and at least one new token species ([Paishu]). An 8-batch assessment workflow was authored (`workflows/scripts/ex12-shambala-vb-assess-wf_6f7700f2-6c5.js`, run `wf_6f7700f2-6c5`) but died on usage credits before any batch ran — its prompts predate the DCGO update and must be refreshed to embed per-card DCGO paths before any re-run.

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
- Recording-based DCGO parity replay for EX12 (`dcgo-replay` needs JSONL recordings from the modded client, which don't exist for EX12 yet). Static C#-consultation during authoring/review IS in scope; replay-funnel parity is a follow-up once EX12 recordings exist.

## Decisions

**D1 — Keywords first, as engine substrate, before any card wave.**
Guard and Engage gate 5+ cards and tokens; implementing them per-card would fork semantics. Guard auto-emits a protect-others leave replacement (the round-3 `protect_others` substrate: cost `delete_self`, outcome `prevent`) with the cause scope narrowed to *opponent effects* (the printed reminder text's "by your opponent's effects" — narrower than the generic substrate default; the replacement-cause filter already models this via `replacement_cause` predicates). Engage adds an end-of-turn optional attack window; it shares Vortex's EOT machinery but is a distinct `Keyword` variant. **DCGO's `Engage.cs` (behavioral authority) settles the semantics**: the attack may target the player OR any opponent Digimon (`canAttackPlayerCondition: () => true`, `defenderCondition: (permanent) => true`), and — unlike Vortex, which passes `isVortex: true` into `CanAttack` — Engage uses the plain `CanAttack` gate, so there is **no played-this-turn allowance** (normal summoning sickness applies). Cross-check against wiki rulings during the keyword phase; if an official ruling contradicts DCGO, the rules source wins and the divergence is logged. Alternative considered: express both purely in per-card YAML `grant_keyword`+clauses — rejected because printed keywords must parse from card text (tokens carry them; future sets reuse them).

**D2 — Scan + DCGO authoring discipline (updated 2026-07-07; supersedes the original "scan-first, no DCGO").**
Standard source priority applies now that DCGO ships EX12. For **printed text**: card scan (`.claude/skills/digimon-card-lookup/.cache/EX12-*.webp`) is authoritative; per-card JSON (digimoncard.io) is paraphrased for this set — hint only, divergences reconciled into `card_overrides.json`. For **behavior**: `general_rule.pdf` + `docs/digimon-rules/` for pure rules, then the card's DCGO C# (`$BASE_DCGO/Assets/Scripts/CardEffect/EX12/<Color>/EX12_<NNN>.cs`, underscore naming) for how the card resolves, then official rulings (wiki card pages, fetched via browser when bot-blocked). Every implementer and reviewer prompt embeds this order and requires reading BOTH the scan and the DCGO script. Caveat: EX12 C# is freshly authored upstream and not yet battle-tested — a scan-vs-DCGO conflict is flagged for adjudication (rulings/PDF), not silently resolved either way. Where text is ambiguous and neither DCGO nor a ruling settles it, the card ships PARTIAL with a named tracker entry rather than a guessed behavior.

**D3 — Refresh the assessment workflow's prompts for the DCGO oracle, keep its structure.**
The 8-batch audit script's structure is right (pre-named gap ids G-KEYWORD-GUARD/G-KEYWORD-ENGAGE, consolidation stage), but its per-card prompts were authored under the no-DCGO premise. Before re-running, update the prompts to require reading the card's DCGO C# alongside the scan (per D2) — this materially improves verdict quality (behavioral edge cases DCGO already resolves would otherwise land as PARTIAL/BLOCKED guesses). The original run (`wf_6f7700f2-6c5`) died on credits before any batch ran and its script lives in a prior session dir, so nothing is lost by re-authoring; re-run live once credits allow.

**D4 — Wave structure mirrors store-champs, sized to the slices.**
Waves of ~8–16 cards grouped by sub-engine (Shambala: eggs+Lv3, Lv4–5 SW, Lv4–5 TB, Tentei Hachibushu Lv6s + Susanoomon + Options; VB: Gammamon line + DUAL, Agu/Gabu lines, Omnimon/support/Tamer). Each card: Sonnet-class implementer in an isolated worktree with a worktree-sync preamble, returning FULL yaml/test bodies via structured output under the honesty contract; Opus reviewer judging the embedded bodies against the scan AND the card's DCGO C# (file-not-on-disk is never a rejection). Orchestrator merges via entity-unescape + 3-way patches, runs scoped suites, records verdicts, commits per wave. The DUAL Siriusmon follows the shipped dual-YAML shape (ST23-09/ST24-07/BT25-043/057; BT25-085 if merged by then).

**D5 — Gap closure is batched by subsystem, not by card.**
Assessment output is consolidated into capability-centric entries; closure rounds group by subsystem (keyword machinery, any new DSL leaves/steps, token registry) exactly as the store-champs rounds did, each round TDD with DSL-level tests, clone-safety (resume frames, no closures), schema regen + vocab-doc drift gate, and tracker RESOLVED marks.

## Risks / Trade-offs

- **[DCGO's EX12 scripts are freshly authored — the oracle itself may carry bugs]** → keep the scan-grounded per-card Opus review (the store-champs omission catcher) with DCGO as a cross-check, not a blind source; scan-vs-DCGO conflicts adjudicated against rulings/`general_rule.pdf` and logged; conservative PARTIAL-over-guess policy stands. Upstream EX12 fix commits are still landing (e.g. EX12_033/034/059/064 were revised post-release) — re-check the submodule tip before each wave.
- **[digimoncard.io text is paraphrased for new sets]** → per-card JSON demoted to hint; scans mandatory in every prompt; reviewers must flag any JSON-vs-scan divergence for `card_overrides.json` reconciliation.
- **[Engage semantics under-specified by reminder text]** → resolved from DCGO's `Engage.cs` (player-or-Digimon targets; no played-this-turn allowance — see D1); wiki rulings cross-check during the keyword phase; a contradicting official ruling wins and gets logged.
- **[Usage credits gate every agent phase]** → phases are independently resumable: the assessment workflow resumes by runId; waves are commit-gated so a credit outage never strands unmerged work.
- **[Guard on tokens]** → token registry entries must carry printed keywords through the same parse path as real cards; the Paishu test must exercise Guard from a token instance, not only from a printed card.
- **[Machine saturation during waves]** → cap concurrent implementers per wave (learned limit ~14 live agents); full-suite seals run between waves, not inside them.

## Migration Plan

No deployment/migration surface: additive card + engine work behind the existing embedded registry. Rollback = revert the wave commits (each wave is an atomic commit with green scoped suites). The final seal is a full `cards_behavioral` + `dsl` run before the change is archived.

## Open Questions

- ~~＜Engage＞ target legality / played-this-turn~~ — **answered by DCGO** (2026-07-07): targets player or any opponent Digimon; no played-this-turn allowance (plain `CanAttack`, no `isVortex` flag). Confirm against wiki rulings during the keyword phase; a contradicting official ruling wins.
- ~~[Kotenken] token stats/keywords~~ — **answered by DCGO** (EX12_034.cs effect description): Digimon / Black / 9000 DP / ＜Blocker＞. Verify against the EX12-034 scan during assessment (note: NOT Yellow like [Paishu]).
- Whether assessment surfaces DSL-vocabulary gaps beyond the two keywords (e.g. Shambala's SW/TB cross-references or Susanoomon's Lv7 clauses) — sizing of the closure rounds depends on this. The refreshed assessment can now lean on each card's DCGO C# to classify required primitives precisely.
