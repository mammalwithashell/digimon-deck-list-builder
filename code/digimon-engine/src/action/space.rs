/// Standard action space constants (matching Python engine).
/// These are for Rules::standard() — EDH/Titan will derive larger spaces from Rules.

/// Action-space schema version. Consumed by external recorders (e.g. the DCGO
/// game-recording mod) and by the `action-space-export` codegen tool to
/// stamp a version into generated artifacts. Bump whenever the action-space
/// layout, ID assignments, or formula semantics change in a way that would
/// invalidate existing recordings or trained models.
///
/// Independent of `ACTION_SPACE_SIZE` because non-size changes (e.g. swapping
/// the meaning of an existing ID) also need to break old artifacts.
pub const SCHEMA_VERSION: u32 = 1;

/// Total action space size for standard 2-player game.
///
/// Bumped 2168 → 2192 in Task S1.3 to append a breeding-carrier
/// source-selection sub-range (`BREEDING_SOURCE_SELECT_*` below). This is a
/// deliberate action-space version bump: it changes the policy/value head
/// width, so existing trained RL models must be retrained — see
/// `docs/MODEL_CATALOG.md` and `docs/TRAINING_RUNBOOK.md`.
pub const ACTION_SPACE_SIZE: usize = 2192;

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
/// Concede the game. Always legal at agent decision points (per the BO3
/// match-training spec). Decodes to `Game::concede(player)`. Reserved at
/// 93 per `docs/ACTION_SPEC.md`.
pub const CONCEDE_GAME: u16 = 93;
/// Best-of-three play-order selection: "I will play first." Legal only
/// during `GamePhase::SelectPlayOrder`. Reserved at 94.
pub const PLAY_FIRST: u16 = 94;
/// Best-of-three play-order selection: "I will play second." Legal only
/// during `GamePhase::SelectPlayOrder`. Reserved at 95.
pub const PLAY_SECOND: u16 = 95;
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

/// Number of breeding carriers addressable for source selection: one
/// digivolving stack per player in standard 2-player.
pub const BREEDING_SOURCE_CARRIERS: u16 = 2;
/// Start of the breeding-carrier source-selection sub-range. Appended
/// immediately after `SOURCE_SELECT_END` so all source-selection action
/// IDs stay contiguous.
pub const BREEDING_SOURCE_SELECT_START: u16 = SOURCE_SELECT_END; // 2168
/// End (exclusive) of the breeding-carrier source-selection sub-range.
/// 2 carriers x SOURCES_PER_FIELD (12) = 24 slots. Layout:
/// `2168..2180` = player 0 breeding sources 0..12,
/// `2180..2192` = player 1 breeding sources 0..12.
pub const BREEDING_SOURCE_SELECT_END: u16 =
    BREEDING_SOURCE_SELECT_START + BREEDING_SOURCE_CARRIERS * SOURCES_PER_FIELD; // 2192

// Sub-range constants
pub const TARGETS_PER_ATTACKER: u16 = 15; // 14 field slots + security
pub const FIELDS_PER_HAND: u16 = 15; // 14 field slots + breeding
pub const EFFECTS_PER_PERMANENT: u16 = 10;
pub const SOURCES_PER_FIELD: u16 = 12;
pub const MAX_FIELD_SLOTS: u16 = 14;
pub const SECURITY_TARGET: u16 = 14; // attack target index for security
pub const BREEDING_TARGET: u16 = 14; // digivolve target for breeding area
pub const BREEDING_SELECT_PLAYER_0: u16 = BREEDING_TARGET;
pub const BREEDING_SELECT_PLAYER_1: u16 = BREEDING_TARGET + 1;

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

pub fn encode_breeding_select(player: u8) -> Option<u16> {
    match player {
        0 => Some(BREEDING_SELECT_PLAYER_0),
        1 => Some(BREEDING_SELECT_PLAYER_1),
        _ => None,
    }
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

/// Encode a source selection from (field_index, source_index).
pub fn encode_source_select(field: u16, source: u16) -> Option<u16> {
    if field >= MAX_FIELD_SLOTS || source >= SOURCES_PER_FIELD {
        return None;
    }
    Some(SOURCE_SELECT_START + field * SOURCES_PER_FIELD + source)
}

/// Encode a breeding-carrier source selection from (carrier_owner, source).
///
/// The breeding carrier is addressed by its owning `player` (not a
/// `PermanentHandle.index`) — consistent with `encode_breeding_select`,
/// since each player holds exactly one digivolving stack in the breeding
/// area. Returns `None` for an out-of-range player or source slot.
pub fn encode_breeding_source_select(player: u8, source: u16) -> Option<u16> {
    if (player as u16) >= BREEDING_SOURCE_CARRIERS || source >= SOURCES_PER_FIELD {
        return None;
    }
    Some(BREEDING_SOURCE_SELECT_START + player as u16 * SOURCES_PER_FIELD + source)
}

/// Decode a breeding-carrier source selection into (carrier_owner, source).
/// Caller must ensure `action` is in `BREEDING_SOURCE_SELECT_START..END`.
pub fn decode_breeding_source_select(action: u16) -> (u16, u16) {
    let offset = action - BREEDING_SOURCE_SELECT_START;
    (offset / SOURCES_PER_FIELD, offset % SOURCES_PER_FIELD)
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

    #[test]
    fn breeding_source_select_roundtrip() {
        // Player 0, source 3 → 2168 + 0*12 + 3 = 2171.
        let action = encode_breeding_source_select(0, 3).expect("valid p0 encode");
        assert_eq!(action, BREEDING_SOURCE_SELECT_START + 3);
        let (player, source) = decode_breeding_source_select(action);
        assert_eq!(player, 0);
        assert_eq!(source, 3);

        // Player 1, source 0 → 2168 + 1*12 + 0 = 2180.
        let action = encode_breeding_source_select(1, 0).expect("valid p1 encode");
        assert_eq!(action, BREEDING_SOURCE_SELECT_START + SOURCES_PER_FIELD);
        let (player, source) = decode_breeding_source_select(action);
        assert_eq!(player, 1);
        assert_eq!(source, 0);

        // Player 1, last source slot (11) → 2168 + 12 + 11 = 2191.
        let action = encode_breeding_source_select(1, SOURCES_PER_FIELD - 1)
            .expect("valid p1 last-slot encode");
        assert_eq!(action, BREEDING_SOURCE_SELECT_END - 1);
        let (player, source) = decode_breeding_source_select(action);
        assert_eq!(player, 1);
        assert_eq!(source, SOURCES_PER_FIELD - 1);
    }

    #[test]
    fn breeding_source_select_range() {
        // The sub-range is exactly 24 IDs and abuts the new ceiling.
        assert_eq!(BREEDING_SOURCE_SELECT_START, SOURCE_SELECT_END);
        assert_eq!(BREEDING_SOURCE_SELECT_START, 2168);
        assert_eq!(BREEDING_SOURCE_SELECT_END, 2192);
        assert_eq!(
            BREEDING_SOURCE_SELECT_END - BREEDING_SOURCE_SELECT_START,
            BREEDING_SOURCE_CARRIERS * SOURCES_PER_FIELD
        );
        // Source-select range stays contiguous and the new range is LAST.
        assert_eq!(ACTION_SPACE_SIZE, 2192);
        assert_eq!(BREEDING_SOURCE_SELECT_END as usize, ACTION_SPACE_SIZE);

        // Out-of-range carriers / source slots are rejected.
        assert_eq!(
            encode_breeding_source_select(BREEDING_SOURCE_CARRIERS as u8, 0),
            None
        );
        assert_eq!(encode_breeding_source_select(2, 0), None);
        assert_eq!(encode_breeding_source_select(0, SOURCES_PER_FIELD), None);
        assert_eq!(encode_breeding_source_select(0, 99), None);
    }
}
