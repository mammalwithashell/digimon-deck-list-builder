# Engine rules-conformance validation campaign — design

**Date:** 2026-10-15
**Status:** **approved 2026-10-15** with scope steer (see §0). Ver. 4.3 is canon. Fixes F2/F3 and Phase 0 dispatched as sessions.
**Owner intent:** "I want confidence in the simulator before I start training on meta decks. The TCG is a state machine; if the state machine and the effect primitives are correct, I have confidence."

## 0. Owner steer at approval (2026-10-15)

> "Let's not consider individual card tests but rather replacement effects, keywords, interruptive effects and general rules processing. I'm actually looking to relax the training gate because having agents write tests for every clause on a card and every interaction is proving infeasible and should be unnecessary if the underlying mechanics and engine can be provably correct."

Consequences for this design:

- **Scope is mechanics-first.** Priority order is now: §15-8-5 immediate-type (replacement) effects and §15-4 trigger states; §16 keywords (every keyword's lifecycle against its 4.3 text); §17 rule checks and §18 pending/overwrite processing; the attack / block / security / battle procedure (§11–14); turn procedure, playing, digivolution, using cards (§6–9). Card-information and terminology chapters (§2, §4) drop to "only where a mechanic depends on them" (DUAL, Arts, "also", "with X in its text"). Phase 4 (Q&A rulings) keeps only **rules-level** rulings.
- **The training gate changes shape** (§5b). Today `data/oracle_readiness.json` admits 75 of 3,045 cards and **0 of 5,097 decklists**; the per-clause-plus-interaction gate is the binding constraint on training, not a safety margin. The campaign's exit criterion is a gate that trusts proven mechanics plus light per-card checks, instead of an adjudicated verdict for every clause and interaction of every card.
- The question in §8.5 is answered: an open **mechanics** gap blocks training; an unmeasured card clause does not.

This document records what the 2026-10-15 investigation found, and designs the campaign that turns "confidence" into a measured, cited, re-runnable artifact. Nothing in the engine was changed by the investigation. The two confirmed divergences are filed in `docs/RUST_ENGINE_GAPS.md`; fixing them is step 1 of the campaign, not part of this document.

---

## 1. Why

Card coverage is already measured (per-clause DCGO exams, `data/oracle_readiness.json`, the training gate in rule 34). What is **not** measured is the layer underneath the cards: the turn state machine, the attack/block/security/battle procedure, the trigger-resolution loop, replacement ("would") timing, rule checks, and the effect primitives every YAML card lowers to. A bug there is systematic: it corrupts every card that touches it, and a DCGO per-card exam can only catch it when a scripted card happens to exercise the path.

RL training amplifies this. An agent will find and exploit any rules error that produces free value, and it will learn that exploit instead of the game.

## 2. What exists today (verified 2026-10-15)

### 2.1 Reference sources, and a stale canon

| Source | Role in `CLAUDE.md` | State found |
|---|---|---|
| `Digimon TCG resources/general_rule.pdf` | **Canonical** rules source | **Ver. 3.6 (2025-12-25).** The official manual is **Ver. 4.3 (2026-09-18)**: seven revisions behind (3.7, 4.0, 4.1, 4.2, 4.3). |
| `docs/digimon-rules/*` (digest, keyword table, index) | Verified cheap derivations | Stamped 3.6. Missing the 4.x keywords `<Use Req.>`, `<Ascension>`, `<Engage>`, `<Guard>`, `<Detach>`, `<Succession>`, `<Overflow>`, Arts Digivolve, DUAL cards. |
| DCGO (C#, base repo) | Behavioural authority per card | Present. Aegis independently audited its own trigger loop against DCGO and documents DCGO quirks (see §2.3). |
| Aegis (`github.com/vinicius3333/aegis-digimon-tcg`, clone `D:/tmp_aegis/a` @ `ec0cd22f0`) | Previously used only for card-IR drafts | Has a full **rules-conformance** layer we never looked at (§2.3). Ships a `pdftotext -raw` extraction of the **4.3** manual at `data/kb/rules/comprehensive.md`. |
| `data/card_qa.json` | Official Q&A rulings (6,671) | Current and larger than Aegis's copy (6,622). **No Rust test reads it**; only 33 of 727 DCGO exams carry `expect_ruling`. |

### 2.2 Our rules-level test corpora

| Corpus | Size | Notes |
|---|---|---|
| `tests/judge_quiz` | 44 tests, 30 questions, 1 ignored (Q8, Burst Digivolve EOT trash) | 29/30 pass. Good adversarial depth, narrow breadth. |
| `tests/rules_faq` | 31 tests | From the fandom FAQ page; ledger `qa/qa-reports/rules-faq.md` shows uncovered rows (MP-19/22/28, OR-5, MC/NL/IT clusters). Generator has **3** cases. |
| `tests/keyword_semantics_matrix` | 3 tests over `qa/rules/keyword_semantics_matrix.json` | Optional/mandatory per keyword. |
| Subsystem binaries (`combat` 257, `effect_context` 164, `replacements` 128, `option_flow` 140, `selection` 79, `cost_hooks` 78, `phase_flow` 72, `timing_dispatch` 51, keyword_phase_d/e/f 95, …) | ~2,000 tests | **Not in CI** unless their own files change (`impact_scope.py`). Only `lib`, `cards_behavioral`, `dsl`, `archetypes`, `judge_quiz`, `selection`, `clone_fuzz` run on every engine push. |

**Rule-citation density** (script in the investigation scratchpad, re-runnable): engine `src/` + `tests/` cite **168 of 1,070** Ver. 3.6 rule IDs (15.7%). By chapter: ch 7 Playing a Card **0%**, ch 18 Pending/Overwrite processing **0%**, ch 1 **4%**, ch 2 **6%**, ch 15 Effect Rules **17%** (42/248), ch 16 Keywords **23%**, ch 13 Security **41%**, ch 10 Link **50%**. The attack state-machine order (Raid → Alliance → Counter → Block), the security-phase order, the separate trigger batches, and replacement ordering are essentially uncited. `DCGO` is cited 743 times in `src/`; `general_rule` 38 times.

### 2.3 What Aegis gives us that we lack

- **`apps/api/src/engine/conformance/`** — 78 test files, ~700 tests, organised **one file per Comprehensive Rules chapter** (`ch01` … `ch18`, plus `ch15-01..04`, `ch16a..c`) and per keyword lifecycle. Every test `cite()`s a rules chunk id with a SHA-256 fingerprint of the exact rule text, so a rules revision fails the citation before it silently passes the test.
- **`data/kb/rule-obligations.json`** — 1,119 hyphenated 4.3 rule IDs, each tagged with one of 18 mechanisms (`effects-and-ordering` 244, `keywords` 206, `terminology-and-identity` 139, …) and a status. Aegis itself marks only **66 rules proven**; the rest `gap`. This is a ready-made **denominator**.
- **`docs/audits/engine/`** — ~100 engine audit notes, one per mechanism (`trigger-ordering.md`, `replacement-effects.md`, `security-removal-trigger-ordering.md`, `attack-end-before-counter.md`, `piercing-multiple-battles.md`, `dp-deletion-replacement-thresholds.md`, …), and **`engine-reference-sequencing.md`: Aegis vs DCGO trigger-loop audit** with DCGO file:line citations. It records DCGO facts we should not re-discover: rule-check sweeps are serialised although §17-1-3 declares them simultaneous; `trashExcessLinkCards` tail-trims where Q6370 says the player chooses; `BattleWithoutDigimon` is dead code.
- **`rules-v4-3-source-drift.md`** — their own 3-week migration from 4.2 to 4.3, listing the semantic reversals and which tests had to flip.

Aegis is **not** an oracle of truth: it has three open engine failures of its own, and 94% of its rule obligations are unproven. It is a **scenario and citation source**, and a third opinion when DCGO and the PDF disagree.

## 3. Findings

Ranked by expected impact on training. "Confirmed" = text of the rule read, our code read, DCGO read (or Aegis where DCGO was not needed). "Candidate" = flagged by code reading, not yet run.

### F1 — Our canonical rules manual is five revisions stale (confirmed)

A renumber-aware diff (scratchpad `diff_rules2.py`; 3.6 → 4.3: 133 rules added, 84 removed, 59 reworded, 367 renumbered-only) shows these **semantic** changes that touch the state machine or primitives:

| Rule (4.3) | Change | Engine relevance |
|---|---|---|
| **§15-7-4** | **Reversal.** An optional processing condition whose contents cannot be executed **cannot be chosen** (3.6 said it *could*). | Our open gap "unpayable optional costs still park a prompt" is now definitively a bug, and 99 code/test citations of `15-7-4` encode the old rule. Also matches owner ruling 6 (2026-10-10). |
| **§15-14-1-5** | OPT counters reset on turn change **and** whenever the card becomes a new card. | Verify DNA/leave-and-return reset. |
| **§15-13-3** | A *gained* `[On Deletion]` stays pending after the card is in the trash. | Gained-effect lifecycle across trash. |
| **§15-3-3 / §15-3-4** | Gained inherited-effect state carries through stacking; "this card" in inherited text = the digivolution card. | Inherited-effect identity. |
| **§15-1-2** | Optional conditions are decided **before** processing the effect. | Prompt timing. |
| **§15-15-7-4** | A mandatory invoking effect does **not** force the invoked effect's optional condition. | Borrowed effects. |
| **§15-15-7-2** | Multiple borrowed effects: choose one, process, choose next. | Borrowed effects. |
| **§15-12-2-3** | A persistent info-change always overwrites; reverts when lost. | Modifier layering. |
| **§4-27-4** | One trigger condition met several times at once triggers once. | Matches owner ruling 4. |
| **§4-29 "Also"** | "They also gain X" shares the prior processing's duration **and targets**. | DSL lowering of "also". |
| **§2-3-3-3 / -4** | "A card with `<Blocker>`" references only cards that **always** possess it; "keyword that executes processing" excludes conditional forms. | Predicates like `has_keyword`. |
| **§3-4-7-3..8** | Breeding-area cards can't be affected, triggered, chosen, or referenced except by effects that name the breeding area. | Aegis wave-6 lead (any_permanent vs breeding) confirms we have drift here. |
| **§11-5-1-4-1** | An unsuccessful attack still reaches end-of-attack timing. | `[End of Attack]` must fire on fizzled attacks. |
| **§13-1-2 / §16-7-5** | One security check per attack; `<Security A. +N>` modifies the *count inside one check*; multiple `<Piercing>` = one check. | Security loop. |
| **§16-7-4** | `<Piercing>` is pending processing "immediately before the transition to end-of-attack timing"; simultaneous with battle triggers, resolved after them. | Our `fire_piercing_or_finish` cites the old 16-6. |
| **§16-xx-4 (deleted ×17)** | The per-keyword "multiple instances overlap" rules are gone; replaced by generic §16-3 "specified cards". | Keyword multiplicity handling may now differ (Alliance ×2, Barrier ×2). |
| **§17-1-3** rewrite | Rule-check targets enumerated; excess link cards trashed (player chooses which, Q6370). | `run_state_based_rules_check` covers only 17-1-3-1 and 17-1-3-2-1. |
| **§8-2-2-1-7** | Digivolving the turn it was played removes the "played this turn" state. | Attack legality. |
| **§9-1-5** | Used Option trashed as pending processing immediately after its first `[Main]`; nothing interrupts. | Option lifecycle. |
| **§1-3-11** rewrite | Declaring utilisation: must perform as much optional processing as needed to pay. | Declare-then-pay flows (judge-quiz cluster C). |
| New keywords | `<Use Req.>`, `<Ascension>`, `<Engage>`, `<Guard>`, `<Detach>`, `<Succession>`, `<Overflow>`, Arts Digivolve, DUAL | Some implemented (Engage, Guard, Ascension, Detach exist in tests) but **never verified against their actual rule text**. |

### F2 — Security check: we battle the Security Digimon *before* resolving effects the check triggered (confirmed)

- Rule (identical in 3.6 §13-1-7 and 4.3 §13-1-8): reveal → **effects triggered by the security check resolve** → **then** battle against a Security Digimon → trash → next check. `[Security]` effects go first within that window (§15-16-10-2).
- DCGO (`CardController.cs` `ISecurityCheck.SecurityCheck`, ~4329–4600): collects `OnSecurityCheck` skill infos at reveal, resolves `[Security]` skill, then the collected triggered skills, **then** `#region battle with security Digimon`.
- Ours (`combat/mod.rs` `SecurityPhase`): `SecuritySkillDrain → BattleResolved → OnSecurityCheckDrain`. Observer effects ("when a security check is performed", "when your opponent's Digimon checks security") resolve **after** the battle. Any such observer that changes DP, deletes, or suspends changes the battle outcome.
- No existing test pins either order (`OnSecurityCheck` appears only in `st2_13.rs` as a negative and in `phase3d_event_context.rs` via direct enqueue). Filed as `G-ENGINE-SECURITY-CHECK-OBSERVERS-AFTER-BATTLE`.

### F3 — DNA digivolution keeps the base permanent's orientation (confirmed)

- Rule §8-2-2-1-1: the DNA result "digivolves unsuspended without carrying over the Digimon's orientation".
- Ours (`game/mod.rs` `dna_digivolve_inner`, ~2585–2650): permanent A is kept in place, B's sources and the hand card are appended, `turn_digivolved` is set. No orientation reset anywhere in the DNA path (`grep suspend` over the range is empty). A suspended base yields a suspended DNA result that cannot attack.
- DCGO builds a fresh permanent for a jogress result (`Effects.CreateFieldPermanentCardEffect(..., jogressEvoRoots)`), unsuspended by construction.
- Related 8-2-2-1 sub-rules to verify in the same fix: materials are "new cards" (effects applied to them end, §8-2-2-1-5; OPT resets, §8-2-2-1-6; link cards trashed before stacking, §8-2-2-1-2). Filed as `G-ENGINE-DNA-RESULT-KEEPS-BASE-ORIENTATION`.

### F4 — The trigger loop is fragmented into separate batches where the rules define one simultaneous pool (candidates)

§15-4-3-2: "If multiple effects are triggered before a single rule or effect is resolved, they all trigger simultaneously"; §15-4-3-5: turn player orders theirs one at a time, then the non-turn player. The code survey found these places where we drain **separate** batches instead:

| Site | Batches drained in sequence | Rule risk |
|---|---|---|
| `fire_on_attack` (`combat/mod.rs` ~4068–4156) | `OnAttack` → `WhenAttacking`+declaration `OnSuspend` → `OnAllyAttack` → `OnOpponentAttack` | Opponent's "when an opponent's Digimon attacks" resolves after all of the attacker's, which §15-4-3-5 already guarantees, **but** the turn player's own `OnAttack` vs `WhenAttacking` ordering is fixed by batch rather than chosen; derived triggers interleave wrongly. Adjacent open gap: `G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS`. |
| `<Alliance>` | Own `AttackState::AllianceOpen` window | §16-24 (4.3): Alliance is a trigger-type `[When Attacking]` effect; it belongs in the pool, ordered by the player with the other `[When Attacking]`s. |
| `commit_digivolve_from_hand_no_replace` and every digivolve path | `WhenDigivolving` → `OnDigivolve` | Same timing (§15-16-4); one pool. |
| `end_turn` | `EndOfYourTurn` → … → `EndOfOpponentsTurn` | Same timing; one pool with turn player first. Separate batches are equivalent only when no derived trigger crosses the boundary. |
| `delete_permanents_batch` | `WhenWouldLeaveBattleArea` then `WhenWouldBeDeleted`, **per permanent** | §15-8-5-3: immediate-type effects trigger simultaneously with each other; the whole batch is one window. Main's PR #737 landed `replacement_window.rs` with a `ReplacementOrder`; re-verify against this after the merge. |
| Single-source `[Security]` bundle (`effect_queue.rs` ~1634) | Fires in order, no rules check between | §17-1 rule checks happen between resolutions. |

Known-open and directly relevant: **PR #726** (inline drain of source-trash observers, §15-8-3-2) is still open; the owner's 2026-10-10 rulings 1–6 define the target and the memory notes it still mis-handles rules 4 and 5.

### F5 — Silent failure modes in the drainer (confirmed by reading)

- `MAX_CHAIN_DEPTH` overflow **clears the queue silently** (`effect_queue.rs` ~1303). A real infinite loop is a rules event (§18-3-3-2 says skip the step); silently discarding pending triggers is neither, and an RL agent cannot observe it.
- Pending-attack and security "trivial connect" paths return `AttackerWins` when the defender vanished; §11-5-1-4 says the attack is simply unsuccessful. The outcome label leaks into `battle_defender_deleted`/Piercing logic and events.

### F6 — Test infrastructure gaps

- ~2,000 subsystem tests never run in CI unless their own files change; a state-machine regression introduced via `src/` is invisible until the nightly `cards_behavioral`.
- No rules-chapter-indexed conformance suite; no per-rule denominator; no citation fingerprinting, so a rules revision cannot fail a test.
- The 6,671 official Q&A rulings are not a test corpus.

### Confirmed agreements (these raise confidence)

- Memory cap: a cost that would push the gauge past 10 is **refused** (not clamped) by ours (`pay_memory`, mask), by DCGO (`Player.MaxMemoryCost`), and by Aegis (`insufficient-memory`); consistent with §1-3-11-1 + §4-2-2 + §1-4-2-2.
- Deck-out only in the draw phase (§1-2-3-2); effect draws from an empty deck are not a loss.
- Turn-end check is deferred until all processing resolves (§6-1-4-1) via `check_turn_end`/`turn_end_check_deferred`.
- Rules checks run only at checkpoints, never inside effect processing (§17-1-2).
- Derived triggers first; turn player chooses first (§15-4-5, §15-4-3-5-1) in `drain_effect_queue_inner`.
- Judge quiz 29/30 (the 30th is a known primitive gap, not a wrong answer).

## 4. Goals, non-goals, anti-goals

**Goals**
1. A **measured** statement of rules conformance: for every normative rule in chapters 1, 3–18 of the current manual, a status of `proven` (test path + rule citation), `not-testable` (no normative content), or `gap`, with the denominator always printed.
2. Every confirmed divergence fixed TDD-first, or filed with a reproducer and a cited rule.
3. The canonical rules source is current, and a freshness check fails CI when Bandai publishes a new version.
4. The state-machine subsystem tests run on every engine change.

**Non-goals**
- Re-implementing cards. Card faithfulness stays with the DCGO exam loop (rule 34).
- Making Aegis a runtime oracle like DCGO (possible later; see open question 3).
- UI changes.

**Anti-goals (what would make this a failure even if green)**
- A conformance suite that asserts what the engine does instead of what the rule says. Every test is **discover-then-pin**: assert the rule; a red test is a gap entry, never a weakened assertion (same discipline as the judge quiz).
- Trusting any single source. Adjudication order is unchanged: PDF (current version) for rules, DCGO for card behaviour, official DB for printed data; Aegis is a third opinion and scenario source. When the PDF is silent and DCGO and Aegis disagree, the case is **escalated to the owner**, not guessed.
- Counting citations as coverage. A `cite()` proves identity, not behaviour (Aegis says this of itself; it holds here too).

## 5. The campaign

Six phases. Phases 0 and 1 are prerequisites; 2–4 are the work; 5 locks it in. Each phase ends in a committed artifact.

### Phase 0 — Make the canon current (small)
- Place the official **Ver. 4.3** `general_rule.pdf` (and the current `manual.pdf`/`glossary.pdf`) in the base repo's `Digimon TCG resources/` (git-ignored, base-only, per rule 32). *Owner action or owner-approved download.*
- Commit a text extraction (`docs/digimon-rules/cr-4.3.txt`, `pdftotext -raw`) so worktrees and tools can read the rules without the PDF, and a `code/tools/rules/check_rules_freshness.py` that compares the committed version stamp with the live header (Aegis has an equivalent; ours is a 50-line script).
- Regenerate `docs/digimon-rules/` for 4.3: keyword table (add the 9 new mechanics, drop the 17 deleted overlap rules), digest, `rules-index.json` page map. Re-stamp the SessionStart hook table.
- Commit the 3.6→4.3 semantic delta (table in §F1, expanded) as `docs/digimon-rules/changelog-3.6-to-4.3.md` so every engine citation of a changed rule can be audited. Grep and re-point the 99 `15-7-4` citations.

### Phase 1 — Build the denominator (small)
- `qa/rules/cr_obligations.json`: every hyphenated rule ID in 4.3 with its text, chapter, a mechanism tag (reuse Aegis's 18-mechanism taxonomy so the two ledgers are comparable), and `status: gap | proven | not-testable` + `tests: [...]`. Generated by `code/tools/rules/build_obligations.py` from the committed extraction; statuses are hand-maintained in a sidecar so regeneration never loses verdicts.
- Seed `not-testable` from headings/TOC (Aegis's `not-testable.ts` lists 91; ours will be similar).
- Seed `proven` by scanning existing tests for rule citations (the 168 already there) **only after a human reads each test and confirms it asserts the rule** — a citation in a comment is a lead, not proof.
- `code/tools/rules/coverage_report.py` prints the per-chapter table (like §2.2) and is wired into verify tier 0 as a non-failing report, exactly as Aegis does, until coverage is high enough to enforce.

### Phase 2 — Rules-conformance test suite (the bulk)
New Cargo test target `code/digimon-engine/tests/rules_conformance/`, one module per chapter (`ch06_game_procedures.rs`, `ch11_attacking.rs`, …), `DebugRunner` (`src/debug_runner.rs`: `place_stack`, `push_source`, `add_to_hand`, `inject_trash`, `force_base_dp`, `attack_digimon`, `battle_digimon`) plus the synthetic-card helpers under `tests/support/` and `tests/archetypes/support.rs`, each test opening with `cite!("15-4-3-5-1", "<fingerprint>")` — a macro that looks the ID up in the committed extraction and fails if the text's hash moved. Port order by state-machine criticality, using Aegis's chapter files as the **scenario checklist** (not copied assertions: each scenario is re-derived from the 4.3 text and cross-checked against DCGO where behaviour is involved):

1. **ch15-02 Timing & resolution** (§15-4 trigger states, §15-8 categories) and **ch15-01 / 15-03 / 15-04** — the trigger loop. This is where F4 is adjudicated: write the single-pool tests for `[When Attacking]`+Alliance, `[When Digivolving]`+`[On Digivolve]`, end-of-turn, batched replacements; red tests become gap entries with the rule cited.
2. **ch11–14 Attack / Block / Security / Battle** — F2 lands here, plus §11-5-1-4-1, §13-1-2, §16-7-4..6 Piercing, §16-24 Alliance persistence, §12-1-x block constraints.
3. **ch17 Rule checks + ch18 Pending/Overwrite** — all §17-1-3 targets (we cover 2 of 8), §17-1-2 not-during-processing, §18-1 pending processing, §18-2 "instead", §18-3 loops (F5 silent clear).
4. **ch06–09 Turn procedure, playing, digivolution (F3 here), using cards** — §6-6-4 postponed turn end, §7-1-2-1, §8-2-2-1-x DNA, §8-3 Burst (Q8 gap), §9-1-5 Option disposal.
5. **ch16 Keywords** — one lifecycle module per keyword against its 4.3 text; the 9 new mechanics first because they have never been checked; then the 17 whose "overlap" rule was deleted.
6. **ch01–05, 10** — identity, zones, memory, link; mostly cheap.

Target: every `effects-and-ordering`, `attack-and-timing`, `security`, `battle`, `blocking`, `rule-checks`, `turn-flow`, `digivolution-and-costs`, `play-and-costs` obligation (≈ 600 rules) has a status other than `gap` by the end of the phase. Chapters 2 and 4 (terminology, card information, ≈ 250 rules) are second priority.

### Phase 3 — Fix confirmed divergences (TDD, each its own PR)
Order: F2 (security order), F3 (DNA orientation + new-card semantics), the F4 single-pool items that Phase 2 turned red, F5 (loop guard emits an event and stops the step instead of clearing), then the F1 reversals (§15-7-4 unpayable-condition gate; §4-29 "also"; §3-4-7 breeding isolation; §17-1-3 remaining targets). #726 is reworked to the owner's rulings 1–6 as part of the F4 work, not separately.

Every fix cites the rule in the commit and flips the obligation to `proven`. A fix that DCGO contradicts is held until the owner rules (rulings worksheet pattern, memory `feedback-rulings-worksheet-pages`).

### Phase 4 — Rulings as a corpus
- Mine `data/card_qa.json` for **rules-level** rulings (those whose answer cites a procedure rather than a card quirk — roughly: mentions of "simultaneously", "pending", "rule check", "end of attack", "security check", "new card", "once per turn"). Expect low hundreds. Each becomes a `rules_conformance` test with `qa: Qnnnn` in its citation, or a DCGO exam `expect_ruling` where a real card is required.
- Fold the official **Rule Q&A** page (general rulings, not per card; Aegis notes it has no ingestion for it either) into `scrape_official_qa.py` as a second corpus.
- Grow `rules_faq` from 31 by closing the ledger's uncovered rows.

### Phase 5 — Lock it in
- `verification-ladder` tier 1 runs `rules_conformance` and the state-machine subsystem binaries (`combat`, `replacements`, `effect_context`, `option_flow`, `cost_hooks`, `phase_flow`, `timing_dispatch`, `deletion_batching`) on every engine-path PR, not only when their own files change. Measured cost first; split like `cards_behavioral` if needed.
- `check_rules_freshness.py` in tier 0; a new manual version fails the build with the update-history diff printed.
- Coverage report becomes an enforcing floor once Phase 2's targets are met.

## 5b. The training gate: from per-clause oracle verdicts to mechanics conformance

**Today (rule 34, `2026-10-04-dcgo-oracle-readiness-design.md` §4.7):** a decklist is admitted only if every card is `ready`: every printed clause **and** every gating interaction (official Q&A + promoted probes, `data/interaction_denominator.json`) has an adjudicated DCGO verdict. Result on main: 75 ready cards, 0 admissible decklists. Cost ≈ $1.72 per clause plus interaction exams, and the pool grows faster than adjudication.

**Target:** two layers, both artifacts committed and `--check`ed like today's.

1. **Mechanics gate (global, blocking).** `data/mechanics_readiness.json`, generated from `qa/rules/cr_obligations.json` (Phase 1): training is admitted only if **no** obligation in the mechanics set is an open `gap` flagged `blocks_training`, and the `rules_conformance` suite is green at the committed manual version. The mechanics set is the union of these taxonomy tags: `effects-and-ordering`, `keywords`, `attack-and-timing`, `security`, `battle`, `blocking`, `rule-checks`, `turn-flow`, `digivolution-and-costs`, `play-and-costs`, `card-use-and-costs`, `link`. `blocks_training` is set on a gap when the divergence can change a game outcome (ordering, legality, deletion, memory, security); a wording-only or UI-only gap does not block. The artifact prints the denominator (`N obligations: proven / not-testable / gap(blocking) / gap(non-blocking)`) and never a bare "ready".
2. **Per-card light gate (per decklist, blocking).** A card is admissible when: it is in the Rust registry (today's `unregistered` check, kept); its YAML/raw-rust implementation has a behavioural test module and no `FAILED` verdict in `validated_cards_dsl.json` / `validated_cards.json`; its authored text matches the official bundle (`text_sha256` parity against `data/card_official.json`, which already invalidates verdicts on drift); and it has **no `diverged` exam verdict** that lacks a `dcgo_quirk` citation. **`unmeasured` no longer blocks.** A known-wrong card still cannot train; an un-examined card can.

**What stops being a gate and becomes a report:** `oracle_readiness.json`'s clause/interaction `ready` status, and the interaction denominator. The DCGO exam loop stays as an **opt-in deepening** for the archetypes a run targets (`--require-oracle-ready <archetypes>` for someone who wants the old strictness on a slice), and as the mechanism that produces `diverged` findings. Rules-level Q&A rulings move into the conformance suite (Phase 4).

**Rollout:** artifact `version: 3` with `gating: ["mechanics", "card_light"]`; `gauntlet.py` learns the two-layer admission with the same fail-fast `EmptyTrainingPoolError` and blocker report; a `--legacy-clause-gate` flag reproduces today's pool for comparison during the transition and is removed once Phase 2 reaches its mechanics target. Tests keep injecting their own ready sets (`conftest.py`), unchanged. Rule 34 in `CLAUDE.md` is rewritten when the switch lands. This is the campaign's **exit criterion**: the day the mechanics artifact is clean and the suite is green, the gate flips.

**Honesty clause:** the light gate trusts that a card *lowers* correctly onto proven mechanics. It does not prove each card's text was authored faithfully; that residual risk is what the opt-in exams, the four systemic drift-family checks (memory `project-buglist-faithfulness-campaign`), and `text_sha256` parity are for, and the design says so rather than claiming per-card proof.

## 5c. Idiom library: lift repeated standard effects out of card YAML (owner request, 2026-10-15)

**Request:** "fold in an improvement to the DSL of maybe lifting the prompts away from the YAML for some repeated standard effects like end of turn DNA digivolve."

**Census (2026-10-15, 1,099 YAML cards, 2,977 clauses):** 92 clause *shapes* are shared by three or more cards and together cover **999 clauses (34%)**. The DSL already names some of them (`grant_keyword`, `aura`, `flood_gate`, `replacement`, `delay`, `link_condition`, and compound steps such as `play_from_security` 124 cards, `place_self_as_delay_option` 86, `may_dna_digivolve_now`), but authoring is inconsistent: the end-of-turn DNA effect exists in a compact form (`may_dna_digivolve_now`, 17 cards) **and** as a hand-rolled `optional → select_hand → select_dna_pair → effect_initiated_dna_digivolve` sequence with per-card prompt strings (15 cards, e.g. AD1-009, AD1-012, BT18-015, EX3-008). The Tamer memory setter (`[Start of Your Turn] If you have N or less memory, set it to 3`) appears on 30 cards with four parameter variants; `[Start of Your Main Phase] if the opponent has a Digimon, gain 1 memory` on 12; `[On Security] Play this card without paying the cost` on 124. Each hand-rolled copy is a place where a prompt can be mis-authored (optional on a mandatory step, wrong filter, missing `scope`), and each is tested, if at all, only through that card's own test.

**Design: `idiom` clauses lowered by the DSL compiler.**

- A clause may be written as `- idiom: <name>` plus named parameters, e.g.
  `- idiom: end_of_turn_dna_digivolve` with `into: {name_contains: "Omnimon Alter-S"}`, `cost: 0`, `scope: inherited|face_up`, and the printed `summary`. The compiler (`digimon-dsl` `compile_clause`) expands the idiom into exactly the `CompiledClause` the long form produces today; **the engine does not change** and the pack format does not change. Idioms are defined once, in Rust (`code/digimon-dsl/src/idioms/<name>.rs`), as a function from parameters to a `ClauseSpec`, so they are validated and linted like any authored clause and appear in the JSON schema with their parameter list.
- **One conformance test per idiom**, in `tests/rules_conformance/idioms_*.rs`, proving the idiom against the rules it touches (for the DNA idiom: §15-7 optionality, one declaration, §8-2-2 DNA requirements and new-card semantics incl. §8-2-2-1-1 orientation, §8-2-3-3 draw, `[When Digivolving]` fires, §15-4-3-5 pool ordering at end of turn) and against the printed phrasing family it implements. Once an idiom is proven, every card that uses it inherits the proof; this is the mechanics-first thesis applied to card authoring.
- **Migration is behaviour-preserving by construction:** a tool compiles the long form and the idiom form of each candidate card and requires byte-identical `CompiledClause` IR before rewriting the YAML (through the existing `pretty::format_spec` canonical printer). Cards whose long form differs from the idiom's expansion are either genuinely different (left alone, documented) or mis-authored (a finding; the diff is the evidence).
- **Discovery is a lint:** the census becomes `dsl-lint --idioms`, which reports clauses structurally matching an idiom's expansion but authored long-hand, so new cards converge on idioms. Per-card prompt strings disappear into the idiom's canonical prompts (localisation keys derive positionally as today).
- **First idioms** (by card count and by prompt complexity): `security_play_self` (124), `start_of_turn_memory_floor` (30; params `lte`, `set_to`), `start_of_main_gain_memory_if_opp_digimon` (12), `end_of_turn_dna_digivolve` (17 + 15 → one form), `security_place_self_as_delay` (32), `delay_gain_memory` (18), `when_attacking_draw_if_hand_lte` (7), and the replacement-keyword bodies (`detach`, `decode`, 7 each) that today re-author `<Detach>`/`<Decode>` as bespoke replacement clauses.

**Non-goals:** no macro language inside YAML (no anchors, includes, or templating); idioms are a closed, Rust-defined, schema-visible set, each with a conformance test. A card that does not fit an idiom keeps the long form.

Plan: `docs/superpowers/plans/2026-10-15-rules-conformance-3-idiom-library.md`.

## 6. Decisions made in this design (owner can override)

- **Chapter-indexed Rust tests with text-fingerprinted citations**, not a port of Aegis's TypeScript tests and not a YAML scenario format. Rationale: `DebugRunner` already exists, the judge quiz proved the pattern, and fingerprinting is the only cheap defence against the stale-canon failure we just found.
- **Reuse Aegis's 18-mechanism taxonomy** for obligations so the two projects' ledgers can be compared rule-for-rule later.
- **Aegis stays read-only** (scenario and citation source). Running it as a live oracle is deferred (open question 3).
- **Discover-then-pin** everywhere; red tests file gaps, never assert engine behaviour.
- **Synthetic cards** in conformance tests (rules don't depend on printings); real cards only where a rule's example names a mechanic that needs printed text (DUAL, Arts, Overflow).

Settled after the builder check (my calls; the owner can override any of them):

- **PDF vs DCGO's engine.** `CLAUDE.md` already rules it: on a rules question the PDF governs, so a divergence where DCGO's *loop* contradicts explicit 4.3 text (serialised rule checks, Alliance in its own window, tail-trimmed link cards) is fixed, not held. Only cases where the PDF is silent or ambiguous wait for the owner, and those are **batched into one rulings worksheet at the end of each Phase 2 chapter**, not raised PR by PR, so Phase 3 is not owner-bandwidth-bound.
- **No training-pool shrinkage from conformance fixes.** A Phase 3 fix that makes a previously `confirmed` DCGO exam `diverged` must, in the same PR, re-triage those verdicts as `dcgo_quirk` with the rule citation, so `oracle_readiness.json` does not lose cards. The campaign pre-registers the known DCGO quirks (from Aegis's `engine-reference-sequencing.md` and §F4) in `qa/rules/dcgo_quirks.md` so the re-triage is a lookup. Substrate-wide fixes (§3-4-7 breeding isolation changes `any_permanent` for every card) are in scope under the same rule.
- **Bootstrap from Aegis's 4.3 extraction** without waiting for the PDF. Fingerprints are SHA-256 over *normalised* text (whitespace collapsed, glued page numbers stripped, unicode dashes/quotes folded), so a later re-extraction from our own PDF with a different poppler build does not invalidate them. Only the `rules-index.json` page map waits for the PDF.
- **Fingerprint failures never block card PRs.** The suite is pinned to the committed manual version; `cite!` fails only the `rules_conformance` job, and only on a *text* change for a cited ID. The freshness check is a separate tier-0 report. Migrating to a new manual version is a deliberate PR that uses the renumber-mapping tool (the investigation's `diff_rules2.py`, productised in Phase 1) to re-point citations.
- **Vendor the Aegis inputs we depend on** (4.3 extraction, obligation taxonomy, the audit notes we cite) into `qa/rules/aegis/` with MIT attribution and the source commit hash, instead of reading a temp clone or adding a submodule. Rule-for-rule ledger comparison later can re-fetch at that hash.
- **The owner's 2026-10-10 trigger-timing rulings 1–6 are 4.3-compatible** and are the pin for the Phase 2 single-pool tests: 1 ↔ §15-8-3-2, 2 ↔ §15-4-3-5, 3 ↔ §15-4-5-2, 4 ↔ §4-27-4 (new in 4.0, confirms the ruling), 5 ↔ §15-4-2-4, 6 ↔ §15-7-4 (reversed in 4.3, confirms the ruling). The rulings are silent on Alliance; §16-24-2 (trigger-type, `[When Attacking]`) puts it in the pool.

## 7. Left to the builder

- Exact macro shape for `cite!` and where the fingerprint file lives.
- How `cr_obligations.json` statuses survive regeneration (sidecar vs. inline).
- Test-count split per CI job.
- Which of Aegis's ~700 scenarios are redundant with existing subsystem tests (reading each existing test is part of Phase 1 seeding).

## 8. Open questions for the owner

**Resolved 2026-10-15:** 1 → yes, 4.3 is canon. 2 → yes, fix now (sessions proposed). 5 → an open *mechanics* gap blocks; unmeasured card clauses do not (§5b). 3 and 4 remain open; defaults below stand until the owner says otherwise.

1. **Adopt Ver. 4.3 as canonical now?** It changes `CLAUDE.md`, the hooks, `docs/digimon-rules/`, and reverses §15-7-4 behaviour. (Recommended: yes; the alternative is validating against a manual Bandai no longer publishes.)
2. **Fix F2 and F3 immediately** (two small TDD PRs) or hold them for Phase 3 ordering? (Recommended: immediately; both are state-machine bugs with no card dependence.)
3. **Aegis as a live oracle?** Installing its toolchain (pnpm, Colyseus) lets us run a scenario through three engines; cost is a day of setup plus a TypeScript harness. (Recommended: not now; revisit after Phase 2 shows how often PDF+DCGO leave a case undecided.)
4. **#726**: rework inside this campaign (Phase 3/F4) or land as-is first? (Recommended: rework here; the owner's rulings already supersede its current behaviour.)
5. **Does an open rules-conformance gap block training?** This is the campaign's exit criterion. Options: (a) a `blocks_training` flag on gap entries in the `attack-and-timing`, `security`, `battle`, `blocking`, `rule-checks`, `effects-and-ordering` and `turn-flow` mechanisms gates `gauntlet.py` admission globally, the way an unready card does per decklist; (b) report-only, and the owner reads the coverage table before launching a run. (Recommended: (a); it is what "confidence before training" means operationally, and retrofitting the flag later means re-triaging every gap.)

## 9. Investigation artifacts (session scratchpad, not committed)

- `rules/cr_3_6.txt` (pdftotext of our PDF), `rules/cr_4_3.md` (copy of Aegis's 4.3 extraction), `rules/diff_rules.py`, `rules/diff_rules2.py` (renumber-aware diff), `rules/diff2.txt` (full 3.6→4.3 report), `rules/cite_coverage.py` (citation density). Phase 0 commits durable versions of the first two and Phase 1 the scripts.
