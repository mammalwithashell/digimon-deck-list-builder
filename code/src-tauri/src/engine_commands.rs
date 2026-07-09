//! Tauri commands that expose the Rust `digimon-engine` directly to the
//! frontend. These are the sole gameplay backend on desktop — there is no
//! Python sidecar; ONNX inference runs in-process via `InferenceState`.
//! Response shapes mirror the hosted API's `/games/*` router so the web
//! and desktop frontends share one set of TypeScript types.
//!
//! **Threading model (see the `desktop-engine-worker-thread` change).** All
//! engine work runs on a single dedicated worker thread spawned with an
//! explicit 64 MB stack — large enough to absorb the bounded recursive
//! `bincode` deserialization of the `CompiledStep` card pack that overflowed
//! Tauri's 1 MB UI main thread in debug builds. The worker is the SOLE owner
//! of an [`EngineWorld`] (`game` + `session` + ONNX `inference`); there is no
//! `Arc<Mutex<Game>>` shared across threads. Commands are `async fn`s that
//! capture their (owned) args, dispatch a boxed closure to the worker via
//! [`EngineHandle::run`], and await the reply over a `oneshot` channel — so
//! the UI event loop is never blocked while the worker runs (e.g. during a
//! long bot turn). The external command contract (names, args, DTOs, errors)
//! is byte-identical to the previous synchronous version.
//!
//! The frontend calls `create_test_game` first, then uses `get_state` /
//! `play_card` / `attack_digimon` / `attack_player` / `end_turn` to drive it.

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;

use digimon_engine::action::build_action_mask;
use digimon_engine::action::explain::{explain_action, legal_decoded_actions, ActionExplanation};
use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::combat::AttackResult;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::rules::Rules;
use digimon_engine::tensor::build_tensor;
use digimon_engine::tensor_profiles::default_profile;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::inference_state::InferenceState;

/// Per-player game configuration — which kind of decider drives each seat,
/// and (for `Trained`) which loaded model to consult.
#[derive(Default)]
pub struct GameSession {
    pub registry: Option<CardRegistry>,
    pub player_kinds: Vec<PlayerKind>,
    pub player_model_ids: Vec<Option<String>>,
}

// ─── Engine worker thread ─────────────────────────────────────────────

/// The single owner of all desktop engine state. Lives entirely on the
/// `digimon-engine` worker thread — never shared across threads, so the
/// `Game`, its session metadata, and the ONNX `inference` sessions need no
/// `Send`/`Sync` plumbing. Constructed once, inside the worker closure, via
/// `EngineWorld::default()`.
#[derive(Default)]
pub struct EngineWorld {
    pub game: Option<Game>,
    pub session: GameSession,
    pub inference: InferenceState,
}

/// A unit of engine work: a boxed closure run on the worker against the
/// world. `Send` so it can cross the channel; `'static` so it can outlive
/// the dispatching command's stack frame.
type EngineJob = Box<dyn FnOnce(&mut EngineWorld) + Send + 'static>;

/// Tauri-managed handle to the engine worker. Cheap to clone (just an
/// `UnboundedSender`); the dev-only debug bridge holds its own clone.
#[derive(Clone)]
pub struct EngineHandle {
    tx: mpsc::UnboundedSender<EngineJob>,
}

impl EngineHandle {
    /// Spawn the engine worker thread and return a handle to it.
    ///
    /// The thread is created with an explicit 64 MB stack — far beyond the
    /// engine's bounded recursion (notably `bincode`-deserializing the
    /// recursive `CompiledStep` card pack) that overflows Tauri's 1 MB UI
    /// main thread in debug builds. The worker owns the [`EngineWorld`] for
    /// its entire lifetime and services jobs serially. Each job is wrapped in
    /// `catch_unwind` so a panic in one unit of work doesn't tear down the
    /// thread; subsequent commands still work (the panicked job's awaiting
    /// caller observes a dropped reply and returns `Err`).
    pub fn spawn() -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<EngineJob>();
        std::thread::Builder::new()
            .name("digimon-engine".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                let mut world = EngineWorld::default();
                while let Some(job) = rx.blocking_recv() {
                    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| job(&mut world)));
                }
            })
            .expect("failed to spawn digimon-engine worker thread");
        Self { tx }
    }

    /// Run `f` on the engine worker and await its result without blocking the
    /// calling (UI/event-loop) thread. `send` failure → the worker has
    /// stopped; a dropped reply → the job panicked mid-flight (D7).
    pub async fn run<R, F>(&self, f: F) -> Result<R, String>
    where
        R: Send + 'static,
        F: FnOnce(&mut EngineWorld) -> R + Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel::<R>();
        self.tx
            .send(Box::new(move |w| {
                let _ = reply_tx.send(f(w));
            }))
            .map_err(|_| "engine worker stopped".to_string())?;
        reply_rx
            .await
            .map_err(|_| "engine worker did not reply".to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlayerKind {
    Human,
    Greedy,
    Trained,
}

impl Default for PlayerKind {
    fn default() -> Self {
        PlayerKind::Human
    }
}

// ─── DTOs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDto {
    pub card_id: String,
    pub card_name: String,
    pub card_kind: String,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub play_cost: u16,
    pub colors: Vec<String>,
    /// ACE printed-cost overflow (Track C synth profile). `None` for non-ACE
    /// cards. Frontend chooses whether to render an ACE chip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ace_overflow: Option<i32>,
    /// DigiXros alternate names parsed from effect text. Empty for cards
    /// without DigiXros aliases. Frontend uses these for material-match hints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub digixros_aliases: Vec<String>,
    /// Printed digivolution costs (color/level/cost rows). The desktop UI
    /// previously had no digivolve-cost data; the browser path exposes these.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evo_costs: Vec<EvoCostDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermanentDto {
    pub field_index: u8,
    pub top_card: CardDto,
    pub effective_dp: Option<i32>,
    pub is_suspended: bool,
    pub stack_size: usize,
    pub turn_played: u16,
    /// The digivolution-source cards UNDER the active top card, ordered
    /// bottom→top (the top/active card is excluded). Index `i` here lines up
    /// with the engine's `source_index` in `encode_source_select(field, i)`
    /// (= `SOURCE_SELECT_START + field*SOURCES_PER_FIELD + i`), so the
    /// frontend's source picker can map a `SOURCE_SELECT`-range action id back
    /// to the card it removes/plays. Empty when the permanent has no sources.
    #[serde(default)]
    pub sources: Vec<SourceCardDto>,
    /// Active keyword display names (mirrors `serialization::perm_data`'s
    /// `keywords`). Empty for the breeding-area construction (no live handle).
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Innate vs modifier-gained keyword split.
    #[serde(default)]
    pub keyword_breakdown: KeywordBreakdownDto,
    /// Security-attack modifier (delta from the default of 1).
    #[serde(default)]
    pub security_attack_modifier: i32,
    /// Linked-card ids (e.g. for cards with a linked-Digimon mechanic).
    #[serde(default)]
    pub linked_card_ids: Vec<String>,
    /// The top card's printed main effect text.
    #[serde(default)]
    pub main_effect_text: String,
    /// Inherited effects conferred by non-top digivolution sources.
    #[serde(default)]
    pub inherited_effects: Vec<InheritedEffectDto>,
    /// Active modifiers on this permanent (DCGO PermanentDetail parity).
    #[serde(default)]
    pub modifiers: Vec<PermanentModifierDto>,
    /// Printed (base) vs effective DP split.
    #[serde(default)]
    pub dp_breakdown: DpBreakdownDto,
}

/// A single digivolution-source card under a permanent. Minimal shape for the
/// source picker (and stack inspector); richer per-source detail for the
/// inspector lives in the engine's own serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCardDto {
    pub card_id: String,
    pub card_name: String,
    pub colors: Vec<String>,
    /// This source card's printed main effect text (for the stack inspector).
    #[serde(default)]
    pub main_effect_text: String,
    /// This source card's printed inherited effect text.
    #[serde(default)]
    pub inherited_effect_text: String,
}

/// A single digivolution cost entry (one color/level/cost row from
/// `CardData.evo_costs`). Mirrors `EvoCost` (`card_data.rs`) with the
/// frontend's field names (`color`/`level`/`cost`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvoCostDto {
    pub color: u8,
    pub level: u8,
    pub cost: u16,
}

/// Innate vs modifier-gained keyword split (mirrors `serialization::perm_data`'s
/// `keywordBreakdown`). "innate" = printed/inherited on the cards; "gained" =
/// present via a runtime grant modifier.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeywordBreakdownDto {
    pub innate: Vec<String>,
    pub gained: Vec<String>,
}

/// One inherited effect conferred by a non-top digivolution source (mirrors
/// `serialization::perm_data`'s `inheritedEffects`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritedEffectDto {
    pub source_index: usize,
    pub card_id: String,
    pub card_name: String,
    pub text: String,
}

/// One active modifier on a permanent (mirrors `serialization::perm_data`'s
/// `modifiers` — DCGO PermanentDetail parity). `type_` is the stable wire
/// string from `serialization::modifier_type_str`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermanentModifierDto {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: i32,
    pub expiry: String,
    pub source_card_id: Option<String>,
}

/// Printed (base) vs effective DP split (mirrors `serialization::perm_data`'s
/// `dpBreakdown`). `temporary` is the delta contributed by modifiers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DpBreakdownDto {
    pub base: Option<i32>,
    pub temporary: i32,
    pub total: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDto {
    pub id: PlayerId,
    pub hand: Vec<CardDto>,
    pub battle_area: Vec<PermanentDto>,
    pub breeding: Option<PermanentDto>,
    pub deck_count: usize,
    /// Number of cards remaining in the digitama (egg) deck. The desktop UI
    /// previously had no egg-deck count.
    #[serde(default)]
    pub egg_deck_count: usize,
    pub trash_count: usize,
    /// Full trash contents (top-of-pile last). The trash is a PUBLIC zone in
    /// the Digimon TCG — both players see both trashes — so the cards are sent
    /// for every player. The desktop UI previously got only `trash_count`, so
    /// the trash always rendered empty even as deletions piled up.
    pub trash: Vec<CardDto>,
    pub security_count: usize,
    pub is_eliminated: bool,
}

/// Pure-data view of `game.pending_selection` for the frontend. Without
/// this, any human-driven effect that requires a reveal-pick, target-pick,
/// material-pick, or effect-choice would park the engine with no UI ever
/// prompting the user. Mirrors the Python hosted-API's `pendingSelection`
/// dict shape (see `digimon_gym/engine/game/serialization.py:338`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSelectionDto {
    /// Game phase the engine is parked in (`SelectReveal`, `SelectTarget`, …).
    pub phase: String,
    pub selecting_player: PlayerId,
    /// Every action ID the decoder will accept for this selection.
    pub valid_action_ids: Vec<u16>,
    pub is_optional: bool,
    pub prompt: String,
    /// Engine `SelectionKind` discriminant as a stable string (e.g.
    /// `"OwnField"` / `"OppField"` / `"Hand"`, via `PendingSelection::
    /// kind_str()`). The frontend's board-click router keys off this to map a
    /// slot click to a field-target action — WITHOUT it (the prior desktop DTO
    /// omitted it) every own- AND opponent-field selection is unclickable in
    /// the Tauri app, because `isFieldSelectionKind(undefined)` is false.
    pub kind: String,
    /// For `Hand`/`Trash` kinds, the player whose zone `valid_action_ids`
    /// index into — effects like EX11-012 Medusamon prompt the selecting
    /// player to pick from the OPPONENT's trash. `None` = the selecting
    /// player's own zone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_owner: Option<PlayerId>,
    /// For `SelectionKind::EffectChoice` prompts (`select_effect_choice`),
    /// the branches the player picks among. Each entry's `action_id` is one
    /// of the `valid_action_ids` above; the engine uses the
    /// `HAND_EFFECT_START`-based range (30+) which the frontend can't
    /// derive without this list because the same action IDs are reused
    /// across reveal/effect-choice phases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effect_choices: Vec<EffectChoiceDto>,
}

/// One branch of an `EffectChoice` prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectChoiceDto {
    /// Position in the `effect_choices` array — frontend uses this as a
    /// stable index key.
    pub index: u8,
    /// Human-readable label (e.g. "Hatch", "Don't hatch").
    pub label: String,
    /// Concrete action ID the engine accepts for this branch.
    pub action_id: u16,
    /// Card whose effect this branch resolves (e.g. the digivolution source
    /// granting an inherited trigger). The trigger-order chooser renders this
    /// card's image; without it the UI shows a blank placeholder.
    #[serde(default)]
    pub card_id: Option<String>,
    #[serde(default)]
    pub card_name: Option<String>,
}

/// One card from `game.revealed_cards`. Effects like "reveal X cards" push
/// here; the frontend's `RevealedCardsZone` reads it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealedCardDto {
    pub card_id: String,
    pub owner: PlayerId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateDto {
    pub turn_count: u16,
    pub turn_player: PlayerId,
    /// The seat that takes the first turn (`turn_order[0]`). Stable from game
    /// creation, so the mulligan UI can tell the player whether they go first
    /// or second. The local human is always seat 0 in a desktop game.
    pub first_player: PlayerId,
    pub current_phase: String,
    pub memory: i16,
    pub game_over: bool,
    pub winner: Option<PlayerId>,
    pub players: Vec<PlayerDto>,
    /// The player expected to make the next mulligan decision. `None` once
    /// mulligan is finalized (i.e., during every normal phase of play).
    pub mulligan_current_player: Option<PlayerId>,
    /// Whether each player has used their one re-draw. Indexed by player id.
    /// During mulligan, the UI hides the "Mulligan" button for a player whose
    /// entry here is `true`.
    pub mulligan_used: Vec<bool>,
    /// `Some(...)` whenever the engine is parked on a human-driven choice.
    /// React's `SelectionPanel` / `PromptBar` / `KeywordPromptDialog` consume
    /// this; without it those components render nothing and the user is stuck.
    #[serde(default)]
    pub pending_selection: Option<PendingSelectionDto>,
    /// Cards revealed during the most recent effect resolution. The
    /// `RevealedCardsZone` overlay renders these for the active player.
    #[serde(default)]
    pub revealed_cards: Vec<RevealedCardDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackResultDto {
    pub result: String,
    pub state: GameStateDto,
}

// ─── DTO builders ─────────────────────────────────────────────────────

fn card_kind_str(k: CardKind) -> &'static str {
    match k {
        CardKind::Digimon => "Digimon",
        CardKind::Tamer => "Tamer",
        CardKind::Option => "Option",
        CardKind::DigiEgg => "DigiEgg",
        CardKind::Token => "Token",
        CardKind::Dual => "Dual",
    }
}

fn color_str(c: CardColor) -> &'static str {
    match c {
        CardColor::Red => "Red",
        CardColor::Blue => "Blue",
        CardColor::Yellow => "Yellow",
        CardColor::Green => "Green",
        CardColor::Black => "Black",
        CardColor::Purple => "Purple",
        CardColor::White => "White",
    }
}

fn phase_str(p: GamePhase) -> &'static str {
    match p {
        GamePhase::Mulligan => "Mulligan",
        GamePhase::Unsuspend => "Unsuspend",
        GamePhase::Draw => "Draw",
        GamePhase::Breeding => "Breeding",
        GamePhase::Main => "Main",
        GamePhase::EndTurn => "EndTurn",
        GamePhase::SelectTarget => "SelectTarget",
        GamePhase::SelectMaterial => "SelectMaterial",
        GamePhase::SelectTrash => "SelectTrash",
        GamePhase::SelectSource => "SelectSource",
        GamePhase::SelectHand => "SelectHand",
        GamePhase::SelectReveal => "SelectReveal",
        GamePhase::SelectSecurity => "SelectSecurity",
        GamePhase::EffectChoice => "EffectChoice",
        GamePhase::BlockTiming => "BlockTiming",
        GamePhase::CounterTiming => "CounterTiming",
        GamePhase::AllianceTiming => "AllianceTiming",
        GamePhase::EndOfTurnAction => "EndOfTurnAction",
        GamePhase::GameOver => "GameOver",
        GamePhase::SelectUnion => "SelectUnion",
        GamePhase::SelectPermutation => "SelectPermutation",
        GamePhase::SelectBudgeted => "SelectBudgeted",
        GamePhase::SelectBreedingPermanent => "SelectBreedingPermanent",
        GamePhase::SelectPlayOrder => "SelectPlayOrder",
    }
}

fn attack_result_str(r: AttackResult) -> &'static str {
    match r {
        AttackResult::Invalid => "Invalid",
        AttackResult::AttackerWins => "AttackerWins",
        AttackResult::DefenderWins => "DefenderWins",
        AttackResult::MutualDestruction => "MutualDestruction",
        AttackResult::SecurityCheckSurvived => "SecurityCheckSurvived",
        AttackResult::AttackerDeletedBySecurity => "AttackerDeletedBySecurity",
        AttackResult::GameWon => "GameWon",
        AttackResult::InProgress => "InProgress",
        AttackResult::Cancelled => "Cancelled",
    }
}

fn card_dto(card: &digimon_engine::card_source::CardSource, data: &[CardData]) -> CardDto {
    // Exhaustive destructure — adding a new field to `CardData` forces a
    // compile error here so the next maintainer makes a deliberate choice
    // about whether the desktop UI needs the field. This is the structural
    // drift detector that catches the kind of bug PR #457 flagged.
    let CardData {
        card_id,
        card_name,
        card_kind,
        level,
        dp,
        play_cost,
        colors,
        traits: _,
        evo_costs,
        dna_costs: _,
        effect_text: _,
        inherited_text: _,
        security_text: _,
        effect_class_name: _,
        index: _,
        norm_id: _,
        keywords: _,
        ace_overflow,
        dual: _,
        digixros_aliases,
        also_treated_as: _,
    } = &data[card.data_index];
    CardDto {
        card_id: card_id.clone(),
        card_name: card_name.clone(),
        card_kind: card_kind_str(*card_kind).to_string(),
        level: *level,
        dp: *dp,
        play_cost: *play_cost,
        colors: colors.iter().map(|&c| color_str(c).to_string()).collect(),
        ace_overflow: *ace_overflow,
        digixros_aliases: digixros_aliases.clone(),
        evo_costs: evo_costs
            .iter()
            .map(|e| EvoCostDto {
                color: e.card_color,
                level: e.level,
                cost: e.memory_cost,
            })
            .collect(),
    }
}

/// Non-top digivolution-source cards of a permanent, ordered bottom→top so
/// index `i` matches the engine's `source_index` in `encode_source_select`
/// (`SOURCE_SELECT_START + field*SOURCES_PER_FIELD + i`). The active top card
/// (last in `card_sources`) is excluded — it is never a `select_material`
/// candidate. The frontend source picker maps a `SOURCE_SELECT` action id back
/// to the card here.
fn perm_sources_dto(
    perm: &digimon_engine::permanent::Permanent,
    data: &[CardData],
) -> Vec<SourceCardDto> {
    let stack = &perm.card_sources;
    let non_top = stack.len().saturating_sub(1);
    stack[..non_top]
        .iter()
        .map(|cs| {
            let cd = &data[cs.data_index];
            SourceCardDto {
                card_id: cd.card_id.clone(),
                card_name: cd.card_name.clone(),
                colors: cd
                    .colors
                    .iter()
                    .map(|&c| color_str(c).to_string())
                    .collect(),
                main_effect_text: cd.effect_text.clone(),
                inherited_effect_text: cd.inherited_text.clone(),
            }
        })
        .collect()
}

fn perm_dto(game: &Game, player: PlayerId, index: usize) -> PermanentDto {
    use digimon_engine::serialization::{expiry_str, modifier_type_str, DISPLAY_KEYWORDS};

    let perm = &game.player(player).battle_area[index];
    let data = &game.card_data;
    let handle = PermanentHandle {
        player,
        index: index as u8,
    };
    let stack_size = perm.stack_size();
    let top = perm.top_card();
    let top_data = &data[top.data_index];
    let base_dp = top_data.dp.unwrap_or(0);

    // ── Active keywords + innate/gained breakdown (mirrors perm_data) ──
    let mut keywords: Vec<String> = Vec::new();
    let mut innate: Vec<String> = Vec::new();
    let mut gained: Vec<String> = Vec::new();
    for (kw, name) in DISPLAY_KEYWORDS {
        if game.has_keyword(handle, *kw) {
            keywords.push((*name).to_string());
            if game.modifiers.has_keyword(handle, *kw) {
                gained.push((*name).to_string());
            } else {
                innate.push((*name).to_string());
            }
        }
    }

    // ── Security-attack modifier (delta from the default of 1) ──
    // Mirror combat's effective strike (`Game::effective_security_strike`),
    // which is BASE-INCLUSIVE of the `security_attack_fn` formula aura (e.g.
    // WarGreymon's `1 + floor(materials/2)`). Summing only the flat keyword +
    // modifier deltas dropped the formula, so a WarGreymon that actually
    // checks 4 security rendered as "Security Attack: 2" in the desktop UI.
    let security_attack_modifier = game.effective_security_strike(handle) - 1;

    // ── DP: base (printed) vs effective (with modifiers) ──
    let total_dp = game.effective_dp(handle).unwrap_or(base_dp);
    let temporary_dp = total_dp - base_dp;

    // ── Inherited effects conferred by non-top digivolution sources ──
    let inherited_effects: Vec<InheritedEffectDto> = perm
        .card_sources
        .iter()
        .enumerate()
        .filter(|(i, _)| i + 1 < stack_size) // exclude the top card
        .filter_map(|(i, cs)| {
            let cd = &data[cs.data_index];
            if cd.inherited_text.trim().is_empty() {
                return None;
            }
            Some(InheritedEffectDto {
                source_index: i,
                card_id: cs.card_id(data).to_string(),
                card_name: cd.card_name.clone(),
                text: cd.inherited_text.clone(),
            })
        })
        .collect();

    // ── Active modifier list (DCGO PermanentDetail parity) ──
    let modifiers: Vec<PermanentModifierDto> = game
        .modifiers
        .permanent_modifiers_iter(handle)
        .filter_map(|entry| {
            let type_str = modifier_type_str(entry.modifier)?;
            let source_card_id = entry.source_permanent.and_then(|sh| {
                game.players
                    .get(sh.player as usize)
                    .and_then(|pl| pl.battle_area.get(sh.index as usize))
                    .map(|p| p.top_card().card_id(data).to_string())
            });
            Some(PermanentModifierDto {
                type_: type_str.to_string(),
                value: entry.value,
                expiry: expiry_str(entry.expiry).to_string(),
                source_card_id,
            })
        })
        .collect();

    let linked_card_ids: Vec<String> = perm
        .linked_cards
        .iter()
        .map(|c| c.card_id(data).to_string())
        .collect();

    PermanentDto {
        field_index: index as u8,
        top_card: card_dto(top, data),
        effective_dp: game.effective_dp(handle),
        is_suspended: perm.is_suspended,
        stack_size,
        turn_played: perm.turn_played,
        sources: perm_sources_dto(perm, data),
        keywords,
        keyword_breakdown: KeywordBreakdownDto { innate, gained },
        security_attack_modifier,
        linked_card_ids,
        main_effect_text: top_data.effect_text.clone(),
        inherited_effects,
        modifiers,
        dp_breakdown: DpBreakdownDto {
            base: Some(base_dp),
            temporary: temporary_dp,
            total: Some(total_dp),
        },
    }
}

fn player_dto(game: &Game, id: PlayerId) -> PlayerDto {
    let p = game.player(id);
    let battle_area: Vec<PermanentDto> = (0..p.battle_area.len())
        .map(|i| perm_dto(game, id, i))
        .collect();
    let hand: Vec<CardDto> = p
        .hand
        .iter()
        .map(|c| card_dto(c, &game.card_data))
        .collect();
    let breeding = p.breeding_area.as_ref().map(|perm| {
        // Breeding permanents are not battle-area-addressable, so the live
        // keyword / modifier / effective-DP queries (which take a
        // PermanentHandle) don't apply — fall back to printed/neutral values,
        // mirroring `serialization::perm_data`'s `handle == None` branch.
        let base_dp = perm.base_dp(&game.card_data);
        PermanentDto {
            field_index: 255, // breeding indicator
            top_card: card_dto(perm.top_card(), &game.card_data),
            effective_dp: base_dp,
            is_suspended: perm.is_suspended,
            stack_size: perm.stack_size(),
            turn_played: perm.turn_played,
            sources: perm_sources_dto(perm, &game.card_data),
            keywords: Vec::new(),
            keyword_breakdown: KeywordBreakdownDto::default(),
            security_attack_modifier: 0,
            linked_card_ids: Vec::new(),
            main_effect_text: game.card_data[perm.top_card().data_index]
                .effect_text
                .clone(),
            inherited_effects: Vec::new(),
            modifiers: Vec::new(),
            dp_breakdown: DpBreakdownDto {
                base: base_dp,
                temporary: 0,
                total: base_dp,
            },
        }
    });
    PlayerDto {
        id,
        hand,
        battle_area,
        breeding,
        deck_count: p.deck.len(),
        egg_deck_count: p.digitama_deck.len(),
        trash_count: p.trash.len(),
        trash: p
            .trash
            .iter()
            .map(|c| card_dto(c, &game.card_data))
            .collect(),
        security_count: p.security.len(),
        is_eliminated: p.is_eliminated,
    }
}

fn pending_selection_dto(game: &Game) -> Option<PendingSelectionDto> {
    let sel = game.pending_selection.as_ref()?;
    let effect_choices = sel
        .effect_choices
        .as_ref()
        .map(|choices| {
            choices
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let card = entry.source_card.map(|h| game.card(h));
                    EffectChoiceDto {
                        index: i as u8,
                        label: entry.label.clone(),
                        action_id: entry.action_id,
                        card_id: card.map(|c| c.card_id.clone()),
                        card_name: card.map(|c| c.card_name.clone()),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    Some(PendingSelectionDto {
        phase: phase_str(game.current_phase).to_string(),
        selecting_player: sel.selecting_player,
        valid_action_ids: sel.valid_action_ids.clone(),
        is_optional: sel.is_optional,
        prompt: sel.prompt.clone(),
        // Same stable discriminant string the engine's own serialization
        // emits (`PendingSelectionView::kind_str()` == `format!("{:?}", kind)`).
        kind: format!("{:?}", sel.kind),
        zone_owner: sel.zone_owner,
        effect_choices,
    })
}

fn revealed_cards_dto(game: &Game) -> Vec<RevealedCardDto> {
    game.revealed_cards
        .iter()
        .map(|cs| RevealedCardDto {
            card_id: cs.card_id(&game.card_data).to_string(),
            owner: cs.owner,
        })
        .collect()
}

pub fn game_state_dto(game: &Game) -> GameStateDto {
    let players: Vec<PlayerDto> = (0..game.rules.player_count)
        .map(|i| player_dto(game, i))
        .collect();
    GameStateDto {
        turn_count: game.turn_count,
        turn_player: game.turn_player(),
        first_player: game.turn_order.first().copied().unwrap_or(0),
        current_phase: phase_str(game.current_phase).to_string(),
        memory: game.memory,
        game_over: game.game_over,
        winner: game.winner,
        players,
        mulligan_current_player: game.mulligan_current_player(),
        mulligan_used: game.mulligan_used.clone(),
        pending_selection: pending_selection_dto(game),
        revealed_cards: revealed_cards_dto(game),
    }
}

// ─── Test card database ───────────────────────────────────────────────

fn synth_card(id: &str, name: &str, kind: CardKind, dp: Option<i32>, cost: u16) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: name.to_string(),
        card_kind: kind,
        level: match kind {
            CardKind::DigiEgg => Some(2),
            CardKind::Digimon => Some(3),
            _ => None,
        },
        dp,
        play_cost: cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        keywords: Vec::new(),
        ace_overflow: None,
        dual: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Build a tiny test card database that works with the engine's built-in
/// TEST-001..005 effects plus some vanilla Digimon for combat.
///
/// `pub` so integration tests can create games using the same card pool.
pub fn test_card_db() -> HashMap<String, CardData> {
    let cards = vec![
        synth_card("TEST-001", "Memory Boost", CardKind::Digimon, Some(2000), 3),
        synth_card("TEST-002", "Draw Power", CardKind::Digimon, Some(2000), 3),
        synth_card("TEST-003", "Buff Captain", CardKind::Digimon, Some(2000), 4),
        synth_card("TEST-004", "Opp Scout", CardKind::Digimon, Some(3000), 4),
        synth_card("TEST-005", "Last Stand", CardKind::Digimon, Some(3000), 4),
        synth_card("VANILLA-3K", "Vanilla 3K", CardKind::Digimon, Some(3000), 3),
        synth_card("VANILLA-5K", "Vanilla 5K", CardKind::Digimon, Some(5000), 5),
        synth_card("EGG-01", "Testmon Egg", CardKind::DigiEgg, None, 0),
    ];
    let mut map = HashMap::new();
    for c in cards {
        map.insert(c.card_id.clone(), c);
    }
    map
}

/// `pub` so integration tests can create standard test decks.
pub fn test_deck() -> Vec<String> {
    // 50 main deck + 5 eggs.
    let mut deck = Vec::with_capacity(55);
    let mains = [
        "TEST-001",
        "TEST-002",
        "TEST-003",
        "TEST-004",
        "TEST-005",
        "VANILLA-3K",
        "VANILLA-5K",
    ];
    // Fill to ~50 cards by repeating.
    for i in 0..50 {
        deck.push(mains[i % mains.len()].to_string());
    }
    for _ in 0..5 {
        deck.push("EGG-01".to_string());
    }
    deck
}

// ─── Tauri commands ───────────────────────────────────────────────────

/// Create a new 2-player game with built-in test cards.
#[tauri::command]
pub async fn create_test_game(
    engine: tauri::State<'_, EngineHandle>,
) -> Result<GameStateDto, String> {
    engine
        .run(move |world| -> Result<GameStateDto, String> {
            let db = test_card_db();
            let decks = vec![test_deck(), test_deck()];
            let mut game = Game::new(&decks, &db, Rules::standard(), Some(42))
                .map_err(|e| format!("Game::new failed: {}", e))?;
            game.start_game();
            let dto = game_state_dto(&game);
            world.game = Some(game);
            Ok(dto)
        })
        .await?
}

/// Get the current game state as JSON.
#[tauri::command]
pub async fn get_rust_game_state(
    engine: tauri::State<'_, EngineHandle>,
) -> Result<GameStateDto, String> {
    engine
        .run(move |world| -> Result<GameStateDto, String> {
            let game = world.game.as_ref().ok_or("No active game")?;
            Ok(game_state_dto(game))
        })
        .await?
}

/// Play a card from a player's hand.
#[tauri::command]
pub async fn rust_play_card(
    engine: tauri::State<'_, EngineHandle>,
    player_id: PlayerId,
    hand_index: usize,
) -> Result<GameStateDto, String> {
    engine
        .run(move |world| -> Result<GameStateDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            let field_index = game
                .play_from_hand(player_id, hand_index)
                .ok_or("Cannot play that card (invalid hand index or field is full)")?;
            let _ = field_index; // currently unused in the response
            Ok(game_state_dto(game))
        })
        .await?
}

/// Attack an opposing Digimon.
#[tauri::command]
pub async fn rust_attack_digimon(
    engine: tauri::State<'_, EngineHandle>,
    attacker_player: PlayerId,
    attacker_index: u8,
    defender_player: PlayerId,
    defender_index: u8,
) -> Result<AttackResultDto, String> {
    engine
        .run(move |world| -> Result<AttackResultDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            let attacker = PermanentHandle {
                player: attacker_player,
                index: attacker_index,
            };
            let defender = PermanentHandle {
                player: defender_player,
                index: defender_index,
            };
            // Tauri UI only drives Main-phase attacks today; <Vortex> end-of-turn
            // attacks will get a dedicated command once §4.6 mask coverage lands.
            let result = game.attack_digimon(attacker, defender, false);
            Ok(AttackResultDto {
                result: attack_result_str(result).to_string(),
                state: game_state_dto(game),
            })
        })
        .await?
}

/// Attack the opposing player (security check).
#[tauri::command]
pub async fn rust_attack_player(
    engine: tauri::State<'_, EngineHandle>,
    attacker_player: PlayerId,
    attacker_index: u8,
    defender_player: PlayerId,
) -> Result<AttackResultDto, String> {
    engine
        .run(move |world| -> Result<AttackResultDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            let attacker = PermanentHandle {
                player: attacker_player,
                index: attacker_index,
            };
            let result = game.attack_player(attacker, defender_player, false);
            Ok(AttackResultDto {
                result: attack_result_str(result).to_string(),
                state: game_state_dto(game),
            })
        })
        .await?
}

/// End the current turn.
#[tauri::command]
pub async fn rust_end_turn(engine: tauri::State<'_, EngineHandle>) -> Result<GameStateDto, String> {
    engine
        .run(move |world| -> Result<GameStateDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            game.end_turn();
            Ok(game_state_dto(game))
        })
        .await?
}

/// Pass turn (memory to -3, then end turn).
#[tauri::command]
pub async fn rust_pass_turn(
    engine: tauri::State<'_, EngineHandle>,
) -> Result<GameStateDto, String> {
    engine
        .run(move |world| -> Result<GameStateDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            game.pass_turn();
            Ok(game_state_dto(game))
        })
        .await?
}

/// Apply a mulligan decision for the currently-deciding player.
/// `keep = true` keeps the opening hand; `keep = false` shuffles it back
/// and redraws the same count. Returns the updated state.
#[tauri::command]
pub async fn rust_mulligan_decide(
    engine: tauri::State<'_, EngineHandle>,
    keep: bool,
) -> Result<GameStateDto, String> {
    engine
        .run(move |world| -> Result<GameStateDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            let p = game
                .mulligan_current_player()
                .ok_or("Mulligan is already complete")?;
            game.accept_mulligan(p, keep).map_err(|e| e.to_string())?;
            Ok(game_state_dto(game))
        })
        .await?
}

/// Hatch: move top egg to breeding area.
#[tauri::command]
pub async fn rust_hatch(
    engine: tauri::State<'_, EngineHandle>,
    player_id: PlayerId,
) -> Result<GameStateDto, String> {
    engine
        .run(move |world| -> Result<GameStateDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            if !game.hatch(player_id) {
                return Err("Cannot hatch (no egg or breeding occupied)".into());
            }
            Ok(game_state_dto(game))
        })
        .await?
}

/// Move from breeding area to battle area.
#[tauri::command]
pub async fn rust_move_from_breeding(
    engine: tauri::State<'_, EngineHandle>,
    player_id: PlayerId,
) -> Result<GameStateDto, String> {
    engine
        .run(move |world| -> Result<GameStateDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            if !game.move_from_breeding(player_id) {
                return Err("Cannot move from breeding".into());
            }
            Ok(game_state_dto(game))
        })
        .await?
}

// ─── Action-ID dispatch envelope (parity with hosted-API shape) ───
//
// The frontend's `gameApi.ts` speaks in flat action IDs and expects
// responses shaped like the hosted API's `ActionResponse`. These
// commands and DTOs let desktop and web callers bind against one set of
// TypeScript types regardless of which backend is serving the request.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEventDto {
    #[serde(rename = "type")]
    pub event_type: String,
    pub seq: u32,
    pub player: i32,
    pub source_card_id: Option<String>,
    pub source_card_name: Option<String>,
    pub source_slot: Option<i32>,
    pub target_card_id: Option<String>,
    pub target_card_name: Option<String>,
    pub target_slot: Option<i32>,
    pub meta: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSummaryDto {
    pub player_id: PlayerId,
    pub profile_id: String,
    pub profile_version: u16,
    pub tensor_size: usize,
    pub mask_size: usize,
    pub legal_action_count: usize,
    pub card_id_slot_count: usize,
    pub scalar_slot_count: usize,
    pub turn_count: u16,
    pub phase: String,
    pub memory: i16,
    pub tensor_head: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTraceDto {
    pub actor: String,
    pub player_id: PlayerId,
    pub action_id: u16,
    pub decoded: ActionExplanation,
    pub tensor_summary: Option<TensorSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponseDto {
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
    pub is_game_over: bool,
    pub logs: Vec<String>,
    pub events: Vec<GameEventDto>,
    pub action_context: serde_json::Value,
    pub action_traces: Vec<ActionTraceDto>,
    /// True when a non-human agent still has actions to take — a paced
    /// caller should invoke `rust_step_game { paced: true }` to advance the
    /// next beat. Always `false` for unpaced calls (they drain agents to the
    /// next human decision point). Field name must stay aligned with the
    /// browser wire's `agent_pending` (cross-wire parity).
    pub agent_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGameResponseDto {
    pub game_id: String,
    pub seed: String,
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
    /// Whether the next decision belongs to an agent (so the frontend locks the
    /// human's controls and the paced driver advances it). True when the AI
    /// acts first — e.g. it mulligans first when the human goes second. Without
    /// this the frontend defaulted to `false` and presented the AI's pending
    /// mulligan to the human as a clickable prompt. Mirrors
    /// `ActionResponseDto.agent_pending`.
    pub agent_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResponseDto {
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
    pub logs: Vec<String>,
    pub events: Vec<GameEventDto>,
    pub is_human_turn: bool,
    pub is_game_over: bool,
    pub action_traces: Vec<ActionTraceDto>,
    /// See `ActionResponseDto::agent_pending` — same semantics and same
    /// cross-wire field-name contract.
    pub agent_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrenderResponseDto {
    pub state: GameStateDto,
    pub action_mask: Vec<u8>,
    pub logs: Vec<String>,
    pub events: Vec<GameEventDto>,
    pub is_game_over: bool,
    pub surrendered_by: PlayerId,
}

/// Mirror of HeadlessRunner::current_decision_player — who is expected to
/// submit the next action.
pub fn current_decision_player(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

/// Build a `u8` action mask for the current decider. The frontend stores
/// `number[]`; `0`/`1` bytes round-trip transparently through Tauri/serde.
pub fn action_mask_bytes(game: &Game) -> Vec<u8> {
    let pid = current_decision_player(game);
    build_action_mask(game, pid)
        .into_iter()
        .map(|v| if v > 0.0 { 1u8 } else { 0u8 })
        .collect()
}

fn py_player_id(player: PlayerId) -> i32 {
    i32::from(player) + 1
}

fn event_to_dto(event: GameEvent) -> GameEventDto {
    let mut dto = GameEventDto {
        event_type: event.type_str().to_string(),
        seq: event.seq() as u32,
        player: 0,
        source_card_id: None,
        source_card_name: None,
        source_slot: None,
        target_card_id: None,
        target_card_name: None,
        target_slot: None,
        meta: serde_json::json!({}),
    };

    match event {
        GameEvent::MemoryChange {
            player,
            delta,
            total,
            source_card_id,
            source_card_name,
            ..
        } => {
            dto.player = py_player_id(player);
            dto.source_card_id = source_card_id;
            dto.source_card_name = source_card_name;
            dto.meta = serde_json::json!({ "delta": delta, "total": total });
        }
        GameEvent::TurnStart {
            player, turn_count, ..
        } => {
            dto.player = py_player_id(player);
            dto.meta = serde_json::json!({ "turn_count": turn_count });
        }
        GameEvent::PhaseChange { player, phase, .. } => {
            dto.player = py_player_id(player);
            dto.meta = serde_json::json!({ "phase": format!("{:?}", phase) });
        }
        GameEvent::Play {
            player,
            card_id,
            card_name,
            field_index,
            cost_paid,
            cost_printed,
            via_alt_path,
            ..
        } => {
            dto.player = py_player_id(player);
            dto.source_card_id = Some(card_id);
            dto.source_card_name = Some(card_name);
            dto.source_slot = Some(i32::from(field_index));
            dto.meta = serde_json::json!({
                "cost_paid": cost_paid,
                "cost_printed": cost_printed,
                "via_alt_path": via_alt_path,
            });
        }
        GameEvent::Digivolve {
            player,
            top_card_id,
            card_name,
            field_index,
            from_stack_top,
            was_dna,
            was_blast_dna,
            memory_paid,
            ..
        } => {
            dto.player = py_player_id(player);
            dto.source_card_id = Some(top_card_id);
            dto.source_card_name = Some(card_name);
            dto.source_slot = Some(i32::from(field_index));
            dto.meta = serde_json::json!({
                "from_stack_top": from_stack_top,
                "was_dna": was_dna,
                "was_blast_dna": was_blast_dna,
                "memory_paid": memory_paid,
            });
        }
        GameEvent::Attack {
            player,
            attacker_field_index,
            target_field_index,
            target_player,
            attacker_card_id,
            attacker_card_name,
            target_card_id,
            target_card_name,
            attacker_dp,
            target_dp,
            ..
        } => {
            dto.player = py_player_id(player);
            dto.source_card_id = Some(attacker_card_id);
            dto.source_card_name = Some(attacker_card_name);
            dto.source_slot = Some(i32::from(attacker_field_index));
            dto.target_card_id = target_card_id;
            dto.target_card_name = target_card_name;
            dto.target_slot = target_field_index.map(i32::from);
            dto.meta = serde_json::json!({
                "target_player": target_player.map(py_player_id),
                "attacker_dp": attacker_dp,
                "target_dp": target_dp,
            });
        }
        GameEvent::Trash {
            player,
            card_id,
            card_name,
            ..
        }
        | GameEvent::Mill {
            player,
            card_id,
            card_name,
            ..
        } => {
            dto.player = py_player_id(player);
            dto.source_card_id = Some(card_id);
            dto.source_card_name = Some(card_name);
        }
        GameEvent::SecurityReveal {
            defender,
            card_id,
            card_name,
            ..
        } => {
            dto.player = py_player_id(defender);
            dto.source_card_id = Some(card_id);
            dto.source_card_name = Some(card_name);
        }
        GameEvent::EffectTarget {
            player,
            source_card_id,
            source_card_name,
            targets,
            ..
        } => {
            dto.player = py_player_id(player);
            dto.source_card_id = Some(source_card_id);
            dto.source_card_name = Some(source_card_name);
            let targets_json: Vec<serde_json::Value> = targets
                .into_iter()
                .map(|t| serde_json::json!({ "card_id": t.card_id, "card_name": t.card_name }))
                .collect();
            dto.meta = serde_json::json!({ "targets": targets_json });
        }
        GameEvent::Reveal {
            player,
            card_id,
            card_name,
            source_zone,
            ..
        } => {
            dto.player = py_player_id(player);
            dto.source_card_id = Some(card_id);
            dto.source_card_name = Some(card_name);
            dto.meta = serde_json::json!({ "source_zone": format!("{:?}", source_zone) });
        }
        GameEvent::GameOver { winner, reason, .. } => {
            dto.meta = serde_json::json!({
                "winner": winner.map(py_player_id),
                "reason": reason.as_str(),
                "result": reason.result(),
            });
        }
        GameEvent::Concede { player, .. } => {
            dto.player = py_player_id(player);
        }
        GameEvent::EffectFizzled {
            source_permanent,
            reason,
            ..
        } => {
            if let Some(handle) = source_permanent {
                dto.player = py_player_id(handle.player);
                dto.source_slot = Some(i32::from(handle.index));
            }
            dto.meta = serde_json::json!({ "reason": reason });
        }
        _ => {}
    }
    dto
}

fn parse_optional_seed(seed: Option<String>) -> Result<Option<u64>, String> {
    let Some(raw) = seed else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err("seed must be a base-10 integer in the u64 range".to_string());
    }
    trimmed
        .parse::<u64>()
        .map(Some)
        .map_err(|_| "seed must be in the u64 range".to_string())
}

/// Drain structured gameplay events emitted by the engine during the last
/// action. The DTO mirrors the browser/PyO3 `GameEvent.to_dict` shape so the
/// shared frontend can consume desktop and browser responses identically.
fn drain_events(game: &mut Game) -> Vec<GameEventDto> {
    game.drain_events().into_iter().map(event_to_dto).collect()
}

/// Ensure a game exists (auto-seed a test game on first call) and return a
/// mutable reference. Lets the frontend skip the explicit create-game step
/// during rapid iteration. Operates directly on the worker's owned
/// `Option<Game>`.
fn ensure_game(game: &mut Option<Game>) -> Result<&mut Game, String> {
    if game.is_none() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut g = Game::new(&decks, &db, Rules::standard(), Some(42))
            .map_err(|e| format!("Game::new failed: {}", e))?;
        g.start_game();
        *game = Some(g);
    }
    game.as_mut().ok_or_else(|| "No active game".to_string())
}

/// Build the initial game for a new session, left ready for the opening
/// mulligan. `Game::new` starts a standard game in the `Mulligan` phase; we
/// hand it back in that phase so the opening keep/redraw decision is exposed
/// to the human (via the frontend) and to the AI (via the agent loop's
/// `current_decision_player`, which returns the mulligan decider). The final
/// `accept_mulligan` triggers `finalize_mulligan`, which begins turn 1. Only
/// when the ruleset leaves no mulligan pending (defensive) do we start turn 1
/// directly via `start_game`.
fn new_session_game(
    decks: &[Vec<String>],
    db: &std::collections::HashMap<String, CardData>,
    rules: Rules,
    seed: u64,
) -> Result<Game, String> {
    let mut game =
        Game::new(decks, db, rules, Some(seed)).map_err(|e| format!("Game::new failed: {}", e))?;
    if game.mulligan_current_player().is_none() {
        game.start_game();
    }
    Ok(game)
}

/// Create a new local game. When `deck1`/`deck2` are provided by the
/// caller, the real `data/cards.json`-derived card pool is used so any
/// production card ID resolves. When both are absent, falls back to the
/// synthetic `test_card_db()` (used by the legacy smoke flow and the
/// `create_test_game` command).
///
/// Accepts optional per-player kinds (`human` / `greedy` / `trained`) and
/// model IDs (one per player, only honored for `trained`). When both are
/// omitted, both seats default to `human`.
///
/// Player indices in the request are 0-based (matching the Rust engine's
/// `PlayerId` convention).
#[tauri::command]
pub async fn rust_create_game(
    engine: tauri::State<'_, EngineHandle>,
    deck1: Option<Vec<String>>,
    deck2: Option<Vec<String>>,
    player_kinds: Option<Vec<PlayerKind>>,
    player_model_ids: Option<Vec<Option<String>>>,
    seed: Option<String>,
) -> Result<CreateGameResponseDto, String> {
    // Parse the seed on the dispatching thread so a bad value short-circuits
    // before reaching the worker (preserving the prior error contract).
    let parsed_seed = parse_optional_seed(seed)?;
    engine
        .run(move |world| -> Result<CreateGameResponseDto, String> {
            // When the caller provided real decks, use the production card pool so
            // BT/EX/AD/P/ST card IDs resolve. Without this, `Game::new` would fail
            // on the very first card-id lookup against the 8-card synthetic
            // `test_card_db()` that this command historically defaulted to.
            let caller_provided_decks = deck1.is_some() || deck2.is_some();
            let db = if caller_provided_decks {
                digimon_engine::deck_tools::full_card_data()
            } else {
                test_card_db()
            };
            let decks = vec![
                deck1.unwrap_or_else(test_deck),
                deck2.unwrap_or_else(test_deck),
            ];
            let player_count = decks.len();
            let effective_seed = parsed_seed.unwrap_or_else(rand::random::<u64>);
            // Leave the game in the Mulligan phase so BOTH players make their
            // opening keep/redraw decision: the frontend drives the human's, and
            // the agent loop drives the AI's. Auto-resolving mulligan here was
            // the bug that skipped it entirely and dropped the going-second human
            // into a Hatch prompt before the opponent moved.
            let game = new_session_game(&decks, &db, Rules::standard(), effective_seed)?;

            let kinds = normalize_player_kinds(player_kinds, player_count)?;
            let model_ids = normalize_player_model_ids(player_model_ids, player_count, &kinds)?;
            validate_models_loaded(&world.inference, &kinds, &model_ids)?;
            // Fresh episode — reset LSTM state on every model any player will drive.
            // Silent for models the user passed but never loaded; the predict path
            // will surface that as a clean error instead.
            for model_id in model_ids.iter().flatten() {
                let _ = world.inference.reset(model_id);
            }

            let registry = CardRegistry::from_cards(&db);
            let session = GameSession {
                registry: Some(registry),
                player_kinds: kinds,
                player_model_ids: model_ids,
            };
            let mask = action_mask_bytes(&game);
            let dto = game_state_dto(&game);
            // Compute BEFORE moving game/session into `world` so the frontend
            // knows up-front whether the opening decision (e.g. the first
            // mulligan when the human goes second) belongs to the AI.
            let pending = agent_pending(&game, &session);

            world.game = Some(game);
            world.session = session;

            Ok(CreateGameResponseDto {
                game_id: "rust-local".to_string(),
                seed: effective_seed.to_string(),
                state: dto,
                action_mask: mask,
                agent_pending: pending,
            })
        })
        .await?
}

fn normalize_player_kinds(
    provided: Option<Vec<PlayerKind>>,
    player_count: usize,
) -> Result<Vec<PlayerKind>, String> {
    match provided {
        None => Ok(vec![PlayerKind::Human; player_count]),
        Some(v) if v.len() == player_count => Ok(v),
        Some(v) => Err(format!(
            "player_kinds length {} does not match player count {}",
            v.len(),
            player_count
        )),
    }
}

fn normalize_player_model_ids(
    provided: Option<Vec<Option<String>>>,
    player_count: usize,
    kinds: &[PlayerKind],
) -> Result<Vec<Option<String>>, String> {
    let v = match provided {
        None => vec![None; player_count],
        Some(v) if v.len() == player_count => v,
        Some(v) => {
            return Err(format!(
                "player_model_ids length {} does not match player count {}",
                v.len(),
                player_count
            ));
        }
    };
    for (i, (kind, model_id)) in kinds.iter().zip(v.iter()).enumerate() {
        if *kind == PlayerKind::Trained && model_id.is_none() {
            return Err(format!(
                "player {i} is 'trained' but no model_id was provided"
            ));
        }
    }
    Ok(v)
}

fn validate_models_loaded(
    inference: &InferenceState,
    kinds: &[PlayerKind],
    model_ids: &[Option<String>],
) -> Result<(), String> {
    for (i, (kind, model_id)) in kinds.iter().zip(model_ids.iter()).enumerate() {
        if *kind != PlayerKind::Trained {
            continue;
        }
        let id = model_id.as_deref().expect("trained kind without model_id");
        if !inference.is_loaded(id)? {
            return Err(format!(
                "player {i} requested model '{id}' which is not loaded; \
                 call rust_load_model first"
            ));
        }
    }
    Ok(())
}

/// Execute a flat RL-action-ID against the current game. Mirrors the
/// hosted API's `POST /games/{id}/actions` surface so the frontend's
/// `gameApi.ts` can bind against this without knowing which backend is
/// running.
///
/// After the human's action lands we also drain any agent turns that
/// immediately follow, so the response reflects the next state the
/// player needs to see — same pattern the hosted API uses in
/// `InteractiveGame.step`.
#[tauri::command]
pub async fn rust_submit_action(
    engine: tauri::State<'_, EngineHandle>,
    action: u32,
    paced: Option<bool>,
) -> Result<ActionResponseDto, String> {
    engine
        .run(move |world| -> Result<ActionResponseDto, String> {
            let game = ensure_game(&mut world.game)?;
            let session = &world.session;
            let inference = &world.inference;
            let pid = current_decision_player(game);
            let action_u16 = u16::try_from(action)
                .map_err(|_| format!("action {action} is out of range for a u16 action ID"))?;
            let mask_before = build_action_mask(game, pid);
            let human_trace = action_trace_for(
                game,
                "human",
                pid,
                action_u16,
                optional_tensor_summary_for(game, pid, session.registry.as_ref(), &mask_before),
            );
            game.decode_action(action_u16, pid);
            // Paced mode: the human's action lands, then at most ONE agent action
            // executes per request so the frontend can render each beat (the
            // pacing driver keeps calling `rust_step_game { paced: true }` while
            // `agent_pending`).
            let max_agent_actions = if paced.unwrap_or(false) {
                Some(1)
            } else {
                None
            };
            let mut action_traces = vec![human_trace];
            action_traces.extend(run_agent_steps(
                game,
                session,
                inference,
                max_agent_actions,
            )?);
            let events = drain_events(game);
            let mask = action_mask_bytes(game);
            let is_over = game.game_over;
            let pending = agent_pending(game, session);
            Ok(ActionResponseDto {
                state: game_state_dto(game),
                action_mask: mask,
                is_game_over: is_over,
                logs: Vec::new(),
                events,
                action_context: serde_json::json!({}),
                action_traces,
                agent_pending: pending,
            })
        })
        .await?
}

/// Drive the game forward until a human decision is required, or the game
/// ends. Each loop iteration inspects the current decider's `PlayerKind`:
///
/// - `Human`: stop — frontend should render and await the next `rust_submit_action`.
/// - `Greedy`: pick the first valid action (deterministic heuristic used for
///   local testing until a stronger built-in bot lands).
/// - `Trained`: run the ONNX policy attached to that player's `model_id`.
///
/// The response's `is_human_turn` reflects state *after* the loop: `true`
/// means the frontend should wait for user input, `false` means the game
/// ended while the last decider was still an agent.
#[tauri::command]
pub async fn rust_step_game(
    engine: tauri::State<'_, EngineHandle>,
    paced: Option<bool>,
) -> Result<StepResponseDto, String> {
    engine
        .run(move |world| -> Result<StepResponseDto, String> {
            let game = ensure_game(&mut world.game)?;
            let session = &world.session;
            let inference = &world.inference;
            // Paced mode executes exactly one agent action per invoke; this command
            // doubles as the "advance one beat" call the frontend pacing driver
            // repeats while `agent_pending` is true.
            let max_agent_actions = if paced.unwrap_or(false) {
                Some(1)
            } else {
                None
            };
            let action_traces = run_agent_steps(game, session, inference, max_agent_actions)?;
            // Drain the events the agent's action(s) produced this beat so the
            // frontend streams them live (security-reveal overlay + the
            // event-derived game log). Previously this was `Vec::new()`, so the
            // AI's events piled up in the engine queue and only flushed on the
            // human's NEXT submit — the attack's security reveal and the log
            // appeared a turn late (after the player's breeding action).
            let events = drain_events(game);
            let mask = action_mask_bytes(game);
            let pid = current_decision_player(game);
            let is_human_turn =
                matches!(decider_kind(session, pid), PlayerKind::Human) || game.game_over;
            let is_over = game.game_over;
            let pending = agent_pending(game, session);
            Ok(StepResponseDto {
                state: game_state_dto(game),
                action_mask: mask,
                logs: Vec::new(),
                events,
                is_human_turn,
                is_game_over: is_over,
                action_traces,
                agent_pending: pending,
            })
        })
        .await?
}

fn decider_kind(session: &GameSession, pid: PlayerId) -> PlayerKind {
    session
        .player_kinds
        .get(pid as usize)
        .copied()
        .unwrap_or(PlayerKind::Human)
}

/// True when the next decision belongs to a non-human agent and the game is
/// still live — i.e. a paced caller should request another agent advance.
/// Unpaced calls always land on a human decision point or game end, so this
/// reports `false` for them by construction.
pub fn agent_pending(game: &Game, session: &GameSession) -> bool {
    !game.game_over
        && !matches!(
            decider_kind(session, current_decision_player(game)),
            PlayerKind::Human
        )
}

/// Step the game forward while the current decider is a non-human agent.
/// Bail out as soon as the game ends or a human seat is up — matches the
/// hosted API's `InteractiveGame.run_step` contract so the frontend
/// state machine doesn't care which backend is driving.
///
/// `max_actions` caps how many agent actions execute in this call: `None`
/// runs to the next human decision point / game end (legacy behavior);
/// `Some(n)` returns after at most `n` actions so the frontend can pace
/// the agent's turn into human-perceivable beats (see the
/// `add-bot-action-pacing` change). Whether agent actions *remain* after
/// a capped return is reported separately by [`agent_pending`].
///
/// `pub` so integration tests under `tests/` can drive the game loop
/// directly without going through the Tauri IPC layer.
pub fn run_agent_steps(
    game: &mut Game,
    session: &GameSession,
    inference: &InferenceState,
    max_actions: Option<usize>,
) -> Result<Vec<ActionTraceDto>, String> {
    // Cap iterations defensively so a bug in mask generation can't turn this
    // into an infinite spin. Normal games resolve in far fewer than this.
    const MAX_AGENT_STEPS: usize = 10_000;
    let mut traces = Vec::new();
    for _ in 0..MAX_AGENT_STEPS {
        if game.game_over {
            return Ok(traces);
        }
        if matches!(max_actions, Some(cap) if traces.len() >= cap) {
            return Ok(traces);
        }
        let pid = current_decision_player(game);
        let kind = decider_kind(session, pid);
        let (actor, action) = match kind {
            PlayerKind::Human => return Ok(traces),
            PlayerKind::Greedy => {
                let mask_before = build_action_mask(game, pid);
                (
                    "agent_greedy",
                    digimon_engine::policies::greedy_action(game, &mask_before) as usize,
                )
            }
            PlayerKind::Trained => {
                let model_id = session
                    .player_model_ids
                    .get(pid as usize)
                    .and_then(|m| m.as_deref())
                    .ok_or_else(|| format!("trained agent for player {pid} has no model_id"))?;
                let registry = session.registry.as_ref().ok_or_else(|| {
                    "inference: session has no card registry (game not created?)".to_string()
                })?;
                let obs = build_tensor(game, pid, registry);
                let mask_before = build_action_mask(game, pid);
                validate_shapes(&obs, &mask_before, model_id)?;
                (
                    "agent_trained",
                    inference.predict(model_id, &obs, &mask_before)?,
                )
            }
        };
        let action_u16 = u16::try_from(action)
            .map_err(|_| format!("agent returned out-of-range action {action}"))?;
        let mask_before = build_action_mask(game, pid);
        traces.push(action_trace_for(
            game,
            actor,
            pid,
            action_u16,
            optional_tensor_summary_for(game, pid, session.registry.as_ref(), &mask_before),
        ));
        game.decode_action(action_u16, pid);
    }
    Err(format!(
        "agent step loop exceeded {MAX_AGENT_STEPS} iterations; possible mask bug"
    ))
}

/// Sanity-check obs/mask sizes before feeding them to the policy so a
/// shape mismatch shows up here (with a clear error) rather than inside
/// the ONNX session.
fn validate_shapes(obs: &[f32], mask: &[f32], model_id: &str) -> Result<(), String> {
    if obs.len() != digimon_engine::tensor::TENSOR_SIZE {
        return Err(format!(
            "model '{model_id}' obs length {} != engine TENSOR_SIZE {}",
            obs.len(),
            digimon_engine::tensor::TENSOR_SIZE
        ));
    }
    if mask.len() != digimon_engine::action::space::ACTION_SPACE_SIZE {
        return Err(format!(
            "model '{model_id}' mask length {} != engine ACTION_SPACE_SIZE {}",
            mask.len(),
            digimon_engine::action::space::ACTION_SPACE_SIZE
        ));
    }
    Ok(())
}

fn tensor_summary_for(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
    mask: &[f32],
) -> TensorSummaryDto {
    let tensor = build_tensor(game, player_id, registry);
    let profile = default_profile();
    TensorSummaryDto {
        player_id,
        profile_id: profile.id.to_string(),
        profile_version: profile.version as u16,
        tensor_size: tensor.len(),
        mask_size: mask.len(),
        legal_action_count: mask.iter().filter(|&&v| v > 0.0).count(),
        card_id_slot_count: profile.card_id_slot_count,
        scalar_slot_count: profile.scalar_slot_count,
        turn_count: game.turn_count,
        phase: format!("{:?}", game.current_phase),
        memory: game.memory,
        tensor_head: tensor.iter().take(16).copied().collect(),
    }
}

fn optional_tensor_summary_for(
    game: &Game,
    player_id: PlayerId,
    registry: Option<&CardRegistry>,
    mask: &[f32],
) -> Option<TensorSummaryDto> {
    registry.map(|registry| tensor_summary_for(game, player_id, registry, mask))
}

fn action_trace_for(
    game: &Game,
    actor: &str,
    player_id: PlayerId,
    action_id: u16,
    tensor_summary: Option<TensorSummaryDto>,
) -> ActionTraceDto {
    ActionTraceDto {
        actor: actor.to_string(),
        player_id,
        action_id,
        decoded: explain_action(game, player_id, action_id),
        tensor_summary,
    }
}

/// Read the current action mask.
#[tauri::command]
pub async fn rust_get_mask(engine: tauri::State<'_, EngineHandle>) -> Result<Vec<u8>, String> {
    engine
        .run(move |world| -> Result<Vec<u8>, String> {
            let game = world.game.as_ref().ok_or("No active game")?;
            Ok(action_mask_bytes(game))
        })
        .await?
}

/// Decoded list of every currently-legal action for the current decision
/// player, each carrying source-card and effect identity (`card_name`,
/// `effect_name`, `source_zone`, `source_index`, `label`). The action bar
/// renders activatable effects from this list so it never re-derives action
/// semantics from raw mask bit-ranges (which mis-decodes trash/hand [Main]
/// effects). Parallels `rust_get_mask` and is fetched per state update.
#[tauri::command]
pub async fn rust_get_decoded_actions(
    engine: tauri::State<'_, EngineHandle>,
) -> Result<Vec<ActionExplanation>, String> {
    engine
        .run(move |world| -> Result<Vec<ActionExplanation>, String> {
            let game = world.game.as_ref().ok_or("No active game")?;
            let pid = current_decision_player(game);
            Ok(legal_decoded_actions(game, pid))
        })
        .await?
}

#[tauri::command]
pub async fn rust_get_board_tensor_summary(
    engine: tauri::State<'_, EngineHandle>,
    player_id: PlayerId,
) -> Result<TensorSummaryDto, String> {
    engine
        .run(move |world| -> Result<TensorSummaryDto, String> {
            let game = world.game.as_ref().ok_or("No active game")?;
            let registry = world.session.registry.as_ref().ok_or_else(|| {
                "tensor summary: session has no card registry (game not created?)".to_string()
            })?;
            let mask = build_action_mask(game, player_id);
            Ok(tensor_summary_for(game, player_id, registry, &mask))
        })
        .await?
}

/// Read the accumulated log (empty for now — Rust engine doesn't log yet).
#[tauri::command]
pub async fn rust_get_log(engine: tauri::State<'_, EngineHandle>) -> Result<Vec<String>, String> {
    engine
        .run(move |_world| -> Result<Vec<String>, String> { Ok(Vec::new()) })
        .await?
}

/// Surrender ends the game with the opposing player as winner.
#[tauri::command]
pub async fn rust_surrender(
    engine: tauri::State<'_, EngineHandle>,
    player_id: PlayerId,
) -> Result<SurrenderResponseDto, String> {
    engine
        .run(move |world| -> Result<SurrenderResponseDto, String> {
            let game = world.game.as_mut().ok_or("No active game")?;
            let winner = if player_id == 0 { 1 } else { 0 };
            game.declare_winner(winner);
            let mask = action_mask_bytes(game);
            Ok(SurrenderResponseDto {
                state: game_state_dto(game),
                action_mask: mask,
                logs: Vec::new(),
                events: Vec::new(),
                is_game_over: true,
                surrendered_by: player_id,
            })
        })
        .await?
}

/// Delete the active game (used by the frontend cleanup path). Clears the
/// session (player kinds, model-id bindings, card registry) at the same
/// time — but loaded ONNX policies stay in the inference cache since the
/// next game will likely reuse them.
#[tauri::command]
pub async fn rust_delete_game(engine: tauri::State<'_, EngineHandle>) -> Result<(), String> {
    engine
        .run(move |world| {
            world.game = None;
            world.session = GameSession::default();
        })
        .await
}

/// Load an ONNX model into the inference cache, keyed by `model_id`. This
/// is the low-level command the Phase-C model manager will wrap — for now
/// the frontend can call it directly with a filesystem path for testing.
/// Idempotent: reloading the same `model_id` replaces the previous entry.
#[tauri::command]
pub async fn rust_load_model(
    engine: tauri::State<'_, EngineHandle>,
    model_id: String,
    onnx_path: String,
) -> Result<(), String> {
    engine
        .run(move |world| -> Result<(), String> {
            let path = std::path::PathBuf::from(&onnx_path);
            if !path.exists() {
                return Err(format!("ONNX file not found: {onnx_path}"));
            }
            world.inference.load(model_id, &path)
        })
        .await?
}

/// Unload a model from the cache, freeing its ONNX session.
#[tauri::command]
pub async fn rust_unload_model(
    engine: tauri::State<'_, EngineHandle>,
    model_id: String,
) -> Result<bool, String> {
    engine
        .run(move |world| world.inference.unload(&model_id))
        .await?
}

/// List currently-loaded model IDs.
#[tauri::command]
pub async fn rust_list_loaded_models(
    engine: tauri::State<'_, EngineHandle>,
) -> Result<Vec<String>, String> {
    engine
        .run(move |world| world.inference.loaded_ids())
        .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── new-session mulligan (mulligan must NOT be auto-skipped) ────────

    #[test]
    fn new_session_game_starts_in_mulligan_awaiting_a_decision() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let game = new_session_game(&decks, &db, Rules::standard(), 42).unwrap();
        // Regression: rust_create_game used to call start_game(), which auto-kept
        // every mulligan — skipping the opening decision and dropping the
        // going-second human straight into a Hatch prompt. The session game must
        // now be handed over in the Mulligan phase with a pending decider.
        assert_eq!(game.current_phase, GamePhase::Mulligan);
        assert!(
            game.mulligan_current_player().is_some(),
            "a fresh session game must await an opening mulligan decision"
        );
    }

    #[test]
    fn new_session_game_finalizes_to_turn_one_once_mulligans_resolve() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game = new_session_game(&decks, &db, Rules::standard(), 42).unwrap();
        // Resolving both players' mulligans (keep) must finalize into turn 1 —
        // i.e. the game is playable, not stuck in Mulligan.
        while let Some(p) = game.mulligan_current_player() {
            game.accept_mulligan(p, /* keep */ true).unwrap();
        }
        assert!(game.mulligan_current_player().is_none());
        assert_eq!(game.turn_count, 1, "the last accept_mulligan begins turn 1");
        assert_ne!(game.current_phase, GamePhase::Mulligan);
    }

    #[test]
    fn new_session_game_first_player_follows_seed_parity() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        // Deterministic: even seed → seat 0 (local human) goes first; odd → AI.
        for seed in 0u64..8 {
            let game = new_session_game(&decks, &db, Rules::standard(), seed).unwrap();
            assert_eq!(
                game.turn_order[0],
                (seed % 2) as PlayerId,
                "seed {seed}: first player must follow seed parity"
            );
        }
    }

    #[test]
    fn new_session_game_first_player_is_balanced_over_random_seeds() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let (mut p0, mut p1) = (0u32, 0u32);
        for _ in 0..400 {
            let seed = rand::random::<u64>();
            let game = new_session_game(&decks, &db, Rules::standard(), seed).unwrap();
            if game.turn_order[0] == 0 {
                p0 += 1;
            } else {
                p1 += 1;
            }
        }
        println!("FIRST_PLAYER_DIST p0={p0} p1={p1}");
        // A correct random first-player split is ~50/50; generous slack guards
        // against a regression to "always the same seat goes first".
        assert!(
            p0 > 120 && p1 > 120,
            "first player should vary: p0={p0} p1={p1}"
        );
    }

    // ─── engine worker thread (desktop-engine-worker-thread) ────────────

    /// `EngineHandle::run` must dispatch the closure to the worker thread —
    /// which owns the `EngineWorld` — and return its value to the awaiting
    /// caller. We prove the closure ran on the named worker thread (not the
    /// caller's) and that it observed/mutated the single-owner world.
    #[tokio::test]
    async fn engine_handle_run_dispatches_to_worker_and_returns_value() {
        let engine = EngineHandle::spawn();

        let thread_name = engine
            .run(|_world| std::thread::current().name().map(str::to_string))
            .await
            .expect("worker must reply");
        assert_eq!(
            thread_name.as_deref(),
            Some("digimon-engine"),
            "the closure must execute on the dedicated engine worker thread"
        );

        // The world persists across jobs (single owner): seed a test game in
        // one job, observe it in the next.
        engine
            .run(|world| {
                world.session.player_kinds = vec![PlayerKind::Greedy, PlayerKind::Human];
            })
            .await
            .expect("worker must reply");
        let kinds = engine
            .run(|world| world.session.player_kinds.clone())
            .await
            .expect("worker must reply");
        assert_eq!(kinds, vec![PlayerKind::Greedy, PlayerKind::Human]);
    }

    /// A panicking job must be contained (D7): the awaiting caller observes a
    /// dropped reply (`Err`) but the worker thread survives, so a subsequent
    /// `run` still succeeds.
    #[tokio::test]
    async fn engine_handle_panicking_job_errors_without_killing_worker() {
        let engine = EngineHandle::spawn();

        let err = engine
            .run(|_world| panic!("boom"))
            .await
            .expect_err("a panicked job must surface as an Err, not hang or abort");
        assert!(
            err.contains("did not reply"),
            "a panicked job drops its reply; got: {err}"
        );

        // The worker is still alive — the next job runs normally.
        let ok = engine
            .run(|_world| 7u32)
            .await
            .expect("worker survived panic");
        assert_eq!(ok, 7);
    }

    #[test]
    fn dto_roundtrip_through_json() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
        game.start_game();
        let dto = game_state_dto(&game);
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"turn_count\":1"));
        let parsed: GameStateDto = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.turn_count, 1);
        assert_eq!(parsed.players.len(), 2);
    }

    /// Regression: the desktop DTO previously omitted `kind`, so EVERY own- and
    /// opponent-field selection was unclickable in the Tauri app (the frontend
    /// routes board clicks via `isFieldSelectionKind(pendingSelection.kind)`,
    /// and `isFieldSelectionKind(undefined)` is false). Drive a real
    /// opponent-field selection (ST1-15 Giga Destroyer) and assert the
    /// engine's `SelectionKind` reaches the DTO as the string `"OppField"`.
    #[test]
    fn pending_selection_dto_threads_field_selection_kind() {
        use digimon_engine::card_data::CardData;
        use digimon_engine::debug_runner::{make_test_card, DebugRunner};

        fn opp_digimon(id: &str, dp: i32) -> CardData {
            let mut c = make_test_card(id, id);
            c.card_kind = CardKind::Digimon;
            c.colors = vec![CardColor::Red];
            c.level = Some(3);
            c.dp = Some(dp);
            c.play_cost = 3;
            c
        }

        let mut runner = DebugRunner::builder()
            .dsl_card("ST1-15")
            .expect("ST1-15 in embedded DSL pack")
            .add_card(opp_digimon("OPP-SMALL", 2000))
            .hand(0, &["ST1-15"])
            .memory(10)
            .start();
        runner.place_on_field(1, "OPP-SMALL", Some(0));
        runner.play(0, 0).expect("play Giga Destroyer from hand");

        let dto = pending_selection_dto(&runner.game)
            .expect("Giga Destroyer installs an opponent-field selection");
        assert_eq!(
            dto.kind, "OppField",
            "field-selection kind must reach the desktop DTO so the frontend can route board clicks"
        );
    }

    /// The source picker needs each permanent's non-top digivolution cards (in
    /// `encode_source_select` order) — the desktop DTO previously exposed only
    /// `stack_size`, so the frontend had no card to render for a
    /// `select_material` pick (ST2-15 Kaiser Nail "play 1 source from under 1
    /// of your Digimon").
    #[test]
    fn perm_dto_exposes_non_top_sources_in_encode_order() {
        use digimon_engine::debug_runner::{make_test_card, DebugRunner};

        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("SRC-A", "Source A"))
            .add_card(make_test_card("SRC-B", "Source B"))
            .add_card(make_test_card("TOP-C", "Top C"))
            .start();
        // place_stack is bottom→top: [SRC-A, SRC-B] are sources, TOP-C is the
        // active top card (must be excluded).
        runner.place_stack(0, &["SRC-A", "SRC-B", "TOP-C"]);

        let dto = perm_dto(&runner.game, 0, 0);
        let ids: Vec<&str> = dto.sources.iter().map(|s| s.card_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["SRC-A", "SRC-B"],
            "non-top sources, bottom→top order matching encode_source_select; top card excluded"
        );
    }

    /// The desktop DTO previously omitted per-permanent runtime data (keyword
    /// badges, security-attack value, modifiers, dp breakdown) that the browser
    /// `serialization::perm_data` already exposes, so the Tauri UI rendered
    /// empty keyword badges and an empty stack inspector. Assert the keyword
    /// reaches the typed DTO (mirrors `perm_data`'s `keywords` / breakdown).
    #[test]
    fn perm_dto_exposes_keywords() {
        use digimon_engine::debug_runner::{make_test_card, DebugRunner};
        use digimon_engine::enums::Keyword;

        let mut blocker = make_test_card("BLK-A", "Blocker A");
        blocker.keywords = vec![Keyword::Blocker];

        let mut runner = DebugRunner::builder().add_card(blocker).start();
        runner.place_on_field(0, "BLK-A", Some(0));

        let dto = perm_dto(&runner.game, 0, 0);
        assert!(
            dto.keywords.iter().any(|k| k == "blocker"),
            "innate Blocker must surface in the desktop DTO keywords: {:?}",
            dto.keywords
        );
        assert!(
            dto.keyword_breakdown.innate.iter().any(|k| k == "blocker"),
            "an innate (printed) keyword belongs in the innate breakdown, not gained"
        );
        assert!(
            !dto.keyword_breakdown.gained.iter().any(|k| k == "blocker"),
            "printed keyword is not a modifier-granted (gained) keyword"
        );
    }

    /// The egg-deck count and per-card digivolution costs are exposed by the
    /// browser path but were dropped on desktop (empty egg-deck count, missing
    /// digivolve costs in hand). Assert both reach the typed DTO.
    #[test]
    fn player_and_card_dto_expose_egg_deck_and_evo_costs() {
        use digimon_engine::card_data::EvoCost;
        use digimon_engine::card_source::CardSource;
        use digimon_engine::debug_runner::{make_test_card, DebugRunner};

        let mut evo_card = make_test_card("EVO-A", "Evo A");
        evo_card.evo_costs = vec![EvoCost {
            card_color: 1,
            level: 3,
            memory_cost: 2,
        }];

        let mut runner = DebugRunner::builder().add_card(evo_card).start();
        // Seed the digitama (egg) deck so the count is non-zero.
        let next = runner.game.next_card_index();
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "EVO-A")
            .unwrap();
        runner.game.players[0]
            .digitama_deck
            .push(CardSource::new(idx, 0, next));

        let dto = player_dto(&runner.game, 0);
        assert_eq!(
            dto.egg_deck_count, 1,
            "egg-deck count must mirror digitama_deck.len()"
        );

        let source = CardSource::new(idx, 0, 0);
        let cdto = card_dto(&source, &runner.game.card_data);
        assert_eq!(cdto.evo_costs.len(), 1);
        assert_eq!(cdto.evo_costs[0].color, 1);
        assert_eq!(cdto.evo_costs[0].level, 3);
        assert_eq!(cdto.evo_costs[0].cost, 2);
    }

    /// The trash is a PUBLIC zone — the desktop DTO previously exposed only
    /// `trash_count`, so the trash always rendered empty even as deletions
    /// piled cards in. Assert the cards themselves reach the DTO.
    #[test]
    fn player_dto_exposes_trash_cards_not_just_count() {
        use digimon_engine::card_source::CardSource;
        use digimon_engine::debug_runner::{make_test_card, DebugRunner};

        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TRASHED-A", "Trashed A"))
            .add_card(make_test_card("TRASHED-B", "Trashed B"))
            .start();
        let next_a = runner.game.next_card_index();
        let idx_a = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TRASHED-A")
            .unwrap();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx_a, 0, next_a));
        let next_b = runner.game.next_card_index();
        let idx_b = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TRASHED-B")
            .unwrap();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx_b, 0, next_b));

        let dto = player_dto(&runner.game, 0);
        let ids: Vec<&str> = dto.trash.iter().map(|c| c.card_id.as_str()).collect();
        assert_eq!(ids, vec!["TRASHED-A", "TRASHED-B"]);
        assert_eq!(dto.trash_count, 2, "count stays consistent with the cards");
    }

    /// Track C synth-profile fields (`ace_overflow`, `digixros_aliases`) must
    /// propagate from `CardData` into `CardDto`. Regression for the drift PR
    /// #457 flagged: the engine grew the fields, the Tauri builder didn't,
    /// and `cargo build --manifest-path code/src-tauri/Cargo.toml` failed.
    #[test]
    fn card_dto_propagates_synth_profile_fields() {
        use digimon_engine::card_source::CardSource;
        let mut card = synth_card(
            "ACE-XROS",
            "Ace Xros Test",
            CardKind::Digimon,
            Some(7000),
            6,
        );
        card.ace_overflow = Some(3);
        card.digixros_aliases = vec!["Greymon".to_string(), "MetalGreymon".to_string()];

        let data: Vec<CardData> = vec![card];
        let source = CardSource::new(
            /* data_index */ 0, /* owner */ 0, /* card_index */ 0,
        );
        let dto = card_dto(&source, &data);

        assert_eq!(dto.ace_overflow, Some(3));
        assert_eq!(
            dto.digixros_aliases,
            vec!["Greymon".to_string(), "MetalGreymon".to_string()]
        );

        // JSON round-trip — frontend relies on this envelope shape.
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"ace_overflow\":3"), "got {json}");
        assert!(json.contains("\"digixros_aliases\""), "got {json}");
        let parsed: CardDto = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ace_overflow, Some(3));
        assert_eq!(parsed.digixros_aliases.len(), 2);
    }

    #[test]
    fn test_deck_is_legal_size() {
        let deck = test_deck();
        let mains = deck.iter().filter(|c| !c.starts_with("EGG")).count();
        let eggs = deck.iter().filter(|c| c.starts_with("EGG")).count();
        assert_eq!(mains, 50);
        assert_eq!(eggs, 5);
    }

    // Rust-side mirror of the Tauri command logic: drives `decode_action`
    // directly on a `Game`. The Tauri handlers themselves need a runtime
    // State<>, which is awkward to fabricate in a unit test — but the
    // interesting logic (dispatch + mask rebuild + envelope) can be exercised
    // directly.
    #[test]
    fn rust_submit_action_dispatches_and_rebuilds_mask() {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
        game.start_game();

        // Drive through any initial mulligan decisions to reach a playable phase.
        while let Some(p) = game.mulligan_current_player() {
            game.accept_mulligan(p, /* keep */ true).unwrap();
        }

        // Submit a PASS action via decode_action — the same path
        // rust_submit_action takes — and confirm the envelope round-trips.
        let pid = current_decision_player(&game);
        game.decode_action(digimon_engine::action::space::PASS, pid);

        let mask = action_mask_bytes(&game);
        let dto = game_state_dto(&game);
        let resp = ActionResponseDto {
            state: dto,
            action_mask: mask,
            is_game_over: game.game_over,
            logs: Vec::new(),
            events: Vec::<GameEventDto>::new(),
            action_context: serde_json::json!({}),
            action_traces: Vec::new(),
            agent_pending: false,
        };

        assert_eq!(
            resp.action_mask.len(),
            digimon_engine::action::space::ACTION_SPACE_SIZE
        );
        assert!(!resp.is_game_over);
        // Serde round-trip — frontend relies on this envelope shape.
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"action_mask\""));
        assert!(json.contains("\"events\""));
        assert!(json.contains("\"is_game_over\":false"));
        // Cross-wire contract: the pacing flag serializes under the same
        // snake_case key the browser wire emits.
        assert!(json.contains("\"agent_pending\":false"));
    }

    // ─── agent step loop ───────────────────────────────────────────────
    //
    // The Tauri command wrappers take `tauri::State<'_, _>` which we can't
    // construct in a unit test, so these tests exercise the internal
    // `run_agent_steps` entry point directly with real state objects.

    use digimon_engine::inference::{InferenceError, OnnxPolicy};

    /// Mock policy that always returns a fixed action (or the first valid
    /// action if the preferred one is masked off). Lets us test the Trained
    /// branch without an engine-shape .onnx file.
    struct FixedActionPolicy {
        preferred: usize,
        reset_count: std::cell::Cell<usize>,
        predict_count: std::cell::Cell<usize>,
    }

    impl FixedActionPolicy {
        fn new(preferred: usize) -> Self {
            Self {
                preferred,
                reset_count: std::cell::Cell::new(0),
                predict_count: std::cell::Cell::new(0),
            }
        }
    }

    impl OnnxPolicy for FixedActionPolicy {
        fn predict(&mut self, _obs: &[f32], mask: &[f32]) -> Result<usize, InferenceError> {
            self.predict_count.set(self.predict_count.get() + 1);
            if mask.get(self.preferred).copied().unwrap_or(0.0) > 0.0 {
                Ok(self.preferred)
            } else {
                // Fall back to first valid — matches how a real policy behaves
                // after masking kills its preferred action's logit.
                Ok(mask.iter().position(|&v| v > 0.0).unwrap_or(0))
            }
        }
        fn reset(&mut self) {
            self.reset_count.set(self.reset_count.get() + 1);
        }
    }

    fn build_playable_game() -> (Game, CardRegistry) {
        let db = test_card_db();
        let decks = vec![test_deck(), test_deck()];
        let mut game = Game::new(&decks, &db, Rules::standard(), Some(42)).unwrap();
        game.start_game();
        while let Some(p) = game.mulligan_current_player() {
            game.accept_mulligan(p, /* keep */ true).unwrap();
        }
        let registry = CardRegistry::from_cards(&db);
        (game, registry)
    }

    #[test]
    fn tensor_summary_reports_engine_contract() {
        let (game, registry) = build_playable_game();
        let pid = current_decision_player(&game);
        let mask = digimon_engine::action::build_action_mask(&game, pid);
        let summary = tensor_summary_for(&game, pid, &registry, &mask);
        let profile = default_profile();

        assert_eq!(summary.player_id, pid);
        assert_eq!(summary.profile_id, profile.id);
        // After `flip-engine-default-to-lite-deck-v2`, the default profile is
        // `standard_lite_deck_v2` (version 2, 8850 floats).
        assert_eq!(summary.profile_id, "standard_lite_deck_v2");
        assert_eq!(summary.profile_version, profile.version as u16);
        assert_eq!(summary.tensor_size, digimon_engine::tensor::TENSOR_SIZE);
        assert_eq!(summary.tensor_size, 8850);
        assert_eq!(
            summary.mask_size,
            digimon_engine::action::space::ACTION_SPACE_SIZE
        );
        assert_eq!(summary.card_id_slot_count, profile.card_id_slot_count);
        assert_eq!(summary.scalar_slot_count, profile.scalar_slot_count);
        assert_eq!(
            summary.card_id_slot_count + summary.scalar_slot_count,
            summary.tensor_size
        );
        assert!(summary.legal_action_count > 0);
        assert_eq!(summary.phase, format!("{:?}", game.current_phase));
    }

    #[test]
    fn action_trace_serializes_human_action_context() {
        let (mut game, registry) = build_playable_game();
        let pid = current_decision_player(&game);
        let mask = digimon_engine::action::build_action_mask(&game, pid);
        let trace = action_trace_for(
            &game,
            "human",
            pid,
            digimon_engine::action::space::PASS,
            Some(tensor_summary_for(&game, pid, &registry, &mask)),
        );

        assert_eq!(trace.actor, "human");
        assert_eq!(trace.player_id, pid);
        assert_eq!(trace.action_id, digimon_engine::action::space::PASS);
        assert_eq!(
            trace.decoded.kind,
            digimon_engine::action::explain::ActionKind::Pass
        );
        assert!(trace.tensor_summary.is_some());

        let json = serde_json::to_string(&trace).unwrap();
        assert!(json.contains("\"actor\":\"human\""));
        // After `flip-engine-default-to-lite-deck-v2` the engine reports
        // `standard_lite_deck_v2` shapes through the default surface.
        assert!(
            json.contains("\"tensor_size\":8850"),
            "expected tensor_size 8850 in trace JSON; got: {json}"
        );
        assert!(
            json.contains("\"mask_size\":2192"),
            "expected mask_size 2192 (action-space unchanged); got: {json}"
        );

        game.decode_action(digimon_engine::action::space::PASS, pid);
    }

    #[test]
    fn action_response_includes_human_trace() {
        let (mut game, registry) = build_playable_game();
        let pid = current_decision_player(&game);
        let mask_before = digimon_engine::action::build_action_mask(&game, pid);
        let action = digimon_engine::action::space::PASS;
        let human_trace = action_trace_for(
            &game,
            "human",
            pid,
            action,
            Some(tensor_summary_for(&game, pid, &registry, &mask_before)),
        );
        game.decode_action(action, pid);

        let resp = ActionResponseDto {
            state: game_state_dto(&game),
            action_mask: action_mask_bytes(&game),
            is_game_over: game.game_over,
            logs: Vec::new(),
            events: Vec::<GameEventDto>::new(),
            action_context: serde_json::json!({}),
            action_traces: vec![human_trace],
            agent_pending: false,
        };

        assert_eq!(resp.action_traces.len(), 1);
        assert_eq!(resp.action_traces[0].actor, "human");
        assert_eq!(resp.action_traces[0].action_id, action);
    }

    #[test]
    fn drain_events_converts_engine_events_and_is_one_shot() {
        use digimon_engine::events::GameEvent;
        use digimon_engine::game::TerminalOutcomeReason;

        let (mut game, _registry) = build_playable_game();
        // Game setup (`start_game` → turn-1 `begin_turn`) legitimately emits its
        // own events (`TurnStart`, the initial `MemoryChange`, …). Drain them
        // first so this test isolates conversion of the four events it pushes
        // below — otherwise the assertion counts setup noise.
        let _ = drain_events(&mut game);
        let memory_seq = game.next_event_seq();
        game.events.push(GameEvent::MemoryChange {
            seq: memory_seq,
            player: 0,
            delta: -3,
            total: 0,
            source_card_id: Some("BT25-098".to_string()),
            source_card_name: Some("Cyber Engage".to_string()),
        });
        let play_seq = game.next_event_seq();
        game.events.push(GameEvent::Play {
            seq: play_seq,
            player: 0,
            card_id: "BT1-001".to_string(),
            card_name: "Agumon".to_string(),
            field_index: 0,
            cost_paid: 3,
            cost_printed: 3,
            via_alt_path: None,
        });
        let attack_seq = game.next_event_seq();
        game.events.push(GameEvent::Attack {
            seq: attack_seq,
            player: 0,
            attacker_field_index: 0,
            target_field_index: Some(1),
            target_player: Some(1),
            attacker_card_id: "BT1-009".to_string(),
            attacker_card_name: "Greymon".to_string(),
            target_card_id: Some("BT25-020".to_string()),
            target_card_name: Some("Tyrannomon".to_string()),
            attacker_dp: Some(5000),
            target_dp: Some(3000),
        });
        let game_over_seq = game.next_event_seq();
        game.events.push(GameEvent::GameOver {
            seq: game_over_seq,
            winner: Some(0),
            reason: TerminalOutcomeReason::SecurityAttack,
        });

        let drained = drain_events(&mut game);

        assert_eq!(drained.len(), 4);
        assert_eq!(drained[0].event_type, "MemoryChange");
        assert_eq!(drained[0].player, 1);
        assert_eq!(drained[0].meta["delta"], -3);
        // Effect-source attribution surfaces on the desktop DTO.
        assert_eq!(drained[0].source_card_id.as_deref(), Some("BT25-098"));
        assert_eq!(drained[0].source_card_name.as_deref(), Some("Cyber Engage"));
        assert_eq!(drained[1].event_type, "Play");
        assert_eq!(drained[1].source_card_id.as_deref(), Some("BT1-001"));
        assert_eq!(drained[1].source_card_name.as_deref(), Some("Agumon"));
        assert_eq!(drained[1].source_slot, Some(0));
        assert_eq!(drained[1].meta["cost_paid"], 3);
        // Attack carries attacker + target identity (no more "slot N").
        assert_eq!(drained[2].event_type, "Attack");
        assert_eq!(drained[2].source_card_id.as_deref(), Some("BT1-009"));
        assert_eq!(drained[2].source_card_name.as_deref(), Some("Greymon"));
        assert_eq!(drained[2].target_card_id.as_deref(), Some("BT25-020"));
        assert_eq!(drained[2].target_card_name.as_deref(), Some("Tyrannomon"));
        assert_eq!(drained[2].meta["attacker_dp"], 5000);
        assert_eq!(drained[2].meta["target_dp"], 3000);
        assert_eq!(drained[3].event_type, "GameOver");
        assert_eq!(drained[3].meta["winner"], 1);
        assert_eq!(drain_events(&mut game).len(), 0);
    }

    #[test]
    fn action_response_allows_human_trace_without_registry() {
        let (mut game, _registry) = build_playable_game();
        let pid = current_decision_player(&game);
        let mask_before = digimon_engine::action::build_action_mask(&game, pid);
        let action = digimon_engine::action::space::PASS;
        let human_trace = action_trace_for(
            &game,
            "human",
            pid,
            action,
            optional_tensor_summary_for(&game, pid, None, &mask_before),
        );
        game.decode_action(action, pid);

        let resp = ActionResponseDto {
            state: game_state_dto(&game),
            action_mask: action_mask_bytes(&game),
            is_game_over: game.game_over,
            logs: Vec::new(),
            events: Vec::<GameEventDto>::new(),
            action_context: serde_json::json!({}),
            action_traces: vec![human_trace],
            agent_pending: false,
        };

        assert_eq!(resp.action_traces.len(), 1);
        assert_eq!(resp.action_traces[0].actor, "human");
        assert_eq!(resp.action_traces[0].action_id, action);
        assert!(resp.action_traces[0].tensor_summary.is_none());
    }

    #[test]
    fn run_agent_steps_stops_when_current_decider_is_human() {
        let (mut game, registry) = build_playable_game();
        let session = GameSession {
            registry: Some(registry),
            player_kinds: vec![PlayerKind::Human, PlayerKind::Human],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();
        let before = (game.turn_count, game.current_phase);
        let traces = run_agent_steps(&mut game, &session, &inference, None).unwrap();
        let after = (game.turn_count, game.current_phase);
        assert!(traces.is_empty());
        assert_eq!(before, after, "human seat should not advance state");
    }

    #[test]
    fn run_agent_steps_greedy_advances_until_game_resolves_or_human_up() {
        let (mut game, registry) = build_playable_game();
        // Make both seats greedy — the loop should drive to game_over, since
        // neither side ever stops for a human decision. `MAX_AGENT_STEPS`
        // caps the loop if decode_action somehow no-ops.
        let session = GameSession {
            registry: Some(registry),
            player_kinds: vec![PlayerKind::Greedy, PlayerKind::Greedy],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();
        let traces = run_agent_steps(&mut game, &session, &inference, None).unwrap();
        assert!(!traces.is_empty());
        assert!(traces.iter().all(|trace| trace.actor.starts_with("agent_")));
        assert!(
            game.game_over,
            "two greedy agents should play the game to completion"
        );
    }

    #[test]
    fn run_agent_steps_greedy_traces_without_registry() {
        let (mut game, _registry) = build_playable_game();
        let session = GameSession {
            registry: None,
            player_kinds: vec![PlayerKind::Greedy, PlayerKind::Greedy],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();
        let traces = run_agent_steps(&mut game, &session, &inference, None).unwrap();

        assert!(!traces.is_empty());
        assert!(traces.iter().all(|trace| trace.actor == "agent_greedy"));
        assert!(traces.iter().all(|trace| trace.tensor_summary.is_none()));
        assert!(
            game.game_over,
            "two greedy agents should still play to completion without registry"
        );
    }

    // ─── paced agent stepping (add-bot-action-pacing) ───────────────────

    #[test]
    fn run_agent_steps_paced_caps_at_one_action_and_reports_pending() {
        let (mut game, registry) = build_playable_game();
        let session = GameSession {
            registry: Some(registry),
            player_kinds: vec![PlayerKind::Greedy, PlayerKind::Greedy],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();

        let traces = run_agent_steps(&mut game, &session, &inference, Some(1)).unwrap();
        assert_eq!(
            traces.len(),
            1,
            "paced advance must execute exactly one agent action"
        );
        assert!(
            agent_pending(&game, &session),
            "with both seats greedy and the game live, another agent action must be pending"
        );
    }

    #[test]
    fn paced_advances_replay_the_exact_unpaced_action_sequence() {
        // Same seed + deterministic greedy policy → the paced one-beat-at-a-
        // time path must reproduce the unpaced run's action sequence exactly,
        // and `agent_pending` must flip false only at game end.
        let session_for = |registry| GameSession {
            registry: Some(registry),
            player_kinds: vec![PlayerKind::Greedy, PlayerKind::Greedy],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();

        let (mut unpaced_game, registry_a) = build_playable_game();
        let session_a = session_for(registry_a);
        let unpaced = run_agent_steps(&mut unpaced_game, &session_a, &inference, None).unwrap();
        assert!(unpaced_game.game_over);

        let (mut paced_game, registry_b) = build_playable_game();
        let session_b = session_for(registry_b);
        let mut paced: Vec<ActionTraceDto> = Vec::new();
        // Generous safety bound — every iteration must advance one action.
        for _ in 0..20_000 {
            let step = run_agent_steps(&mut paced_game, &session_b, &inference, Some(1)).unwrap();
            assert!(
                step.len() <= 1,
                "a paced advance must never execute more than one action"
            );
            paced.extend(step);
            if !agent_pending(&paced_game, &session_b) {
                break;
            }
        }
        assert!(
            paced_game.game_over,
            "paced advances must drive a two-greedy game to completion"
        );
        assert!(!agent_pending(&paced_game, &session_b));

        let ids = |ts: &[ActionTraceDto]| ts.iter().map(|t| t.action_id).collect::<Vec<_>>();
        assert_eq!(
            ids(&paced),
            ids(&unpaced),
            "paced and unpaced runs must produce identical action sequences"
        );
    }

    #[test]
    fn agent_pending_is_false_for_human_decider_and_finished_games() {
        let (game, registry) = build_playable_game();
        let human_session = GameSession {
            registry: Some(registry),
            player_kinds: vec![PlayerKind::Human, PlayerKind::Human],
            player_model_ids: vec![None, None],
        };
        assert!(
            !agent_pending(&game, &human_session),
            "a human decision point must not report a pending agent"
        );

        let (mut over_game, registry2) = build_playable_game();
        let greedy_session = GameSession {
            registry: Some(registry2),
            player_kinds: vec![PlayerKind::Greedy, PlayerKind::Greedy],
            player_model_ids: vec![None, None],
        };
        let inference = InferenceState::default();
        run_agent_steps(&mut over_game, &greedy_session, &inference, None).unwrap();
        assert!(over_game.game_over);
        assert!(
            !agent_pending(&over_game, &greedy_session),
            "a finished game must not report a pending agent"
        );
    }

    #[test]
    fn run_agent_steps_trained_seat_consults_loaded_policy() {
        let (mut game, registry) = build_playable_game();
        let inference = InferenceState::default();

        // Player 0 starts as decider — pass until player 1 (the trained
        // seat) is up so the loop is forced into the Trained branch.
        while current_decision_player(&game) == 0 && !game.game_over {
            let pid = current_decision_player(&game);
            game.decode_action(digimon_engine::action::space::PASS, pid);
        }
        let trained_pid = current_decision_player(&game);
        assert!(
            !game.game_over,
            "test setup expected game to still be live after player-0 passes"
        );

        let policy = Box::new(FixedActionPolicy::new(0)); // always PASS
        inference.insert_for_test("bot-42", policy);

        let mut kinds = vec![PlayerKind::Human; 2];
        kinds[trained_pid as usize] = PlayerKind::Trained;
        let mut model_ids: Vec<Option<String>> = vec![None; 2];
        model_ids[trained_pid as usize] = Some("bot-42".into());
        let session = GameSession {
            registry: Some(registry),
            player_kinds: kinds,
            player_model_ids: model_ids,
        };

        let before_pid = current_decision_player(&game);
        let traces = run_agent_steps(&mut game, &session, &inference, None).unwrap();
        assert!(!traces.is_empty());
        assert!(traces.iter().all(|trace| trace.actor.starts_with("agent_")));
        // After the trained seat's turn the loop should have left us on a
        // human decider (or the game should be over).
        assert!(
            current_decision_player(&game) != before_pid || game.game_over,
            "trained policy never advanced state; decider still {before_pid}"
        );
    }

    #[test]
    fn run_agent_steps_errors_when_trained_model_is_unloaded() {
        let (mut game, registry) = build_playable_game();
        let pid = current_decision_player(&game);
        // Force player 0 (who's up) to be Trained with a model_id that
        // doesn't exist in the cache.
        let kinds = match pid {
            0 => vec![PlayerKind::Trained, PlayerKind::Human],
            _ => vec![PlayerKind::Human, PlayerKind::Trained],
        };
        let model_ids = match pid {
            0 => vec![Some("nope".into()), None],
            _ => vec![None, Some("nope".into())],
        };
        let session = GameSession {
            registry: Some(registry),
            player_kinds: kinds,
            player_model_ids: model_ids,
        };
        let inference = InferenceState::default();
        let err = run_agent_steps(&mut game, &session, &inference, None)
            .expect_err("missing model should error");
        assert!(
            err.contains("not loaded") || err.contains("nope"),
            "error message should call out the missing model, got {err:?}"
        );
    }

    #[test]
    fn normalize_player_kinds_defaults_to_all_human() {
        let kinds = normalize_player_kinds(None, 2).unwrap();
        assert_eq!(kinds, vec![PlayerKind::Human, PlayerKind::Human]);
    }

    #[test]
    fn normalize_player_kinds_rejects_length_mismatch() {
        let err = normalize_player_kinds(Some(vec![PlayerKind::Greedy]), 2).unwrap_err();
        assert!(err.contains("length"));
    }

    #[test]
    fn normalize_player_model_ids_rejects_trained_without_model() {
        let kinds = vec![PlayerKind::Trained, PlayerKind::Human];
        let err = normalize_player_model_ids(Some(vec![None, None]), 2, &kinds).unwrap_err();
        assert!(err.contains("player 0"));
        assert!(err.contains("model_id"));
    }

    #[test]
    fn parse_optional_seed_accepts_blank_and_max_u64() {
        assert_eq!(parse_optional_seed(None).unwrap(), None);
        assert_eq!(parse_optional_seed(Some("   ".to_string())).unwrap(), None);
        assert_eq!(
            parse_optional_seed(Some("18446744073709551615".to_string())).unwrap(),
            Some(u64::MAX)
        );
    }

    #[test]
    fn parse_optional_seed_rejects_invalid_values() {
        for value in ["-1", "1.5", "seed", "18446744073709551616"] {
            let err = parse_optional_seed(Some(value.to_string())).unwrap_err();
            assert!(
                err.contains("base-10") || err.contains("u64"),
                "unexpected error for {value}: {err}"
            );
        }
    }

    #[test]
    fn validate_shapes_rejects_wrong_obs_and_mask() {
        let ok_obs = vec![0.0f32; digimon_engine::tensor::TENSOR_SIZE];
        let ok_mask = vec![0.0f32; digimon_engine::action::space::ACTION_SPACE_SIZE];
        assert!(validate_shapes(&ok_obs, &ok_mask, "m").is_ok());

        let short_obs = vec![0.0f32; 10];
        assert!(validate_shapes(&short_obs, &ok_mask, "m").is_err());

        let short_mask = vec![0.0f32; 10];
        assert!(validate_shapes(&ok_obs, &short_mask, "m").is_err());
    }
}
