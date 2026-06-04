## Context

The Digimon TCG **General Rules/FAQ** (fandom wiki, sourced from official Carddass/Bandai Q&A — see the page's "Sources" section) is ~60 Q&A entries across 12 sections. Unlike the `add-judge-quiz-faithfulness-suite` corpus (30 adversarial multi-card scenarios needing ~39 cards authored), this corpus is **foundational rules**, mostly card-agnostic. It is the bedrock the judge-quiz combos sit on, and it is currently unpinned.

The repo already has the proven machinery: `DebugRunner` behavioral tests, the `tests/judge_quiz/` cluster layout + `loader.rs` gate pattern, a `tests/deck_tools/` deck-validation surface, the discover-then-pin discipline, and the `engine-gaps.md` / `dsl-vocab-gaps.md` / `validated_cards_dsl.json` trackers. This change reuses all of it; it introduces no new harness.

Source priority for any rules dispute (per CLAUDE.md): `general_rule.pdf` (canonical) > base-repo `DCGO/` C# (battle-tested) > the FAQ/wiki text itself. The FAQ answer is the *target*; where it is terse or ambiguous the PDF rule number and DCGO behavior are the tiebreakers cited in each test docstring.

## Goals / Non-Goals

**Goals:**
- Freeze the FAQ into an authoritative, surface-triaged ledger so coverage is provable and nothing silently drops.
- Pin every in-scope, uncovered foundational rule as a permanent regression test using **real implemented DSL cards**.
- Cross-link (not duplicate) rules already covered elsewhere in the test tree.
- Convert discovered faithfulness gaps into logged, reproducible failing tests routed to the shared gap trackers / chips — never weakened assertions.

**Non-Goals:**
- Not a card-authoring project. Authoring is the last resort, only when no implemented card carries a needed property.
- Not re-testing the judge-quiz combos (that suite owns adversarial multi-card interactions).
- Not modeling table-procedure items the engine intentionally abstracts (RPS first-player, physical security-stack placement order, "public information" disclosure) — these are documented `N/A`, not faked.
- No RL action-space or tensor contract change beyond an additive sub-range if a confirmed gap fix needs one.

## Decisions

### D1 — Four test surfaces, one ledger spine
Every FAQ item is classified into exactly one surface, and the ledger (`qa/qa-reports/rules-faq.md`) is the spine the whole change hangs off:

```
① RUNTIME BEHAVIORAL  → tests/rules_faq/*.rs (DebugRunner)
② DECK VALIDATION     → tests/deck_tools/*  (deck legality)
③ DATA / METADATA     → CardData / registry assertion
④ NOT MODELED (N/A)   → documented in ledger, no test
```

**Why:** the corpus is heterogeneous; forcing deck-creation or metadata rules through `DebugRunner` would be contortion. One ledger with a `surface` column keeps the corpus whole while routing each item to its natural harness. *Alternative rejected:* runtime-only scope (drops ⅓ of the corpus) — the user chose full-corpus coverage.

### D2 — Reuse implemented DSL cards; author only on a true gap
The unit under test is the rule. A test needing a property (2-color, `-` Level, `-` DP, conditional keyword, On-Deletion, Once-Per-Turn) **reuses an already-implemented DSL card** that exhibits it. A `loader.rs`-style gate confirms each reused card loads from the embedded DSL pack before the section's tests run. Authoring a card is the escape hatch when no implemented vehicle exists, and is recorded in `validated_cards_dsl.json`.

**Why:** keeps the suite cheap and rule-focused; avoids the judge-quiz authoring-multiplier. *Alternative rejected:* author the canonical card each FAQ item implies — far more expensive, and the rule, not the printing, is what's being pinned.

### D3 — Audit-then-cross-link before writing
Before encoding a runtime rule, grep the existing tree for prior coverage. Already-pinned rules (likely: 0-DP deletion, memory cap-at-10, can't-attack-turn-played, mandatory draw / deck-out loss, breeding-move ≠ play) get an `XLINK` ledger row citing the existing test path; **no new test is written**. New tests cover only the genuinely-uncovered remainder.

**Why:** the user chose dedup-by-cross-link. Redundant assertions rot and dilute the ledger's signal. *Alternative rejected:* a self-contained 1:1 re-encoding of every item — more duplication, more maintenance, no added faithfulness signal.

### D4 — Layout mirrors the FAQ's own sections (not judge-quiz A–G clusters)
`tests/rules_faq/` modules: `deck_creation`, `phases` (unsuspend/draw/breeding), `main_phase`, `effect_resolution`, `keyword_identity`, `multicolor`, `no_level_no_value`, `security_digimon`, `in_its_text`, plus `main.rs` wiring + a `loader.rs` reused-card gate. Each test docstring quotes the FAQ Q+A and the `general_rule.pdf` §/DCGO citation.

**Why:** the FAQ already ships a clean section taxonomy; mirroring it makes the ledger ↔ test mapping 1:1 and obvious. Judge-quiz invented A–G clusters because its scenarios cut across machinery; this corpus doesn't need that.

### D5 — Discover-then-pin, gaps routed not silenced
Test asserts the FAQ-correct outcome. PASS → `PIN`. FAIL → `GAP`: log to `engine-gaps.md` (engine) or `dsl-vocab-gaps.md` (DSL vocabulary) with the citation, mark the test `#[ignore]` with a `// GAP: <tracker-ref>` note (or gate it behind the chip), and spin off a scoped fix. The assertion is committed as-written; it is never softened to go green.

### D6 — The canary gap
FAQ Main-Phase item "*two effects, one +DP one −DP, ending simultaneously at end of turn... returns the Digimon to its original DP*" is expected to FAIL against the known latent **17-1-2-2** mid-effect-deletion bug (`add_dp_modifier deletes mid-effect`). It is sequenced early as the proof the discover-then-pin loop produces real chips, and is the most likely `MODIFIED` delta to `permanent-deletion-semantics`.

## Frozen corpus ledger (triage — design output, refined into `qa/qa-reports/rules-faq.md` during tasks)

Surface key: ①runtime ②deck-val ③data ④N/A. Verdict is filled during the discovery wave.

**Deck Creation (5)** — all ②
- 4 copies keyed by card *number* (same name, diff number OK) · no Digi-Eggs in main deck · only Digi-Eggs in egg deck · ≤4 per card number in egg deck · main deck is exactly 50 (egg deck separate, not additive).

**Game Setup (3)** — ④ RPS-winner-goes-first (engine: `seed % 2`) · ① determine-first-then-draw (ordering) · ② security-stack built from deck top (order-preserving) — likely ④ if placement order isn't observable.

**Unsuspend (3) / Draw (2)** — all ①
- only active player unsuspends · unsuspend is mandatory · opponent's cards not unsuspended · draw is mandatory · empty-deck draw = loss · no max hand size.

**Breeding (7)** — all ①
- hatch & promote are optional · can't hatch while breeding occupied · can't trash breeding Digimon to hatch · egg-deck-out ≠ loss · no digivolve in breeding during breeding phase · promote ≠ play (no On-Play) · breeding Digimon don't count for "if you have a Digimon" conditions.

**Main Phase (~22)** — mostly ①, the dense core
- digivolve a just-played Digimon: yes · still can't attack the turn it entered, even if digivolved · promoted-from-breeding *can* attack this turn · digivolving a suspended Digimon keeps it suspended · On-Play does **not** fire on digivolve · can't activate effects on breeding Digimon · cost ≥11 unplayable from 0 memory · On-Play/When-Digivolving still fires when cost pushes memory to opp's side · breeding Digimon excluded from "if you have" · multi-effect resolution order = active player chooses · both-players-simultaneous = turn player first · attack only *suspended* opponent Digimon · attacking a Digimon ≠ being blocked · attack/EoA effects fire only for the attacking Digimon · multiple When-Attacking + inherited: choose targets individually · effects-on-others end when source leaves (timing-dependent) · Option On-Play fires even when cost pushes memory over · DP can't go negative, 0 DP deleted · Once-Per-Turn opt-out doesn't consume it · resolve order chosen one-at-a-time, newly-triggered first · memory caps at 10 · return-multiple-to-deck order is public (④) · When-Digivolving fires *after* the digivolve draw · 3+ simultaneous: choose one at a time · keyword-as-text selectable even outside its timing · **gained keyword only counts in BA while condition met** · **simultaneous +/−DP end together, no intermediate 0-DP deletion (CANARY)** · On-Play/When-Digivolving/On-Deletion/When-Attacking mandatory unless "can"/"you may" · target "2 of opp" with <2 in play still activates · "2" must affect two; "up to 2" any number ≤2 · can't activate When-Attacking after block declared.

**Other Rulings (8)** — ① + ③
- attack/EoA only for the attacking Digimon (dup) · suspend-on-attack effects fire same time as When-Attacking · already-triggered effect still resolves after source state changes · **Security Digimon can't activate non-[Security] effects** (③/①) · **Security Digimon not treated as "Digimon" in effects** (③) · cost-reduced ≥11 card becomes playable from 0 · opponent-effect "to bottom in any order" → effect *controller* chooses order.

**Multi-Colour (8)** — ③ + ① gating
- 2-color Digimon is a target for single-color effects · counted as 1 (not 2) for "for each" · choose which digivolve-cost color to pay · 2-color Option usable if both colors present across BA+breeding · [Security] effect activates regardless of color requirement · "return a blue Option" can return a blue+green Option · color requirement still enforced for "use a blue Option" effects unless "ignore color" · can't treat a multi-color/trait card as having fewer colors/traits.

**No-Level (5) / No-Value (1)** — ① + ③, need real `-`Level / `-`DP cards
- `-` Level → "Digimon without Lv."; can't normal-digivolve to/from; not targeted by "Lv.X or less/more" · no-Level in breeding promotes if it has DP · D-Reaper trait treated as Digimon · no-Level placeable as a digivolution source · `<De-Digivolve>` can still delete a no-Level Digimon with sources · `-` DP Digimon can't gain DP (stays no-value).

**In its Text (2)** — ③
- "with X in its text" = name/effect/inherited/security/DNA-cond/special-digivolve/DigiXros-cond, top card only, not digivolution cards · `<Save>`-icon match is **icon-exact** (`<Material Save>` ≠ "Digimon with `<Save>` in its text"); plain-word match is substring. Also the name-substring rule ("Agumon" matches "ToyAgumon"/"BushiAgumon").

## Risks / Trade-offs

- **Some "FAQ answers" are terse or JP-translated** → cite `general_rule.pdf` §/DCGO in the docstring as the authoritative tiebreaker; if they conflict with the FAQ wording, follow PDF>DCGO and note the discrepancy in the ledger.
- **Property-card availability** → a few rules (no-Level, no-DP, conditional-keyword) may lack any implemented DSL vehicle; the triage spike flags these up front so authoring (D2 escape hatch) is scoped, not discovered mid-wave.
- **Over-broad blast radius (full corpus, 2 harnesses)** → phased tasks (triage → audit/cross-link → discovery wave by section → gap chips) so each section is independently shippable; a stalled section doesn't block the rest.
- **N/A items read as "skipped coverage"** → the ledger's `N/A` rows carry an explicit reason (engine abstraction), so the corpus is provably whole, not silently truncated.
- **Discovery surfaces more gaps than expected** → each gap is its own scoped chip via the existing trackers; the suite lands the *failing pin* regardless of when the fix ships.

## Open Questions

- Does the engine expose deck-legality validation reachable from the `tests/deck_tools/` surface for all five deck-creation rules, or do some assert against the deck-builder gate (`tested_cards.json`) instead? (Resolve in the triage spike.)
- Is physical security-stack placement order observable anywhere in engine state, or is it strictly ④ N/A? (Resolve in triage.)
- Final reused-card picks for the property-carrying rows (no-Level, no-DP, conditional-`<Blocker>`, multi-color, On-Deletion, Once-Per-Turn) — pinned in the triage spike before the discovery wave.
