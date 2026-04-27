use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::enums::{GameMode, SkipDraw, TitanRole};

/// Built once, cloned on each `CardRestriction::official_eng()` call — the data
/// is effectively static config, so don't re-allocate the map and vectors every time.
static OFFICIAL_ENG_RESTRICTION: LazyLock<CardRestriction> = LazyLock::new(|| {
    let mut card_limits = BTreeMap::new();

    // 3 Banned cards (0 copies allowed)
    for id in ["BT2-090", "BT5-109", "EX5-065"] {
        card_limits.insert(id.to_string(), 0);
    }

    // 47 Restricted cards (max 1 copy)
    for id in [
        "BT1-090", "BT10-009", "BT11-033", "BT11-064", "BT13-012", "BT13-110", "BT14-002",
        "BT14-084", "BT15-057", "BT15-102", "BT16-011", "BT17-069", "BT19-040", "BT2-047",
        "BT2-069", "BT3-054", "BT3-103", "BT4-104", "BT4-111", "BT6-100", "BT6-104", "BT7-038",
        "BT7-064", "BT7-069", "BT7-072", "BT7-107", "BT9-098", "BT9-099", "EX1-021", "EX1-068",
        "EX2-039", "EX2-070", "EX3-057", "EX4-006", "EX4-019", "EX4-030", "EX5-015", "EX5-018",
        "EX5-062", "P-008", "P-025", "P-029", "P-030", "P-123", "P-130", "ST2-13", "ST9-09",
    ] {
        card_limits.insert(id.to_string(), 1);
    }

    let choice_groups = vec![
        // Choice 1: Mother D-Reaper vs Shoto Kazama
        (vec!["EX2-007".to_string()], vec!["EX7-064".to_string()]),
        // Choice 2: Chaosmon: Valdur Arm vs Taomon + Sakuyamon (X Antibody)
        (
            vec!["BT20-037".to_string()],
            vec!["BT17-035".to_string(), "EX8-037".to_string()],
        ),
    ];

    CardRestriction {
        card_limits,
        choice_groups,
    }
});

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
    /// Must stay in sync with `digimon_gym/engine/data/deck_loader.py`:
    /// `_BANNED`, `_RESTRICTED`, `_CHOICE_GROUPS`. Update both sides together
    /// until the Python engine is retired.
    ///
    /// Clones from a process-wide cached copy (see `OFFICIAL_ENG_RESTRICTION`);
    /// use `official_eng_ref()` if you only need a shared reference.
    pub fn official_eng() -> Self {
        OFFICIAL_ENG_RESTRICTION.clone()
    }

    /// Shared reference to the cached official ENG restricted list.
    /// Prefer this over `official_eng()` when you don't need ownership.
    pub fn official_eng_ref() -> &'static Self {
        &OFFICIAL_ENG_RESTRICTION
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
            (GameMode::NoRestriction, None) => Ok(Self::no_restriction()),
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
    }

    #[test]
    fn no_restriction_is_empty() {
        let r = Rules::no_restriction();
        assert!(r.restriction.card_limits.is_empty());
        assert!(r.restriction.choice_groups.is_empty());
        // All other fields match standard()
        let std = Rules::standard();
        assert_eq!(r.player_count, std.player_count);
        assert_eq!(r.deck_size, std.deck_size);
        assert_eq!(r.security_count, std.security_count);
        assert_eq!(r.starting_hand, std.starting_hand);
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
            Rules::for_mode(GameMode::NoRestriction, None).unwrap(),
            Rules::no_restriction()
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
