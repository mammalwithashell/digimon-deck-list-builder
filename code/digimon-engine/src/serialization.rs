//! UI-state serialization — builds a `serde_json::Value` tree that
//! matches the dict shape emitted by Python's
//! `digimon_gym/engine/game/serialization.py::to_ui_json`.
//!
//! Consumed by the PyO3 layer (PyDict conversion) and, transitively,
//! by `state_filter.py` and the React frontend.
//!
//! Player IDs are translated to the Python 1/2 convention at this layer
//! so downstream consumers don't need to know about the Rust 0-indexed
//! internal convention.

use serde_json::{json, Map, Value};

use crate::action::space::SECURITY_TARGET;
use crate::card_data::CardData;
use crate::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use crate::game::Game;
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::selection::{AttackTarget, PendingAttack, PendingSelectionView};

/// Translate a Rust `PlayerId` (0 / 1) into the Python 1 / 2 convention.
fn py_pid(rust_pid: u8) -> i64 {
    (rust_pid as i64) + 1
}

/// Map a Rust `GamePhase` variant to Python's `GamePhase.value` integer.
///
/// Python's `GamePhase` enum (`digimon_gym/engine/data/enums.py`):
///   Start=0, Draw=1, Breeding=2, Main=3, End=4,
///   SelectTarget=5, SelectMaterial=6, BlockTiming=7, CounterTiming=8,
///   SelectTrash=9, SelectSource=10, SelectHand=11, SelectReveal=12,
///   SelectEffectChoice=13, SelectSecurity=14, EndOfTurnAction=15,
///   AllianceTiming=16, Mulligan=17
///
/// Rust-only variants with no direct Python equivalent:
///   Unsuspend → treated as Start (0); an Unsuspend phase is never serialised
///               in practice because Rust auto-resolves it without pausing.
///   GameOver  → reused from Python's End (4) since the game is over either way.
fn phase_int(p: GamePhase) -> i64 {
    match p {
        GamePhase::Mulligan => 17,
        GamePhase::Unsuspend => 0, // no Python equivalent; Start=0 is nearest
        GamePhase::Draw => 1,
        GamePhase::Breeding => 2,
        GamePhase::Main => 3,
        GamePhase::EndTurn => 4,
        GamePhase::SelectTarget => 5,
        GamePhase::SelectMaterial => 6,
        GamePhase::SelectTrash => 9,
        GamePhase::SelectSource => 10,
        GamePhase::SelectHand => 11,
        GamePhase::SelectReveal => 12,
        GamePhase::SelectSecurity => 14,
        GamePhase::EffectChoice => 13,
        GamePhase::BlockTiming => 7,
        GamePhase::CounterTiming => 8,
        GamePhase::AllianceTiming => 16,
        GamePhase::EndOfTurnAction => 15,
        GamePhase::GameOver => 4, // no Python equivalent; End=4 is nearest
        // Phase 4 variants — no Python equivalent yet; reuse SelectTarget (5)
        // as a placeholder. Tasks 2-5 will add proper Python-side values.
        GamePhase::SelectUnion => 5,
        GamePhase::SelectPermutation => 5,
        GamePhase::SelectBudgeted => 5,
        GamePhase::SelectBreedingPermanent => 5,
        // BO3 match training — no Python equivalent. Placeholder under
        // SelectEffectChoice (13) since play-order is a binary effect-choice.
        GamePhase::SelectPlayOrder => 13,
    }
}

/// Build the player-level dict. Matches `player_ui_data()` in Python.
fn player_ui_data(player: &Player, data: &[CardData], game: &Game) -> Value {
    let hand_ids: Vec<&str> = player.hand.iter().map(|c| c.card_id(data)).collect();

    let hand_cards: Vec<Value> = player
        .hand
        .iter()
        .map(|cs| {
            let cd = &data[cs.data_index];
            json!({
                "cardId": cs.card_id(data),
                "cardName": cd.card_name,
                "playCost": cd.play_cost,
                "level": cd.level,
                "dp": cd.dp,
                "colors": colors_of(cd),
                "cardKind": kind_int(cd),
                "evoCosts": evo_costs_of(cd),
            })
        })
        .collect();

    // Security IDs: null for face-down cards, card_id string for face-up.
    // `face_up_security` contains `card_index` values (u16) of revealed cards.
    let security_ids: Vec<Value> = player
        .security
        .iter()
        .map(|cs| {
            if player.face_up_security.contains(&cs.card_index) {
                json!(cs.card_id(data))
            } else {
                Value::Null
            }
        })
        .collect();

    let security_face_up: Vec<bool> = player
        .security
        .iter()
        .map(|cs| player.face_up_security.contains(&cs.card_index))
        .collect();

    let battle_area: Vec<Value> = player
        .battle_area
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let handle = PermanentHandle {
                player: player.id,
                index: i as u8,
            };
            perm_data(p, data, game, Some(handle))
        })
        .collect();

    // Breeding-area permanents are not battle-area-addressable, so live
    // keyword/modifier/DP queries (which key off a `PermanentHandle` into
    // `battle_area`) don't apply — pass `None` to fall back to printed state.
    let breeding_area = player
        .breeding_area
        .as_ref()
        .map(|p| perm_data(p, data, game, None))
        .unwrap_or(Value::Null);

    let trash_ids: Vec<&str> = player.trash.iter().map(|c| c.card_id(data)).collect();

    // Player-relative memory: turn player sees +gauge, opponent sees -gauge.
    // Matches Python's `game._get_memory_for(p)` in serialization.py:300.
    let memory: i64 = if player.id == game.turn_player() {
        game.memory as i64
    } else {
        -(game.memory as i64)
    };

    json!({
        "id": py_pid(player.id),
        "memory": memory,
        "handCount": player.hand.len(),
        "handIds": hand_ids,
        "handCards": hand_cards,
        "securityCount": player.security.len(),
        "securityIds": security_ids,
        "securityFaceUp": security_face_up,
        "deckCount": player.deck.len(),
        "eggDeckCount": player.digitama_deck.len(),
        "battleAreaCount": player.battle_area.len(),
        "battleArea": battle_area,
        "breedingArea": breeding_area,
        "trashIds": trash_ids,
    })
}

fn colors_of(cd: &CardData) -> Vec<i64> {
    cd.colors.iter().map(|c| color_to_python_int(*c)).collect()
}

/// Map Rust `CardColor` → Python `CardColor` enum int value. Hand-written
/// because the two enums have different declaration orders.
///
/// Python (`digimon_gym/engine/data/enums.py`):
///   Red=0, Blue=1, Yellow=2, Green=3, White=4, Black=5, Purple=6
///
/// Rust (`digimon-engine/src/enums.rs`):
///   Red=0, Blue=1, Yellow=2, Green=3, Black=4, Purple=5, White=6
///
/// White, Black, and Purple are in different positions, so `as u8` gives
/// wrong values for those three. This mapping must be kept in sync with
/// Python's `CardColor` enum.
fn color_to_python_int(c: CardColor) -> i64 {
    match c {
        CardColor::Red => 0,
        CardColor::Blue => 1,
        CardColor::Yellow => 2,
        CardColor::Green => 3,
        CardColor::White => 4,
        CardColor::Black => 5,
        CardColor::Purple => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_to_python_int_mapping() {
        assert_eq!(color_to_python_int(CardColor::Red), 0);
        assert_eq!(color_to_python_int(CardColor::Blue), 1);
        assert_eq!(color_to_python_int(CardColor::Yellow), 2);
        assert_eq!(color_to_python_int(CardColor::Green), 3);
        assert_eq!(color_to_python_int(CardColor::White), 4);
        assert_eq!(color_to_python_int(CardColor::Black), 5);
        assert_eq!(color_to_python_int(CardColor::Purple), 6);
    }
}

fn kind_int(cd: &CardData) -> i64 {
    // Match Python CardKind int values: 0=Digimon, 1=Tamer, 2=Option, 3=DigiEgg.
    // Token and Dual field permanents serialize as Digimon (0) for Python-parity.
    match cd.card_kind {
        CardKind::Digimon => 0,
        CardKind::Tamer => 1,
        CardKind::Option => 2,
        CardKind::DigiEgg => 3,
        CardKind::Token => 0, // treated as Digimon
        CardKind::Dual => 0,  // field face is Digimon
    }
}

fn evo_costs_of(cd: &CardData) -> Vec<Value> {
    // EvoCost fields: card_color: u8, level: u8, memory_cost: u16.
    // Key names match Python's serialization.py:313 — {"color", "level", "cost"}.
    // card_color is a raw u8 (same integer value Python's enum.value emits).
    cd.evo_costs
        .iter()
        .map(|ec| {
            json!({
                "color": ec.card_color,
                "level": ec.level,
                "cost": ec.memory_cost,
            })
        })
        .collect()
}

/// Non-parameterised keywords surfaced as inspector "chips", paired with the
/// snake_case wire string the frontend's `KEYWORD_DISPLAY`/`KEYWORD_COLORS`
/// maps key on. Parameterised keywords (`SecurityAttackPlus`, `DeDigivolve`,
/// `DrawX`, `MaterialSave`, `DigiBurst`, `Fragment`, `Decoy`) are conveyed via
/// `securityAttackModifier` and printed effect text rather than as chips.
pub const DISPLAY_KEYWORDS: &[(Keyword, &str)] = &[
    (Keyword::Blocker, "blocker"),
    (Keyword::Rush, "rush"),
    (Keyword::Jamming, "jamming"),
    (Keyword::Piercing, "piercing"),
    (Keyword::Reboot, "reboot"),
    (Keyword::Blitz, "blitz"),
    (Keyword::Raid, "raid"),
    (Keyword::Alliance, "alliance"),
    (Keyword::BlastDigivolve, "blast_digivolve"),
    (Keyword::Save, "save"),
    (Keyword::Fortitude, "fortitude"),
    (Keyword::Overclock, "overclock"),
    (Keyword::Barrier, "barrier"),
    (Keyword::Partition, "partition"),
    (Keyword::Vortex, "vortex"),
    (Keyword::Collision, "collision"),
    (Keyword::Evade, "evade"),
    (Keyword::Decode, "decode"),
    (Keyword::ArmorPurge, "armor_purge"),
    (Keyword::Progress, "progress"),
    (Keyword::Retaliation, "retaliation"),
    (Keyword::Scapegoat, "scapegoat"),
    (Keyword::Execute, "execute"),
    (Keyword::Engage, "engage"),
    (Keyword::Guard, "guard"),
    (Keyword::Iceclad, "iceclad"),
    (Keyword::MindLink, "mind_link"),
    (Keyword::Training, "training"),
    (Keyword::ArtsDigivolve, "arts_digivolve"),
    (Keyword::Ascension, "ascension"),
];

/// Stable wire string for a display-relevant, permanent-scoped `ModifierType`.
/// Returns `None` for modifiers that are internal/bookkeeping or player-scoped
/// (those are not buffs on a single permanent and are not emitted). Uses an
/// explicit match — NOT `{:?}` — so the UI contract survives enum refactors.
pub fn modifier_type_str(m: ModifierType) -> Option<&'static str> {
    Some(match m {
        // Stat changes
        ModifierType::ChangeDp => "ChangeDp",
        ModifierType::ChangeBaseDp => "ChangeBaseDp",
        ModifierType::DpFloor => "DpFloor",
        ModifierType::DontHaveDp => "DontHaveDp",
        ModifierType::SecurityAttackChange => "SecurityAttackChange",
        ModifierType::ChangeLevel => "ChangeLevel",
        ModifierType::ChangeColor => "ChangeColor",
        ModifierType::AddColor => "AddColor",
        // Immunities / protection
        ModifierType::CannotBeDestroyed => "CannotBeDestroyed",
        ModifierType::CannotBeDestroyedByBattle => "CannotBeDestroyedByBattle",
        ModifierType::CannotBeDestroyedByEffect => "CannotBeDestroyedByEffect",
        ModifierType::CannotBeRemoved => "CannotBeRemoved",
        ModifierType::CannotBeReturnedToDeck => "CannotBeReturnedToDeck",
        ModifierType::CannotBeReturnedToHand => "CannotBeReturnedToHand",
        ModifierType::CannotBeTrashedByEffect => "CannotBeTrashedByEffect",
        ModifierType::CannotBeDeDigivolved => "CannotBeDeDigivolved",
        ModifierType::CannotBeSelectedByEffect => "CannotBeSelectedByEffect",
        ModifierType::CannotBeAffected => "CannotBeAffected",
        ModifierType::ImmunityToOpponentEffects => "ImmunityToOpponentEffects",
        ModifierType::ImmuneFromDPMinus => "ImmuneFromDPMinus",
        // Attack / suspend / block restrictions
        ModifierType::CannotAttack => "CannotAttack",
        ModifierType::CannotAttackPlayer => "CannotAttackPlayer",
        ModifierType::CannotBlock => "CannotBlock",
        ModifierType::CannotCounter => "CannotCounter",
        ModifierType::CannotSuspend => "CannotSuspend",
        ModifierType::CannotUnsuspend => "CannotUnsuspend",
        ModifierType::CannotDigivolve => "CannotDigivolve",
        // Effect-activation restrictions
        ModifierType::CannotActivateOnPlayEffects => "CannotActivateOnPlayEffects",
        ModifierType::CannotActivateMainEffects => "CannotActivateMainEffects",
        ModifierType::CannotActivateWhenDigivolvingEffects => {
            "CannotActivateWhenDigivolvingEffects"
        }
        ModifierType::CannotActivateWhenAttackingEffects => "CannotActivateWhenAttackingEffects",
        ModifierType::CannotActivateSecurityEffects => "CannotActivateSecurityEffects",
        // Attack grants / mandates
        ModifierType::MayAttack => "MayAttack",
        ModifierType::ForceAttack => "ForceAttack",
        // NOTE: granted keywords (`GrantBlocker`, etc.) are intentionally NOT
        // listed here. They are stored in a separate `permanent_keywords` store
        // (via `grant_keyword`), so they never appear in `permanent_modifiers`;
        // they surface to the UI through `keywordBreakdown.gained` instead.
        //
        // Everything else (player-scoped flood gates, internal bookkeeping) is
        // not a per-permanent buff and is intentionally not surfaced.
        _ => return None,
    })
}

/// Stable wire string for a modifier's `Expiry` (when it wears off).
pub fn expiry_str(e: Expiry) -> &'static str {
    match e {
        Expiry::Permanent => "Permanent",
        Expiry::EndOfTurn => "EndOfTurn",
        Expiry::EndOfOpponentsTurn => "EndOfOpponentsTurn",
        Expiry::EndOfYourTurn => "EndOfYourTurn",
        Expiry::EndOfOpponentsNextTurn => "EndOfOpponentsNextTurn",
        Expiry::EndOfYourNextTurn => "EndOfYourNextTurn",
        Expiry::EndOfAttack => "EndOfAttack",
        Expiry::EndOfBattle => "EndOfBattle",
        Expiry::UntilLeaveField => "UntilLeaveField",
        Expiry::UntilCondition => "UntilCondition",
        Expiry::OnceUsed(_) => "OnceUsed",
    }
}

/// Per-permanent dict for `to_ui_json`. When `handle` is `Some` (battle-area
/// permanent) the runtime fields — keywords, keyword breakdown, security-attack
/// modifier, effective DP, and the active-modifier list — are computed from
/// live engine state. When `handle` is `None` (breeding area) those fields fall
/// back to printed/neutral values, since breeding permanents are not
/// battle-area-addressable for the live queries.
fn perm_data(
    perm: &Permanent,
    data: &[CardData],
    game: &Game,
    handle: Option<PermanentHandle>,
) -> Value {
    let top = perm.top_card();
    let top_data = &data[top.data_index];
    let base_dp = top_data.dp.unwrap_or(0);
    let level = top_data.level.unwrap_or(0);
    let colors = colors_of(top_data);
    let stack_size = perm.card_sources.len();

    // ── Active keywords + innate/gained breakdown ──
    // "gained" = present via a registry grant modifier; "innate" = present
    // via the cards themselves (printed face / inherited from sources).
    let mut keywords: Vec<&str> = Vec::new();
    let mut innate: Vec<&str> = Vec::new();
    let mut gained: Vec<&str> = Vec::new();
    if let Some(h) = handle {
        for (kw, name) in DISPLAY_KEYWORDS {
            if game.has_keyword(h, *kw) {
                keywords.push(name);
                if game.modifiers.has_keyword(h, *kw) {
                    gained.push(name);
                } else {
                    innate.push(name);
                }
            }
        }
    }

    // ── Security-attack modifier (delta from the default of 1) ──
    // Mirror combat's effective strike (`Game::effective_security_strike`),
    // which is BASE-INCLUSIVE of the `security_attack_fn` formula aura
    // (e.g. WarGreymon's `1 + floor(materials/2)`). The earlier display
    // computation summed only the flat `<Security A.>` keyword + modifier
    // deltas and dropped the formula, so a WarGreymon that actually checks
    // 4 security rendered as "2". The total minus the implicit base of 1 is
    // the delta the UI adds back.
    let security_attack_modifier = handle
        .map(|h| game.effective_security_strike(h) - 1)
        .unwrap_or(0);

    // ── DP: base (printed) vs effective (with modifiers) ──
    let total_dp = handle.and_then(|h| game.effective_dp(h)).unwrap_or(base_dp);
    let temporary_dp = total_dp - base_dp;

    // ── Stack sources with printed effect text ──
    let sources: Vec<Value> = perm
        .card_sources
        .iter()
        .enumerate()
        .map(|(i, cs)| {
            let cd = &data[cs.data_index];
            json!({
                "cardId": cs.card_id(data),
                "cardName": cd.card_name,
                "isTop": i + 1 == stack_size,
                "optState": 0.0,
                "dpContribution": 0,
                "mainEffectText": cd.effect_text,
                "inheritedEffectText": cd.inherited_text,
                "colors": colors_of(cd),
            })
        })
        .collect();

    // ── Inherited effects conferred by non-top digivolution sources ──
    let inherited_effects: Vec<Value> = perm
        .card_sources
        .iter()
        .enumerate()
        .filter(|(i, _)| i + 1 < stack_size) // exclude the top card
        .filter_map(|(i, cs)| {
            let cd = &data[cs.data_index];
            if cd.inherited_text.trim().is_empty() {
                return None;
            }
            Some(json!({
                "sourceIndex": i,
                "cardId": cs.card_id(data),
                "cardName": cd.card_name,
                "text": cd.inherited_text,
            }))
        })
        .collect();

    // ── Active modifier list (DCGO PermanentDetail parity) ──
    let modifiers: Vec<Value> = handle
        .map(|h| {
            game.modifiers
                .permanent_modifiers_iter(h)
                .filter_map(|entry| {
                    let type_str = modifier_type_str(entry.modifier)?;
                    let source_card_id = entry.source_permanent.and_then(|sh| {
                        game.players
                            .get(sh.player as usize)
                            .and_then(|pl| pl.battle_area.get(sh.index as usize))
                            .map(|p| p.top_card().card_id(data).to_string())
                    });
                    Some(json!({
                        "type": type_str,
                        "value": entry.value,
                        "expiry": expiry_str(entry.expiry),
                        "sourceCardId": source_card_id,
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    let linked_card_ids: Vec<&str> = perm.linked_cards.iter().map(|c| c.card_id(data)).collect();

    json!({
        "topCardId": top.card_id(data),
        "topCardName": top_data.card_name,
        "dp": base_dp,
        "level": level,
        "isSuspended": perm.is_suspended,
        "sourceCount": stack_size,
        "keywords": keywords,
        "keywordBreakdown": json!({ "innate": innate, "gained": gained }),
        "securityAttackModifier": security_attack_modifier,
        "linkedCardIds": linked_card_ids,
        "sources": sources,
        "mainEffectText": top_data.effect_text,
        "inheritedEffects": inherited_effects,
        "modifiers": modifiers,
        "dpBreakdown": json!({
            "base": base_dp,
            "sources": [],
            "temporary": temporary_dp,
            "aura": 0,
            "total": total_dp,
        }),
        "turnPlayed": perm.turn_played,
        "colors": colors,
    })
}

/// Build the pendingSelection dict from a resolved `PendingSelectionView`.
///
/// Extracted as a private helper symmetric with `player_ui_data` / `perm_data`
/// so it is unit-testable and keeps `to_ui_json` lean.
///
/// "kind" is a deliberate Rust-additive key not present in Python's
/// serialization.py:338 output — it surfaces the SelectionKind variant string
/// so typed WebSocket/UI consumers can route selection prompts without
/// re-deriving the kind from phase + validIndices alone.
fn pending_selection_data(v: &PendingSelectionView, game: &Game) -> Value {
    let mut m = Map::new();
    m.insert("kind".into(), Value::String(v.kind_str()));
    // phase: int matching Python's GamePhase.value (not a debug string).
    m.insert("phase".into(), Value::from(phase_int(v.previous_phase)));
    m.insert(
        "selectingPlayer".into(),
        Value::from(py_pid(v.selecting_player)),
    );
    // Whose zone Hand/Trash valid indices refer to (e.g. EX11-012 picks from
    // the OPPONENT's trash). Absent/null = the selecting player's own zone.
    if let Some(owner) = v.zone_owner {
        m.insert("zoneOwner".into(), Value::from(py_pid(owner)));
    }
    m.insert(
        "validIndices".into(),
        Value::Array(v.valid_action_ids.iter().map(|i| json!(*i)).collect()),
    );
    m.insert("isOptional".into(), Value::from(v.is_optional));
    m.insert("prompt".into(), Value::String(v.prompt.clone()));
    if let Some(choices) = v.effect_choices.as_ref() {
        m.insert(
            "effectChoices".into(),
            Value::Array(
                choices
                    .iter()
                    .map(|c| {
                        // Resolve the branch's source card so the chooser can
                        // render the actual card face instead of a blank box
                        // (mirrors the desktop wire's EffectChoiceDto).
                        let card = c.source_card.map(|h| game.card(h));
                        json!({
                            "label": c.label,
                            "actionId": c.action_id,
                            "cardId": card.map(|cd| cd.card_id.clone()),
                            "cardName": card.map(|cd| cd.card_name.clone()),
                        })
                    })
                    .collect(),
            ),
        );
    }
    Value::Object(m)
}

/// Build the pendingAttack dict.
///
/// Shape matches Python's serialization.py:370-373:
///   attacker_slot: index of the attacker in the turn-player's battle_area.
///   target_slot: index in enemy battle_area (Digimon target) or
///     SECURITY_TARGET for a direct/player attack.
fn pending_attack_data(pa: &PendingAttack, _game: &Game) -> Value {
    let attacker_slot = pa.attacker.index as i64;
    let target_slot: i64 = match pa.effective_target {
        AttackTarget::Digimon(ph) => ph.index as i64,
        AttackTarget::Player(_) => SECURITY_TARGET as i64,
    };
    json!({
        "attackerSlot": attacker_slot,
        "targetSlot": target_slot,
    })
}

/// Build the full UI-state dict. `state_filter.py` consumes this directly.
pub fn to_ui_json(game: &Game) -> Value {
    let pending_sel_value = game
        .pending_selection
        .as_ref()
        .map(|s| pending_selection_data(&s.view(), game))
        .unwrap_or(Value::Null);

    let pending_attack = game
        .pending_attack
        .as_ref()
        .map(|pa| pending_attack_data(pa, game))
        .unwrap_or(Value::Null);

    let revealed: Vec<Value> = game
        .revealed_cards
        .iter()
        .map(|cs| json!({"cardId": cs.card_id(&game.card_data), "owner": py_pid(cs.owner)}))
        .collect();

    // memoryGauge: from player1's perspective. Matches Python's
    // `game._get_memory_for(game.player1)` at serialization.py:379.
    // Positive when player1 is the turn player, negative otherwise.
    let memory_gauge: i64 = if game.players[0].id == game.turn_player() {
        game.memory as i64
    } else {
        -(game.memory as i64)
    };

    json!({
        "turnCount": game.turn_count,
        "currentPhase": phase_int(game.current_phase),
        "currentPlayer": py_pid(game.turn_player()),
        "memoryGauge": memory_gauge,
        "isGameOver": game.game_over,
        "winner": game.winner.map(py_pid),
        "player1": player_ui_data(&game.players[0], &game.card_data, game),
        "player2": player_ui_data(&game.players[1], &game.card_data, game),
        "revealedCards": revealed,
        "pendingSelection": pending_sel_value,
        "pendingAttack": pending_attack,
    })
}
