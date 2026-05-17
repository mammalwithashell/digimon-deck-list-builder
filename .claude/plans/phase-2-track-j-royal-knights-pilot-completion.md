# Phase 2 Track J — Royal Knights Pilot Completion

You are unblocking the Royal Knights pilot archetype (51 stuck cards as of 2026-05-17). Royal Knights is the **largest pilot pile** and has the most architectural depth — many "load-only gap stub" cards waiting on a small set of reusable primitives plus a long tail of card-shaped substrate edges.

Largest individual session in Phase 2. Has a soft consumer relationship with Track B (`activation_cost(...)`) for RK-G002. Independent of other tracks. Consider splitting into 2–3 PRs to keep review tractable.

## Why this matters

Royal Knights has 53 cards attempted, 51 stuck — only 2 IMPLEMENTED. Many cards are "load-only gap stub" entries waiting on:

| Gap | Type | Blocks |
|---|---|---|
| **RK-G001** | DSL: filter on `select_own_breeding_permanent` | BT13-093, BT20-083, BT13-110, EX11-053 (4+ cards) |
| **RK-G002** | DSL: source-bound return-self cost → reduced hand play | EX11-071 Cool Boy + sibling Tamer plays (consumes Track B) |
| **RK-G003** | engine: Delay/keyword leave-prevention replacements | BT23-054, BT23-058, BT20-091, BT20-100 |
| **Token Atho/Rene/Por registration** | data/engine: register Jesmon tokens | BT20-017, BT23-013 (Jesmon family) |
| **Hand-Main bottom-source placement** | DSL/engine | BT23-072 King Drasil, EX11-053 Omekamon |
| **BT17-077 bulk operations** | DSL: bulk trash-to-deck + returned-card binding | BT17-077 Imperialdramon: Paladin Mode |
| **Counter Blast DNA / DNA-origin tail** | (closed) | already unblocked |
| **Force-follow-up-attack** | (closed) | already unblocked |
| **Inherited security-removed observer** | (closed by Phase 1 + Track A) | un-ignore sweep |
| **Modifier preventing attack-target redirection** | (closed 2026-05-08) | un-ignore sweep |
| Various card-local "selection lowering" / "stack source-DP filter" | DSL eval-arm bridges | per-card |

Expected unblock after this track: **~30 Royal Knights cards advance from BLOCKED/PARTIAL to IMPLEMENTED**, with the remaining ~20 still waiting on the broader substrate roadmap (Blast Digivolve from hand+breeding, ACE Overflow variants beyond the closed slice, etc.).

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17, §18, §22 (no Python engine imports — this archetype was Python-implemented prior to Rust pivot; do NOT cross-reference).
2. `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md` — full archetype gap doc; ~155 lines walk through every card and its blockers. **Required reading.**
3. `qa/qa-reports/validated_cards_dsl.json` — `"archetype": "Royal Knights"` for the 51 PARTIAL/BLOCKED entries.
4. `qa/dsl-vocab-gaps.md` — search RK-G001, RK-G002, RK-G003, RK-G004, RK-G005.
5. `code/digimon-engine/src/effect_context/selections.rs::select_own_breeding_permanent` — site for RK-G001's `filter:` field.
6. `code/digimon-engine/src/effect_context/mod.rs` — `place_as_bottom_source` with `BREEDING_TARGET` (Group 4, 2026-05-02 closure).
7. `code/digimon-engine/src/replacement.rs` — Track B replacement framework. RK-G003 lands on top of this.
8. `code/digimon-engine/src/token_registry.rs` — token creation. Atho/Rene/Por registration site.
9. Royal Knights card YAMLs under `code/digimon-engine/cards/bt13/`, `bt17/`, `bt19/`, `bt20/`, `bt22/`, `bt23/`, `ad1/`, `ex8/`, `ex10/`, `ex11/` — the partial-but-stubbed authoring corpus.
10. DCGO references for the heavyweight RK cards:
    - `DCGO/Assets/Scripts/CardEffect/BT13/.../BT13_007.cs` (King Drasil_7D6)
    - `DCGO/Assets/Scripts/CardEffect/BT17/.../BT17_077.cs` (Imperialdramon Paladin Mode)
    - `DCGO/Assets/Scripts/CardEffect/BT20/.../BT20_017.cs` (Jesmon token creation)
    - `DCGO/Assets/Scripts/CardEffect/BT23/.../BT23_072.cs` (King Drasil_7D6 hand main)

## Work to be done

This track is large enough to split. Suggested split:

- **PR 1: Substrate enablers** (RK-G001, RK-G003, Token registration, bottom-source DSL)
- **PR 2: Card authoring wave 1** (~15 cards now unblocked)
- **PR 3: Card authoring wave 2** (remaining ~15 cards)

### 1. `RK-G001` — `filter:` field on `select_own_breeding_permanent`

Per `qa/dsl-vocab-gaps.md` § "Royal Knights — filtered breeding permanent target [RK-G001]". `select_own_breeding_permanent` exists post-2026-04-29 but doesn't take a card-name / trait / level / id filter. Add a `filter: BoolPredicate` field; thread through the selection installer; the mask emits only matching breeding permanents.

For King Drasil-specific shape: `filter: { card_name_contains: "King Drasil_7D6" }`. Variant-coverage compliance.

Behavioral test: BT13-093 trigger places a Royal Knight from hand under a [King Drasil_7D6] in breeding when present; gracefully passes if no King Drasil.

### 2. `RK-G003` — Delay/keyword leave-prevention replacements

The Track B replacement framework supports `WhenWouldLeaveBattleArea`. RK-G003 is the family of inherited [All Turns] Delay-Option-driven leave-prevention shapes (BT20-100 The Last Guardian, BT23-054 Magnamon protection clauses, etc.). The substrate is mostly there per the archetype's own status notes — what's missing per `qa/dsl-vocab-gaps.md` is the broader DSL surface to wire keyword-grant replacement bodies, and the per-card test coverage.

Audit cards: BT23-054 (Armor Purge), BT23-058 (Craniamon suspend-self prevention), BT20-091 (Cool Boy opponent-turn play). Close per-card; close the RK-G003 entry once all cards are covered.

### 3. Token registration: Atho, Rene, Por

Per BT20-017 Jesmon and BT23-013 Jesmon — both need token-creation primitives for Atho/Rene/Por tokens. Per `code/digimon-engine/src/cards/tokens.rs` (or wherever token_registry lives), register the three named tokens with their printed stats. Add `EffectContext::play_token("atho")` / similar usage.

Also BT20-017 has a "may-attack" rider on tokens that ties to the Track B `may_attack_now` primitive (closed 2026-05-08) — confirm consumption.

### 4. Hand-Main bottom-source placement

BT23-072 King Drasil_7D6 and EX11-053 Omekamon both need a DSL verb that picks a card from hand and places it as the bottom digivolution source of a target battle-area permanent. `place_as_bottom_source` exists for under-self / under-named-binding; the DSL verb to consume it for "from hand" is the residual.

DSL surface:

```yaml
- select_hand:
    filter: { trait_has: "Royal Knight" }
    bind_as: chosen_hand
- place_as_bottom_source:
    target: <permanent-binding>
    source: { hand_binding: chosen_hand }
```

If `place_as_bottom_source` doesn't accept a hand-binding source today (likely accepts only permanent/source bindings), extend it.

### 5. BT17-077 — bulk trash-to-deck + returned-card binding

`BT17-077` Imperialdramon: Paladin Mode (ACE) needs:

- Bulk trash-to-deck: trash all of a target permanent's digivolution sources back to its owner's deck (top or bottom per text).
- Returned-card binding: bind the set of returned cards as a downstream-readable property (for "for each card returned, do X").
- Sourceless bottom-deck cost: pay a cost by placing this Digimon at the bottom of the deck (sibling of return-self-as-cost).

The bottom-deck cost is a Track B sibling (`return_self_to_deck_bottom_as_cost` exists for triggered abilities; this is an action-time variant).

This is the most card-shaped item in Royal Knights. Bulk operations + binding the result set is reusable but Royal-Knights-flavor in shape.

### 6. Track-A / Track-B / 2026-05-08 closure sweep

Sweep Royal Knights test annotations for tags closed by other tracks:

- Inherited security-removed observer (closed) — un-ignore RK observer tests.
- Modifier preventing attack-target redirection (closed 2026-05-08) — un-ignore.
- `may_attack_now` (closed) — un-ignore force-attack tests.
- `G-PRED-DP-LTE` (Track A) — un-ignore.

### 7. RK-G002 — Track B consumer

If Track B has landed: rewrite EX11-071 Cool Boy YAML to use `activation_cost: return_self_to_deck_bottom` then a reduced-cost hand-play in the body. Close RK-G002.

### 8. Card authoring waves

Walk per-card list in the RK archetype gap doc § "2026-05-05 Batch 4-15 Implementation Notes". For each card whose Stubbed gap is now closed, complete YAML + behavioral test. Pace yourself — expect 10–15 cards per session.

## Acceptance gates

- RK-G001 `filter:` field lands; BT13-093 / BT20-083 / BT13-110 / EX11-053 King Drasil placements work.
- RK-G003 per-card leave-prevention coverage complete.
- Atho/Rene/Por tokens registered; BT20-017 and BT23-013 token-play works.
- Hand-Main bottom-source placement DSL verb lands; BT23-072 / EX11-053 author end-to-end.
- BT17-077 bulk operations + returned-card binding land; ACE Imperialdramon authored.
- RK-G002 closed (if Track B available) via EX11-071 migration.
- Sweep absorbs ≥ 5 already-closed tags.
- ≥ 25 Royal Knights cards advance to IMPLEMENTED.
- `dsl_eval_arm_coverage` lint passes.
- No regression in cards_behavioral, dsl, combat, option_flow.

## Constraints

- No-approximations: every King Drasil filter choice, every token-creation target choice, every hand-Main placement choice surfaces through pending_selection.
- Working Rule 1: no `ACTION_SPACE_SIZE` / tensor change.
- Working Rule 17: token creation must surface via `play_token` helper; do NOT inline token-permanent construction.
- Source priority: printed text → Rules Manual → fandom wiki → DCGO. Royal Knights cards have heavy printed text — read each carefully.

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_007 bt13_093 bt17_077 bt20_017 bt20_083 bt20_100 bt23_054 bt23_058 bt23_072 ex11_053 ex11_071
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker discipline

- `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md` — annotate each card's status note with PR # citations. Close RK-G001, RK-G002, RK-G003.
- `qa/dsl-vocab-gaps.md` — close RK entries.
- `qa/qa-reports/validated_cards_dsl.json` — advance Royal Knights cards.
- `docs/RUST_ENGINE_GAPS.md` — no canonical RK entries; only sweep notes.

## Order of operations (PR 1 — substrate enablers)

1. RK-G001 filter field.
2. RK-G003 per-card audit + Track B replacement consumption.
3. Token registration (Atho/Rene/Por).
4. Hand-Main bottom-source DSL.
5. BT17-077 substrate (bulk + binding + sourceless cost).
6. Tag-closure sweep.

## Order of operations (PR 2 — card authoring wave 1)

7. King Drasil core trio: BT13-007, BT13-093, BT23-072.
8. Cool Boy + EX11-071 (Track B consumer, if available).
9. Omekamon trio: BT20-083, EX11-053, BT13-093.
10. Examon trio: AD1-004, BT20-045, BT23-047.

## Order of operations (PR 3 — card authoring wave 2)

11. Long tail per the RK archetype gap doc.

## Out of scope

- Blast Digivolve from hand+breeding (substrate gap; if a card needs it, defer).
- ACE Overflow variants beyond the Track B closed slice.
- Counter Blast DNA (closed).
- Force-follow-up-attack (closed).
- Any new ACE metadata fields.
- Hand-resident observer fan-out (substrate gap, separate track).
- Effect-spawned permanent EOT-deletion rider for the Royal Knights cards that need it (Apocalymon-family substrate, separate planned).

## Discovery rider

Royal Knights has the highest density of cards waiting on substrate that's already landed but not card-author-recognized. Many "BLOCKED" entries may actually be authorable today — when you walk the card list, attempt YAML before reaching for the blocker tag. If a card opens up, that's a free card.

If a card surfaces a NEW substrate gap (one of the 12 BLOCKING items in `RUST_ENGINE_GAPS.md` for instance), file the per-card test, leave the card PARTIAL, and DO NOT pull the substrate work into this PR.
