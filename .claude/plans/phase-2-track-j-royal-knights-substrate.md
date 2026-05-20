# Phase 2 Track J — Royal Knights Remaining Substrate Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the reusable engine/DSL substrate gaps that still block Royal Knights card authoring, so Track J PR 2 / PR 3 can finish the load-only gap stubs without approximations.

**Architecture:** Three substrate PRs (S1, S2, optional S3). Each adds a reusable primitive to the Rust engine and/or DSL crate, proven by a failing-first behavioral test plus a DSL lowering test, then updates the three gap trackers. No card authoring in this plan — that is Track J PR 2 / PR 3.

**Tech Stack:** Rust (`code/digimon-engine/` library crate, `code/digimon-dsl/` scripting DSL), `cargo test`, YAML card specs, DebugRunner behavioral harness.

---

## Why this matters

Track J PR 1 (the "substrate enabler" PR, landed 2026-05-17) closed `RK-G001`
(filtered breeding-permanent selection) and registered the Atho/René/Por tokens.
The 2026-05-15 hygiene sweep separately closed most of the rollup's original
"Reusable Open Gaps" — breeding trigger fan-out, security-removed observers,
force-follow-up-attack, leave-field replacement, Raid retarget, and the headline
aggregate-sum multi-select.

What remains is a **small, well-scoped set of reusable primitives** that the
Track J PR 2 / PR 3 card-authoring waves depend on. Without them the King Drasil
payoff cards (BT13-112, BT13-110, EX11-053, BT13-019, BT23-072) and the Jesmon
line (BT20-017, BT23-013, BT20-021) stay as load-only gap stubs. This plan
resolves that substrate so card authoring is unblocked.

Source rollup: [`qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md`](../../qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md).

## Already closed — do NOT reopen

Verified against `docs/RUST_ENGINE_GAPS.md` and `qa/dsl-vocab-gaps.md` (2026-05-15/05-17 state):

| Closed gap | Closure |
|---|---|
| `RK-G001` filtered breeding-permanent target | Track J PR 1 |
| `G-BREEDING-TRIGGER-DISPATCH` (RK slices: start-main, security-removed) | wired; remaining slices are non-RK |
| Global `OnOpponentSecurityRemoved` / `OnOwnSecurityRemoved` observer | resolved 2026-05-15 |
| Force-follow-up-attack / `may_attack_now` / without-suspending | resolved 2026-05-15 |
| Leave-field replacement framework / `<Armor Purge>` / `<Barrier>` | resolved — card-local follow-up only |
| Raid target-switch interrupt | resolved 2026-05-15 |
| Aggregate-sum multi-select headline (BT17-018 "total DP ≤ 15000") | resolved by Group 2 |
| `G-OPTION-PLACED-TIMING`, `G-ATTACK-RETARGET`, `RK-G003`, `RK-G004` | resolved |
| `RK-G002` return-self-cost half | resolved by Track B; reduced-cost play half is card-author DSL |

## The remaining substrate — five gaps

| Gap | Type | Tracker entry | RK consumers |
|---|---|---|---|
| **Gap 1** — count-capped / different-name source-play sugar | DSL + light engine | `RUST_ENGINE_GAPS.md` "Decode residual…"; `dsl-vocab-gaps.md` `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES` residual + `RK-G005` | BT13-112, BT13-110, EX11-053, BT13-019, BT23-072 |
| **Gap 2** — effect-play On Play suppression | hybrid (engine provenance + DSL flag) | `RUST_ENGINE_GAPS.md` "Effect play with played-Digimon On Play suppression" 🔴; `dsl-vocab-gaps.md` `PUPPETS-G030` | BT13-110, BT13-112 |
| **Gap 3** — ally-played may-attack observer | hybrid | **unfiled** — propose `G-ALLY-PLAYED-MAY-ATTACK` | BT20-017, BT23-013 |
| **Gap 4** — hand/trash play with name-exclusion filter | hybrid | **unfiled** — propose `G-UNION-HAND-TRASH-NAME-EXCLUSION` (named in RK rollup header; `RK-G005` mentions it generically) | BT20-017, BT23-013, BT13-019, BT20-021 |
| **Gap 5** (deferred) — Craniamon self-on-suspend + aggregate play-cost delete | hybrid | **unfiled** — propose `G-SELF-ON-SUSPEND` + `G-PLAY-COST-AGGREGATE` | BT23-058 |

PR mapping: **S1 = Gap 1 + Gap 2** (same cards need both), **S2 = Gap 3 + Gap 4 + Hinukamuy token**, **S3 = Gap 5** (deferred / optional).

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17 (no approximations / every choice via `pending_selection`), §18 (TDD for new Rust effects), §19 (check `docs/RUST_PYTHON_PARITY.md`).
2. `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md` — full archetype gap doc; "Reusable Open Gaps" + "Spec Input Checklist".
3. `docs/RUST_ENGINE_GAPS.md` — sections "Decode residual: EX10-061 Apocalymon batch…" and "Effect play with played-Digimon On Play suppression".
4. `qa/dsl-vocab-gaps.md` — `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES`, `RK-G005`, `PUPPETS-G030`.
5. `docs/RUST_ENGINE_API.md` — `EffectContext` API, `Effect` builder, `CardEffect` trait, TDD walkthrough.
6. `.claude/plans/phase-2-track-j-royal-knights-pilot-completion.md` — the parent Track J orchestration brief; this plan supplies the substrate its PR 2 / PR 3 assume.

Engine surface (confirmed file paths):

- `code/digimon-engine/src/effect_context/selections.rs` — `select_material`, `select_count_capped`, `select_own_breeding_permanent`.
- `code/digimon-engine/src/effect_context/mod.rs` — `play_from_materials`, `place_as_bottom_source`, effect-play helpers.
- `code/digimon-engine/src/dsl_cards/step/selections.rs` — DSL lowering for selection steps.
- `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` — DSL lowering for play steps.
- `code/digimon-engine/src/dsl_cards/step/combat.rs` — `may_attack_now` lowering.
- `code/digimon-engine/src/dsl_cards/timing_map.rs` — timing tokens (`on_any_digimon_played`).
- `code/digimon-engine/src/game.rs` — `play_from_*` paths + On Play enqueue.
- `code/digimon-engine/src/token_registry.rs` + `code/digimon-engine/src/cards/tokens/` — token registration.

DCGO behavioral references:

- `DCGO/Assets/Scripts/CardEffect/BT13/.../BT13_112.cs` — Omnimon different-name source play.
- `DCGO/Assets/Scripts/CardEffect/BT20/.../BT20_017.cs` — Jesmon token + may-attack observer.
- `DCGO/Assets/Scripts/CardEffect/BT23/.../BT23_013.cs` — Jesmon union play.

---

# PR S1 — King Drasil source-play substrate

Adds Gap 2 then Gap 1. Gap 2 lands first because it is the smaller, 🔴-blocking
primitive and BT13-112 / BT13-110 need it composed with Gap 1.

## Task S1.1: Effect-play On Play suppression (Gap 2)

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs` (effect-play helpers — `play_from_materials`, `play_from_hand`/`play_from_trash` variants)
- Modify: `code/digimon-engine/src/game.rs` (play event path + On Play enqueue site)
- Modify: `code/digimon-dsl/` predicate/step spec for the new `suppress_on_play` flag
- Modify: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` (lower the flag)
- Test: `code/digimon-engine/tests/cards_behavioral/bt5/bt5_106.rs` (canonical first test from the tracker) and a DSL lowering test under `code/digimon-engine/tests/dsl/`

**Behavior:** an effect-play of a Digimon must be able to suppress *only the
just-played permanent's* On Play enqueue for that play event. Broad/global On
Play suppression is wrong — it would silence unrelated permanents.

**API shape** (from `RUST_ENGINE_GAPS.md` "Effect play with played-Digimon On Play suppression"):
add `PlayOptions { suppress_on_play: bool, .. }` (or an equivalent parameter)
to the effect-play helpers; carry it through the play event context; make the
On Play enqueue skip the just-played permanent's On Play clauses when set.
DSL surface: `suppress_on_play: true` on `play_from_materials` / `play_from_hand` / `play_from_trash`.

- [ ] **Step 1: Write the failing behavioral test.** `bt5_106` security-check: select a level 3 purple Digimon from trash that has a visible On Play memory-gain effect; assert the Digimon enters play AND its On Play effect does NOT fire (memory unchanged beyond the play itself). Reference: `qa/dsl-vocab-gaps.md` `PUPPETS-G030` "First test".
- [ ] **Step 2: Run it; confirm it fails** (`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt5_106` — On Play currently fires).
- [ ] **Step 3: Add `suppress_on_play` to the play-options struct** threaded through the effect-play helpers in `effect_context/mod.rs` and the play event context in `game.rs`.
- [ ] **Step 4: Gate the On Play enqueue** so the just-played permanent's On Play clauses are skipped when the flag is set; verify a sibling permanent's On Play still fires (regression assertion in the same test or a second test).
- [ ] **Step 5: Add the DSL `suppress_on_play` flag** to the DSL spec + lowering in `play_digivolve.rs`; write a DSL lowering test asserting `suppress_on_play: true` in YAML reaches the engine flag.
- [ ] **Step 6: Run both tests; confirm PASS.**
- [ ] **Step 7: Update trackers** — move "Effect play with played-Digimon On Play suppression" / `PUPPETS-G030` to `qa/resolved-gaps.md`; note the closure in the RK rollup.
- [ ] **Step 8: Commit.** `git commit -m "feat(engine): effect-play On Play suppression flag"`

## Task S1.2: Count-capped / different-name source-play sugar (Gap 1)

**Files:**
- Modify: `code/digimon-engine/src/effect_context/selections.rs` (extend `select_count_capped` to accept a source-stack zone selector, or add `select_materials_multi`)
- Modify: `code/digimon-engine/src/effect_context/mod.rs` (`play_from_materials` batch consumption)
- Modify: `code/digimon-dsl/` step spec + `code/digimon-engine/src/dsl_cards/step/selections.rs` (new `select_materials` multi-pick step)
- Test: `code/digimon-engine/tests/cards_behavioral/bt13/bt13_112.rs` + DSL lowering test under `code/digimon-engine/tests/dsl/`

**Behavior:** select up to N cards from a permanent's digivolution-source stack
(battle-area or breeding carrier) in one count-capped multi-pick, optionally
constrained by a **name-uniqueness** predicate ("1 of each different name"),
resolved as a single batch that hands the picked sources to `play_from_materials`.
Single + sequential `select_material` already exists; this is the batch sibling.

**API shape** (from `RUST_ENGINE_GAPS.md` "Decode residual" + `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES`):
extend `select_count_capped` with a source-stack zone selector, or add
`select_materials_multi(target, max, filter, uniqueness, callback)`.
DSL surface:

```yaml
- select_materials:
    target: <carrier-binding>      # battle-area permanent or BREEDING_TARGET
    max: 4
    uniqueness: name               # "1 of each different name"
    filter: { trait_has: "Royal Knight" }
    bind_as: picked
- play_from_materials:
    source: picked                 # batch — all picked sources become new permanents
    suppress_on_play: true         # composes with Gap 2
```

- [ ] **Step 1: Write the failing behavioral test.** `bt13_112`: a King Drasil carrier with multiple Royal Knight sources of duplicate AND distinct names; resolve BT13-112; assert the action mask allows at most one pick per name, that distinct-name picks are all selectable, that picked cards leave the source stack and enter the battle area as fresh permanents, and that King Drasil is trashed. Reference: rollup "Stack-Source Multi-Selection" first test.
- [ ] **Step 2: Run it; confirm it fails** (no `select_materials` multi-pick step exists).
- [ ] **Step 3: Add the engine multi-pick** over a source stack with a uniqueness predicate, surfacing a `PendingSelection` / action-mask path for the picks (no `ACTION_SPACE_SIZE` change — reuse existing count-capped masks).
- [ ] **Step 4: Wire `play_from_materials` batch consumption** so all picked sources are played (composing with `suppress_on_play` from S1.1).
- [ ] **Step 5: Add the DSL `select_materials` step** + lowering; write a DSL lowering test asserting `uniqueness: name` + `max` reach the engine selection.
- [ ] **Step 6: Run both tests; confirm PASS.** Add `PendingSelection` assertions for the name-uniqueness mask.
- [ ] **Step 7: Update trackers** — narrow/close "Decode residual…" and the `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES` residual; update `RK-G005` and the RK rollup "Stack-Source Multi-Selection" entry.
- [ ] **Step 8: Commit.** `git commit -m "feat(dsl): count-capped different-name source-play multi-select"`

## PR S1 acceptance gates

- `suppress_on_play` flag lands; `bt5_106` On Play suppression test passes; sibling-permanent On Play regression holds.
- `select_materials` multi-pick lands; `bt13_112` different-name source play passes with name-uniqueness mask assertions.
- No `ACTION_SPACE_SIZE` / tensor change (Working Rule 1).
- No regression in `cards_behavioral`, `dsl`, `combat`, `option_flow`.

---

# PR S2 — Jesmon engine line

Files two new gap IDs, then closes them, plus the last RK token registration.

## Task S2.0: File the two new gap IDs

**Files:**
- Modify: `docs/RUST_ENGINE_GAPS.md` (add `G-ALLY-PLAYED-MAY-ATTACK`, `G-UNION-HAND-TRASH-NAME-EXCLUSION`)
- Modify: `qa/dsl-vocab-gaps.md` (same two entries with DSL surface + first test)

Both IDs are named in the RK rollup's 2026-05-17 header as "independent gaps
tracked separately" but have no canonical tracker entry yet. Write each entry
with: card consumers, effect text, what's missing, suggested API shape, first
failing test — matching the format of neighboring entries.

- [ ] **Step 1: Add both gap entries** to both trackers with the content below (Tasks S2.1 / S2.2).
- [ ] **Step 2: Commit.** `git commit -m "docs: file G-ALLY-PLAYED-MAY-ATTACK and G-UNION-HAND-TRASH-NAME-EXCLUSION"`

## Task S2.1: Ally-played may-attack observer (Gap 3 — `G-ALLY-PLAYED-MAY-ATTACK`)

**Files:**
- Verify/modify: `code/digimon-engine/src/dsl_cards/step/combat.rs` (`may_attack_now` — does it target only `self`, or can it take an event-bound permanent?)
- Modify: DSL spec for `may_attack_now` to accept an event-target binding if it currently does not
- Test: `code/digimon-engine/tests/cards_behavioral/bt20/bt20_017.rs` (un-ignore the gap test) + DSL lowering test

**Behavior:** "When another of your Digimon is played, [delete a target / that
Digimon] may attack." The `when: on_any_digimon_played` timing is already wired
(2026-05-08, per `RK-G005`). `may_attack_now` exists (Track D). The missing
piece is granting the may-attack to the **event-played Digimon** (the
`event_permanent`), not `self`, plus exposing the delete sub-clause target.

**API shape:** allow `may_attack_now` to take `target: event_permanent` (or the
relevant event binding). If `may_attack_now` is hard-bound to `self`, add an
optional `target:` field that resolves through the existing event-permanent
binding used by `on_any_digimon_played` consumers.

- [ ] **Step 1: Investigate** `may_attack_now` in `combat.rs` — confirm whether it can target a non-`self` permanent. Record the finding in the gap entry from S2.0.
- [ ] **Step 2: Write the failing behavioral test.** `bt20_017`: with BT20-017 Jesmon in play, play another of your Digimon; assert a pending selection offers that newly-played Digimon an attack (with PASS exposed), and that the delete sub-clause surfaces its target choice.
- [ ] **Step 3: Run it; confirm it fails.**
- [ ] **Step 4: Add the `target:` binding** to `may_attack_now` (engine + DSL lowering) so the event-played Digimon can be the attacker.
- [ ] **Step 5: Run the test; confirm PASS.** Assert PASS is in the mask (optional may-attack, Working Rule §17).
- [ ] **Step 6: Add a DSL lowering test** for `may_attack_now: { target: event_permanent }`.
- [ ] **Step 7: Update trackers** — close `G-ALLY-PLAYED-MAY-ATTACK`; update `RK-G005` and the RK rollup BT20-017 / BT23-013 rows.
- [ ] **Step 8: Commit.** `git commit -m "feat(dsl): may_attack_now targets the event-played Digimon"`

## Task S2.2: Hand/trash play with name-exclusion filter (Gap 4 — `G-UNION-HAND-TRASH-NAME-EXCLUSION`)

**Files:**
- Modify: `code/digimon-engine/src/effect_context/selections.rs` (selection across hand + trash with a name-exclusion predicate)
- Modify: `code/digimon-dsl/` predicate spec (add a `name_not_in` / `name_excludes` predicate leaf) + `code/digimon-engine/src/dsl_cards/predicate.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs` (lower the multi-zone hand+trash selection)
- Test: `code/digimon-engine/tests/cards_behavioral/bt23/bt23_013.rs` (un-ignore the gap test) + DSL lowering test

**Behavior:** the Jesmon-family play that selects a card from **hand OR trash**
restricted to a name set, **excluding names already in play** (the
"union … with name exclusion" shape). Two missing pieces: (a) a selection that
spans the hand and trash zones in one prompt, and (b) a name-exclusion
predicate leaf that filters out names matching an own-side battle-area permanent.

**API shape:** a hand+trash union selection (`select_from_zones([Hand, Trash], filter)`),
plus a DSL predicate `name_not_in` evaluated against own battle-area permanent
names. Confirm the actual card text in `data/cards.json` for BT20-017 / BT23-013
before fixing the exclusion semantics (printed text is the source of truth per
CLAUDE.md "Source priority").

- [ ] **Step 1: Read the printed text** for BT20-017 and BT23-013 in `data/cards.json`; confirm the exact name set and the exclusion rule. Record it in the S2.0 gap entry.
- [ ] **Step 2: Write the failing behavioral test.** `bt23_013`: with a name already in play, resolve the Jesmon play; assert the hand+trash selection prompt offers the legal names and excludes the in-play name.
- [ ] **Step 3: Run it; confirm it fails.**
- [ ] **Step 4: Add the hand+trash union selection** + the `name_not_in` predicate leaf (engine + DSL).
- [ ] **Step 5: Run the test; confirm PASS.** Add `PendingSelection` assertions for the excluded name.
- [ ] **Step 6: Add a DSL lowering test** for the multi-zone selection + `name_not_in`.
- [ ] **Step 7: Update trackers** — close `G-UNION-HAND-TRASH-NAME-EXCLUSION`; update `RK-G005` and the RK rollup BT20-017 / BT23-013 / BT13-019 / BT20-021 rows.
- [ ] **Step 8: Commit.** `git commit -m "feat(dsl): hand+trash play selection with name-exclusion filter"`

## Task S2.3: Hinukamuy token registration

**Files:**
- Modify: `code/digimon-engine/src/token_registry.rs` (+ a module under `code/digimon-engine/src/cards/tokens/` if the pattern requires one)
- Test: `code/digimon-engine/tests/cards_behavioral/bt23/bt23_057.rs`

**Behavior:** BT23-057 Gankoomon creates a Hinukamuy token. Register it with its
printed stats (confirm in `data/cards.json` / fandom wiki), mirroring how
Atho/René/Por were registered in Track J PR 1.

- [ ] **Step 1: Confirm Hinukamuy printed stats** (color, DP, level, keywords) from `data/cards.json` and the fandom wiki.
- [ ] **Step 2: Write the failing test** — BT23-057 creates a Hinukamuy token via `play_token`; assert the token enters battle with the printed stats.
- [ ] **Step 3: Run it; confirm it fails.**
- [ ] **Step 4: Register the token** in `token_registry.rs`.
- [ ] **Step 5: Run the test; confirm PASS.**
- [ ] **Step 6: Update `RK-G005`** token-registration note.
- [ ] **Step 7: Commit.** `git commit -m "feat(engine): register Hinukamuy token"`

## PR S2 acceptance gates

- `G-ALLY-PLAYED-MAY-ATTACK` and `G-UNION-HAND-TRASH-NAME-EXCLUSION` filed, then closed.
- `bt20_017` and `bt23_013` gap tests un-ignored and passing.
- Hinukamuy token registered; `bt23_057` token-creation test passes.
- No `ACTION_SPACE_SIZE` / tensor change.
- No regression in `cards_behavioral`, `dsl`, `combat`.

---

# PR S3 — Craniamon (deferred / optional)

Out of Track J scope per the RK rollup header. Schedule only if BT23-058 is
prioritized; it has a single consumer.

## Task S3.1: Self-scoped on_suspend predicate + aggregate play-cost delete

**Files:**
- Modify: `code/digimon-engine/src/dsl_cards/predicate.rs` + `code/digimon-dsl/` (self-scoped `on_suspend` predicate threading — mirror the `self_digivolution_contains_name` subject-threading fix in `engine-gaps.md`)
- Modify: `code/digimon-engine/src/effect_context/selections.rs` (aggregate lowest-play-cost delete-all)
- Test: `code/digimon-engine/tests/cards_behavioral/bt23/bt23_058.rs`

**Behavior:** `G-SELF-ON-SUSPEND` — a triggered clause condition that evaluates
whether *this* permanent was the one suspended. `G-PLAY-COST-AGGREGATE` —
delete all permanents in an aggregate selected by lowest play-cost.

- [ ] **Step 1: Write the failing behavioral test** for BT23-058's suspend-self prevention + aggregate delete clause.
- [ ] **Step 2: Run it; confirm it fails.**
- [ ] **Step 3: Thread the `PredicateSubject::Permanent(source_h)`** into the `on_suspend` condition closure (same pattern as the `self_digivolution_contains_name` fix).
- [ ] **Step 4: Add the aggregate lowest-play-cost delete primitive.**
- [ ] **Step 5: Run the test; confirm PASS.**
- [ ] **Step 6: File + close** `G-SELF-ON-SUSPEND` and `G-PLAY-COST-AGGREGATE` in the trackers.
- [ ] **Step 7: Commit.**

---

## Cross-cutting requirements (RK rollup "Spec Input Checklist")

Every task in this plan MUST satisfy:

- one failing Rust behavioral test under `code/digimon-engine/tests/` before implementation;
- one DSL lowering/compiler test when YAML vocabulary changes;
- action-mask / `PendingSelection` assertions for every player-visible choice (Working Rule §17 — no auto-selection);
- NO `ACTION_SPACE_SIZE` or tensor-contract expansion unless `docs/ACTION_SPEC.md` / `docs/TENSOR_SPEC.md` are updated in the same change (Working Rule 1);
- tracker updates in `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, and `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md` whenever a gap closes or splits; move fully-closed gaps to `qa/resolved-gaps.md`.

## Constraints

- No-approximations (§17): every source pick, every may-attack decision, every hand/trash selection surfaces through `pending_selection`.
- TDD (§18): failing behavioral test via DebugRunner before the `CardEffect` / DSL change.
- Check `docs/RUST_PYTHON_PARITY.md` (§19) before editing engine code in these areas.
- Source priority: printed text (`data/cards.json`) → `docs/RULES_CONTEXT.md` → fandom wiki → DCGO. DCGO is a behavioral tiebreaker only.
- This plan adds substrate only — no card YAML authoring (that is Track J PR 2 / PR 3).

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt5_106 bt13_112 bt20_017 bt23_013 bt23_057
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage
cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker discipline

- `docs/RUST_ENGINE_GAPS.md` — close "Decode residual…" and "Effect play with played-Digimon On Play suppression"; file then close `G-ALLY-PLAYED-MAY-ATTACK`, `G-UNION-HAND-TRASH-NAME-EXCLUSION` (and `G-SELF-ON-SUSPEND` / `G-PLAY-COST-AGGREGATE` if S3 runs).
- `qa/dsl-vocab-gaps.md` — same closures; update `RK-G005` residual list.
- `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md` — annotate the "Reusable Open Gaps" entries with PR-S# citations; update affected card rows.
- `qa/resolved-gaps.md` — move fully-closed gaps here.

## Order of operations

1. PR S1 — Task S1.1 (On Play suppression), then Task S1.2 (count-capped source-play).
2. PR S2 — Task S2.0 (file gap IDs), S2.1 (may-attack observer), S2.2 (name-exclusion play), S2.3 (Hinukamuy token).
3. PR S3 — Task S3.1 (Craniamon), only if prioritized.

## Out of scope

- Royal Knights card YAML authoring — Track J PR 2 / PR 3 (`phase-2-track-j-royal-knights-pilot-completion.md`).
- BLOCKED-card residual shapes not yet proven reusable: BT13-030 sourceless-opponent aura, BT19-093 two-target security modifier / color-bypass predicate, EX8-073 source-gated DP swings, BT15-092. These need a first-failing-test triage pass before promotion to substrate (RK rollup: "should not be promoted to generic gaps until a failing test proves a reusable primitive is missing").
- Non-RK breeding trigger fan-out slices still open under `G-BREEDING-TRIGGER-DISPATCH`.
- Blast Digivolve from hand+breeding; ACE Overflow variants beyond the closed slice.

## Discovery rider

If a task surfaces a NEW substrate gap, file the per-card failing test, leave
the consumer card PARTIAL, and DO NOT pull the new substrate work into the
current PR — file it as its own tracker entry. Conversely, if investigation
shows a gap is already composable from landed primitives (e.g. `may_attack_now`
already accepts an event target), close it with a card-shaped test instead of
new engine code.
