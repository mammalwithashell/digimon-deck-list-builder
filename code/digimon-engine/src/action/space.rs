/// Standard action space constants (matching Python engine).
/// These are for Rules::standard() — EDH/Titan will derive larger spaces from Rules.

/// Total action space size for standard 2-player game.
pub const ACTION_SPACE_SIZE: usize = 2168;

// Action ranges
pub const PLAY_HAND_START: u16 = 0;
pub const PLAY_HAND_END: u16 = 30; // exclusive
pub const HAND_EFFECT_START: u16 = 30;
pub const HAND_EFFECT_END: u16 = 60;
// Phase-disambiguated selection sub-ranges sharing the 30-59 raw ID space.
// Under `GamePhase::SelectReveal`, IDs 30-39 index revealed cards;
// under `GamePhase::SelectSecurity`, IDs 40-49 index the selecting player's
// own security stack and 50-59 index the opponent's. Matches Python's
// `SEL_REVEALED_START` / `SEL_MY_SECURITY_START` / `SEL_OPP_SECURITY_START`.
pub const SEL_REVEAL_START: u16 = 30;
pub const SEL_REVEAL_END: u16 = 40;
pub const SEL_MY_SECURITY_START: u16 = 40;
pub const SEL_MY_SECURITY_END: u16 = 50;
pub const SEL_OPP_SECURITY_START: u16 = 50;
pub const SEL_OPP_SECURITY_END: u16 = 60;
pub const MAX_SECURITY: usize = 10;
pub const MAX_REVEALED: usize = 10;
/// Action ID for `SelectionKind::Replacement` "accept this replacement" path.
/// Decline uses the standard `PASS` (62).
///
/// Raw ID 59 is the last valid ID inside the `HAND_EFFECT` (`30..60`) and
/// `SEL_OPP_SECURITY` (`50..60`) ranges. Phase-disambiguated: only legal
/// when `current_phase == GamePhase::EffectChoice` with a
/// `SelectionKind::Replacement` prompt installed; the mask builder renders
/// only `valid_action_ids` in that phase, so no collision at runtime.
/// See `replacement.rs::try_replace_impl` for the installer.
pub const REPLACEMENT_ACCEPT: u16 = 59;

pub const HATCH: u16 = 60;
pub const MOVE_FROM_BREEDING: u16 = 61;
pub const PASS: u16 = 62;
pub const DNA_DIGIVOLVE_START: u16 = 63;
pub const DNA_DIGIVOLVE_END: u16 = 93;
/// Selection-only action id for the controller's breeding-area permanent.
/// `docs/ACTION_SPEC.md` reserves 99 for this convention.
pub const BREEDING_SELECTION_TARGET: u16 = 99;
pub const ATTACK_START: u16 = 100;
pub const ATTACK_END: u16 = 400;
pub const DIGIVOLVE_START: u16 = 400;
pub const DIGIVOLVE_END: u16 = 1000;
pub const FIELD_EFFECT_START: u16 = 1000;
pub const FIELD_EFFECT_END: u16 = 1150;
pub const TRASH_EFFECT_START: u16 = 1150;
pub const TRASH_EFFECT_END: u16 = 1195;
pub const SOURCE_SELECT_START: u16 = 2000;
pub const SOURCE_SELECT_END: u16 = 2168;

// Sub-range constants
pub const TARGETS_PER_ATTACKER: u16 = 15; // 14 field slots + security
pub const FIELDS_PER_HAND: u16 = 15; // 14 field slots + breeding
pub const EFFECTS_PER_PERMANENT: u16 = 10;
pub const SOURCES_PER_FIELD: u16 = 12;
pub const MAX_FIELD_SLOTS: u16 = 14;
pub const SECURITY_TARGET: u16 = 14; // attack target index for security
pub const BREEDING_TARGET: u16 = 14; // digivolve target for breeding area

/// Within the 10-slot per-permanent effect sub-range, index 2 is the
/// conventional slot for the [Field] [Main] activated ability (§4.5c).
/// Matches Python's `action_mask.py` layout (offset `+ 2` per permanent).
pub const FIELD_EFFECT_SLOT_FOR_MAIN: u16 = 2;

/// Sub-slot 0 of the per-permanent effect range is the Overclock
/// sacrifice-and-attack action in the `EndOfTurnAction` phase (§4.6c).
/// Matches Python's `action_mask.py:354-361` bit layout
/// `1000 + i * EFFECTS_PER_PERM + 0`.
pub const FIELD_EFFECT_SLOT_FOR_OVERCLOCK: u16 = 0;

/// Upper bound on how many trash cards the mask inspects for [Trash] [Main]
/// activations. Python enforces 45 in `action_mask.py` (TRASH_EFFECT_END -
/// TRASH_EFFECT_START = 1195 - 1150).
pub const TRASH_MAIN_LIMIT: usize = 45;

/// Upper bound on how many hand cards the mask inspects for [Hand] [Main]
/// activations (one bit per hand index). Matches the HAND_EFFECT range width.
pub const HAND_MAIN_LIMIT: usize = 30;

/// Decode an attack action into (attacker_field_index, target_index).
/// target_index 14 = attack security.
pub fn decode_attack(action: u16) -> (u16, u16) {
    let offset = action - ATTACK_START;
    let attacker = offset / TARGETS_PER_ATTACKER;
    let target = offset % TARGETS_PER_ATTACKER;
    (attacker, target)
}

/// Encode an attack action from (attacker_field_index, target_index).
pub fn encode_attack(attacker: u16, target: u16) -> u16 {
    ATTACK_START + attacker * TARGETS_PER_ATTACKER + target
}

/// Decode a digivolve action into (hand_index, field_index).
/// field_index 14 = breeding area.
pub fn decode_digivolve(action: u16) -> (u16, u16) {
    let offset = action - DIGIVOLVE_START;
    let hand = offset / FIELDS_PER_HAND;
    let field = offset % FIELDS_PER_HAND;
    (hand, field)
}

/// Encode a digivolve action from (hand_index, field_index).
pub fn encode_digivolve(hand: u16, field: u16) -> u16 {
    DIGIVOLVE_START + hand * FIELDS_PER_HAND + field
}

/// Decode a field effect action into (permanent_index, effect_index).
pub fn decode_field_effect(action: u16) -> (u16, u16) {
    let offset = action - FIELD_EFFECT_START;
    let perm = offset / EFFECTS_PER_PERMANENT;
    let effect = offset % EFFECTS_PER_PERMANENT;
    (perm, effect)
}

/// Decode a source selection into (field_index, source_index).
pub fn decode_source_select(action: u16) -> (u16, u16) {
    let offset = action - SOURCE_SELECT_START;
    let field = offset / SOURCES_PER_FIELD;
    let source = offset % SOURCES_PER_FIELD;
    (field, source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_roundtrip() {
        let action = encode_attack(3, 7);
        let (a, t) = decode_attack(action);
        assert_eq!(a, 3);
        assert_eq!(t, 7);
    }

    #[test]
    fn attack_security() {
        let action = encode_attack(2, SECURITY_TARGET);
        assert!(action >= ATTACK_START && action < ATTACK_END);
        let (a, t) = decode_attack(action);
        assert_eq!(a, 2);
        assert_eq!(t, SECURITY_TARGET);
    }

    #[test]
    fn digivolve_roundtrip() {
        let action = encode_digivolve(5, 3);
        let (h, f) = decode_digivolve(action);
        assert_eq!(h, 5);
        assert_eq!(f, 3);
    }

    #[test]
    fn digivolve_breeding() {
        let action = encode_digivolve(2, BREEDING_TARGET);
        assert!(action >= DIGIVOLVE_START && action < DIGIVOLVE_END);
    }

    #[test]
    fn field_effect_decode() {
        let (perm, effect) = decode_field_effect(1025);
        assert_eq!(perm, 2);
        assert_eq!(effect, 5);
    }

    #[test]
    fn source_select_decode() {
        let (field, source) = decode_source_select(2015);
        assert_eq!(field, 1);
        assert_eq!(source, 3);
    }
}
