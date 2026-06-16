//! Data-driven deck-format registry — the single source of truth for format
//! descriptors, named card restrictions, and the EDEN anomaly protocol.
//!
//! Parsed once from `data/deck_formats.json` (baked into the crate via
//! `include_str!`, the same pattern `deck_tools.rs` uses for `cards.json`).
//! The hosted API reads the same file at runtime, so banlists and the anomaly
//! definition live in exactly one place and can be edited without touching
//! Rust or Python logic. See `docs/EDEN_FORMAT_RULES.md` and the
//! `add-deck-format-registry` change.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::enums::{GameMode, Rarity};
use crate::rules::CardRestriction;

const DECK_FORMATS_JSON: &str = include_str!("../../../data/deck_formats.json");

/// How a format gates card rarity. `EdenAnomaly` is C/U-legal plus the
/// anomaly-protocol exceptions defined in the data file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RarityPolicy {
    All,
    CommonUncommon,
    EdenAnomaly,
}

/// One anomaly-protocol matching rule: a rare-or-higher card of `card_kind`
/// whose rarity is in `rarities` (and whose lowercased name contains
/// `name_contains`, if set) is an anomaly card.
#[derive(Debug, Clone)]
pub struct AnomalyCategory {
    /// 0=Digimon, 1=Tamer, 2=Option, 3=Digi-Egg (matches `CardSummary.card_kind`).
    pub card_kind: u8,
    /// Lowercased substring the card name must contain, or `None` for any.
    pub name_contains: Option<String>,
    pub rarities: Vec<Rarity>,
}

/// The EDEN Anomaly Protocol: which rare+ cards are allowed, and how many.
#[derive(Debug, Clone)]
pub struct AnomalyProtocol {
    pub max_total: u32,
    pub categories: Vec<AnomalyCategory>,
    pub extra_card_ids: HashSet<String>,
}

impl AnomalyProtocol {
    /// True if a (rare-or-higher) card qualifies as an anomaly-protocol card.
    /// `name_lower` must already be lowercased by the caller.
    pub fn matches(&self, card_id: &str, card_kind: u8, rarity: Rarity, name_lower: &str) -> bool {
        if self.extra_card_ids.contains(card_id) {
            return true;
        }
        self.categories.iter().any(|cat| {
            cat.card_kind == card_kind
                && cat.rarities.contains(&rarity)
                && cat
                    .name_contains
                    .as_deref()
                    .map_or(true, |needle| name_lower.contains(needle))
        })
    }
}

/// A fully-resolved format: identity + display metadata + deck-construction
/// rules. The registry is the authority — validation and `card_legality`
/// derive everything from this struct.
#[derive(Debug, Clone)]
pub struct FormatDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub deck_size: u16,
    pub egg_max: u8,
    pub rarity_policy: RarityPolicy,
    pub restriction: CardRestriction,
    pub singleton: bool,
    pub default_max_copies: u8,
    pub playable: bool,
}

// ─── Raw parse structs (mirror deck_formats.json) ─────────────────────────

#[derive(Deserialize)]
struct RawFile {
    restrictions: HashMap<String, RawRestriction>,
    anomaly_protocol: RawAnomaly,
    formats: Vec<RawFormat>,
}

#[derive(Deserialize)]
struct RawRestriction {
    #[serde(default)]
    banned: Vec<String>,
    #[serde(default)]
    limited: Vec<String>,
    #[serde(default)]
    limited_to: BTreeMap<String, u8>,
    #[serde(default)]
    choice_groups: Vec<(Vec<String>, Vec<String>)>,
}

#[derive(Deserialize)]
struct RawAnomaly {
    max_total: u32,
    #[serde(default)]
    categories: Vec<RawCategory>,
    #[serde(default)]
    extra_card_ids: Vec<String>,
}

#[derive(Deserialize)]
struct RawCategory {
    card_kind: String,
    #[serde(default)]
    name_contains: Option<String>,
    rarities: Vec<String>,
}

#[derive(Deserialize)]
struct RawFormat {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    deck_size: u16,
    egg_max: u8,
    rarity_policy: String,
    #[serde(default)]
    banlist: Option<String>,
    #[serde(default)]
    singleton: bool,
    default_max_copies: u8,
    #[serde(default)]
    playable: bool,
}

fn parse_card_kind(s: &str) -> u8 {
    match s.to_ascii_lowercase().as_str() {
        "digimon" => 0,
        "tamer" => 1,
        "option" => 2,
        "digi-egg" | "digiegg" => 3,
        other => panic!("deck_formats.json: unknown card_kind {other:?}"),
    }
}

fn parse_rarity_code(s: &str) -> Rarity {
    match s {
        "C" => Rarity::C,
        "U" => Rarity::U,
        "R" => Rarity::R,
        "SR" => Rarity::SR,
        "SEC" => Rarity::SEC,
        "P" => Rarity::P,
        other => panic!("deck_formats.json: unknown rarity code {other:?}"),
    }
}

fn parse_rarity_policy(s: &str) -> RarityPolicy {
    match s {
        "all" => RarityPolicy::All,
        "common_uncommon" => RarityPolicy::CommonUncommon,
        "eden_anomaly" => RarityPolicy::EdenAnomaly,
        other => panic!("deck_formats.json: unknown rarity_policy {other:?}"),
    }
}

/// Merge a raw restriction's banned/limited/limited_to into a single
/// `card_id -> max copies` map. Insertion order (banned → limited →
/// limited_to) makes `limited_to` win on overlap, mirroring the prior
/// hardcoded "insert then override" behaviour (e.g. Eosmon `BT6-085`).
fn build_restriction(raw: &RawRestriction) -> CardRestriction {
    let mut card_limits: BTreeMap<String, u8> = BTreeMap::new();
    for id in &raw.banned {
        card_limits.insert(id.clone(), 0);
    }
    for id in &raw.limited {
        card_limits.insert(id.clone(), 1);
    }
    for (id, n) in &raw.limited_to {
        card_limits.insert(id.clone(), *n);
    }
    CardRestriction {
        card_limits,
        choice_groups: raw.choice_groups.clone(),
    }
}

struct Registry {
    formats: Vec<FormatDescriptor>,
    by_id: HashMap<String, usize>,
    anomaly: AnomalyProtocol,
    restrictions: HashMap<String, CardRestriction>,
}

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw: RawFile = serde_json::from_str(DECK_FORMATS_JSON)
            .expect("deck_formats.json is malformed (compiled-in resource)");

        let restrictions: HashMap<String, CardRestriction> = raw
            .restrictions
            .iter()
            .map(|(name, r)| (name.clone(), build_restriction(r)))
            .collect();

        let anomaly = AnomalyProtocol {
            max_total: raw.anomaly_protocol.max_total,
            categories: raw
                .anomaly_protocol
                .categories
                .iter()
                .map(|c| AnomalyCategory {
                    card_kind: parse_card_kind(&c.card_kind),
                    name_contains: c.name_contains.as_ref().map(|s| s.to_ascii_lowercase()),
                    rarities: c.rarities.iter().map(|r| parse_rarity_code(r)).collect(),
                })
                .collect(),
            extra_card_ids: raw.anomaly_protocol.extra_card_ids.iter().cloned().collect(),
        };

        let mut formats = Vec::with_capacity(raw.formats.len());
        let mut by_id = HashMap::new();
        for f in &raw.formats {
            let restriction = match &f.banlist {
                Some(name) => restrictions
                    .get(name)
                    .unwrap_or_else(|| {
                        panic!(
                            "deck_formats.json: format {:?} references unknown banlist {:?}",
                            f.id, name
                        )
                    })
                    .clone(),
                None => CardRestriction::none(),
            };
            if by_id.insert(f.id.clone(), formats.len()).is_some() {
                panic!("deck_formats.json: duplicate format id {:?}", f.id);
            }
            formats.push(FormatDescriptor {
                id: f.id.clone(),
                name: f.name.clone(),
                description: f.description.clone(),
                deck_size: f.deck_size,
                egg_max: f.egg_max,
                rarity_policy: parse_rarity_policy(&f.rarity_policy),
                restriction,
                singleton: f.singleton,
                default_max_copies: f.default_max_copies,
                playable: f.playable,
            });
        }

        Registry {
            formats,
            by_id,
            anomaly,
            restrictions,
        }
    })
}

/// All formats in declaration order (drives the deck-builder selector and
/// the play/format catalog).
pub fn list_formats() -> &'static [FormatDescriptor] {
    &registry().formats
}

/// Look up a format descriptor by its `game_mode` id (e.g. `"eden_singleton"`).
pub fn descriptor(id: &str) -> Option<&'static FormatDescriptor> {
    let reg = registry();
    reg.by_id.get(id).map(|&i| &reg.formats[i])
}

/// The (single) anomaly protocol definition.
pub fn anomaly_protocol() -> &'static AnomalyProtocol {
    &registry().anomaly
}

/// A named restriction (banlist) by name, e.g. `"official_eng"` / `"eden"`.
pub fn restriction_by_name(name: &str) -> Option<&'static CardRestriction> {
    registry().restrictions.get(name)
}

/// Map a typed `GameMode` to its registry id, or `None` for modes that are
/// not registry-backed deck-legality formats (EDH / Titan).
pub fn game_mode_id(mode: GameMode) -> Option<&'static str> {
    Some(match mode {
        GameMode::Standard => "standard",
        GameMode::NoRestriction => "no_restriction",
        GameMode::Pauper => "pauper",
        GameMode::Eden => "eden",
        GameMode::EdenSingleton => "eden_singleton",
        GameMode::EdhCommander | GameMode::Titan => return None,
    })
}

/// Descriptor for a typed `GameMode`, or `None` for non-registry modes.
pub fn descriptor_for_mode(mode: GameMode) -> Option<&'static FormatDescriptor> {
    descriptor(game_mode_id(mode)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_formats() {
        let ids: Vec<&str> = list_formats().iter().map(|f| f.id.as_str()).collect();
        assert_eq!(
            ids,
            ["standard", "no_restriction", "pauper", "eden", "eden_singleton"]
        );
    }

    #[test]
    fn every_banlist_resolves_and_policy_is_known() {
        for f in list_formats() {
            // build_restriction / parse_rarity_policy already panic on bad
            // data during init; reaching here means every reference resolved.
            assert!(!f.name.is_empty(), "format {} has empty name", f.id);
            // Every GameMode that maps to an id must have a descriptor.
        }
        for mode in [
            GameMode::Standard,
            GameMode::NoRestriction,
            GameMode::Pauper,
            GameMode::Eden,
            GameMode::EdenSingleton,
        ] {
            let id = game_mode_id(mode).expect("registry mode must have an id");
            assert!(descriptor(id).is_some(), "no descriptor for {id}");
        }
        assert!(game_mode_id(GameMode::EdhCommander).is_none());
        assert!(game_mode_id(GameMode::Titan).is_none());
    }

    #[test]
    fn rarity_policies_per_format() {
        assert_eq!(descriptor("standard").unwrap().rarity_policy, RarityPolicy::All);
        assert_eq!(
            descriptor("no_restriction").unwrap().rarity_policy,
            RarityPolicy::All
        );
        assert_eq!(
            descriptor("pauper").unwrap().rarity_policy,
            RarityPolicy::CommonUncommon
        );
        assert_eq!(
            descriptor("eden").unwrap().rarity_policy,
            RarityPolicy::EdenAnomaly
        );
        let es = descriptor("eden_singleton").unwrap();
        assert_eq!(es.rarity_policy, RarityPolicy::EdenAnomaly);
        assert!(es.singleton);
        assert_eq!(es.default_max_copies, 1);
    }

    #[test]
    fn no_restriction_has_empty_banlist() {
        let nb = descriptor("no_restriction").unwrap();
        assert!(nb.restriction.card_limits.is_empty());
        assert!(nb.restriction.choice_groups.is_empty());
    }

    #[test]
    fn anomaly_protocol_loaded() {
        let a = anomaly_protocol();
        assert_eq!(a.max_total, 4);
        assert!(!a.categories.is_empty());
        // A rare Tamer is an anomaly card; a rare Digimon is not.
        assert!(a.matches("X-000", 1, Rarity::R, "some tamer"));
        assert!(!a.matches("X-000", 0, Rarity::R, "some digimon"));
        // Memory Boost name gate.
        assert!(a.matches("X-001", 2, Rarity::SR, "gallantmon's memory boost!!"));
        assert!(!a.matches("X-002", 2, Rarity::SR, "some other option"));
    }
}
