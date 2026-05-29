## Why

DSL `bind_as` on `play_from_hand_free` (and its `play_from_trash` / `play_from_revealed_free` / `play_from_materials` / `play_union_bound_free` / `play_token` siblings) stores a positional `PermanentHandle { player, index }` into the bindings map. When that handle is captured by a `schedule_delayed` body and the bound permanent is later consumed by a stack-changing event — DNA digivolve, regular digivolve, deletion-with-reslot — the handle stops referring to the originally-played card but the body resolves it as if nothing happened. The delayed action then mis-targets whatever permanent currently occupies that slot.

Concretely, **BT16-085 Davis Motomiya & Ken Ichijoji** plays a Veemon/Wormmon for free and schedules "return *it* to the hand" at the next end-of-opponent's-turn. If the played Veemon digivolves to ExVeemon (Blue Lv.4) and then DNA-digivolves with a Stingmon (Green Lv.4) into BT16-025 Paildramon, the engine's `dna_digivolve_inner` keeps `target_a`'s positional handle for the merged stack. At end of opponent's turn the scheduled `return_to_hand` resolves the stale handle, finds Paildramon at that slot, and bounces the Paildramon stack — sending Paildramon to hand and Veemon + ExVeemon + Stingmon to trash. DCGO's [`BT16_085.cs`](DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_085.cs#L115) registers the bounce on the `Permanent` object reference and guards it with `IsPermanentExistsOnBattleArea(selectedPermanent)`; the Jogress flow ([`CardController.cs`](DCGO/Assets/Scripts/Script/CardController.cs#L1505)) destroys the old evo-root Permanents and constructs a fresh Paildramon Permanent, so DCGO's check returns false and the bounce silently fizzles — which is the rules-correct reading of "return *it*" when *it* no longer exists as a Digimon.

This is a no-approximations-policy violation: the engine fires a deletion/bounce on a permanent the printed text never referenced, with collateral trashing of digivolution cards.

## What Changes

- DSL `bind_as` on `play_from_hand_free`, `play_from_revealed_free`, `play_from_materials`, `play_union_bound_free`, and `play_token` SHALL track the played card by stable identity (`ProvenanceToken` keyed to the played `CardHandle`), not by positional `PermanentHandle`.
- `BindingValue::Permanent(PermanentHandle)` continues to support intra-resolution selection targeting (e.g. `select_own_permanent: { bind_as: tgt }`). A new variant `BindingValue::PlayedPermanent { token: ProvenanceToken, fallback: PermanentHandle }` is introduced for play-verb bindings that need to survive a `schedule_delayed` boundary.
- The DSL resolver SHALL convert a `PlayedPermanent` back to a `PermanentHandle` at consume time via a new strict helper `Game::resolve_token_as_battle_area_top` that yields `Some(handle)` only when the played card is still the **top card** of a battle-area permanent. Resolution to anything else (digivolution card under a different top, hand/trash/security/deck, unresolvable) yields `None` and downstream consumers (`return_to_hand`, `delete_permanent`, `suspend`, etc.) silently no-op — matching DCGO's `IsPermanentExistsOnBattleArea(selectedPermanent)` semantics.
- `schedule_delete_played_at_turn_end` (used by EX11-022 Karakurumon, EX11-061 Mirai Kinosaki, P-165 ShoeShoemon) SHALL keep its existing **permissive** carrier-deletion semantics via a new sibling helper `resolve_played_permanent_permissive` that calls the existing `Game::resolve_provenance_token` (which returns the carrier handle for cards anywhere in the stack — top card OR digivolution card). This preserves "delete the Digimon this effect played" semantics for cards where the rules-intent is to delete the carrier even after a digivolve buries the played card.
- A regression behavioral test under `code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs` SHALL exercise the Davis-and-Ken → DNA-into-Paildramon → opp-EOT sequence and assert (a) Paildramon stays on the field, (b) the merged stack still has its digivolution cards (no return-to-hand happened), (c) P0's hand does NOT contain Paildramon.
- Bonus invariant tests SHALL cover the adjacent cases: (a) played Digimon regularly digivolves (the bounce STILL fizzles), (b) played Digimon is deleted by another effect before opp EOT (silent no-op).
- **NOT changing**: the inner mechanics of `dna_digivolve_inner` (target_a kept, target_b removed, card_sources merged) remain as-is. Provenance is the right place to capture identity; the merge code does not need to special-case Davis-style hooks.

## Capabilities

### New Capabilities

(none — this is a refinement of an existing capability)

### Modified Capabilities

- `dsl-card-scripting-vocabulary`: the contract for `bind_as` on play verbs is sharpened. A played-permanent binding survives stack-changing events (regular digivolve, DNA digivolve, deletion) only as a provenance token; downstream verbs that consume the binding via the strict resolver (`return_to_hand`, `delete_permanent`, `add_modifier`, etc., through `resolve_binding_ref`) SHALL silently no-op when the played card is no longer a battle-area top. Verbs that need carrier-aware semantics (`schedule_delete_played_at_turn_end`) use the permissive resolver explicitly. Intra-resolution bindings produced by selection verbs (e.g. `select_own_permanent`'s `bind_as`) are unchanged — they remain positional handles because they cannot outlive their resolution.

## Impact

- **Affected code (Rust engine)**:
  - `code/digimon-engine/src/dsl_cards/bindings.rs` — `BindingValue` enum extension + `insert_played_permanent` / `get_played_permanent` helpers.
  - `code/digimon-engine/src/game.rs` — new strict helper `Game::resolve_token_as_battle_area_top`.
  - `code/digimon-engine/src/dsl_cards/binding_ref.rs` — `resolve_binding_ref` learns to resolve a `PlayedPermanent` binding via the strict helper; new sibling `resolve_played_permanent_permissive` for carrier-deletion semantics.
  - `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` — new `bind_played_with_provenance` helper; all 5 play-verb `bind_as` sites switch to it; `ScheduleDeletePlayedAtTurnEnd` switches to the permissive resolver.
- **Affected card YAML**: none. The DSL surface (`bind_as: played` on play verbs) is unchanged from the author's perspective. Cards using the existing pattern (BT16-085 today; future siblings) get the corrected behavior automatically.
- **Affected tests**: new behavioral tests in `code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs`. Two dsl-suite tests (`play_token_bind_as::play_token_binds_created_handle`, `phase2f1_play_steps::play_from_revealed_free_step_consumes_reveal_and_keeps_memory`) call `bindings.get_permanent` directly on play-verb bindings; they need to switch to `bindings.get_played_permanent` (the new diagnostic getter).
- **Cross-engine parity**: this brings Rust in line with DCGO's `Permanent`-object-reference identity model. `docs/RUST_ENGINE_API.md`'s section on `bind_as` semantics is updated to document the played-token contract; `docs/RUST_PYTHON_PARITY.md` is NOT updated (Python is sunsetted; this is a Rust-vs-DCGO alignment).
- **Performance**: `ProvenanceToken` resolution is O(battle-area-size) on the resolver side and scheduled-effect queue depth is small (typically < 5); no measurable cost.
- **Risk**: scheduled bounces for cards like Davis & Ken will START fizzling in DNA scenarios where they currently fire — this is a behavioral regression *for the buggy behavior*, the new behavior matches DCGO and the printed text. RL pipeline impact is minimal (the bounce that wasn't supposed to happen stops happening).
