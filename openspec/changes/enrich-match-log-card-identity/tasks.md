## 1. Engine event payload — identity on existing variants

- [x] 1.1 Add `card_name: String` to `Play`, `Digivolve`, `Trash`, `Mill`, `SecurityReveal` in `code/digimon-engine/src/events.rs` (non-optional; update `type_str`/`seq` matches as needed)
- [x] 1.2 Add `attacker_card_id`, `attacker_card_name` (non-optional) and `target_card_id`, `target_card_name` (`Option`) to `Attack` in `events.rs`
- [x] 1.2b Add `attacker_dp`/`target_dp` (`Option<i32>`, effective DP via `Game::effective_dp`) to `Attack` for battle visibility; populate at the combat emission site; thread through both adapters (meta) and render `(N DP)` per Digimon in the frontend attack line. Spec deltas + engine/desktop/frontend tests updated.
- [x] 1.3 Add `source_card_id: Option<String>`, `source_card_name: Option<String>` to `MemoryChange` in `events.rs`
- [x] 1.4 Populate `Attack` identity at the emission site in `code/digimon-engine/src/combat/mod.rs` (attacker top card always; target top card when `AttackTarget::Digimon`)
- [x] 1.5 Populate `card_name` at every `Trash` emission site (`game/mod.rs` `trash_card`/`trash_permanent_stack`; deletion routes through these). NOTE: `Mill` variant carries `card_name` but has no live emission site (mill is emitted as `Reveal{TrashFromDeckTop}` per task 4.3); deck→trash also covered there.
- [x] 1.6 Populate `card_name` at every `Play` site (`effect_context/action/play.rs`, `game_actions/misc.rs`, `game/mod.rs` x2) and `Digivolve` site (`game_actions/digivolve.rs` x2, `game_actions/breeding.rs`, `game/mod.rs` DNA x2)
- [x] 1.7 Populate `card_name` at the `SecurityReveal` emission site in `combat/mod.rs`

## 2. Engine event payload — memory source threading

- [x] 2.1 Thread an effect-source argument from `EffectContext::gain_memory` / `lose_memory` (`effect_context/action/lifecycle.rs`) into the memory-mutation path, resolving id+name from `self.source_card` via `card_data_for_handle`
- [x] 2.2 Add `Game::gain_memory_for_player_sourced` (`game/memory.rs`); `gain_memory_for_player` delegates with `None`; `pay_memory`/`pay_memory_unchecked`/`set_memory` emit `None` (no signature churn)
- [x] 2.3 Audit all `MemoryChange` emission sites carry the new fields (cost-payment and structural paths emit `None`) — engine lib compiles

## 3. Engine — new EffectTarget event

- [x] 3.1 Add `GameEvent::EffectTarget { seq, player, source_card_id, source_card_name, targets: Vec<EventCardRef> }` to `events.rs` (+ `EventCardRef`, `type_str`/`seq`)
- [x] 3.2 Emit `EffectTarget` at the card-bearing selection installers via `push_effect_target` / `push_effect_target_multi`: field (Target/OwnField/OppField), hand, trash, reveal, security, union-zone, material, breeding-permanent (single-pick) + DP-budget, play-cost-budget, count-capped (multi-select, one event listing all targets). Fires for forced single-target selections. Excluded by design: reveal-pool kinds (OrderedPermutation/RevealBucket — already shown via Reveal events) and SourceMulti/partition source picks.

## 4. Engine — new reveal events

- [x] 4.1 Single parameterized `Reveal { seq, player, card_id, card_name, source_zone: RevealZone }` + `RevealZone { DeckTop, TrashFromDeckTop, Hand }` added to `events.rs`
- [x] 4.1b `RevealZone` reduced to `DeckTop`/`TrashFromDeckTop` — `Hand` removed (no engine primitive / no card reveals from hand; was dead code). Specs + frontend updated.
- [x] 4.2 Emit `Reveal{DeckTop}` at `reveal_top_deck` + `reveal_top_digitama` (`game_actions/zones.rs`)
- [x] 4.3 Emit `Reveal{TrashFromDeckTop}` per card at `trash_from_top`; route deck→trash through `trash_card` so a canonical `Trash` also fires (event-stream completeness); frontend suppresses the paired trash line so the log shows one mill reveal.
- [x] 4.4 Reveal-from-hand: RESOLVED as out-of-scope — no engine primitive exists and no card reveals from hand. Re-add `RevealZone::Hand` + emission when such a card is implemented.

## 5. Adapters — desktop + browser in lockstep

- [x] 5.1 Update `event_to_dto` in `code/src-tauri/src/engine_commands.rs`: Attack attacker/target id+name, `card_name` on play/digivolve/trash/mill/securityreveal, MemoryChange source, EffectTarget (meta.targets), reveals (meta.source_zone) — desktop lib compiles
- [x] 5.2 Update `event_to_pydict` in `code/digimon-engine-py/src/lib.rs` with the identical field/variant mapping — PyO3 crate compiles
- [x] 5.3 Extend the desktop `GameEventDto` struct and the Python dict keys with `source_card_name` / `target_card_name` and new meta keys

## 6. Frontend — types + formatter

- [x] 6.1 Extend `GameEvent` in `code/frontend/src/types/game.ts` with `source_card_name`, `target_card_name` and add `EffectTarget`/`Reveal` to the alias map in `gameEvents.ts`
- [x] 6.2 Update `gameLogFormat.ts`: render `[CARD-ID: Name]` from event identity; rewrite the `attack` case to name attacker + target (`security` when no target); add `MemoryChange` source attribution; add `effect_target` and `reveal` cases
- [x] 6.3 Implement the fallback chain in `displayCard`: event name → board lookup → bare id → `slot N` (never blank, never throw)

## 7. Tests

- [x] 7.1 Added `event_emission/{memory_source,effect_target,reveal}.rs` (MemoryChange source incl. unattributed, EffectTarget forced single-target, reveal deck-top + per-card mill order) and extended `attack.rs` (attacker/target identity, security no-target) + `trash.rs` (card_name). 22 event_emission tests pass.
- [x] 7.2 Extended `gameLogFormat.test.ts`: `[CARD-ID: Name]` rendering, named attacks, security target, memory attribution (+unattributed), effect-target, reveal-by-zone, board fallback. 23 frontend tests pass (`tsc` clean).
- [x] 7.3 Desktop-adapter assertion extended in `engine_commands.rs::drain_events_converts_engine_events_and_is_one_shot` (MemoryChange source + Attack attacker/target identity on the DTO). PyO3 `event_to_pydict` mirrors the same mapping by construction.
- [x] 7.4 Ran the full engine suite (`cargo test`) — 0 failed across all binaries (incl. 5246-test behavioral suite); frontend vitest green. No regressions from the new emissions.

## 8. Verification + docs

- [~] 8.1 Verified end-to-end via automated tests (engine emission → both adapters → frontend formatter). Live desktop manual play NOT run: the Tauri **binary** can't build in this environment (`tauri::generate_context!()` needs a built `frontend/dist`); the desktop **lib** + all data-flow tests pass.
- [x] 8.2 Open question resolved (log-only): new fields/variants ride existing serde + adapters; `docs/DCGO_RECORDING_SCHEMA.md` and replay consumers left unchanged (default-skip). Decision recorded in `design.md`.
