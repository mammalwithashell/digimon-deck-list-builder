## Context

The TCG-Judges' rules quiz is a 30-question gauntlet of timing/immunity/rules-check edge cases, each with an official correct answer and a rules rationale. The full question set, answers, and rationales were extracted and are reproduced in the cluster map below. The quiz is barely a *card* corpus — it is a *rules* corpus: ~30 cards are vehicles for roughly 7 underlying rules. That reframing drives the whole design: tests are organized by the rule cluster they pin, not by card.

The ~70 distinct referenced cards, their exact `card_id`s (read off the PDF card images), and
implementation notes are in [`card-resolution.md`](./card-resolution.md). Key facts:

- **Card IDs are now authoritative** from the PDF, not guessed. Confirmed DSL implementations against the correct IDs: **AD1-025, EX1-068, BT17-095, BT22-042, EX4-074, BT6-084, BT1-090** (7). Status of the rest is unconfirmed and must be re-derived against correct IDs (§1.3) — the first inventory used wrong printings.
- **0 behavioral tests** across the whole set.
- **Q4 RESOLVED:** the Q4 Aldamon is **AD1-002** (it has no Security Attack clause of its own). The `<Security A. +1>` comes from **Atomic Inferno (BT4-098)** targeting Aldamon, and the `<Security A. −1>` from **Holy Flame (ST3-15)** checked from Player B's security — netting base 1 + 1 − 1 = 1 check.
- **Q18 RESOLVED → BT17-077.** The card image literally reads "Imperialdramon: Paladin Mode **ACE**" with `Overflow −5`; it carries the `[Hand][Counter] <Blast Digivolve>` clause Q18 needs (the `cards.json` ACE field simply didn't parse). **Q18 unblocked.**
- **`Petrification token`** is not a card — it is a token already present at `code/digimon-engine/src/cards/tokens/petrification.rs`.

The constraint throughout is CLAUDE.md's no-approximations policy: authored cards implement their **full** printed text (not a quiz-scoped subset), every choice surfaces through `pending_selection` (§17), and a card or clause that can't be faithfully expressed is `BLOCKED`, never stubbed.

### The 30 questions → rule cluster → cards → judge answer

The authoritative per-question card-resolution table (with exact `card_id`s read off the printed
card images in the PDF, plus the full board state and judge answer for each question) lives in
[`card-resolution.md`](./card-resolution.md). It is the frozen output of tasks.md §1.1 and is the
source of truth for which printing each question uses.

Cluster tallies: A=5 (Q1,2,17,18,28), B=6 (Q6,7,8,13,14,24), C=4 (Q5,26,27,30), D=5 (Q9,19,20,21,23), F=4 (Q10,11,12,22), E=5 (Q15,16,25,29,30), G=2 (Q3,4). (Q30 spans C and E.) ≈70 distinct cards.

**Inventory caveat:** the initial name-based inventory guessed the WRONG printing for the majority
of these cards (e.g. it had Medusamon as BT21-029, but the quiz uses BT24-017; Venusmon BT10-042 vs
the quiz's BT24-040; LordKnightmon X AD1-018 vs BT19-073; ShoeShoemon BT22-032 vs P-165; Puppetmon
BT2-049 vs EX10-020; the entire Bagra stack in Q29 is EX10, not the BT7/BT10/BT11/BT12 printings
guessed). `card-resolution.md` supersedes it. Implementation status must be re-derived against the
correct IDs (§1.3) — the only confirmed DSL matches against correct IDs are AD1-025, EX1-068,
BT17-095, BT22-042, EX4-074, BT6-084, BT1-090.

## Goals / Non-Goals

**Goals**
- Encode all 30 quiz scenarios as behavioral tests asserting the official judge answer, using the real referenced cards.
- Author every referenced card to full faithful text with its own per-card behavioral tests.
- Run discover-then-pin: surface the rules-engine gaps the quiz exposes, fix them TDD, and pin the corrected behavior.
- Produce a per-question verdict ledger reconciled to `cargo test` reality.

**Non-Goals**
- A general engine/DSL refactor. Each gap fix is the minimum surface to make the failing scenario pass faithfully.
- Re-deriving rules from DCGO. DCGO is the behavioral tiebreaker only; printed text + `RULES_CONTEXT.md` come first (CLAUDE.md source priority). The judge answer + rationale is the spec; DCGO confirms the *how*.
- Quiz-scoped partial card implementations. A card enters the suite only when its full text is faithfully authored.
- Python-engine parity. The Rust engine is the source of truth for this suite.

## Decisions

### D1 — Real cards, authoring in scope, cluster-gated, gap-finding-first
Per the exploration decision, the suite uses the exact cards the judges chose, and authoring the 39 missing cards is in scope. The explicit primary purpose is to **find and patch engine gaps** — so authoring is gated by rule cluster *ordered by gap-likelihood* (A → B → C → D → E → F → G), and the free discovery wave (D6) runs first to rank divergence and can re-order the phases. A cluster the discovery wave shows the engine already handles can be deprioritized. Rationale: synthetic stand-ins would not prove the engine reproduces the *actual* interaction the judges adjudicated, which is the entire value of an external oracle; and an external oracle's worth is the gaps it exposes, so sequence to expose them fastest.

### D2 — Discover-then-pin: the assertion is the judge answer, never the engine's current output
Every scenario test asserts the official judge-correct outcome. If the engine disagrees, the test FAILS and the failure is logged to `engine-gaps.md` with the DCGO citation — we do **not** weaken the assertion to match buggy behavior, and we do **not** `#[ignore]` it to hide the failure (an ignore is permitted only when a scenario is blocked on an unimplemented card/primitive, citing the specific blocker). Rationale: the quiz's worth is as an adversarial oracle; a test that asserts wrong behavior is worse than no test.

### D3 — Organize tests by rule cluster, not by card
Tests live in `tests/judge_quiz/` with one module per cluster (`a_immunity_scope.rs`, `b_deferred_rules_check.rs`, …). Each test name encodes the question (`q1_belphemon_ends_attack_through_progress`) and its docstring quotes the question, the judge answer, the `RULES_CONTEXT.md` citation, and the `DCGO/` reference. Rationale: the quiz is a rules corpus; clustering documents *which rule* each test pins and makes a cluster-wide engine gap obvious (e.g. all of Cluster B failing ⇒ deferred-rules-check ordering is broken).

### D4 — Gating spike resolves the corpus before any test is written
Several questions reference an ambiguous printing, and `Imperialdramon: Paladin Mode ACE` may not exist in our data. The spike (Task 1) pins the exact `card_id` per question, flags any `BLOCKED-DATA` scenario, and confirms which of the 11 implemented cards are the printings the quiz means. Rationale: writing a test against the wrong printing is a silent false oracle. The spike output is a frozen per-question card-resolution table.

### D5 — Authoring rides the existing batch pipeline; the quiz test is layered on top
The 39 cards are authored through `/batch-implement-cards-rust-dsl` (full text, per-card TDD tests, review wave) exactly as any archetype port. The quiz scenario test is an *additional* cross-card interaction test that composes those cards. Rationale: reuse the proven authoring pipeline and its no-approximations discipline; keep the quiz suite as a thin interaction layer rather than a parallel authoring track.

### D6 — Discovery wave runs first, on the 11 already-implemented cards
Before any authoring, write the scenarios whose cards already exist (e.g. Q4 if Aldamon resolves to an implemented printing; Q22 Medusamon + Petrification token; Q12 Venusmon/Sharkmon; the security-count rule already has partial coverage). This yields discovered gaps and pinned scenarios at near-zero cost and calibrates how faithful the engine already is — informing how much gap-fixing the later phases carry. Rationale: cheapest possible signal first; de-risks the estimate the way the Rocks calibration spike did.

### D7 — DCGO as tiebreaker, cited in every test
Per CLAUDE.md source priority, the judge answer + printed text + `RULES_CONTEXT.md` define the expected outcome; `DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs` and the rules chokepoints are consulted for the processing-order detail and cited in the test docstring. Rationale: the gnarly scenarios (Q15 sequential de-digivolve with mid-sequence immunity; Q30 interruptive `<Partition>`) hinge on processing order that printed text under-specifies — DCGO is the implementation reference for exactly that.

### D8 — Gap fixes are scoped slices, routed through the trackers
Each genuine rules-engine gap the suite surfaces (candidate hot spots: declare-then-pay cost window for Cluster C; immunity "affects me vs affects the battle" + granted-effect ownership + self-immunity for Cluster A; deferred rules-check ordering for Cluster B; trigger activation-site for Cluster D) gets a failing test → minimal primitive → green test → archived gap, the same loop the Rocks change used for B2–B5. A gap fix that changes an existing capability's behavior adds a MODIFIED delta to that capability's spec at fix time.

## Risks / Open Questions

- **Corpus RESOLVED.** All 30 questions' card IDs are pinned in `card-resolution.md` from the PDF. No `BLOCKED-DATA` scenarios remain (Q18's BT17-077 exists). The residual spike work (§1.3) is purely re-deriving implementation status against the correct IDs, since the first inventory used wrong printings.
- **Embedded DSL pack coverage.** `DebugRunner::dsl_card` loads from the embedded pack; some authored cards may need to be added there for the test loader to see them (cf. the `patch_evo_costs` workaround in `mid_attack_security_attack_recompute.rs` where the DSL loader drops `evo_costs`). Confirm the loader path for cross-set scenarios.
- **Scope & sequencing (RESOLVED).** The primary intent is to *find and patch engine gaps* (not to maximize pinned-scenario count). So clusters are ordered by gap-likelihood, not by card count: **A (immunity scope) → B (deferred rules-check) → C (declare-then-pay) → D (activation site) → E → F → G**. The discovery wave (§2, free) runs first across all clusters to rank where the engine already diverges, and that ranking can re-order the authoring phases. 39 full-card authorings remain the dominant cost, but a cluster whose discovery wave shows no divergence can be deprioritized — gap density, not completeness, drives the order.
- **Token/egg-deck plumbing (Q22).** Does the engine model Digi-Eggs going to the egg deck (not main deck) while still satisfying a "send 2 to the bottom of the deck" cost? Needs verification against the Petrification token path.
- **Multi-effect memory arithmetic + Once-Per-Turn (Q10/Q11).** Requires faithful OPT tracking across Gravity Crush re-triggers; confirm the engine's per-turn flag semantics match "not [Once Per Turn] ⇒ fires again."
