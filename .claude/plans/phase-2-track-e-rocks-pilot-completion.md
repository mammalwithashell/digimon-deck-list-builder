# Phase 2 Track E — Rocks Pilot Completion

You are unblocking the Rocks pilot archetype (~36 stuck cards as of 2026-05-17) by landing the small set of reusable DSL/engine primitives the archetype specifically gates on, and modernizing the `raw_rust` workarounds in its existing YAML. **This track is mostly DSL surface work, not deep substrate** — per `qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md`, the remaining engine substrate Rocks needed has already landed; what's missing is one ordered-permutation primitive, DSL verbs to consume already-shipped helpers, and authoring follow-through.

Independent of Tracks A–D. Independent of Tracks F–H. Lands as one or two PRs.

## Why this matters

Rocks scored 36 PARTIAL cards in `validated_cards_dsl.json` but **zero** ignored-test tag refs in the test tree. That's the signature of an archetype blocked on *authoring* rather than substrate — but with three reusable primitives that, once they land, unstick every authoring conversation in the archetype.

The Rocks gap-inputs doc names exactly three reusable items left open after Tracks A–J landed:

| Gap | Type | Blocks |
|---|---|---|
| **G-ROCKS-REVEAL-ORDERING** | DSL + engine | P-167, EX8-047, P-107, P-039, P-206, EX7-074, BT16-082 |
| **G-ROCKS-OPTION-SELF-DISPOSITION** | DSL ergonomics / raw_rust retirement | P-206, EX7-074, P-107, P-039, LM-031, EX10-069 |
| **G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS** | engine + DSL | BT9-103 |

Plus a face-up security lifecycle slice that's its own broader gap (do NOT take on here — surfaces in Dark Masters audit too; planned as a separate track).

Expected unblock: **~25 Rocks cards become authorable end-to-end**, plus authoring follow-through completes the remaining ~10. The reveal-ordering primitive is also reusable by every search effect across every archetype.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17 (no-approximations: the "place rest in any order" choice MUST be exposed to RL as an explicit ordering action, not auto-deterministic).
2. `qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md` — the canonical Rocks gap-inputs doc. Read it end-to-end. The three §G entries you're closing are §50, §123, §141.
3. `qa/archetype-qa/dsl/rocks.md` — the per-card walk through Rocks.
4. `qa/qa-reports/validated_cards_dsl.json` — search "Rocks" entries with status `PARTIAL` for the per-card notes describing the gap.
5. `docs/RUST_ENGINE_GAPS.md` § "Selection: ordered permutation" — the ordered-permutation primitive was claimed RESOLVED 2026-05-15, but the Rocks-author-facing DSL surface to drive it (`order_remainder`, `choose_from_reveal`) is the residual.
6. `code/digimon-engine/src/selection.rs` — `SelectionKind::OrderedPermutation` (per the 2026-05-15 sweep). Confirm shape.
7. `code/digimon-engine/src/effect_context/selections.rs` — `select_ordered_permutation` and `select_reveal_buckets`. Existing engine surface.
8. `code/digimon-engine/src/dsl_cards/step/` — find the existing reveal-step file. Add `choose_from_reveal` and `order_remainder` verbs.
9. `code/digimon-engine/cards/p/P-167.yaml` (if exists) and `code/digimon-engine/cards/p/P-206.yaml` — current Rocks card shape and current raw_rust usage.
10. `code/digimon-engine/src/cards/raw_rust/mod.rs` — every Rocks raw_rust entry. These are the modernization targets.
11. `data/cards.json` — printed text for P-167, P-206, EX7-074, P-107, P-039, EX10-069, BT9-103.

## Work to be done

### 1. `G-ROCKS-REVEAL-ORDERING` — author-facing reveal+choose+order verbs

Existing engine: `select_ordered_permutation` and `select_reveal_buckets` are wired. Missing: the DSL syntax authors actually want to write.

Add three composable DSL verbs (per the Rocks gap-inputs doc § "Suggested DSL shape"):

```yaml
- reveal:
    count: 3
    bind_as: revealed
- choose_from_reveal:
    from: revealed
    count: 1
    filter: { trait_any: [Mineral, Rock], play_cost_lte: 4 }
    destination: hand        # or: { play_free: true } / { source_of: <perm-binding>, position: bottom }
    optional: true
- order_remainder:
    from: revealed
    destination:
      choose_one: [deck_top, deck_bottom]
```

Where each verb lowers to a `CompiledStep::*` variant that consumes the existing engine helpers. The `destination` shape needs to support: hand, play-free, place-under-permanent (top/bottom source), deck-top, deck-bottom — match what Rocks cards actually print.

The `order_remainder` step installs an `OrderedPermutation` pending selection, then emits the remainder cards in the player-chosen order to the chosen destination.

Add a variant-coverage arm in the `CompiledStep` dispatcher (the PR #475 lint will require this — that's working as intended).

### 2. `G-ROCKS-OPTION-SELF-DISPOSITION` — modernize raw_rust to DSL

The DSL primitives `place_self_as_delay_option`, `add_this_option_to_hand`, and (likely missing) `trash_this_option` exist as of Group 5 / Track I. Several Rocks Option YAML files still use raw_rust because they were written before those primitives.

Rewrite the YAML for: **P-206, EX7-074, P-107, P-039, LM-031, EX10-069**. For each:

1. Read the printed Main and Security effect text from `data/cards.json`.
2. Replace the raw_rust hook with the matching DSL self-disposition verb.
3. Confirm existing behavioral test still passes (no test edits).
4. Remove the raw_rust function from `code/digimon-engine/src/cards/raw_rust/mod.rs` if no other card uses it.

If `trash_this_option` doesn't exist yet, add it as a small DSL verb lowering to `EffectContext::trash_self_from_option_battle_area()` (or equivalent — discover the right helper). Add variant-coverage arm.

### 3. `G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS` — author BT9-103

`ST13-08` and `BT14-009` already demonstrate the bilateral player-scoped flood-gate pattern via `kind: passive_modifier` with `target_player: opponent` (or `both_players`). BT9-103 needs:

- Player-scoped `CannotAttackPlayer` filtered by play-cost ≤ 7 with `Expiry::EndOfOpponentsTurn`.
- Player-scoped `CannotAddSecurityByEffect` (variant already landed in modifier registry per `docs/RUST_ENGINE_GAPS.md` "Track C/D" updates).

If `CannotAttackPlayer` with a play-cost filter doesn't yet have a DSL surface, that's a Track-I-style follow-up — note it explicitly and either pursue or descope. Otherwise, author BT9-103 YAML using existing primitives + write its behavioral test.

### 4. Author the long-tail Rocks cards

Once the three reusable items land, walk the per-card list in `qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md` § "Rocks-Local Authoring And Test Gaps". For each PARTIAL Rocks card, attempt the full YAML and behavioral test. The expected attrition rate is low — most should complete now.

**Do NOT take on:** face-up security lifecycle (BT20-055's flip rider, BT18-064-style face-up extraction). That's a substrate gap tracked separately; descope explicitly.

## Acceptance gates

- New DSL verbs `choose_from_reveal`, `order_remainder` parse + compile + execute, with variant-coverage lint passing.
- `trash_this_option` DSL verb exists (if not already).
- P-167 behavioral test exercises reveal+choose+order full flow with player-visible ordering selection (no auto-determinism).
- 6 Rocks Option YAML files (P-206, EX7-074, P-107, P-039, LM-031, EX10-069) modernized: raw_rust removed, DSL self-disposition used, tests pass.
- BT9-103 authored end-to-end with passing behavioral test.
- ≥ 20 Rocks PARTIAL cards advanced to IMPLEMENTED in `validated_cards_dsl.json`.
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage` continues to pass.
- No regression in `cards_behavioral`, `option_flow`, `selection`, or `dsl` suites.

## Constraints

- No-approximations: `order_remainder` MUST install a player-visible selection — even if the player would rationally always pick the same order, the choice is exposed.
- Working Rule 1: no `ACTION_SPACE_SIZE` change. Reveal-bucket and ordered-permutation selections reuse existing action surface per the Phase 4/Group 2 closures.
- Do NOT add new face-up security primitives — that's a separate substrate track.
- Do NOT remove raw_rust escapes outside the modernization list — other cards may depend on them.
- Source priority: printed text → Rules Manual → fandom wiki → DCGO. Many Rocks search effects have nuanced "without rearranging order" vs "in any order" vs "in any order to the top/bottom" — read each card's text carefully.

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal choose_from_reveal order_remainder
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_167 p_206 ex7_074 p_107 p_039 lm_031 ex10_069 bt9_103
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker discipline

- `qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md` — close G-ROCKS-REVEAL-ORDERING, G-ROCKS-OPTION-SELF-DISPOSITION, G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS entries with PR # citations.
- `qa/dsl-vocab-gaps.md` — close any reveal-ordering / option-self-disposition DSL entries.
- `qa/qa-reports/validated_cards_dsl.json` — advance Rocks cards from PARTIAL → IMPLEMENTED as their YAML completes.
- `docs/RUST_ENGINE_GAPS.md` § "Selection: ordered permutation" — confirm RESOLVED note holds; otherwise add closure follow-up.

## Order of operations

1. DSL verbs (`choose_from_reveal`, `order_remainder`, `trash_this_option` if missing) + variant-coverage compliance.
2. P-167 behavioral test as the first regression for ordered-permutation end-to-end.
3. Modernize 6 Option YAMLs; remove dead raw_rust.
4. BT9-103 authoring + test.
5. Walk PARTIAL Rocks cards in deck-pool order; author + test each that's now possible.
6. Tracker hygiene + PR(s).

## Out of scope

- Face-up security lifecycle (separate substrate track).
- Reveal-ordering for non-Rocks archetypes (the primitive is reusable, but authoring other archetypes' cards is their own track).
- Token / Plug-In substrate edges.
- Any change to action mask shape.

## Discovery rider

Rocks cards frequently reference DigiBurst N, source-trash inherited triggers, and Plug-Ins — all of which had recent substrate landings. If a Rocks card you attempt surfaces a NEW substrate gap (e.g., a printed text that no existing primitive supports), file it in `qa/archetype-qa/engine-gaps.md` + `docs/RUST_ENGINE_GAPS.md` and leave the card PARTIAL. Do NOT extend this PR to chase a substrate hole.
