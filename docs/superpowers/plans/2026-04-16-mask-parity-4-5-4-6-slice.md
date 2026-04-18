# Plan: §4.5 / §4.6 slice — Vortex mask + DNA Digivolve mask (Rust engine)

## Context

`docs/RUST_PYTHON_PARITY.md` §7 item 8 is "§4.5 / §4.6 — Mask phase coverage" — nominally covering Hand/Field/Trash effect masks, DNA digivolve masks, and all interrupt/selection-phase masks. In practice that's four separable efforts with very different costs; attempting all four at once would either collapse under scope or require multi-week infrastructure work (effect-listing query, selection state machine). See the Explore pass in the prior turn for the full scoping.

This plan lands two **self-contained slices** that don't depend on the big infrastructure pieces:

1. **§4.6 slice — Vortex mask bit emission.** The `EndOfTurnAction` phase enum variant already exists; `can_attack(handle, vortex: bool)` plumbing from §2.1 is already in place. What's missing is (a) a `Keyword::Vortex` variant, (b) a mask arm that emits attack bits for permanents with modifier-granted Vortex when the phase is active. Phase *transition* (end-of-turn → `EndOfTurnAction`) is deferred to §4.6-interrupt state machine work — the mask code stands on its own and is correct when the transition eventually lands (same pattern as §2.1).

2. **§4.5 slice — DNA digivolve mask (range 63-92).** Rust's mask currently emits 0 for this range because `CardData` has no `dna_costs`, and no DNA validator exists. Adding the data types + validator + mask section is a mechanical port of Python's logic. Data *population* (Python scripts' `card.c_entity_base.dna_costs = [...]` → cards.json / auxiliary data file) is out of scope — the field defaults to empty and the mask code is inert until data arrives. Document as §4.5b residual.

**Explicitly deferred (tracked as residuals / future work, not in this plan):**
- §4.5b — Data-population pipeline for `dna_costs` (requires Python export changes).
- §4.5c — Hand/Field/Trash `[Main]` effects (needs `CardSource::effect_list(timing)` query — big architectural lift).
- §4.6b — `Keyword::Vortex` phase transition in `end_turn` (needs the interrupt state machine so the player can pass and resume end-of-turn).
- §4.6c — Overclock / `MAY_ATTACK` / `FORCE_ATTACK` mask bits in `EndOfTurnAction` (each needs its own modifier + mask arm, orthogonal to Vortex).
- §4.6d — Full interrupt/selection-phase mask builders (`BlockTiming`, `CounterTiming`, `AllianceTiming`, `SelectTarget` family) — multi-week architectural project.

**Outcome of this change:** Two more mask ranges go from "always 0" to "correct when data/phase is present." The engine continues to ship without regressions. Residual gaps are documented precisely.

## File structure

Files modified / created by this plan:

| Path | Purpose |
|------|---------|
| `digimon-engine/src/enums.rs` | Add `Keyword::Vortex`, `ModifierType::GrantVortex`. |
| `digimon-engine/src/card_data.rs` | Add `DnaCost` + `DnaRequirement` structs; add `dna_costs: Vec<DnaCost>` to `CardData` (serde default = empty). Update `RawCard` deserialization to accept the optional JSON field. |
| `digimon-engine/src/validation/dna_digivolve.rs` | **New file.** `can_dna_digivolve(evo_card, perm_a, perm_b, card_data) -> bool` + `has_valid_dna_targets(evo_card, battle_area, card_data) -> bool` — direct port of Python's `digivolve_validator.py:204-247`. |
| `digimon-engine/src/validation/mod.rs` | Register the new `dna_digivolve` module. |
| `digimon-engine/src/action/mask.rs` | Add `EndOfTurnAction` arm emitting Vortex attack bits (+ PASS). Add DNA digivolve loop in `GamePhase::Main` arm (range 63-92). |
| `digimon-engine/tests/mask_main_parity.rs` | **Append** DNA digivolve tests — same file, same `make_digimon_dp` / `make_option` helpers, same stylistic pattern as §4.2-4.4. |
| `digimon-engine/tests/mask_end_of_turn_parity.rs` | **New file.** Vortex-phase mask tests. Kept separate from `mask_main_parity.rs` because the `EndOfTurnAction` phase requires manually overriding `game.current_phase` in test setup and the file is phase-scoped rather than feature-scoped. |
| `docs/RUST_PYTHON_PARITY.md` | Flip §4.5 / §4.6 to partial 🟡 with specific sub-items 🟢 (Vortex mask, DNA mask plumbing) and residual 🟡 sub-items (§4.5b, §4.5c, §4.6b, §4.6c, §4.6d). Tick §7 item 8 partially. |

## Implementation

### Task 1 — Vortex enum variants

- **`enums.rs`**: add `Keyword::Vortex` to the `Keyword` enum (alongside `Blitz`, `Raid`, etc., near line 128 of the existing file).
- **`enums.rs`**: add `ModifierType::GrantVortex` to the Grant\* group (near line 198).
- No logic changes, no tests — these are purely structural additions consumed by Task 2.

### Task 2 — Vortex mask in `EndOfTurnAction` phase

In `action/mask.rs`, the current catch-all arm at the bottom of the `match game.current_phase` block returns `mask[PASS] = 1.0` for all non-Main/Breeding/Mulligan phases. Add a dedicated `GamePhase::EndOfTurnAction` arm *before* the catch-all:

```rust
GamePhase::EndOfTurnAction => {
    // Decline end-of-turn action — always legal.
    mask[PASS as usize] = 1.0;
    // §4.6 slice — Vortex. A permanent with modifier-granted Keyword::Vortex
    // whose `can_attack(handle, vortex=true)` passes can attack any enemy
    // Digimon (suspended or not). Mirrors Python action_mask.py:321-335.
    // Overclock / MAY_ATTACK / FORCE_ATTACK are tracked as §4.6c.
    let max_field = me.battle_area.len().min(FIELD_SLOTS);
    for i in 0..max_field {
        let handle = PermanentHandle { player: player_id, index: i as u8 };
        if !game.modifiers.has_keyword(handle, Keyword::Vortex) {
            continue;
        }
        if !game.can_attack(handle, /* vortex = */ true) {
            continue;
        }
        // Security attack allowed.
        mask[encode_attack(i as u16, SECURITY_TARGET) as usize] = 1.0;
        // Any enemy Digimon is a valid target (no suspended/Raid filter —
        // Vortex bypasses the Main-phase target restriction per Python).
        let max_opp = opp.battle_area.len().min(FIELD_SLOTS);
        for j in 0..max_opp {
            let target = &opp.battle_area[j];
            if target.is_digimon(&game.card_data) {
                mask[encode_attack(i as u16, j as u16) as usize] = 1.0;
            }
        }
    }
}
```

**Tests** (`tests/mask_end_of_turn_parity.rs`, new file following the `mask_main_parity.rs` factory pattern):

- `mask_vortex_emits_attacks_in_end_of_turn_phase` — grant Vortex, manually set `game.current_phase = GamePhase::EndOfTurnAction`, assert security bit + digimon bits.
- `mask_vortex_without_keyword_only_emits_pass` — no Vortex grant → only `mask[PASS] = 1.0`.
- `mask_vortex_bypasses_summoning_sickness` — freshly played + Vortex granted → attack bits emitted (regression test for the `can_attack(vortex=true)` plumbing from §2.1).
- `mask_vortex_targets_unsuspended_digimon_too` — opponent has an unsuspended Digimon → Vortex targets it (unlike Main-phase mask which requires Raid/CAN_ATTACK_UNSUSPENDED for unsuspended).

### Task 3 — DNA cost data types

In `card_data.rs`, add:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DnaRequirement {
    /// Required level. 0 means "any level" (name/text-only requirement).
    #[serde(default)]
    pub level: u8,
    /// Optional color constraint. `None` means any color.
    #[serde(default)]
    pub card_color: Option<CardColor>,
    /// Substring match against card_name. Empty = no name constraint.
    #[serde(default)]
    pub name_contains: String,
    /// Substring match against effect_text. Empty = no text constraint.
    #[serde(default)]
    pub text_contains: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DnaCost {
    pub requirement1: DnaRequirement,
    pub requirement2: DnaRequirement,
    #[serde(default)]
    pub memory_cost: i16,
}
```

Add to `CardData`:

```rust
#[serde(default)]
pub dna_costs: Vec<DnaCost>,
```

Mirror the field in `RawCard` so cards.json ingestion passes it through. Default empty keeps every existing cards.json entry valid.

### Task 4 — DNA validator helpers

New file `digimon-engine/src/validation/dna_digivolve.rs`:

```rust
//! DNA digivolve validation — port of Python's
//! `digivolve_validator.py::can_dna_digivolve` / `has_valid_dna_targets`.

use crate::card_data::{CardData, DnaRequirement};
use crate::permanent::Permanent;

fn perm_matches_req(perm: &Permanent, req: &DnaRequirement, data: &[CardData]) -> bool {
    let top = perm.top_card();
    let meta = &data[top.data_index];
    if req.level > 0 {
        match meta.level {
            Some(l) if l == req.level => {}
            _ => return false,
        }
    }
    if let Some(color) = req.card_color {
        if !meta.colors.contains(&color) {
            return false;
        }
    }
    if !req.name_contains.is_empty()
        && !meta.card_name.to_lowercase().contains(&req.name_contains.to_lowercase())
    {
        return false;
    }
    if !req.text_contains.is_empty()
        && !meta.effect_text.to_lowercase().contains(&req.text_contains.to_lowercase())
    {
        return false;
    }
    true
}

pub fn can_dna_digivolve(
    evo_meta: &CardData,
    perm_a: &Permanent,
    perm_b: &Permanent,
    data: &[CardData],
) -> bool {
    for cost in &evo_meta.dna_costs {
        let both_orderings = [
            (&cost.requirement1, &cost.requirement2),
            (&cost.requirement2, &cost.requirement1),
        ];
        for (ra, rb) in both_orderings {
            if perm_matches_req(perm_a, ra, data) && perm_matches_req(perm_b, rb, data) {
                return true;
            }
        }
    }
    false
}

pub fn has_valid_dna_targets(
    evo_meta: &CardData,
    battle_area: &[Permanent],
    data: &[CardData],
) -> bool {
    if evo_meta.dna_costs.is_empty() {
        return false;
    }
    for i in 0..battle_area.len() {
        for j in (i + 1)..battle_area.len() {
            if can_dna_digivolve(evo_meta, &battle_area[i], &battle_area[j], data) {
                return true;
            }
        }
    }
    false
}
```

Unit tests for the helpers are covered by the Task 5 mask tests (which exercise the happy path, both orderings, and no-match cases via hand-crafted `CardData`). No separate test file needed — keep the helper file lean.

### Task 5 — DNA digivolve mask in `GamePhase::Main`

In `action/mask.rs`'s `GamePhase::Main` arm, after the Digivolve section (range 400-999), add:

```rust
// --- DNA Digivolve (63-92) --- §4.5 slice.
// A hand card with dna_costs[] is legal if some pair of permanents in the
// battle_area satisfies any of its requirements (either ordering). Memory
// cost is checked like play_cost. Data population of dna_costs is §4.5b.
for h in 0..max_hand as usize {
    let card = &me.hand[h];
    let evo_meta = &game.card_data[card.data_index];
    if evo_meta.dna_costs.is_empty() {
        continue;
    }
    // Memory check uses the minimum memory_cost across DNA variants.
    let min_mem_cost = evo_meta
        .dna_costs
        .iter()
        .map(|c| c.memory_cost)
        .min()
        .unwrap_or(0);
    if (game.memory - min_mem_cost) < game.rules.memory_range.0 {
        continue;
    }
    if crate::validation::dna_digivolve::has_valid_dna_targets(
        evo_meta,
        &me.battle_area,
        &game.card_data,
    ) {
        mask[(DNA_DIGIVOLVE_START + h as u16) as usize] = 1.0;
    }
}
```

**Tests** (append to `tests/mask_main_parity.rs`):

- `mask_dna_digivolve_emits_when_valid_pair_exists` — hand card with `dna_costs = [DnaCost { req1: Red Lv3, req2: Blue Lv3, memory_cost: 0 }]`, battle_area has a Red Lv3 + Blue Lv3 → mask bit 63 = 1.0.
- `mask_dna_digivolve_accepts_either_ordering` — flip battle_area order (Blue first, Red second) → still legal.
- `mask_dna_digivolve_rejects_when_no_pair` — battle_area only has two Red Lv3 → mask bit 63 = 0.0.
- `mask_dna_digivolve_respects_memory_cost` — `memory_cost = 5`, `game.memory = 2` → mask bit 63 = 0.0 (insufficient memory).
- `mask_dna_digivolve_skips_cards_without_dna_costs` — hand card with empty `dna_costs` → mask bit 63 stays 0.0 even with valid battle_area pair.

### Task 6 — Doc update

In `docs/RUST_PYTHON_PARITY.md`, restructure §4.5 and §4.6 to reflect partial implementation:

- §4.5 header → 🟡 "Partial — DNA digivolve plumbing landed; Hand/Field/Trash `[Main]` effects blocked on effect-listing infra."
  - §4.5a 🟢 DNA digivolve mask (logic + data types). Cite `validation/dna_digivolve.rs` + mask section + tests.
  - §4.5b 🟡 `dna_costs` data-population pipeline — cards.json ingestion doesn't yet carry DNA costs. Rust field defaults to empty; logic is inert until Python export pipeline exports `dna_costs` per card.
  - §4.5c 🔴 Hand/Field/Trash `[Main]` effect masks — blocked on `CardSource::effect_list(timing)` infra.
- §4.6 header → 🟡 "Partial — Vortex mask bit landed; phase transition + other end-of-turn actions + interrupt phases remain."
  - §4.6a 🟢 Vortex mask emission in `EndOfTurnAction` phase. Cite `mask.rs` arm + `Keyword::Vortex` + tests.
  - §4.6b 🔴 Phase transition into `EndOfTurnAction` — nothing in `end_turn` checks for pending vortex/overclock/may-attack; needs the interrupt state machine so the player can pass and resume end-of-turn.
  - §4.6c 🔴 Overclock / MAY_ATTACK / FORCE_ATTACK mask bits in `EndOfTurnAction`. Each needs its own modifier + mask arm.
  - §4.6d 🔴 Full interrupt/selection-phase mask builders (`BlockTiming`, `CounterTiming`, `AllianceTiming`, `Select*` family). Phase-4 architectural project.

§7 item 8 → update strikethrough to reflect partial completion.

## Verification

From the worktree root:

```bash
# Build + unit-test the Rust crate.
cargo test -p digimon-engine 2>&1 | grep "test result"
# Expected: every test result line shows `ok ... 0 failed`.

# Scoped test run for the new tests.
cargo test -p digimon-engine --test mask_main_parity
cargo test -p digimon-engine --test mask_end_of_turn_parity
# Expected: all tests pass.

# Python regression — we don't touch Python, so this must be unchanged.
python -m pytest tests/engine -k "rush or attack or summon or option or dna" -q
# Expected: the same pass count as before this batch (34 after §4.4 + any dna-named tests that existed previously).

# Sanity: cards.json still loads into CardData with the new optional dna_costs field.
cargo test -p digimon-engine --test card_registry_parity
# Expected: all card_registry_parity tests pass (no regression from the schema addition).
```

Expected outcome:
- All existing Rust tests stay green (no regressions from the enum additions, schema addition, or mask branches).
- 4 new Vortex tests pass in `mask_end_of_turn_parity.rs`.
- 5 new DNA digivolve tests pass in `mask_main_parity.rs`.
- No Python changes — zero drift risk on the Python side.
- Doc accurately reflects the partial-done state with precise residual items.

## Out of scope (flagged for future work)

- **§4.5b** — Data population of `dna_costs`. Requires updating the Python card-export pipeline to emit DNA costs alongside evo_costs. Separate cross-language task.
- **§4.5c** — `CardSource::effect_list(registry, timing)` query. Architectural prereq for Hand/Field/Trash effect masks. 3-5 days of work; opens up lots of downstream parity work.
- **§4.6b** — `EndOfTurnAction` phase transition + resume logic. Requires `PendingEndOfTurn` or similar state to track which permanents still have pending vortex/overclock/may-attack.
- **§4.6c** — Overclock (needs sacrifice selection), MAY_ATTACK (`ModifierType::MayAttack`), FORCE_ATTACK restriction logic (already listed in §4.7 too).
- **§4.6d** — Full interrupt-phase mask coverage. Multi-week. Depends on §2.3 combat state machine + `PendingSelection` infra.
