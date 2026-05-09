# Red Hybrid AncientGreymon Rust DSL/Engine Gap Source

Date: 2026-05-03

Assessment workflow: `.codex/skills/assess-rust-engine-archetype/`

Assessment target: `RedHybrid`, resolved through `data/archetype_aliases.json` to `data/deck_library.json` archetype `Red Hybrid (AncientGreymon)`. The refreshed archetype has 15 decklists, 61 unique card IDs, and a generated pool at `qa/archetype-qa/red-hybrid-ancientgreymon/deck_pool.json`.

This is a spec-input artifact. It distills the reusable Rust engine and YAML DSL gaps surfaced by Red Hybrid into cross-archetype capability groups. It is not a card implementation plan, and missing card YAML should not be promoted into the shared roadmap unless authoring proves a reusable primitive is still absent.

## Current Verdict

`blocked`

Red Hybrid is not currently implementable faithfully as executable Rust YAML DSL.

The deck's core gameplay is Tamer-based Hybrid evolution, recursive Tamer play, effect-initiated digivolution, attack-window evolution, and security-pressure payoffs. The current Rust engine/DSL has useful pieces for keywords, reveal selections, Delay options, cost hooks, source placement, and effect-initiated digivolve, but the archetype still needs several reusable primitives before its central cards can be implemented without hidden choices or raw-Rust one-offs.

## Coverage Snapshot

- Archetype pool: 61 unique card IDs.
- Current `RedHybrid` resolution: `Red Hybrid (AncientGreymon)`, 15 decklists, sources `digilab:14` and `digimonmeta:1`.
- Production YAML found under `code/digimon-engine/cards/**`: `BT1-090`, `BT14-001`, `BT16-082`, `BT17-009`, `BT21-013`, `BT21-072`, `BT8-097`, `EX8-074`, `P-035`.
- Example-only YAML also exists for `_examples/BT14-009` and `_examples/BT18-102`; those are not production set specs.
- High-frequency Red Hybrid cards without production YAML include `BT17-012`, `BT18-088`, `BT21-082`, `BT18-011`, `BT7-014`, `BT17-017`, `BT7-008`, `BT17-011`, `BT17-079`, `BT12-009`, `BT12-088`, `BT17-094`, `BT12-017`, `BT4-113`, `BT21-020`, `BT6-010`, `BT7-085`, `BT18-010`, and `EX1-066`.

## Core Archetype Pressure Points

| Card | Role | Rust readiness pressure |
|---|---|---|
| `BT17-012` BurningGreymon | Main attacker and bridge | Tamer-as-level-3 digivolve, Raid, attack-window effect digivolve into Hybrid with cost reduction. |
| `BT17-009` Flamemon | Searcher / recursion source | Implemented 2026-05-03 in production YAML with behavioral tests for multi-bucket reveal, no duplicate bucket picks, bottom remainder, and inherited free Tamer play on deletion. |
| `BT18-088` Takuya Kanbara & Koji Minamoto | Core Tamer / source engine | Security play, memory setter, start-main count-scaled source placement under this Tamer, inherited end-turn player attack. |
| `BT21-013` Agunimon | Hybrid bridge | Production YAML exists, but relies on bottom-source placement and attack-window effect digivolve behaving correctly under masks. |
| `BT21-082` Takuya Kanbara | Main Tamer / payoff | Start-main effect-initiated digivolve from Digimon or Tamer into Hybrid/Hero with dynamic cost reduction; inherited security-removed free Tamer play. |
| `BT17-017` AncientGreymon | Top end | DigiXros, DP-based deletion, on-deletion return two card categories from trash, then optional free Tamer play. |
| `BT17-011` Agunimon | Warp bridge | Tamer-as-level-3 digivolve, conditional hand digivolve into AncientGreymon ignoring requirements, scheduled self-delete if digivolved by this effect. |
| `BT17-094` Ancient Guardian Deity | Option extender | Conditional color-requirement ignore, trash-to-hand, reduced-cost play from hand, security free Tamer play from hand or trash plus add self to hand. |
| `BT21-020` Aldamon | Security pressure / recursion | Cost reduction based on named sources; on-deletion free red inherited-effect Tamer play from hand or trash, both as top and inherited. |
| `BT16-082` / `P-123` Ukkomon | Movement payoff | Breeding-to-battle observer, reveal/add flow, optional hatch tail. |

## Reusable Gap Backlog

### RH-01: Tamer-as-digivolution-base action mask and execution

- **Type:** `engine-gap`, `dsl-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`; `qa/dsl-vocab-gaps.md`
- **Blocks Red Hybrid cards:** `BT17-012`, `BT17-011`, `BT12-012`, `BT12-013`, `BT7-085`, `BT18-088`, `BT21-082`
- **Cross-archetype value:** Every Hybrid deck that evolves onto Tamers; any future "treat this Tamer as level N Digimon" route.
- **Printed shape:** "You may digivolve this card from your hand onto one of your red Tamers as if that card is a level 3 red Digimon."
- **Missing capability:** The normal digivolve mask currently scans hand Digimon against battle-area Digimon and breeding permanents. It does not expose Tamer permanents as legal digivolution bases under card-specific alt-path text.
- **Why it matters:** Red Hybrid's core line is built around turning Tamers into Hybrid stacks. If Tamers cannot be selected as digivolution bases through the action mask, the deck cannot perform its main legal actions.
- **First test:** With `BT17-079` Takuya in battle area and `BT17-012` in hand, the action mask exposes a digivolve action onto the Tamer at the printed Tamer route cost. Resolving the action creates a stack with the Tamer as source and fires When Digivolving.
- **Implementation hint:** `code/digimon-engine/src/action/mask.rs`, `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/game.rs::can_digivolve`, `code/digimon-dsl/src/spec.rs` alt-path representation, and `code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs`.
- **Status update (2026-05-03):** RESOLVED for normal hand-to-battle-area action mask and execution using DSL `alt_paths.kind: digivolve` plus `source_treated_as`; printed Tamer kind remains unchanged. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- hybrid_tamer_digivolve phase2e_select_union_zone --nocapture`.

### RH-02: Effect-initiated digivolve from Tamer or Digimon bases with dynamic cost formulas

- **Type:** `engine-gap`, `dsl-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`; `qa/dsl-vocab-gaps.md`
- **Blocks Red Hybrid cards:** `BT21-082`, `BT18-088`, `BT17-011`, `BT17-012`, `BT21-013`, `P-029`
- **Cross-archetype value:** Hybrid Tamers, warp digivolve effects, and cards that let "1 of your Digimon or Tamers" digivolve during an effect.
- **Printed shape:** "1 of your Digimon or Tamers may digivolve into a Digimon card with the [Hybrid] or [Hero] trait in the hand. For each of your red Tamers with different names, reduce this effect's digivolution cost by 1."
- **Missing capability:** `effect_initiated_digivolve` exists for some hand-to-permanent flows, but Red Hybrid needs effect selection over a union of Digimon and Tamer bases, card-specific Hybrid/Hero filters, ignore-requirements variants, delayed self-delete riders, and cost formulas based on distinct Tamer names.
- **Why it matters:** These effects are player-visible choices with cost-sensitive alternatives. Auto-picking the only candidate or forcing only Digimon targets would violate the no-approximations policy.
- **First test:** `BT21-082` starts main with one Digimon and one red Tamer on field, two legal Hybrid/Hero hand cards, and two differently named red Tamers. The pending selection exposes both bases and legal evolution cards, applies the correct dynamic cost reduction, and declines cleanly.
- **Implementation hint:** `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/action/`, `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`, formula inputs for distinct named Tamers, and event attribution for "digivolved by this effect".

### RH-03: Scheduled self-delete tied to a specific effect-initiated digivolve

- **Type:** `engine-gap`, `dsl-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`
- **Blocks Red Hybrid cards:** `BT17-011`, `P-029`
- **Cross-archetype value:** Any "if digivolved by this effect, delete it at end of turn" or delayed cleanup tied to the exact permanent created/modified by an effect.
- **Printed shape:** "If digivolved by this effect, delete this Digimon at the end of the turn."
- **Missing capability:** A delayed end-of-turn task must bind the exact permanent that resulted from the effect digivolve, survive battle-area index shifts, and no-op if the permanent has already left.
- **Why it matters:** Deleting by card name or current field slot can hit the wrong stack; omitting the deletion gives Red Hybrid a permanent boss with no printed drawback.
- **First test:** `BT17-011` digivolves into `BT17-017` by its own effect. At end of turn, the resulting stack is deleted. If the stack leaves before end of turn, the scheduled cleanup does nothing and does not delete a new permanent in the same slot.
- **Implementation hint:** Reuse or generalize scheduled delayed permanent-handle markers in `code/digimon-engine/src/effect_queue.rs`, `code/digimon-engine/src/effect_context/`, and `code/digimon-engine/src/game_actions.rs`.

### RH-04: Union-zone free play of Tamers/Digimon from hand or trash with inherited-effect filters

- **Type:** `engine-gap`, `dsl-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`; `qa/dsl-vocab-gaps.md`
- **Blocks Red Hybrid cards:** `BT17-017`, `BT21-020`, `BT21-082`, `BT17-094`, `AD1-002`, `AD1-020`, `BT12-012`, `BT12-017`, `BT7-008`. `BT17-009` is no longer blocked by this item after the 2026-05-03 production YAML/tests.
- **Cross-archetype value:** Security effects, on-deletion recursion, and options that play cards from hand/trash while filtering by Tamer inherited text, trait, color, or name.
- **Printed shape:** "You may play 1 red Tamer card with inherited effects from your hand or trash without paying the cost."
- **Missing capability:** The DSL has hand/trash selection and play-free shapes, but Red Hybrid needs one pending selection spanning hand and trash, with zone-preserving movement and filters such as "Tamer with inherited effects" and "red Tamer".
- **Why it matters:** Separate hand-then-trash prompts change the choice surface. Hidden fallback to trash if hand has no target is still a player-visible decision if both zones contain legal cards.
- **First test:** `BT21-020` is deleted with a red inherited-effect Tamer in hand and one in trash. The mask exposes a single optional choice over both candidates, then plays the selected card without paying cost.
- **Implementation hint:** `code/digimon-engine/src/selection.rs`, `code/digimon-engine/src/action/`, `code/digimon-engine/src/effect_context/`, `code/digimon-dsl/src/step.rs::SelectUnionZone`, and card-data predicates for `has_inherited`.

### RH-05: Multi-category reveal selection with ordered remainder handling

- **Type:** resolved reusable DSL primitive; remaining card-authoring / test coverage gap
- **Tracker:** `qa/dsl-vocab-gaps.md`; `docs/RUST_ENGINE_GAPS.md`
- **Blocks Red Hybrid cards:** `BT18-010`, `BT7-008`, `BT16-082`, `P-035`, `LM-033`, `EX1-066` still need production YAML and card-shaped tests. `BT17-009` is implemented and no longer blocked by this item after the 2026-05-03 production YAML/tests.
- **Cross-archetype value:** Searchers that add one card per category, memory boosts, Training cards, and reveal effects that return the rest to bottom in any order.
- **Printed shape:** "Reveal the top 3 cards. Add 1 card with [Hybrid]/[Ten Warriors] and 1 Tamer card with inherited effects among them to the hand. Return the rest to the bottom of the deck."
- **Current evidence:** As of 2026-05-03, the reusable `select_reveal_buckets` / multi-category reveal primitive is resolved. It supports multi-slot picks from one reveal pool with separate predicates, duplicate prevention, bucket result binding, and deck-bottom remainder disposition.
- **Why it matters:** These searchers produce meaningful choices when multiple legal cards satisfy each category. Choosing categories independently without stable reveal references can duplicate a card or hide legal combinations.
- **Remaining work:** Author and test remaining searchers such as `BT18-010`, `BT7-008`, `BT16-082`, `P-035`, `LM-033`, and `EX1-066` against the resolved primitive unless their printed text surfaces a new reusable gap.
- **First remaining test:** `BT18-010` or `BT7-008` reveals cards that satisfy overlapping Red Hybrid categories, exposes legal non-duplicate bucket picks, and bottoms the unchosen cards according to its printed text.
- **Implementation hint:** reuse `select_reveal_buckets` in production YAML; add new engine/DSL gap entries only if a card's printed remainder ordering, category filter, or follow-up branch cannot be expressed faithfully.

### RH-06: Source placement under Tamers and count-scaled multi-source placement

- **Type:** `dsl-gap`, `engine-gap` if Tamer source stacks are incomplete
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`; `qa/dsl-vocab-gaps.md`
- **Blocks Red Hybrid cards:** `BT18-088`, `BT21-013`, `BT17-094`, `AD1-020`
- **Cross-archetype value:** Tamers that accumulate sources, Hybrid decks, and cards that place selected cards under either a Digimon or Tamer.
- **Printed shape:** "You may place up to 1 [Hybrid] trait card with different names from your trash under this Tamer. For each of your other Tamers, add 2 to the maximum number this effect may place."
- **Missing capability:** `place_as_bottom_source` can place under a selected destination in current YAML, but Red Hybrid needs Tamer destinations to behave as source-bearing permanents, count formulas based on other Tamers, different-name constraints across selections, and up-to-N PASS behavior.
- **Why it matters:** Tamer source stacks are a resource for later Hybrid evolution and inherited attacks. Auto-filling the maximum or ignoring name uniqueness changes the deck's strategy.
- **First test:** `BT18-088` with two other Tamers in play and several Hybrid cards in trash exposes up to 5 selectable Hybrid cards with different names, supports PASS after one or more selections, and places the chosen cards under that Tamer in chosen order.
- **Implementation hint:** `code/digimon-engine/src/permanent.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/action/`, `code/digimon-dsl/src/predicate.rs`, and count-capped selection lowering.

### RH-07: In-effect immediate attacks and inherited end-turn player attacks

- **Type:** `engine-gap`, `dsl-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`; `qa/dsl-vocab-gaps.md`
- **Status:** partially resolved. BT21-072's self may-attack-without-suspending route is implemented and tested; player-only inherited end-turn attack variants remain to be card-authored/verified.
- **Blocks Red Hybrid cards:** `BT18-088`, `BT18-018`, `BT7-016`; BT21-072 no longer blocks on this primitive.
- **Cross-archetype value:** Any "this Digimon may attack", "may attack a player", or "attack without suspending" effect.
- **Printed shape:** "[End of Your Turn] [Once Per Turn] This Digimon with the [Hybrid] or [Ten Warriors] trait may attack a player."
- **Implemented capability:** Attack prompts installed by effects can reuse combat target legality and action masks while restricting allowed targets by the effect text. Red Hybrid still needs player-only inherited attack card coverage.
- **Why it matters:** End-turn attacks often happen after memory has passed. Ordinary main-phase attack masks do not capture this effect-specific attack permission, and auto-attacking hides a choice.
- **Evidence / first remaining test:** BT21-072 is covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_072_when_digivolving_may_attack_without_suspending`. A Digimon with `BT18-088` as a source should still be tested at end of turn while having Hybrid/Ten Warriors trait, exposing PASS plus player attack actions only.
- **Implementation hint:** `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/action/`, `code/digimon-engine/src/effect_context/`, and DSL steps such as `may_attack_now` / `may_attack_player`.

### RH-08: Raid target switching and attack-target-change event payloads

- **Type:** `card-yaml/test-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`
- **Blocks Red Hybrid cards:** `BT17-012`, `BT21-072`
- **Cross-archetype value:** Raid decks, Collision/Raid interaction, and cards that observe or prevent attack target changes.
- **Printed shape:** `<Raid>` switches an attack target to one of the opponent's unsuspended Digimon with the highest DP.
- **Status:** The reusable combat primitive is closed. Raid now opens a printed optional `PendingSelection` after declaration, before later interrupt windows; selecting a new target fires `OnAttackTargetChange` with a Raid payload, and PASS keeps the original target.
- **Why it matters:** BurningGreymon uses Raid as a main combat pressure tool. If Raid is parsed but does not install the target-switch choice, gameplay diverges materially.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- raid_retarget`.
- **Next card test:** `BT17-012` attacks a player while the opponent has two unsuspended Digimon tied for highest DP. The pending Raid prompt should expose those choices, update the attack target if selected, and leave the original player attack if declined.

### RH-09: Security-removed inherited observers that play cards

- **Type:** `engine-gap`, `test-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`
- **Blocks Red Hybrid cards:** `BT14-001`, `BT21-082`
- **Cross-archetype value:** Security-pressure decks with inherited observers, including Medusamon and WarGreymon-family shells.
- **Printed shape:** "[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, you may play 1 red Tamer card from your hand without paying the cost."
- **Missing capability:** Global security-removed observer timing and inherited fan-out have been improved, but Red Hybrid still needs a card-shaped test where the inherited effect installs a free-play selection rather than a simple draw/memory gain.
- **Why it matters:** The core payoff is not just observing security removal; it chains into a player-visible hand selection and a free play.
- **First test:** A Hybrid stack with `BT21-082` in sources removes opponent security during the owner's turn. The inherited OPT fires once, exposes legal red inherited-effect Tamer hand candidates, and plays the chosen Tamer for free.
- **Implementation hint:** `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/effect_queue.rs`, inherited effect dispatch, and union-zone/free-play helpers from `RH-04`.

### RH-10: Option self-disposition and conditional color-requirement bypass

- **Type:** `dsl-gap`, `engine-gap` where mask checks are missing
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`; `qa/dsl-vocab-gaps.md`
- **Blocks Red Hybrid cards:** `BT17-094`, `P-035`, `LM-033`, `BT4-098`, `BT8-097`
- **Cross-archetype value:** Memory Boosts, Scrambles, Training cards, two-color Options, and options that ignore color requirements under field conditions.
- **Printed shape:** "While you have a Tamer or [Hybrid] trait Digimon, you can ignore this card's color requirements"; security effects may add the resolving option to hand or place it in battle area.
- **Missing capability:** Group 5 and Group 6 closed many option pieces, but Red Hybrid needs production card tests proving conditional color-bypass masks, option self-add-to-hand, place-as-Delay, and security activation all work through DSL-only syntax.
- **Why it matters:** Options are major extenders. Raw-Rust self-disposition or broad color bypass risks making unrelated option plays legal.
- **First test:** `BT17-094` is playable without normal color requirements only while the player controls a Tamer or Hybrid Digimon. Its Security effect plays a legal inherited-effect Tamer from hand or trash, then adds `BT17-094` to hand.
- **Implementation hint:** `code/digimon-engine/src/action/mask.rs`, option flow/disposition code, `code/digimon-engine/src/dsl_cards/lower_flood_gate.rs`, and `step/zone_moves.rs`.

### RH-11: Battle/security-effect suppression from inherited sources

- **Type:** `engine-gap`, `dsl-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`
- **Blocks Red Hybrid cards:** `BT7-014`
- **Cross-archetype value:** Cards that suppress Option security effects, disable security effects, or alter how checked security cards resolve.
- **Printed shape:** "[Your Turn] While this Digimon has the [Hybrid] or [Ten Warriors] trait, it doesn't activate [Security] effects on Option cards it checks."
- **Missing capability:** A modifier or combat/security hook must suppress only checked Option cards' Security effects for this attacking stack under a trait condition.
- **Why it matters:** Security-effect suppression is combat-relevant and cannot be approximated by deleting the checked card or ignoring all security effects.
- **First test:** A Hybrid stack with `BT7-014` in sources checks an Option with a Security effect. The Option's Security effect does not activate, while non-Option Security Digimon/Tamer effects still follow their normal rules.
- **Implementation hint:** `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/modifiers.rs`, inherited aura/modifier lowering, and security resolution tests.

## Red-Hybrid-Local Authoring Backlog

These items are required for archetype readiness but should remain card-authoring tasks unless they expose missing reusable primitives.

Task 10 production-authoring audit update (2026-05-03): `BT17-009` is now implemented in production YAML and covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_009 --nocapture`. This closes the Flamemon-specific multi-bucket reveal and inherited free Tamer play blocker. Red Hybrid remains `blocked` because the broader archetype pool still has many missing production YAML cards and unresolved card-local blockers below.

| Card(s) | Status | Next Rust test |
|---|---|---|
| `BT17-012` | Core card missing production YAML; depends on card-local use of resolved `RH-01`, plus `RH-02` and `RH-08` | Card-shaped Tamer-base digivolve regression, Raid selection, attack-window Hybrid digivolve with cost reduction. |
| `BT17-009` | Implemented 2026-05-03; production YAML and 3 focused behavioral tests pass | Keep in validated-cards report as implemented; no remaining BT17-009-specific blocker from this audit. |
| `BT18-010`, `BT7-008` | Searcher YAML missing; reveal primitive closed, but full production authoring still depends on card-local inherited play needs | Reveal multi-pick and bottom remainder, then inherited free Tamer play where printed. |
| `BT18-088` | Core Tamer missing YAML; depends on `RH-06` and `RH-07` | Start-turn memory setter, start-main count-scaled source placement, inherited end-turn player attack. |
| `BT21-082` | Core Tamer missing YAML; depends on `RH-02`, `RH-04`, `RH-09` | Start-main Digimon/Tamer effect digivolve with dynamic reduction, inherited security-removed Tamer play. |
| `BT17-017`, `BT4-113`, `BT12-017` | Top-end YAML missing | DP-based deletion, on-deletion category returns, free Tamer/Digimon play, Security Attack behavior. |
| `BT17-011`, `P-029` | Warp bridge YAML missing; depends on `RH-02` and `RH-03` | Effect digivolve into AncientGreymon, ignore requirements where printed, scheduled self-delete. |
| `BT17-094` | Option YAML missing; depends on `RH-04` and `RH-10` | Conditional color bypass, trash-to-hand, reduced-cost play from hand, security free Tamer play plus add self to hand. |
| `BT16-082`, `P-123` | `BT16-082` production YAML is a stale placeholder; `P-123` missing | Breeding-to-battle observer, reveal/add, optional hatch tail. |
| `BT21-013`, `BT21-072`, `BT8-097`, `EX8-074`, `P-035` | Production YAML exists; BT21-072 revalidated for Track D may-attack-without-suspending | Replace remaining comments/workarounds with behavioral coverage or mark exact remaining sub-gaps. |
| `_examples/BT14-009`, `_examples/BT18-102` | Example-only specs | Promote to production only when the printed behavior is fully covered by tests. |

## Cross-Archetype Spec Compile Notes

When compiling the future shared DSL/engine spec, group Red Hybrid's gaps by capability, not by card:

1. **Hybrid/Tamer digivolution surface:** `RH-01` and `RH-02`.
2. **Delayed identity-bound cleanup:** `RH-03`.
3. **Union-zone free play and Tamer filters:** `RH-04`.
4. **Remaining reveal-searcher card authoring/tests using the resolved multi-pick primitive:** `RH-05`.
5. **Source placement under Tamers:** `RH-06`.
6. **Effect-granted attacks:** `RH-07`.
7. **Raid and target-change semantics:** `RH-08`.
8. **Security-removed inherited observer chains:** `RH-09`.
9. **Option disposition and color bypass:** `RH-10`.
10. **Security-effect suppression:** `RH-11`.

Recommended first slices:

1. Start with `RH-01` because Tamer-as-base digivolution is the archetype's legal-action foundation and likely helps every Hybrid color.
2. Follow with `RH-04` and remaining `RH-05` card-authoring/tests because union-zone free play and multi-category reveal searchers recur across many non-Hybrid archetypes.
3. Then tackle `RH-07` because DNA Omnimon, BG Imperial, and Red Hybrid all surfaced immediate/effect-granted attacks.
4. Use `BT21-082` or `BT18-088` as later integration anchors only after the base primitives have isolated fixture tests.

## Acceptance Gates For Any Gap-Closure Spec

- Do not expand `ACTION_SPACE_SIZE` or active tensor contracts as a side effect of Red Hybrid unlock work. If a missing choice cannot fit existing pending-selection action IDs, split that into a separate action/tensor contract plan.
- Every player-visible choice must be surfaced through an action mask or `PendingSelection`, including optional one-card choices.
- DSL vocabulary must lower to real engine behavior; no YAML that compiles into no-op readiness.
- Each reusable primitive needs at least one focused fixture test and one card-shaped regression.
- Tracker updates must distinguish `docs/RUST_ENGINE_GAPS.md` engine primitives from `qa/dsl-vocab-gaps.md` vocabulary/lowering work.
- Card YAML authoring should remain separate from the cross-archetype gap spec unless implementation proves the primitive is missing.

## Verification References

Read-only verification commands used around the 2026-05-03 Red Hybrid refresh:

```powershell
python -m json.tool data\archetype_aliases.json
python -m json.tool data\deck_library.json
python -m json.tool qa\archetype-qa\red-hybrid-ancientgreymon\deck_pool.json
$env:PYTHONIOENCODING='utf-8'; python code\tools\resolve_deck.py RedHybrid --json
```

Expected resolver facts for the refreshed target:

- `archetype_name`: `Red Hybrid (AncientGreymon)`
- `total_decklists`: `15`
- `unique_cards`: `61`
- `deck_pool_path`: `qa\archetype-qa\red-hybrid-ancientgreymon\deck_pool.json`
