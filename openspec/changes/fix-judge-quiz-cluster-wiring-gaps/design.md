## Context

A read-only pre-authoring substrate audit (three parallel Explore probes) characterized the engine state for judge-quiz clusters A/E/G. Findings and code sites:

| Gap | Code site | Probe finding |
|-----|-----------|---------------|
| `G-BLAST-DIGIVOLVE-IMMUNITY` (Q18) | `combat.rs::try_enter_counter` (candidate collection), `combat.rs::execute_blast_digivolve`, `dna_digivolve.rs::valid_blast_dna_field_targets_for_hand_card` | No path consults `permanent_is_unaffected_by_effect`; immunity machinery exists, unwired |
| `G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER` (Q25) | `game_actions.rs::take_digixros_material_origin` (`BattleArea` arm) | Material silently `battle_area.remove`d, no leave-trigger dispatch |
| `G-DIGIVOLVE-TARGET-RESTRICTION` (Q3) | `enums.rs` (`ModifierType`), `game.rs::can_digivolve` | No "can only digivolve into [X]" modifier; only `CannotDigivolve` (source). Breeding isolation already correct (`aura.rs` scans `battle_area` only) |

## Decisions

### D1 — Q18: gate at candidate collection + defensive execution abort (IMPLEMENTED)
The faithful, no-approximations placement is **candidate collection** (`try_enter_counter`): an immune base is never offered as a Blast counter candidate, so the action mask carries no illegal Blast — the RL agent never sees it. Mirror the gate in the Blast DNA field-target generator (`valid_blast_dna_field_targets_for_hand_card`) so an immune base isn't a DNA base. Add a defensive abort at the start of `execute_blast_digivolve` for any effect-driven blast path that bypasses candidate collection. All three consult `permanent_is_unaffected_by_effect(base, base.player, EffectSourceKind::Digimon)` — `base.player` because Blast Digivolve is the controller's own effect, and Quantumon's `Any` immunity blocks even own-controller effects (`OpponentOnly` immunity would still allow self-blast — correct).

### D2 — Q25 / Q3: scope-and-defer, do NOT build blind
The audit revealed the API card text diverges from the judge-quiz mapping (EX3-014 has no `[All Turns]` leave-trigger; EX10-020's restriction is a *self* "can only digivolve into [Apocalymon]"). Building either primitive now would risk the wrong shape (the exact leave-trigger mechanism / cause filter for Q25, the restriction payload + DSL vocab for Q3). Per rule 28 (widen-the-substrate-while-authoring), close them as the first step of authoring EX3-014 / EX10-020 with DCGO in hand. Fix shapes are recorded in `qa/archetype-qa/engine-gaps.md`. This change implements only the clean, card-agnostic Q18 wiring.

## Risks / Open Questions

- **Q18 candidate gate** removes Blast candidates only for effect-immune Digimon (rare) — low blast radius. Regression gate: `combat` suite (the 13 existing `counter_interrupt` tests stay green).
- **Q25/Q3 deferral** means their judge-quiz tests stay BLOCKED-CARD/`#[ignore]` until both the engine primitive (built at authoring) and the card land. The gap entries carry the precise fix shape so authoring is not a re-discovery.
