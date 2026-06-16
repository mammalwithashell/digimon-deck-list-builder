use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::enums::{GameMode, Rarity, SkipDraw, TitanRole};

const PAUPER_RARITY_MASK: u8 = Rarity::C.mask() | Rarity::U.mask();

/// Card-copy restrictions and mutual-exclusivity groups for a format.
/// Mirrors `digimon_gym/engine/data/deck_loader.py:CardRestriction`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardRestriction {
    /// card_id -> max copies allowed. 0 = banned, 1 = restricted.
    /// Absent card_id = use the card's default `max_count_in_deck`.
    pub card_limits: BTreeMap<String, u8>,

    /// Mutual-exclusivity pairs: cards from group_a and group_b cannot coexist.
    pub choice_groups: Vec<(Vec<String>, Vec<String>)>,
}

impl CardRestriction {
    /// Empty restriction — used by `no_restriction` and `edh` modes.
    pub fn none() -> Self {
        Self::default()
    }

    /// Official ENG restricted list from https://digimoncard.io/restricted-list
    /// (matches DCGO's `DataBase.ENGBanList`).
    ///
    /// Now sourced from `data/deck_formats.json` via the format registry — the
    /// single source of truth shared with the hosted API, so there is no longer
    /// a parallel Python list to keep in sync.
    ///
    /// Clones from the process-wide cached registry copy; use
    /// `official_eng_ref()` if you only need a shared reference.
    pub fn official_eng() -> Self {
        Self::official_eng_ref().clone()
    }

    /// Shared reference to the cached official ENG restricted list.
    /// Prefer this over `official_eng()` when you don't need ownership.
    pub fn official_eng_ref() -> &'static Self {
        crate::format::restriction_by_name("official_eng")
            .expect("deck_formats.json must define the `official_eng` restriction")
    }

    /// EDEN custom restricted list (see `data/deck_formats.json`).
    pub fn eden() -> Self {
        Self::eden_ref().clone()
    }

    /// Shared reference to the cached EDEN restricted list.
    pub fn eden_ref() -> &'static Self {
        crate::format::restriction_by_name("eden")
            .expect("deck_formats.json must define the `eden` restriction")
    }
}

/// Configurable game parameters. Drives player count, deck sizes, tensor/action layout,
/// and deck-legality restrictions. Mirrors Python's per-mode rules in
/// `digimon_gym/db/routers/decks.py:_validate_for_mode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    pub player_count: u8,
    pub deck_size: u16,
    pub egg_deck_max: u8,
    pub security_count: u8,
    pub starting_hand: u8,
    pub field_slots: u8,
    pub singleton: bool,
    pub commander: bool,
    pub memory_range: (i16, i16),
    pub max_turns: u16,
    pub skip_first_draw: SkipDraw,
    pub restriction: CardRestriction,
    /// Optional format-level rarity gate as a bitmask of `Rarity::mask()` values.
    /// `None` means all rarities are legal. Pauper uses common + uncommon.
    #[serde(default)]
    pub allowed_card_rarity_mask: Option<u8>,
}

impl Rules {
    /// Standard 2-player Digimon TCG rules with the official ENG restricted list.
    pub fn standard() -> Self {
        Self {
            player_count: 2,
            deck_size: 50,
            egg_deck_max: 5,
            security_count: 5,
            starting_hand: 5,
            field_slots: 14,
            singleton: false,
            commander: false,
            memory_range: (-10, 10),
            max_turns: 200,
            skip_first_draw: SkipDraw::FirstPlayerOnly,
            restriction: CardRestriction::official_eng(),
            allowed_card_rarity_mask: None,
        }
    }

    /// Pauper format: standard 2-player rules and official ENG restrictions,
    /// but only common and uncommon cards are legal.
    pub fn pauper() -> Self {
        Self {
            allowed_card_rarity_mask: Some(PAUPER_RARITY_MASK),
            ..Self::standard()
        }
    }

    /// Standard structure with no banned or restricted cards.
    /// Mirrors Python's `GameMode.NoRestriction`. Used for RL sandbox / casual testing.
    pub fn no_restriction() -> Self {
        Self {
            restriction: CardRestriction::none(),
            ..Self::standard()
        }
    }

    /// EDEN common/uncommon format, using standard gameplay parameters plus
    /// EDEN's custom deck legality and restricted list.
    pub fn eden() -> Self {
        Self {
            restriction: CardRestriction::eden(),
            ..Self::standard()
        }
    }

    /// EDEN Singleton: EDEN's pool and banlist played highlander (one copy of
    /// any card). Standard gameplay parameters; deck legality differs.
    pub fn eden_singleton() -> Self {
        Self {
            restriction: CardRestriction::eden(),
            singleton: true,
            ..Self::standard()
        }
    }

    /// 4-player EDH Commander format. Singleton is enforced via `singleton: true`,
    /// not via the restricted list (matches Python's `edh_commander` mode).
    /// See `docs/EDH_COMMANDER_MODE.md`.
    pub fn edh() -> Self {
        Self {
            player_count: 4,
            deck_size: 70,
            egg_deck_max: 5,
            security_count: 7,
            starting_hand: 5,
            field_slots: 14,
            singleton: true,
            commander: true,
            memory_range: (-10, 10),
            max_turns: 600,
            skip_first_draw: SkipDraw::AllRound1,
            restriction: CardRestriction::none(),
            allowed_card_rarity_mask: None,
        }
    }

    /// Titan mode — the Titan (boss) player. Asymmetric: pairs with `titan_team`
    /// for the other seats. `Game::new` currently takes a single `Rules`; wiring
    /// per-role setup is a follow-up. See `docs/TITAN_MODE.md`.
    pub fn titan_boss() -> Self {
        Self {
            player_count: 3, // 1 titan + 2 team (expandable to 4)
            deck_size: 80,
            egg_deck_max: 5,
            security_count: 15,
            starting_hand: 7,
            field_slots: 14,
            singleton: false,
            commander: false,
            memory_range: (-10, 10),
            max_turns: 400,
            skip_first_draw: SkipDraw::FirstPlayerOnly,
            restriction: CardRestriction::official_eng(),
            allowed_card_rarity_mask: None,
        }
    }

    /// Titan mode — a team player. See `titan_boss` and `docs/TITAN_MODE.md`.
    pub fn titan_team() -> Self {
        Self {
            player_count: 3,
            deck_size: 50,
            egg_deck_max: 5,
            security_count: 5,
            starting_hand: 5,
            field_slots: 14,
            singleton: false,
            commander: false,
            memory_range: (-10, 10),
            max_turns: 400,
            skip_first_draw: SkipDraw::FirstPlayerOnly,
            restriction: CardRestriction::official_eng(),
            allowed_card_rarity_mask: None,
        }
    }

    /// Map a `GameMode` (+ optional `TitanRole` for Titan) to its `Rules` preset.
    /// Mirrors Python's per-mode routing in `_validate_for_mode`.
    ///
    /// Returns `Err` if `mode == Titan` and `role` is `None`, or if `role` is
    /// provided for a non-Titan mode (callers should pass `None` there).
    pub fn for_mode(mode: GameMode, role: Option<TitanRole>) -> Result<Self, ForModeError> {
        match (mode, role) {
            (GameMode::Standard, None) => Ok(Self::standard()),
            (GameMode::Pauper, None) => Ok(Self::pauper()),
            (GameMode::NoRestriction, None) => Ok(Self::no_restriction()),
            (GameMode::Eden, None) => Ok(Self::eden()),
            (GameMode::EdenSingleton, None) => Ok(Self::eden_singleton()),
            (GameMode::EdhCommander, None) => Ok(Self::edh()),
            (GameMode::Titan, Some(TitanRole::Boss)) => Ok(Self::titan_boss()),
            (GameMode::Titan, Some(TitanRole::Team)) => Ok(Self::titan_team()),
            (GameMode::Titan, None) => Err(ForModeError::MissingTitanRole),
            (mode, Some(_)) => Err(ForModeError::UnexpectedRole(mode)),
        }
    }
}

/// Errors returned by `Rules::for_mode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForModeError {
    /// Caller passed `GameMode::Titan` without a `TitanRole`.
    MissingTitanRole,
    /// Caller passed a `TitanRole` for a non-Titan mode.
    UnexpectedRole(GameMode),
}

impl std::fmt::Display for ForModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTitanRole => {
                write!(f, "GameMode::Titan requires a TitanRole")
            }
            Self::UnexpectedRole(mode) => {
                write!(f, "GameMode::{:?} does not accept a TitanRole", mode)
            }
        }
    }
}

impl std::error::Error for ForModeError {}

impl Default for Rules {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_defaults() {
        let r = Rules::standard();
        assert_eq!(r.player_count, 2);
        assert_eq!(r.deck_size, 50);
        assert_eq!(r.security_count, 5);
        assert!(!r.singleton);
        assert!(!r.commander);
    }

    #[test]
    fn standard_has_official_restriction() {
        let r = Rules::standard();
        // 3 banned + 47 restricted = 50 entries
        assert_eq!(r.restriction.card_limits.len(), 50);
        assert_eq!(r.restriction.choice_groups.len(), 2);
        assert!(r.allowed_card_rarity_mask.is_none());
    }

    #[test]
    fn pauper_matches_standard_except_rarity_gate() {
        let r = Rules::pauper();
        let std = Rules::standard();
        assert_eq!(r.player_count, std.player_count);
        assert_eq!(r.deck_size, std.deck_size);
        assert_eq!(r.egg_deck_max, std.egg_deck_max);
        assert_eq!(r.security_count, std.security_count);
        assert_eq!(r.starting_hand, std.starting_hand);
        assert_eq!(r.field_slots, std.field_slots);
        assert_eq!(r.singleton, std.singleton);
        assert_eq!(r.commander, std.commander);
        assert_eq!(r.memory_range, std.memory_range);
        assert_eq!(r.max_turns, std.max_turns);
        assert_eq!(r.skip_first_draw, std.skip_first_draw);
        assert_eq!(r.restriction, std.restriction);
        assert_eq!(r.allowed_card_rarity_mask, Some(PAUPER_RARITY_MASK));
        assert!(Rarity::C.is_in_mask(PAUPER_RARITY_MASK));
        assert!(Rarity::U.is_in_mask(PAUPER_RARITY_MASK));
        assert!(!Rarity::R.is_in_mask(PAUPER_RARITY_MASK));
        assert!(!Rarity::NoRarity.is_in_mask(PAUPER_RARITY_MASK));
    }

    #[test]
    fn no_restriction_is_empty() {
        let r = Rules::no_restriction();
        assert!(r.restriction.card_limits.is_empty());
        assert!(r.restriction.choice_groups.is_empty());
        assert!(r.allowed_card_rarity_mask.is_none());
        // All other fields match standard()
        let std = Rules::standard();
        assert_eq!(r.player_count, std.player_count);
        assert_eq!(r.deck_size, std.deck_size);
        assert_eq!(r.security_count, std.security_count);
        assert_eq!(r.starting_hand, std.starting_hand);
        assert_eq!(r.skip_first_draw, std.skip_first_draw);
    }

    #[test]
    fn eden_settings_use_standard_gameplay() {
        let r = Rules::eden();
        let std = Rules::standard();
        assert_eq!(r.player_count, std.player_count);
        assert_eq!(r.deck_size, std.deck_size);
        assert_eq!(r.egg_deck_max, std.egg_deck_max);
        assert_eq!(r.security_count, std.security_count);
        assert_eq!(r.skip_first_draw, std.skip_first_draw);
    }

    #[test]
    fn eden_has_custom_restriction() {
        let r = Rules::eden();
        assert_eq!(r.restriction.card_limits.get("BT3-097"), Some(&0));
        assert_eq!(r.restriction.card_limits.get("BT1-107"), Some(&1));
        assert_eq!(r.restriction.card_limits.get("BT6-085"), Some(&4));
        assert_eq!(r.restriction.choice_groups.len(), 1);
    }

    #[test]
    fn eden_singleton_is_eden_plus_singleton() {
        let r = Rules::eden_singleton();
        assert!(r.singleton);
        assert_eq!(r.restriction, CardRestriction::eden());
        // Same standard gameplay shape otherwise.
        let std = Rules::standard();
        assert_eq!(r.deck_size, std.deck_size);
        assert_eq!(r.egg_deck_max, std.egg_deck_max);
        assert_eq!(r.security_count, std.security_count);
        assert_eq!(r.skip_first_draw, std.skip_first_draw);
    }

    #[test]
    fn edh_settings() {
        let r = Rules::edh();
        assert_eq!(r.player_count, 4);
        assert_eq!(r.deck_size, 70);
        assert_eq!(r.security_count, 7);
        assert!(r.singleton);
        assert!(r.commander);
        assert_eq!(r.max_turns, 600);
        assert_eq!(r.skip_first_draw, SkipDraw::AllRound1);
    }

    #[test]
    fn edh_has_no_restriction() {
        // Singleton is the rule, not a ban list.
        let r = Rules::edh();
        assert!(r.restriction.card_limits.is_empty());
        assert!(r.restriction.choice_groups.is_empty());
    }

    #[test]
    fn titan_boss_settings() {
        let r = Rules::titan_boss();
        assert_eq!(r.deck_size, 80);
        assert_eq!(r.security_count, 15);
        assert_eq!(r.starting_hand, 7);
        assert_eq!(r.skip_first_draw, SkipDraw::FirstPlayerOnly);
    }

    #[test]
    fn titan_boss_has_official_restriction() {
        let r = Rules::titan_boss();
        assert_eq!(r.restriction.card_limits.len(), 50);
        assert_eq!(r.restriction.choice_groups.len(), 2);
    }

    #[test]
    fn titan_team_settings() {
        let r = Rules::titan_team();
        assert_eq!(r.deck_size, 50);
        assert_eq!(r.security_count, 5);
        assert_eq!(r.starting_hand, 5);
    }

    #[test]
    fn titan_team_has_official_restriction() {
        let r = Rules::titan_team();
        assert_eq!(r.restriction.card_limits.len(), 50);
        assert_eq!(r.restriction.choice_groups.len(), 2);
    }

    #[test]
    fn official_banned_cards() {
        let r = CardRestriction::official_eng();
        let banned: Vec<_> = r
            .card_limits
            .iter()
            .filter(|(_, v)| **v == 0)
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(banned.len(), 3);
        assert_eq!(r.card_limits.get("BT2-090"), Some(&0));
        assert_eq!(r.card_limits.get("BT5-109"), Some(&0));
        assert_eq!(r.card_limits.get("EX5-065"), Some(&0));
    }

    #[test]
    fn official_restricted_count() {
        let r = CardRestriction::official_eng();
        let restricted = r.card_limits.values().filter(|v| **v == 1).count();
        assert_eq!(restricted, 47);
    }

    #[test]
    fn official_choice_groups_count() {
        let r = CardRestriction::official_eng();
        assert_eq!(r.choice_groups.len(), 2);
        // Choice 1: Mother D-Reaper vs Shoto Kazama
        assert_eq!(r.choice_groups[0].0, vec!["EX2-007".to_string()]);
        assert_eq!(r.choice_groups[0].1, vec!["EX7-064".to_string()]);
        // Choice 2: Chaosmon: Valdur Arm vs Taomon + Sakuyamon (X Antibody)
        assert_eq!(r.choice_groups[1].0, vec!["BT20-037".to_string()]);
        assert_eq!(
            r.choice_groups[1].1,
            vec!["BT17-035".to_string(), "EX8-037".to_string()]
        );
    }

    #[test]
    fn for_mode_routing() {
        assert_eq!(
            Rules::for_mode(GameMode::Standard, None).unwrap(),
            Rules::standard()
        );
        assert_eq!(
            Rules::for_mode(GameMode::Pauper, None).unwrap(),
            Rules::pauper()
        );
        assert_eq!(
            Rules::for_mode(GameMode::NoRestriction, None).unwrap(),
            Rules::no_restriction()
        );
        assert_eq!(
            Rules::for_mode(GameMode::Eden, None).unwrap(),
            Rules::eden()
        );
        assert_eq!(
            Rules::for_mode(GameMode::EdenSingleton, None).unwrap(),
            Rules::eden_singleton()
        );
        assert_eq!(
            Rules::for_mode(GameMode::EdhCommander, None).unwrap(),
            Rules::edh()
        );
        assert_eq!(
            Rules::for_mode(GameMode::Titan, Some(TitanRole::Boss)).unwrap(),
            Rules::titan_boss()
        );
        assert_eq!(
            Rules::for_mode(GameMode::Titan, Some(TitanRole::Team)).unwrap(),
            Rules::titan_team()
        );
    }

    #[test]
    fn for_mode_titan_without_role_errors() {
        assert_eq!(
            Rules::for_mode(GameMode::Titan, None),
            Err(ForModeError::MissingTitanRole)
        );
    }

    #[test]
    fn for_mode_role_on_non_titan_errors() {
        assert_eq!(
            Rules::for_mode(GameMode::Standard, Some(TitanRole::Boss)),
            Err(ForModeError::UnexpectedRole(GameMode::Standard))
        );
    }

    #[test]
    fn official_eng_ref_matches_owned() {
        // Both APIs must agree on contents.
        let owned = CardRestriction::official_eng();
        let by_ref = CardRestriction::official_eng_ref();
        assert_eq!(&owned, by_ref);
    }
}
