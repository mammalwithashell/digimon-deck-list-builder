use serde::{Deserialize, Serialize};

use crate::enums::SkipDraw;

/// Configurable game parameters. Drives player count, deck sizes, tensor/action layout.
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
}

impl Rules {
    /// Standard 2-player Digimon TCG rules.
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
        }
    }

    /// 4-player EDH Commander format.
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
        }
    }

    /// Titan mode — the Titan (boss) player.
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
        }
    }

    /// Titan mode — a team player.
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
        }
    }
}

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
    fn edh_settings() {
        let r = Rules::edh();
        assert_eq!(r.player_count, 4);
        assert_eq!(r.deck_size, 70);
        assert_eq!(r.security_count, 7);
        assert!(r.singleton);
        assert!(r.commander);
        assert_eq!(r.max_turns, 600);
    }

    #[test]
    fn titan_boss_settings() {
        let r = Rules::titan_boss();
        assert_eq!(r.deck_size, 80);
        assert_eq!(r.security_count, 15);
        assert_eq!(r.starting_hand, 7);
    }
}
