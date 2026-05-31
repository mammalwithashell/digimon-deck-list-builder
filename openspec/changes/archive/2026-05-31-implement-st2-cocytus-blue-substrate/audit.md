## Baseline Audit

### 1.1 Deck Composition Source

Deck composition source: DigimonCardGame Wiki page `ST-2: Starter Deck Cocytus Blue`, user-provided URL:

`https://digimoncardgame.fandom.com/wiki/ST-2:_Starter_Deck_Cocytus_Blue`

The page lists Worldwide / English deck contents as 54 playable cards, 16 unique cards:

- `ST2-01` x4
- `ST2-02` x4
- `ST2-03` x4
- `ST2-04` x4
- `ST2-05` x4
- `ST2-06` x2
- `ST2-07` x4
- `ST2-08` x4
- `ST2-09` x4
- `ST2-10` x2
- `ST2-11` x2
- `ST2-12` x4
- `ST2-13` x4
- `ST2-14` x4
- `ST2-15` x2
- `ST2-16` x2

`ST2-01` is the 4-card Digi-Egg deck. The remaining cards total 50 main-deck cards.

### 1.2 Card Text Audit from `data/cards.json`

`data/cards.json` has metadata for every card from `ST2-01` through `ST2-16`.

| Card | Name | Printed behavior summary |
| --- | --- | --- |
| `ST2-01` | Tsunomon | Digi-Egg inherited: `[Your Turn]` carrier gets +1000 DP when battling an opponent Digimon with no digivolution cards. |
| `ST2-02` | Gomamon | Vanilla blue Lv.3 Digimon. |
| `ST2-03` | Gabumon | Inherited `[When Attacking]`: trash bottom digivolution card of 1 opponent Digimon with level 5 or less. |
| `ST2-04` | Bearmon | Vanilla blue Lv.3 Digimon. |
| `ST2-05` | Ikkakumon | Vanilla blue Lv.4 Digimon. |
| `ST2-06` | Garurumon | Inherited `[When Attacking]`: trash bottom digivolution card of 1 opponent Digimon. |
| `ST2-07` | Grizzlymon | `<Blocker>` and `[When Attacking] Lose 2 memory.` |
| `ST2-08` | WereGarurumon | Inherited `[Your Turn]`: carrier gains `<Security A. +1>` while opponent has a Digimon with no digivolution cards. |
| `ST2-09` | Zudomon | `[When Digivolving]`: trash 2 bottom digivolution cards of 1 opponent Digimon. |
| `ST2-10` | Plesiomon | Vanilla blue Lv.6 Digimon. |
| `ST2-11` | MetalGarurumon | `[When Attacking] [Once Per Turn] Unsuspend this Digimon.` |
| `ST2-12` | Matt Ishida | Tamer: `[Start of Your Turn]` gain 1 memory if opponent has a Digimon with no digivolution cards; security plays self free. |
| `ST2-13` | Hammer Spark | Option: `[Main]` gain 1 memory; security gain 2 memory. |
| `ST2-14` | Sorrow Blue | Option: choose opponent Digimon with no digivolution cards; it cannot attack or block until end of opponent's next turn. Security variant lasts until end of your next turn. |
| `ST2-15` | Kaiser Nail | Option: choose 1 Digimon digivolution card under one of your Digimon and play it without paying the cost. Security activates main. |
| `ST2-16` | Cocytus Breath | Option: return 1 opponent Digimon to hand. Security activates main. |

### 1.3 Current ST2 YAML Coverage

`code/digimon-engine/cards/st2/` contains JSON metadata for every ST2 card but production YAML only for `ST2-13`.

`ST2-13.yaml` faithfully maps its two printed clauses:

- `when: main_from_hand` -> `gain_memory: 1`
- `when: on_security` -> `gain_memory: 2`

No ST2 card except `ST2-13` is currently in production DSL ownership.

### 1.4 Current Substrate Verification

Verified by grepping current `code/digimon-dsl/src`, `code/digimon-engine/src`, and `code/digimon-engine/tests/dsl`:

- `select_opponent_sources` exists in DSL spec/compiled/lowering and is covered by `phase2g_select_sources.rs`.
- `select_material`, `select_materials`, and `play_from_materials` exist and have source-play tests, including material extraction and batch play.
- `stack_size_lte`, `stack_size_gte`, and `materials_count_lte` predicates are present and evaluated in `dsl_cards/predicate.rs`.
- `CannotAttack` and `CannotBlock` are recognized by the DSL modifier map and engine modifier consult sites.
- `play_from_security` exists and is used for Tamer/security self-play shapes.
- `return_to_hand` exists as a DSL step and is covered by zone movement tests.
- `gain_memory`, `lose_memory`, `unsuspend`, `grant_keyword`, and `SecurityAttackPlus` are established DSL/engine surfaces.

### 1.5 Clause/Substrate Classification

Implementable with existing vocabulary:

- Vanillas: `ST2-02`, `ST2-04`, `ST2-05`, `ST2-10`.
- `ST2-07` Blocker + lose memory.
- `ST2-08` inherited self aura with opponent no-source board predicate.
- `ST2-11` when-attacking once-per-turn unsuspend.
- `ST2-12` start-turn memory + security self-play.
- `ST2-13` already implemented.
- `ST2-14` target no-source opponent Digimon + `CannotAttack` / `CannotBlock` modifiers.
- `ST2-16` return opponent Digimon to hand.

Needs new or proven substrate:

- `ST2-01`: needs a battle-context predicate that tests the opposing battled Digimon's source count, not the broader board.
- `ST2-03`, `ST2-06`, `ST2-09`: need no-choice bottom-source trash. Existing `select_opponent_sources` would introduce an artificial source-card choice.
- `ST2-15`: likely expressible with `select_material` + `play_from_materials`, but needs explicit ST2 card-shaped proof before authoring is considered complete.
