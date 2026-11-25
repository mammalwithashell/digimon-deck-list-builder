//! Printed standard digivolve circles that the data or the specs once got
//! wrong, shared by the three guards that keep them right:
//!
//! - `tests/data_official_parity.rs` — the JSON data files;
//! - `tests/digivolve_color_requirements.rs` — production card data through the
//!   mask and the digivolve action;
//! - `tests/cards_behavioral/printed_digivolve_circles.rs` — the DSL fixtures
//!   (DebugRunner card data, which never sees `cards.json`).
//!
//! Colour codes follow `CardColor` (Red = 0 .. Purple = 6).
#![allow(dead_code)]

use digimon_engine::enums::CardColor;
use digimon_engine::enums::CardColor::{Black, Blue, Green, Purple, Red, White, Yellow};

pub const ALL_COLOURS: [CardColor; 7] = [Red, Blue, Yellow, Green, White, Black, Purple];

/// Rainbow (ANY-colour) standard digivolve circles:
/// `(card_id, level digivolved from, printed cost)`.
///
/// The card face prints one circle with a rainbow fill; the official English
/// DB lists it as "Multicolor N from Lv.X" (`<span class='cardColor_all'>`),
/// the Japanese DB as 「全 Lv.XからN」 (全 = all), and DCGO's card asset carries
/// one `EvoCosts` row per colour. The card digivolves from a Digimon of that
/// level of ANY colour — NOT "a base with 2+ colours" (rule 2-4-3's
/// "multicolor card" is a different thing; Susanoomon (EX12-076) was once
/// authored that way). `evo_costs` holds it as one row per colour.
///
/// Every entry was checked on 2026-11-21 against the card image, the live
/// official EN and JP pages and the DCGO card asset. The list is closed:
/// `data_official_parity.rs` fails when any other card carries a seven-colour
/// circle, so a new rainbow card has to be read off its face and added here.
///
/// History: the digimoncard.io ingest approximated the circle as one RED row,
/// and the official-mirror builder dropped it altogether, so a reconcile
/// against the mirror wrote `evo_costs: []` for Omnimon: Merciful Mode
/// (EX13-077). In 1,224 logged games that card was never once offered its
/// Lv.6 digivolve (566 opportunities), and Kimeramon (BT8-084) could not
/// digivolve from a blue or purple Lv.4.
pub const RAINBOW_CIRCLES: &[(&str, u8, i16)] = &[
    ("BT7-112", 6, 7),  // Susanoomon
    ("BT8-084", 4, 4),  // Kimeramon
    ("BT16-082", 2, 1), // Ukkomon
    ("BT16-083", 5, 5), // BigUkkomon
    ("BT18-102", 6, 6), // Susanoomon
    ("BT19-102", 4, 4), // Luminamon (Nene Version)
    ("EX9-074", 4, 5),  // Kimeramon
    ("EX12-076", 6, 6), // Susanoomon
    ("EX13-077", 6, 6), // Omnimon: Merciful Mode
    ("P-123", 2, 0),    // Ukkomon
];

/// The rainbow-circle cards that have a YAML spec (the other three are not
/// implemented yet and exist only as card data). Susanoomon (BT18-102)'s spec
/// is `cards/_examples/BT18-102.yaml`.
pub const RAINBOW_CIRCLE_SPECS: &[&str] = &[
    "BT8-084", "BT16-082", "BT18-102", "EX9-074", "EX12-076", "EX13-077", "P-123",
];

/// Colour-specific circles: `(card_id, level digivolved from, printed cost,
/// printed circle colours)`.
///
/// The 35 specs that authored their circle as a colour-free
/// `from: { level_eq: N }` gate — a real route from a base of ANY colour;
/// Dimetromon (P-189) and Cyclonemon (BT24-011), both printed "Red Lv.3: 2",
/// digivolved onto the white Ukkomon (BT16-082) in logged games — plus three
/// whose bare gate carried a cost the card does not print (Pyramidimon
/// (EX10-033) and (EX11-044): 4 for a printed 3; Magneticdramon (EX10-036): 6
/// for a printed 4; MedievalGallantmon (EX8-074), one of the 35, also had 4
/// for a printed 3).
///
/// Every row was read off the card image on 2026-11-21 and agrees with the
/// official mirror and the DCGO asset, except Coredramon (ST1-06) and
/// Phoenixmon (ST1-10), whose images are not in the local mirror: those two
/// rows are the live official page plus the DCGO asset.
pub const COLOURED_CIRCLES: &[(&str, u8, i16, &[CardColor])] = &[
    ("AD1-012", 5, 4, &[Blue, Black]),   // CresGarurumon
    ("BT9-112", 6, 6, &[Purple, Black]), // DeathXmon
    ("BT16-025", 4, 4, &[Blue, Green]),  // Paildramon
    ("BT16-040", 2, 1, &[Blue, Red]),    // Wormmon (a green/purple card; the circle is blue|red)
    ("BT21-007", 2, 0, &[Red]),          // Agumon
    ("BT21-008", 2, 0, &[Red]),          // Elizamon
    ("BT21-013", 3, 2, &[Red]),          // Agunimon
    ("BT21-024", 4, 3, &[Red]),          // Cyberdramon
    ("BT21-025", 4, 3, &[Red]),          // Lamiamon
    ("BT21-026", 5, 3, &[Red]),          // WarGreymon
    ("BT21-029", 5, 4, &[Red]),          // Medusamon
    ("BT21-037", 3, 3, &[Blue, Green]),  // Lighdramon
    ("BT21-072", 4, 4, &[Purple, Red]),  // Arresterdramon: Superior Mode
    ("BT22-013", 5, 4, &[Red, Blue]),    // WarGreymon
    ("BT22-026", 5, 4, &[Blue, Red]),    // MetalGarurumon
    ("BT23-005", 2, 0, &[Red]),          // Elizamon
    ("BT23-014", 5, 3, &[Red]),          // Gallantmon
    ("BT24-011", 3, 2, &[Red]),          // Cyclonemon
    ("BT24-012", 3, 2, &[Red]),          // Dimetromon
    ("BT24-016", 4, 3, &[Red]),          // Lamiamon
    ("EX8-074", 5, 3, &[Green, Red]),    // MedievalGallantmon
    ("EX9-008", 2, 0, &[Red]),           // Biyomon
    ("EX10-033", 5, 3, &[Black]),        // Pyramidimon
    ("EX10-036", 6, 4, &[Black]),        // Magneticdramon
    ("EX11-008", 2, 0, &[Red]),          // Elizamon
    ("EX11-044", 5, 3, &[Black]),        // Pyramidimon
    ("P-137", 3, 3, &[Red, Blue]),       // Flamedramon
    ("P-189", 3, 2, &[Red]),             // Dimetromon
    ("ST1-02", 2, 0, &[Red]),            // Biyomon
    ("ST1-03", 2, 0, &[Red]),            // Agumon
    ("ST1-04", 2, 0, &[Red]),            // Dracomon
    ("ST1-05", 3, 2, &[Red]),            // Birdramon
    ("ST1-06", 3, 2, &[Red]),            // Coredramon
    ("ST1-07", 3, 2, &[Red]),            // Greymon
    ("ST1-08", 4, 3, &[Red]),            // Garudamon
    ("ST1-09", 4, 3, &[Red]),            // MetalGreymon
    ("ST1-10", 5, 2, &[Red]),            // Phoenixmon
    ("ST1-11", 5, 4, &[Red]),            // WarGreymon
];

/// Multi-colour (split) standard circles of cards that the official mirror does
/// not carry (all of EX12 plus two promos), so `cards.json` came from the
/// digimoncard.io API alone — which keeps one colour of a split circle:
/// `(card_id, level digivolved from, printed cost, printed circle colours)`.
///
/// Each row was read off the card face (circle crop) on 2026-11-21 and agrees
/// with the DCGO card asset's `EvoCosts` (one row per colour). The API had
/// dropped the second (and for Chaosdramon (EX12-060) the third) colour, so a
/// base of that colour could not be digivolved onto in production.
/// `tests/data_official_parity.rs` pins `cards.json`, `card_overrides.json`
/// and the per-card JSON against this table.
pub const SPLIT_CIRCLES_OUTSIDE_THE_MIRROR: &[(&str, u8, i16, &[CardColor])] = &[
    ("EX12-023", 2, 1, &[Blue, Yellow]),         // Jellymon
    ("EX12-024", 3, 3, &[Blue, Purple]),         // Garurumon
    ("EX12-027", 3, 3, &[Blue, Yellow]),         // TeslaJellymon
    ("EX12-028", 4, 4, &[Blue, Black]),          // Gusokumon
    ("EX12-030", 4, 4, &[Blue, Yellow]),         // Thetismon
    ("EX12-032", 4, 4, &[Blue, Purple]),         // WereGarurumon
    ("EX12-035", 5, 4, &[Blue, Purple]),         // MetalGarurumon
    ("EX12-038", 2, 0, &[Yellow, Black]),        // Kokuwamon
    ("EX12-041", 3, 3, &[Yellow, Black]),        // Thundermon
    ("EX12-042", 3, 3, &[Yellow, Green]),        // Gatomon
    ("EX12-049", 2, 1, &[Green, Black]),         // Angoramon
    ("EX12-050", 3, 3, &[Green, Black]),         // SymbareAngoramon
    ("EX12-051", 4, 4, &[Green, Black]),         // Lamortmon
    ("EX12-055", 4, 4, &[Black, Yellow]),        // Andromon
    ("EX12-058", 5, 3, &[Black, Yellow]),        // HiAndromon
    ("EX12-059", 5, 4, &[Black, Purple]),        // Machinedramon
    ("EX12-060", 6, 5, &[Black, Purple, Red]),   // Chaosdramon (a three-colour circle)
    ("EX12-064", 4, 4, &[Purple, Black]),        // Megadramon
    ("EX12-077", 6, 5, &[Red, Purple]),          // Proximamon
    ("P-248", 2, 1, &[Blue, Red]),               // Veemon
];

/// Split (two- or three-colour) standard circles whose specs authored only ONE
/// of the printed colours until 2026-11-22:
/// `(card_id, level digivolved from, printed cost, printed circle colours)`.
///
/// Production reads the circle from `cards.json`, which had every colour, so
/// live games were right; a DebugRunner fixture builds its card data from the
/// spec alone, so every test of these 99 cards ran against a card that could
/// not digivolve from its second (or third) colour.
///
/// Each row was read off the card image (circle crop) on 2026-11-22 and agrees
/// with the DCGO card asset's `EvoCosts` and with `cards.json`; where the card
/// has an official bundle it agrees with that too, except Mastemon (EX6-029),
/// whose mirror entry adds a Lv.5 / cost 5 pair the card does not print.
/// `tests/cards_behavioral/printed_digivolve_circles.rs` sweeps the table:
/// the fixture digivolves from exactly these colours, at this cost.
pub const SPLIT_CIRCLES_COMPLETED: &[(&str, u8, i16, &[CardColor])] = &[
    ("AD1-009", 5, 4, &[Red, Black]),         // BlitzGreymon
    ("AD1-017", 5, 3, &[Yellow, Red]),        // Dynasmon
    ("AD1-018", 5, 3, &[Purple, Black]),      // LordKnightmon
    ("AD1-024", 5, 5, &[Blue, Green]),        // Imperialdramon: Fighter Mode
    ("BT11-042", 4, 3, &[Yellow, Purple]),    // Angewomon
    ("BT12-038", 3, 3, &[Red, Yellow]),       // GeoGreymon
    ("BT12-043", 5, 4, &[Red, Yellow]),       // ShineGreymon
    ("BT12-047", 2, 0, &[Green, Blue]),       // Wormmon
    ("BT12-057", 6, 6, &[Green, White, Black]),// Quartzmon
    ("BT13-019", 5, 5, &[Red, Black]),        // Gankoomon
    ("BT13-112", 6, 4, &[Red, Blue]),         // Omnimon
    ("BT16-027", 5, 5, &[Blue, Green]),       // Imperialdramon: Fighter Mode
    ("BT16-028", 5, 4, &[Blue, Green]),       // Imperialdramon: Dragon Mode
    ("BT16-077", 4, 4, &[Purple, Red]),       // Dinobeemon
    ("BT16-101", 5, 3, &[Yellow, Green]),     // Rapidmon (X Antibody)
    ("BT16-102", 5, 4, &[Yellow, Blue, Black]),// Magnamon (X Antibody)
    ("BT17-019", 2, 0, &[Blue, Red]),         // Gabumon
    ("BT17-078", 6, 5, &[Red, Blue]),         // Omnimon
    ("BT18-019", 6, 4, &[Red, Black]),        // Millenniummon
    ("BT19-008", 2, 1, &[Red, Yellow]),       // Shoutmon
    ("BT19-033", 3, 3, &[Yellow, Green]),     // Dorulumon
    ("BT19-047", 3, 3, &[Black, Green]),      // Ballistamon
    ("BT19-057", 2, 1, &[Black, Purple]),     // Sparrowmon
    ("BT19-061", 3, 3, &[Black, Purple]),     // RaptorSparrowmon
    ("BT19-065", 5, 3, &[Black, Purple]),     // Machinedramon
    ("BT19-072", 5, 3, &[Purple, Black]),     // LordKnightmon
    ("BT19-073", 5, 4, &[Purple, Black]),     // LordKnightmon (X Antibody)
    ("BT20-016", 4, 4, &[Red, Purple]),       // Paildramon
    ("BT20-021", 6, 6, &[Red, Black]),        // Jesmon GX
    ("BT20-036", 5, 4, &[Yellow, Black]),     // BanchoLeomon
    ("BT20-055", 5, 3, &[Black, Blue]),       // Invisimon
    ("BT20-059", 5, 5, &[Black, Red]),        // Gankoomon (X Antibody)
    ("BT20-060", 6, 6, &[Black, Yellow, Red]),// Alphamon: Ouryuken
    ("BT20-076", 5, 4, &[Purple, Red]),       // Imperialdramon: Dragon Mode
    ("BT20-081", 6, 6, &[Purple, Yellow]),    // Fenriloogamon: Takemikazuchi
    ("BT21-011", 2, 1, &[Red, Yellow]),       // Shoutmon
    ("BT21-021", 4, 4, &[Red, Yellow]),       // OmniShoutmon
    ("BT21-044", 4, 4, &[Yellow, Red]),       // RizeGreymon
    ("BT22-008", 2, 0, &[Red, Blue]),         // Agumon
    ("BT22-015", 6, 6, &[Red, Blue]),         // Omnimon
    ("BT23-008", 3, 2, &[Red, Blue]),         // Greymon
    ("BT23-018", 3, 2, &[Blue, Red]),         // Garurumon
    ("BT23-037", 2, 1, &[Green, Yellow]),     // Tentomon
    ("BT24-010", 3, 3, &[Black, Red]),        // Greymon
    ("BT24-030", 5, 4, &[Blue, Black]),       // Neptunemon
    ("BT24-035", 3, 2, &[Yellow, Red]),       // Gatomon
    ("BT24-037", 4, 4, &[Yellow, Red]),       // Silphymon
    ("BT24-046", 3, 3, &[Green, Blue]),       // Garurumon
    ("BT24-051", 5, 4, &[Green, Blue]),       // Merukimon
    ("BT25-011", 3, 3, &[Red, Yellow]),       // Aquilamon
    ("BT25-015", 4, 4, &[Red, Green]),        // Garudamon
    ("BT25-016", 4, 4, &[Red, Green]),        // GrapLeomon
    ("BT25-017", 4, 3, &[Red, Blue]),         // Flaremon
    ("BT25-018", 5, 4, &[Red, Yellow]),       // Apollomon
    ("BT25-024", 3, 2, &[Blue, Red]),         // Lekismon
    ("BT25-026", 4, 3, &[Blue, Red]),         // Crescemon
    ("BT25-028", 5, 4, &[Blue, Purple]),      // Dianamon
    ("BT25-037", 3, 3, &[Yellow, Black]),     // Pegasusmon
    ("BT25-038", 4, 4, &[Yellow, Black]),     // Shakkoumon
    ("BT25-054", 4, 4, &[Green, Black]),      // GreatGrizzlymon
    ("BT25-067", 3, 3, &[Black, Purple]),     // Sealsdramon
    ("EX3-008", 3, 2, &[Red, Purple]),        // Flamedramon
    ("EX4-060", 6, 6, &[Blue, Red]),          // Omnimon Alter-S
    ("EX4-074", 6, 5, &[Purple, Yellow]),     // ShineGreymon: Ruin Mode
    ("EX5-015", 2, 1, &[Blue, Purple]),       // Gabumon (X Antibody)
    ("EX6-011", 6, 5, &[Red, Black]),         // RagnaLoardmon
    ("EX6-029", 5, 4, &[Yellow, Purple]),     // Mastemon
    ("EX7-049", 5, 5, &[Black, Red]),         // Metallicdramon
    ("EX8-045", 5, 4, &[Purple, Green]),      // Callismon
    ("EX8-073", 5, 4, &[Red, Blue, Yellow]),  // Gallantmon (X Antibody)
    ("EX9-012", 4, 4, &[Red, Black]),         // MetalGreymon: Alterous Mode
    ("EX9-013", 5, 4, &[Red, Black]),         // BlitzGreymon
    ("EX9-021", 6, 5, &[Blue, Red]),          // Omnimon Alter-S
    ("EX9-024", 2, 1, &[Yellow, Purple]),     // Hanimon
    ("EX9-027", 3, 3, &[Yellow, Purple]),     // Kokeshimon
    ("EX9-032", 4, 4, &[Yellow, Purple]),     // Karakurumon
    ("EX9-033", 5, 4, &[Yellow, Purple]),     // Kaguyamon
    ("EX10-010", 5, 4, &[Red, Black]),        // BlackWarGreymon
    ("EX10-031", 4, 4, &[Black, Purple]),     // DarkKnightmon
    ("EX10-059", 6, 6, &[Purple, Black]),     // DarknessBagramon
    ("EX11-020", 2, 1, &[Yellow, Purple]),    // Hanimon
    ("EX11-021", 3, 3, &[Yellow, Purple]),    // Kokeshimon
    ("EX11-023", 5, 4, &[Yellow, Purple]),    // Kaguyamon
    ("EX12-024", 3, 3, &[Blue, Purple]),      // Garurumon
    ("EX12-032", 4, 4, &[Blue, Purple]),      // WereGarurumon
    ("EX12-035", 5, 4, &[Blue, Purple]),      // MetalGarurumon
    ("EX12-038", 2, 0, &[Yellow, Black]),     // Kokuwamon
    ("EX12-042", 3, 3, &[Yellow, Green]),     // Gatomon
    ("EX12-060", 6, 5, &[Black, Purple, Red]),// Chaosdramon
    ("P-101", 3, 3, &[Purple, Black]),        // Raremon
    ("P-117", 2, 0, &[Blue, Green]),          // Veemon
    ("P-186", 5, 4, &[Red, Yellow]),          // Gallantmon
    ("P-248", 2, 1, &[Blue, Red]),            // Veemon
    ("ST9-05", 4, 4, &[Blue, Green]),         // Paildramon
    ("ST9-06", 5, 4, &[Blue, Green]),         // Imperialdramon Dragon Mode
    ("ST19-04", 2, 1, &[Yellow, Black]),      // PawnChessmon
    ("ST19-05", 2, 1, &[Black, Yellow]),      // PawnChessmon
    ("ST20-10", 2, 0, &[Black, Red]),         // Agumon
    ("ST20-11", 5, 4, &[Black, Red]),         // WarGreymon
];

/// Every standard circle a card prints, read off the card face on 2026-11-22,
/// for the cards whose data held a circle the card does not print or lacked
/// one it does: `(card_id, &[(colour, level digivolved from, printed cost)])`.
///
/// The list is the 57 cards whose `cards.json` circles differed from the DCGO
/// card asset's `EvoCosts` (`Assets/CardBaseEntity`, compared set against set),
/// each judged from the card image (a circle crop where colours were close).
/// On 54 the face disagreed with `cards.json`, and the rows here are what it
/// prints; DCGO agreed with the face on all 54. The other three are not here:
/// Omnimon: Merciful Mode (EX13-077) prints a rainbow circle (`cards.json` is
/// right, the DCGO asset lacks the white row), and Guardromon (BT2-058) and
/// HerculesKabuterimon (BT15-053) have no local image while the official
/// mirror sides with `cards.json`, so they were left as they are.
///
/// Yasyamon (BT12-051), Ganemon (BT12-061) and Bulucomon (P-067) have no local
/// image either; their rows are the official mirror and the DCGO asset, which
/// agree with each other against the `cards.json` row that was removed.
///
/// Most removed rows came from the official mirror, which keeps rows of every
/// printing on its page and reads the cost of a name-gated "Digivolve: N from
/// [X]" line as a circle (Imperialdramon: Dragon Mode (BT12-030) blue|green
/// Lv.4 / 3, RizeGreymon (X Antibody) (BT9-041) yellow|red Lv.4 / 0). The rest
/// came from the digimoncard.io ingest alone (Garurumon (BT21-067) purple Lv.3
/// / 3 beside the printed 2). On the four cards marked "added" the mirror and
/// `cards.json` both missed a circle the face prints.
///
/// `tests/data_official_parity.rs` pins `cards.json`, `card_overrides.json` and
/// the per-card JSON to this table, `tests/digivolve_color_requirements.rs`
/// checks production digivolves over exactly these circles, and
/// `tests/alt_path_printed_cost_guard.rs` uses it as the printed baseline.
pub const FACE_READ_CIRCLES: &[(&str, &[(CardColor, u8, i16)])] = &[
    ("BT10-011", &[(Red, 4, 4)]), // Canoweissmon (not red Lv.4 / 3: the Gammamon line)
    ("BT10-050", &[(Green, 3, 2)]), // WezenGammamon (not green Lv.2 / 0)
    ("BT10-066", &[(Black, 4, 4)]), // DarkKnightmon (not black Lv.5 / 4)
    ("BT11-009", &[(Red, 3, 0), (Yellow, 2, 1)]), // Shoutmon + Star Sword (not red Lv.2 / 0)
    ("BT11-012", &[(Red, 3, 3), (Yellow, 3, 3)]), // Shoutmon X3 (not red Lv.3 / 2)
    ("BT11-019", &[(Red, 5, 6), (Blue, 6, 4)]), // Shoutmon X7 (not blue Lv.5 / 6)
    ("BT11-054", &[(Green, 4, 3), (Blue, 4, 3)]), // Panjyamon (not green|blue Lv.5 / 3)
    ("BT11-068", &[(Black, 4, 3), (Green, 4, 3)]), // Mamemon (not black|green Lv.4 / 0)
    ("BT12-012", &[(Red, 3, 2)]), // Agunimon (not red|yellow Lv.3 / 3)
    ("BT12-030", &[(Blue, 5, 4), (Green, 5, 4)]), // Imperialdramon: Dragon Mode (not blue|green Lv.4 / 3)
    ("BT12-051", &[(Green, 3, 2)]), // Yasyamon (not green Lv.3 / 0; no image)
    ("BT12-061", &[(Black, 3, 2)]), // Ganemon (not black Lv.3 / 0; no image)
    ("BT12-064", &[(Black, 3, 3), (Black, 4, 1)]), // Tuwarmon (not black Lv.4 / 3)
    ("BT13-088", &[(Purple, 5, 3)]), // Belphemon: Sleep Mode (not purple Lv.2 / 0)
    ("BT14-081", &[(Purple, 5, 4)]), // Fenriloogamon (not yellow|black Lv.5 / 4)
    ("BT15-017", &[(Red, 5, 3)]), // Phoenixmon (not purple Lv.5 / 4)
    ("BT15-042", &[(Yellow, 5, 3)]), // Magnadramon (not red Lv.5 / 3)
    ("BT19-007", &[(Red, 2, 0)]), // Guilmon (not red Lv.2 / 2)
    ("BT19-027", &[(Blue, 5, 4)]), // Ryugumon (not purple Lv.5 / 4)
    ("BT19-053", &[(Green, 5, 4), (Black, 5, 4)]), // QueenBeemon (not blue|yellow Lv.5 / 4)
    ("BT20-030", &[(Yellow, 2, 1), (Black, 2, 1)]), // Liollmon (not yellow Lv.3 / 1)
    ("BT21-067", &[(Purple, 3, 2)]), // Garurumon (not purple Lv.3 / 3)
    ("BT23-024", &[(Blue, 5, 4)]), // Poseidomon (not blue Lv.5 / 5)
    ("BT24-070", &[(Purple, 3, 2)]), // Growlmon (not purple Lv.3 / 0)
    ("BT25-079", &[(Purple, 2, 0)]), // Hyemon (not purple Lv.2 / 2)
    ("BT3-055", &[(Green, 4, 3), (Blue, 4, 3)]), // Dinobeemon (added blue Lv.4 / 3)
    ("BT8-013", &[(Red, 3, 2)]), // BetelGammamon (not red Lv.2 / 0)
    ("BT8-064", &[(Black, 3, 2), (Red, 3, 2)]), // Greymon (not black|red Lv.2 / 2)
    ("BT9-023", &[(Blue, 3, 2)]), // KausGammamon (not blue Lv.2 / 0)
    ("BT9-024", &[(Blue, 3, 2)]), // Garurumon (X Antibody) (not blue Lv.4 / 2)
    ("BT9-041", &[(Yellow, 4, 4), (Red, 4, 4)]), // RizeGreymon (X Antibody) (not yellow|red Lv.4 / 0)
    ("EX10-011", &[(Red, 5, 6), (Blue, 5, 6), (Purple, 5, 6)]), // MaloMyotismon (added blue Lv.5 / 6)
    ("EX11-041", &[(Blue, 4, 4), (Black, 4, 4)]), // Oblivimon (not black Lv.4 / 3)
    ("EX11-050", &[(Red, 4, 4), (Purple, 4, 4)]), // Loudmon (not purple Lv.4 / 3)
    ("EX2-044", &[(Purple, 5, 3)]), // Beelzemon (not purple Lv.5 / 4)
    ("EX3-010", &[(Red, 4, 4), (Purple, 4, 4)]), // Paildramon (not black Lv.4 / 4)
    ("EX3-011", &[(Red, 4, 3), (Black, 4, 3)]), // Lavogaritamon (added black Lv.4 / 3; not purple)
    ("EX3-023", &[(Blue, 5, 3)]), // Plesiomon (not blue Lv.4 / 4)
    ("EX4-050", &[(Black, 5, 4), (Yellow, 5, 4)]), // ShadowSeraphimon (not green Lv.5 / 4)
    ("EX6-029", &[(Yellow, 5, 4), (Purple, 5, 4)]), // Mastemon (not yellow|purple Lv.5 / 5)
    ("EX6-062", &[(Yellow, 6, 7), (Green, 6, 7), (Black, 6, 7), (Purple, 6, 7)]), // UltimateChaosmon (added purple)
    ("EX7-030", &[(Yellow, 5, 4)]), // Cendrillmon (not black Lv.5 / 4)
    ("EX7-062", &[(Red, 5, 5), (Purple, 5, 5)]), // HeavyMetaldramon (not green Lv.5 / 5)
    ("EX8-016", &[(Red, 5, 5), (Green, 5, 5)]), // Dinomon (not blue|black Lv.5 / 5)
    ("EX8-027", &[(Blue, 5, 3)]), // Plesiomon (not blue Lv.5 / 4)
    ("EX8-028", &[(Blue, 5, 4), (Yellow, 5, 4)]), // Skadimon (not purple Lv.5 / 4)
    ("EX8-055", &[(Black, 5, 4)]), // Pyramidimon (not red Lv.5 / 4)
    ("EX9-017", &[(Blue, 3, 2)]), // Garurumon (not blue Lv.3 / 0)
    ("EX9-026", &[(Yellow, 3, 2)]), // Angemon (not yellow Lv.3 / 0)
    ("P-067", &[(Blue, 2, 0)]), // Bulucomon (not blue Lv.3 / 0; no image)
    ("RB1-009", &[(Red, 4, 3)]), // Canoweissmon (not red Lv.5 / 3)
    ("RB1-036", &[(Red, 6, 6), (Purple, 6, 6)]), // Proximamon (added: cost 6, not 5)
    ("ST10-04", &[(Yellow, 3, 3), (Purple, 3, 3)]), // Gatomon (not yellow|purple Lv.3 / 0)
    ("ST18-12", &[(Green, 5, 3)]), // Zephagamon (not blue Lv.5 / 3)
];

/// The `cards.json` rows the cards of `FACE_READ_CIRCLES` lost on 2026-11-22:
/// `(card_id, colour, level, cost)`. Each was a live digivolution in real games
/// (production reads `cards.json`); none is printed.
pub const UNPRINTED_ROWS_REMOVED: &[(&str, CardColor, u8, i16)] = &[
    ("BT10-011", Red, 4, 3),
    ("BT10-050", Green, 2, 0),
    ("BT10-066", Black, 5, 4),
    ("BT11-009", Red, 2, 0),
    ("BT11-012", Red, 3, 2),
    ("BT11-019", Blue, 5, 6),
    ("BT11-054", Green, 5, 3),
    ("BT11-054", Blue, 5, 3),
    ("BT11-068", Black, 4, 0),
    ("BT11-068", Green, 4, 0),
    ("BT12-012", Red, 3, 3),
    ("BT12-012", Yellow, 3, 3),
    ("BT12-030", Blue, 4, 3),
    ("BT12-030", Green, 4, 3),
    ("BT12-051", Green, 3, 0),
    ("BT12-061", Black, 3, 0),
    ("BT12-064", Black, 4, 3),
    ("BT13-088", Purple, 2, 0),
    ("BT14-081", Yellow, 5, 4),
    ("BT14-081", Black, 5, 4),
    ("BT15-017", Purple, 5, 4),
    ("BT15-042", Red, 5, 3),
    ("BT19-007", Red, 2, 2),
    ("BT19-027", Purple, 5, 4),
    ("BT19-053", Blue, 5, 4),
    ("BT19-053", Yellow, 5, 4),
    ("BT20-030", Yellow, 3, 1),
    ("BT21-067", Purple, 3, 3),
    ("BT23-024", Blue, 5, 5),
    ("BT24-070", Purple, 3, 0),
    ("BT25-079", Purple, 2, 2),
    ("BT8-013", Red, 2, 0),
    ("BT8-064", Black, 2, 2),
    ("BT8-064", Red, 2, 2),
    ("BT9-023", Blue, 2, 0),
    ("BT9-024", Blue, 4, 2),
    ("BT9-041", Yellow, 4, 0),
    ("BT9-041", Red, 4, 0),
    ("EX11-041", Black, 4, 3),
    ("EX11-050", Purple, 4, 3),
    ("EX2-044", Purple, 5, 4),
    ("EX3-010", Black, 4, 4),
    ("EX3-011", Purple, 4, 3),
    ("EX3-023", Blue, 4, 4),
    ("EX4-050", Green, 5, 4),
    ("EX6-029", Yellow, 5, 5),
    ("EX6-029", Purple, 5, 5),
    ("EX7-030", Black, 5, 4),
    ("EX7-062", Green, 5, 5),
    ("EX8-016", Blue, 5, 5),
    ("EX8-016", Black, 5, 5),
    ("EX8-027", Blue, 5, 4),
    ("EX8-028", Purple, 5, 4),
    ("EX8-055", Red, 5, 4),
    ("EX9-017", Blue, 3, 0),
    ("EX9-026", Yellow, 3, 0),
    ("P-067", Blue, 3, 0),
    ("RB1-009", Red, 5, 3),
    ("RB1-036", Red, 6, 5),
    ("RB1-036", Purple, 6, 5),
    ("ST10-04", Yellow, 3, 0),
    ("ST10-04", Purple, 3, 0),
    ("ST18-12", Blue, 5, 3),
];

/// `FACE_READ_CIRCLES` rows for `card_id` as `(colour code, level, cost)`, or
/// `None` when the card is not in the table.
pub fn face_read_rows(card_id: &str) -> Option<Vec<(u8, u8, i64)>> {
    FACE_READ_CIRCLES
        .iter()
        .find(|(id, _)| *id == card_id)
        .map(|(_, rows)| {
            rows.iter()
                .map(|(colour, level, cost)| (*colour as u8, *level, i64::from(*cost)))
                .collect()
        })
}
