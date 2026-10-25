//! The normalized per-step state projection, plus the parser for the DCGO
//! `StateDumper` sidecar that produces the other side of the comparison.
//!
//! # The governing rule: normalize representation, never semantics
//!
//! Both engines describe the same board, but they *write it down* differently:
//! zones come out in different orders, DP is stored printed-plus-modifiers on
//! one side and as a computed property on the other. Every one of those
//! differences is **representation**, and a differ that reports them produces
//! noise that buries the one real finding.
//!
//! So this projection normalizes representation aggressively:
//!
//! - `hand`, `trash` and `sources` are **sorted on construction**, which makes
//!   the derived `PartialEq` a multiset compare with no custom comparator that
//!   could drift away from the constructor.
//! - `field` is sorted by `(card_id, dp, suspended, sources)` — deliberately
//!   *not* by keywords, so an unmeasured keyword list can never reorder a side.
//!
//! and it normalizes **semantics** not at all:
//!
//! - Duplicate counts still matter. This is a multiset, not a set: holding two
//!   copies of a card is not the same board as holding one.
//! - `suspended` is semantics and must diff.
//! - `dp` is **effective** DP, after modifiers — the printed value would make
//!   every buffed Digimon read as a divergence. `-1` means "no DP" (a Tamer or
//!   an Option), matching DCGO's `Permanent.DP` which returns `-1` when
//!   `!HasDP`.
//! - `security` is a **count** only. The contents are hidden information, and
//!   diffing them would let the differ "confirm" a line whose legality
//!   depended on knowing them.
//!
//! # Keywords are `Option`: `None` means NOT MEASURED
//!
//! DCGO has no keyword collection — it has 22 independent bool properties
//! (`HasBlocker`, `HasJamming`, …), each of which walks the whole field's
//! effect lists. Dumping all of them per permanent per step is
//! `O(22 x field x effects)`, so the DCGO-side dumper puts keywords behind a
//! flag that defaults **off** and the sidecar usually omits them.
//!
//! An omitted keyword list therefore means *"nobody looked"*, not *"this
//! Digimon has no keywords"*. Modelling it as an empty `Vec` would make every
//! keyword-carrying permanent in every keyword-flag-off run report a false
//! divergence. It is an `Option<Vec<String>>`, and `None` compares equal to
//! any `Some(..)` — see [`PermanentProjection`]'s hand-written `PartialEq`.
//!
//! Because of that wildcard, equality here is a **compatibility relation**:
//! reflexive and symmetric but deliberately *not* transitive
//! (`Some(["Blocker"]) == None` and `None == Some(["Rush"])`, yet the two
//! `Some`s differ). That is why these types implement `PartialEq` but
//! **not** `Eq` — claiming `Eq` would assert a total equivalence the type does
//! not have, and would be unsound to rely on in a `HashMap` key.

use serde::{Deserialize, Serialize};

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::memory_from_recording_perspective;
use digimon_engine::enums::{Keyword, PlayerId};
use digimon_engine::{Game, PermanentHandle};

/// The keyword vocabulary shared with the DCGO `StateDumper`.
///
/// Deliberately *not* every `Keyword` our engine knows: the DCGO side can only
/// report the keywords it has a `Has…` bool property for, so probing anything
/// outside this set would produce a value the other side can never match. The
/// string on the left is the wire name (DCGO's getter minus its `Has` prefix);
/// the `Keyword` on the right is our engine's equivalent. Note `Pierce` /
/// `Piercing` — the two engines spell that one differently.
pub const KEYWORD_PROBES: &[(&str, Keyword)] = &[
    ("Ascension", Keyword::Ascension),
    ("Blitz", Keyword::Blitz),
    ("Blocker", Keyword::Blocker),
    ("Engage", Keyword::Engage),
    ("Evade", Keyword::Evade),
    ("Fortitude", Keyword::Fortitude),
    ("Guard", Keyword::Guard),
    ("Iceclad", Keyword::Iceclad),
    ("Jamming", Keyword::Jamming),
    ("MindLink", Keyword::MindLink),
    ("Pierce", Keyword::Piercing),
    ("Raid", Keyword::Raid),
    ("Reboot", Keyword::Reboot),
    ("Retaliation", Keyword::Retaliation),
    ("Rush", Keyword::Rush),
];

/// One permanent on the battle area, normalized.
///
/// `card_id` is the **top** card of the digivolution stack; `sources` is
/// everything underneath it (so a freshly played Digimon has an empty
/// `sources`). `dp` is effective DP, `-1` when the permanent has no DP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermanentProjection {
    pub card_id: String,
    pub dp: i64,
    pub suspended: bool,
    #[serde(default)]
    pub sources: Vec<String>,
    /// `None` = not measured (the DCGO dumper's keyword flag was off).
    /// Never compares as a divergence — see the module docs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
}

impl PermanentProjection {
    fn normalize(&mut self) {
        self.sources.sort();
        if let Some(kw) = self.keywords.as_mut() {
            kw.sort();
            kw.dedup();
        }
    }

    /// The sort key used to order a seat's field. Excludes `keywords` on
    /// purpose: an unmeasured list must not be able to reorder one side
    /// relative to the other.
    fn sort_key(&self) -> (&str, i64, bool, &[String]) {
        (&self.card_id, self.dp, self.suspended, &self.sources)
    }
}

/// Hand-written so an unmeasured keyword list (`None`) is a wildcard rather
/// than "no keywords". Every other field compares exactly.
impl PartialEq for PermanentProjection {
    fn eq(&self, other: &Self) -> bool {
        if self.card_id != other.card_id
            || self.dp != other.dp
            || self.suspended != other.suspended
            || self.sources != other.sources
        {
            return false;
        }
        match (&self.keywords, &other.keywords) {
            // At least one side did not measure keywords — not a divergence.
            (None, _) | (_, None) => true,
            (Some(a), Some(b)) => a == b,
        }
    }
}

/// One player's visible state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatProjection {
    /// A count, never the contents — security is hidden information.
    pub security: usize,
    pub hand: Vec<String>,
    pub trash: Vec<String>,
    pub field: Vec<PermanentProjection>,
    /// None means not measured; only an actual Main decision can measure this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legal_option_plays: Option<Vec<String>>,
    /// None means no live selection was measured for this seat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_can_decline: Option<bool>,
    /// Exact offered card multiset; None means no supported live observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_candidates: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub movement_observations: Option<
        std::collections::BTreeMap<String, digimon_engine::movement_observation::MovementWatch>,
    >,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub movement_audit: Vec<digimon_engine::movement_observation::MovementAudit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub movement_observation_at: Option<u32>,
}

impl PartialEq for SeatProjection {
    fn eq(&self, other: &Self) -> bool {
        // Recorder handles and native decision indices are checked for continuity
        // within each trace; they have no shared identity across engines.
        self.security == other.security
            && self.hand == other.hand
            && self.trash == other.trash
            && self.field == other.field
            && self.legal_option_plays == other.legal_option_plays
            && self.selection_can_decline == other.selection_can_decline
            && self.selection_candidates == other.selection_candidates
            && self.movement_observations == other.movement_observations
    }
}

impl SeatProjection {
    fn normalize(&mut self) {
        if let Some(watches) = &mut self.movement_observations {
            watches.retain(|_, v| v.is_valid_claim());
        }
        if let Some(ids) = &mut self.legal_option_plays {
            ids.sort();
        }
        if let Some(ids) = &mut self.selection_candidates {
            ids.sort();
        }
        self.hand.sort();
        self.trash.sort();
        for perm in &mut self.field {
            perm.normalize();
        }
        // Stable sort: two permanents identical apart from keywords keep
        // their relative order rather than being shuffled by a tie-break the
        // other side cannot reproduce.
        self.field.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    }

    fn from_game(game: &Game, pid: PlayerId, measure_keywords: bool, step: u32) -> Self {
        let data: &[CardData] = &game.card_data;
        let player = game.player(pid);

        let field = player
            .battle_area
            .iter()
            .enumerate()
            .map(|(index, perm)| {
                let handle = PermanentHandle {
                    player: pid,
                    index: index as u8,
                };
                // `split_last` rather than indexing: the top card is the last
                // source, everything before it is the digivolution stack.
                let (top, beneath) = perm
                    .card_sources
                    .split_last()
                    .expect("a permanent always has at least its top card");
                PermanentProjection {
                    card_id: top.card_id(data).to_string(),
                    // DCGO's `Permanent.DP` yields -1 when the permanent has
                    // no DP; `effective_dp` yields None for the same case.
                    dp: game.effective_dp(handle).map(i64::from).unwrap_or(-1),
                    suspended: perm.is_suspended,
                    sources: beneath
                        .iter()
                        .map(|src| src.card_id(data).to_string())
                        .collect(),
                    keywords: measure_keywords.then(|| {
                        KEYWORD_PROBES
                            .iter()
                            .filter(|(_, kw)| game.has_keyword(handle, *kw))
                            .map(|(name, _)| (*name).to_string())
                            .collect()
                    }),
                }
            })
            .collect();

        let mut seat = SeatProjection {
            security: player.security.len(),
            hand: player
                .hand
                .iter()
                .map(|src| src.card_id(data).to_string())
                .collect(),
            trash: player
                .trash
                .iter()
                .map(|src| src.card_id(data).to_string())
                .collect(),
            field,
            legal_option_plays: None,
            selection_can_decline: None,
            movement_observations: Some(
                game.observe_public_movement_at(step)[pid as usize].clone(),
            ),
            movement_audit: game.movement_audit(pid),
            movement_observation_at: Some(step),
            selection_candidates: None,
        };
        seat.normalize();
        seat
    }
}

/// The whole board after one step, in the one shape both engines emit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateProjection {
    pub step: u32,
    pub turn: u64,
    pub phase: String,
    /// Memory **from player 0's perspective**: positive favors p0.
    ///
    /// Our engine stores the seesaw relative to `memory_pair.0`, which flips
    /// every turn; DCGO stores it relative to a fixed PlayerID. Comparing the
    /// raw numbers silently inverts on alternating turns, so both sides are
    /// pinned to p0 here via the engine's own conversion.
    pub memory: i64,
    pub p0: SeatProjection,
    pub p1: SeatProjection,
}

impl StateProjection {
    fn normalize(&mut self) {
        self.p0.normalize();
        self.p1.normalize();
    }

    /// Project our engine's live state. Keywords are left **unmeasured**
    /// (`None`), matching the DCGO dumper's default; use
    /// [`StateProjection::from_game_with_keywords`] for a scenario whose
    /// clause under test is a keyword clause.
    pub fn from_game(game: &Game, step: u32) -> StateProjection {
        Self::project(game, step, false)
    }

    /// Same, but probes [`KEYWORD_PROBES`] on every permanent. Only worth
    /// paying for when the DCGO side also ran with its keyword flag on —
    /// otherwise the extra work buys a wildcard comparison anyway.
    pub fn from_game_with_keywords(game: &Game, step: u32) -> StateProjection {
        Self::project(game, step, true)
    }

    fn project(game: &Game, step: u32, measure_keywords: bool) -> StateProjection {
        StateProjection {
            step,
            turn: u64::from(game.turn_count),
            phase: game.current_phase.py_name().to_string(),
            memory: i64::from(memory_from_recording_perspective(
                game.memory,
                game.memory_pair.0,
                0,
            )),
            p0: SeatProjection::from_game(game, 0, measure_keywords, step),
            p1: SeatProjection::from_game(game, 1, measure_keywords, step),
        }
    }

    /// Opt-in exam observation. Legality comes exclusively from the real mask;
    /// phase alone is insufficient because pending prompts can retain Main.
    pub fn measure_legal_option_plays(&mut self, game: &Game, requested: [bool; 2]) {
        use digimon_engine::action::{
            mask::build_action_mask,
            space::{PLAY_HAND_END, PLAY_HAND_START},
        };
        use digimon_engine::enums::{CardKind, GamePhase};
        if game.game_over
            || game.current_phase != GamePhase::Main
            || game.pending_selection.is_some()
            || game.mulligan_current_player().is_some()
        {
            return;
        }
        let actor = digimon_engine::policies::current_decision_player(game);
        if !requested[actor as usize] {
            return;
        }
        let mask = build_action_mask(game, actor);
        let mut ids: Vec<String> = game
            .player(actor)
            .hand
            .iter()
            .enumerate()
            .filter(|(i, card)| {
                let action = PLAY_HAND_START as usize + i;
                action < PLAY_HAND_END as usize
                    && mask[action] > 0.5
                    && card.card_kind(&game.card_data) == CardKind::Option
            })
            .map(|(_, card)| card.card_id(&game.card_data).to_string())
            .collect();
        ids.sort();
        let seat = if actor == 0 {
            &mut self.p0
        } else {
            &mut self.p1
        };
        seat.legal_option_plays = Some(ids);
    }

    /// Observe the actual pending choice's PASS capability, never the nominal
    /// turn phase or merely the selection's optional flag (PASS may be explicit).
    pub fn measure_selection_can_decline(&mut self, game: &Game, requested: [bool; 2]) {
        self.p0.selection_can_decline = None;
        self.p1.selection_can_decline = None;
        use digimon_engine::action::{mask::build_action_mask, space::PASS};
        if game.game_over {
            return;
        }
        let Some(selection) = game.pending_selection.as_ref() else {
            return;
        };
        let actor = selection.selecting_player;
        if !requested[actor as usize] {
            return;
        }
        let mask = build_action_mask(game, actor);
        let seat = if actor == 0 {
            &mut self.p0
        } else {
            &mut self.p1
        };
        seat.selection_can_decline = Some(mask[PASS as usize] > 0.5);
    }

    /// Exact single source choices use the installed physical snapshot, never
    /// action-range guesses about owner or host. Unsupported/stale is unknown.
    fn single_source_candidate_ids(game: &Game) -> Option<Vec<String>> {
        use digimon_engine::action::{
            mask::build_action_mask,
            space::{encode_source_select, PASS},
        };
        use digimon_engine::resume::ResumeFrame;
        use digimon_engine::selection::SelectionKind;
        use std::collections::HashSet;
        let pending = game.pending_selection.as_ref()?;
        let SelectionKind::SourceMulti {
            min,
            max: 1,
            picked: 0,
        } = pending.kind
        else {
            return None;
        };
        let ResumeFrame::SourceMultiStep(state) =
            game.pending_selection_resume.as_ref()?.frames.last()?
        else {
            return None;
        };
        if pending.selecting_player > 1
            || state.of_player > 1
            || state.selecting_player != pending.selecting_player
            || state.min != min
            || state.max != 1
            || !state.picked.is_empty()
            || state.target_resolution_failed
        {
            return None;
        }
        use digimon_engine::{card_choice_page::CardChoiceResult, owned_card_choice::{CardChoiceOperation, ResolvedCardChoice}};
        // Validate captured physical cards before page views or predicate masks
        // dereference registry indices. Corrupt/unmeasured metadata stays unknown.
        for source in &state.candidates {
            if source.permanent.player != state.of_player || source.field_index != source.permanent.index { return None; }
            let player=game.players.get(source.permanent.player as usize)?;
            let host=if source.permanent.index == digimon_engine::action::space::BREEDING_TARGET as u8 {
                player.breeding_area.as_ref()?
            } else { player.battle_area.get(source.permanent.index as usize)? };
            if source.source_index+1 >= host.card_sources.len() { return None; }
            let card=host.card_sources.get(source.source_index)?;
            if card.handle()!=source.card || game.card_data.get(card.data_index)?.card_id.trim().is_empty() { return None; }
        }
        let choice = pending.card_choice.as_ref()?;
        if pending.valid_action_ids != choice.valid_action_ids(game) { return None; }
        let mut page = choice.clone();
        let mut identities = Vec::new();
        let mut physical = HashSet::new();
        let mut snapshot = HashSet::new();
        for source in &state.candidates {
            if !snapshot.insert(source.card) { return None; }
        }
        loop {
            let view = page.view(game, pending.selecting_player);
            let mask = page.valid_action_ids(game);
            for row in &view.rows {
                if !matches!(row.operation, CardChoiceOperation::CommitCard) { continue; }
                if !mask.contains(&row.action_id) { return None; }
                let CardChoiceResult::Selected(ResolvedCardChoice::Source { host, index, card: handle }) =
                    page.clone().resolve(game, row.action_id).ok()? else { return None; };
                let source = state.candidates.iter().find(|s| s.card == handle)?;
                if source.permanent != host || source.source_index != index
                    || source.field_index != host.index || host.player != state.of_player
                    || !physical.insert(handle) { return None; }
                let player=game.players.get(host.player as usize)?;
                let host=if host.index == digimon_engine::action::space::BREEDING_TARGET as u8 { player.breeding_area.as_ref()? } else { player.battle_area.get(host.index as usize)? };
                if index + 1 >= host.card_sources.len() { return None; }
                let card = host.card_sources.get(index)?;
                if card.handle() != handle { return None; }
                let data = game.card_data.get(card.data_index)?;
                if data.card_id.trim().is_empty() { return None; }
                identities.push(data.card_id.clone());
            }
            let Some(next) = view.rows.iter().find(|r| matches!(r.operation, CardChoiceOperation::NextPage) && mask.contains(&r.action_id)) else { break; };
            if !matches!(page.resolve(game, next.action_id).ok()?, CardChoiceResult::Navigated) { return None; }
        }
        if physical != snapshot { return None; }
        identities.sort();
        Some(identities)
    }

    /// Supports Hand and captured single-source picks only. A requested unsupported prompt
    /// or undecodable candidate stays unknown, never an inferred empty list.
    pub fn measure_selection_candidates(&mut self, game: &Game, requested: [bool; 2]) {
        self.p0.selection_candidates = None;
        self.p1.selection_candidates = None;
        use digimon_engine::action::{
            mask::build_action_mask,
            space::{PASS, PLAY_HAND_END, PLAY_HAND_START},
        };
        use digimon_engine::selection::SelectionKind;
        if game.game_over {
            return;
        }
        let Some(pending) = game.pending_selection.as_ref() else {
            return;
        };
        if matches!(pending.kind, SelectionKind::SourceMulti { .. }) {
            let actor = pending.selecting_player;
            if actor > 1 || !requested[actor as usize] {
                return;
            }
            let ids = Self::single_source_candidate_ids(game);
            let seat = if actor == 0 {
                &mut self.p0
            } else {
                &mut self.p1
            };
            seat.selection_candidates = ids;
            return;
        }
        if pending.kind != SelectionKind::Hand {
            return;
        }
        let actor = pending.selecting_player;
        if !requested[actor as usize] {
            return;
        }
        let mask = build_action_mask(game, actor);
        let hand = &game.player(pending.zone_owner.unwrap_or(actor)).hand;
        let mut ids = Vec::new();
        for &id in &pending.valid_action_ids {
            if id == PASS {
                continue;
            }
            let Some(&legal) = mask.get(id as usize) else {
                return;
            };
            if legal <= 0.5 {
                continue;
            }
            if !(PLAY_HAND_START..PLAY_HAND_END).contains(&id) {
                return;
            }
            let Some(card) = hand.get((id - PLAY_HAND_START) as usize) else {
                return;
            };
            let Some(data) = game.card_data.get(card.data_index) else {
                return;
            };
            if data.card_id.trim().is_empty() {
                return;
            }
            ids.push(data.card_id.clone());
        }
        ids.sort();
        let seat = if actor == 0 {
            &mut self.p0
        } else {
            &mut self.p1
        };
        seat.selection_candidates = Some(ids);
    }

    /// Parse one `.state.jsonl` row. Trailing content after the JSON value is
    /// an error rather than being ignored.
    pub fn from_sidecar_line(line: &str) -> Result<StateProjection, String> {
        let mut projection: StateProjection =
            serde_json::from_str(line).map_err(|e| format!("malformed state row: {e}"))?;
        projection.normalize();
        Ok(projection)
    }
}

/// Parse a whole `.state.jsonl` sidecar.
///
/// Rows are whitespace-separated JSON values, so a pretty-printed row that
/// wraps across lines parses as one row. A row that will not parse is a hard
/// error: silently dropping it would shorten one side of the diff and
/// misalign every step after it, turning a parse bug into a phantom
/// engine divergence.
/// Drop the DCGO sidecar rows that precede the scenario's own origin.
///
/// A scenario line begins AFTER the mulligan: `ScenarioAdapter` resolves both
/// mulligans itself (keep) and DCGO is pinned to match, consuming no scripted
/// step for it. But DCGO still RECORDS the mulligan as ordinary `action` rows,
/// and `StateDumper` dumps state at those steps -- so the sidecar carries two
/// leading rows our trace has no counterpart for.
///
/// The differ aligns by step index, so without this the two traces sit two
/// steps apart and EVERY scenario reports a divergence at step 0 with
/// `security: ours=5 dcgo=0` (DCGO's step 0 is pre-mulligan, before security is
/// laid). That is an artifact of the harness, not a finding about the card, and
/// it would drown every real divergence beneath it.
///
/// Takes the recording alongside the sidecar because `source: "mulligan"` lives
/// on the `action` row, not in the state dump -- the sidecar's `phase` field
/// reads "Active" at that point and cannot identify it.
///
/// Returns the sidecar re-indexed from 0 so both traces share an origin.
pub fn align_to_scenario_origin(
    sidecar: Vec<StateProjection>,
    recording_text: &str,
) -> Result<Vec<StateProjection>, String> {
    let mut mulligan_steps = std::collections::HashSet::new();
    for line in recording_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("malformed recording row: {e}"))?;
        if row.get("type").and_then(|v| v.as_str()) == Some("action")
            && row.get("source").and_then(|v| v.as_str()) == Some("mulligan")
        {
            if let Some(s) = row.get("step").and_then(|v| v.as_u64()) {
                mulligan_steps.insert(s as u32);
            }
        }
    }

    let mut kept: Vec<StateProjection> = sidecar
        .into_iter()
        .filter(|p| !mulligan_steps.contains(&p.step))
        .collect();

    // Re-index from 0: the scenario's step N must line up with the Nth
    // post-mulligan decision, whatever absolute index DCGO gave it.
    for (i, p) in kept.iter_mut().enumerate() {
        p.step = i as u32;
    }
    Ok(kept)
}

/// Which of our rows pairs with which DCGO row, derived from the LOWERED
/// LINE rather than from either trace's length.
///
/// See [`pair_by_wire_rows`] for why this exists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepPairing {
    /// `(our row index, DCGO row index)` for every comparable step, in line
    /// order.
    pub pairs: Vec<(usize, usize)>,
    /// Our rows deliberately excluded: steps that put NOTHING on the DCGO
    /// wire (`LoweredStep::SimOnlyAction`).
    pub ours_unpairable: u32,
    /// DCGO rows deliberately excluded: the 2nd..Nth row of a step that
    /// consumes several. Both traces are PRE-decision snapshots, so a fold's
    /// second row is a mid-resolution state our trace never materializes.
    pub dcgo_unpairable: u32,
}

/// Pair the two traces by LOWERED SCENARIO STEP, not by decision count.
///
/// # Why positional alignment is wrong here
///
/// Every scenario step is one row in OUR trace (the pre-step snapshot), but a
/// step can consume one, two, or zero DCGO decision rows — the job emitter
/// already knows which, because it is the thing that writes them:
///
/// | `LoweredStep` | DCGO wire rows |
/// |---|---|
/// | `Action` | 1 |
/// | `Select` (plain, or a folded `decline:`) | 1 |
/// | `Select` (folded pick: `OptionalSkill(yes)` + the pick) | 2 |
/// | `EndOfTurnGate { attack: None }` | 1 |
/// | `EndOfTurnGate { attack: Some(_) }` | 2 |
/// | `SimOnlyAction` | 0 |
///
/// Aligning 1:1 after mulligan-stripping therefore slides the two traces apart
/// from the first multi-row step onward and manufactures divergences out of an
/// offset. Measured witness (EX12-011#effect#0): a 9-step line against a
/// 10-row DCGO trace put our post-battle step 8 opposite DCGO's *pre*-battle
/// mid-fold row, reporting `p1.trash: ours=[EX12-020] dcgo=[]` — while DCGO's
/// step 13, the row that step really lands on, read `p1.trash:["EX12-020"]`,
/// identical to ours on every projected field.
///
/// # The rule
///
/// Our row `i` pairs with the **first** DCGO row step `i` consumes. The
/// 2nd..Nth rows of a multi-row step are dropped, and so is our row for a
/// `SimOnlyAction` step — but both are COUNTED, in
/// [`StepPairing::dcgo_unpairable`] / [`StepPairing::ours_unpairable`], and
/// the differ prints them. A dropped row that vanished from the denominator
/// would turn "we could not compare that row" into "that row agreed".
///
/// Rows beyond what the line predicts are left unaccounted on purpose: they
/// then show up as a denominator shortfall, which is exactly what a DCGO trace
/// that ran long or short *is*.
/// Pair the two traces when some steps are DCGO's alone.
///
/// `ours_present[i]` is false for a step our engine never makes -- today only
/// `dcgo_only`. Such a step still CONSUMES DCGO rows (DCGO really did decide),
/// but there is no state of ours to compare against them: our engine already
/// stands past that point. Pairing one anyway compares two different moments
/// and manufactures a divergence, which is exactly what BT24-017#effect#3 hit.
/// The rows are counted in `dcgo_unpairable` so the denominator still shows
/// they were not compared.
pub fn pair_by_wire_rows_with_ownership(
    rows_per_step: &[usize],
    ours_present: &[bool],
    dcgo_len: usize,
) -> StepPairing {
    let mut pairing = StepPairing::default();
    let mut cursor = 0usize;
    for (i, &rows) in rows_per_step.iter().enumerate() {
        let ours = ours_present.get(i).copied().unwrap_or(true);
        if !ours {
            // Counted on BOTH sides. `ours_for_diff` is indexed by scenario
            // step, so a `dcgo_only` step still occupies a row of our trace --
            // we simply decline to pair it. Counting it only against DCGO
            // leaves `compared + ours_unpairable != ours_steps`, and
            // `every_row_is_accounted_for` then reports TRUNCATED on a run that
            // compared everything it could. A truncated run is never clean, so
            // that miscount alone would record `diverged` for a clause whose
            // every comparable row matched.
            pairing.ours_unpairable += 1;
            for extra in 0..rows {
                if cursor + extra < dcgo_len {
                    pairing.dcgo_unpairable += 1;
                }
            }
            cursor += rows;
            continue;
        }
        if rows == 0 {
            pairing.ours_unpairable += 1;
            continue;
        }
        if cursor < dcgo_len {
            pairing.pairs.push((i, cursor));
        }
        for extra in 1..rows {
            if cursor + extra < dcgo_len {
                pairing.dcgo_unpairable += 1;
            }
        }
        cursor += rows;
    }
    pairing
}

pub fn pair_by_wire_rows(rows_per_step: &[usize], dcgo_len: usize) -> StepPairing {
    let mut pairing = StepPairing::default();
    let mut cursor = 0usize;
    for (i, &rows) in rows_per_step.iter().enumerate() {
        if rows == 0 {
            pairing.ours_unpairable += 1;
            continue;
        }
        if cursor < dcgo_len {
            pairing.pairs.push((i, cursor));
        }
        for extra in 1..rows {
            if cursor + extra < dcgo_len {
                pairing.dcgo_unpairable += 1;
            }
        }
        cursor += rows;
    }
    pairing
}

pub fn parse_sidecar(text: &str) -> Result<Vec<StateProjection>, String> {
    let stream = serde_json::Deserializer::from_str(text).into_iter::<StateProjection>();
    let mut rows = Vec::new();
    for (i, row) in stream.enumerate() {
        let mut row = row.map_err(|e| format!("state sidecar row {i}: {e}"))?;
        row.normalize();
        rows.push(row);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: &str = r#"{"step":3,"turn":2,"phase":"Main","memory":-2,
        "p0":{"security":5,"hand":["ST1-02","ST1-03"],"trash":[],
              "field":[{"card_id":"EX12-035","dp":4000,"suspended":false,
                        "sources":["ST1-02"],"keywords":["Blocker"]}]},
        "p1":{"security":4,"hand":[],"trash":["BT16-082"],"field":[]}}"#;

    #[test]
    fn parses_a_sidecar_row() {
        let p = StateProjection::from_sidecar_line(ROW).unwrap();
        assert_eq!(p.step, 3);
        assert_eq!(p.memory, -2);
        assert_eq!(p.p0.security, 5);
        assert_eq!(p.p0.field.len(), 1);
        assert_eq!(p.p0.field[0].dp, 4000);
        assert_eq!(p.p1.trash, vec!["BT16-082"]);
    }

    #[test]
    fn hand_and_trash_compare_as_multisets_not_sequences() {
        // Zone ORDER is representation -- the two engines order zones
        // differently and an order-sensitive diff would be pure noise.
        let a = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A","B"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let b = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["B","A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        assert_eq!(a, b, "hand order must not be a divergence");
    }

    #[test]
    fn duplicate_counts_still_matter() {
        // Multiset, not set: holding two copies is not the same as one.
        let a = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A","A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let b = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn suspended_is_semantics_and_must_differ() {
        let a = StateProjection::from_sidecar_line(ROW).unwrap();
        let b = StateProjection::from_sidecar_line(
            &ROW.replace("\"suspended\":false", "\"suspended\":true"),
        )
        .unwrap();
        assert_ne!(a, b, "suspended is semantics, not representation");
    }

    #[test]
    fn align_drops_leading_mulligan_rows_and_reindexes() {
        // The live shape: DCGO records two mulligan actions at steps 0-1, and
        // the sidecar dumps state at those steps. Our scenario trace has no
        // counterpart, so without alignment every scenario "diverges at step 0".
        let rec = concat!(
            r#"{"type":"action","step":0,"actor":0,"action_id":0,"source":"mulligan"}"#,
            "
",
            r#"{"type":"action","step":1,"actor":1,"action_id":0,"source":"mulligan"}"#,
            "
",
            r#"{"type":"action","step":2,"actor":0,"action_id":62,"source":"breeding"}"#,
            "
",
        );
        let side = concat!(
            r#"{"step":0,"turn":0,"phase":"Active","memory":0,"p0":{"security":0,"hand":[],"trash":[],"field":[]},"p1":{"security":0,"hand":[],"trash":[],"field":[]}}"#,
            "
",
            r#"{"step":1,"turn":0,"phase":"Active","memory":0,"p0":{"security":0,"hand":[],"trash":[],"field":[]},"p1":{"security":0,"hand":[],"trash":[],"field":[]}}"#,
            "
",
            r#"{"step":2,"turn":1,"phase":"Breeding","memory":0,"p0":{"security":5,"hand":[],"trash":[],"field":[]},"p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
            "
",
        );
        let out = align_to_scenario_origin(parse_sidecar(side).unwrap(), rec).unwrap();
        assert_eq!(out.len(), 1, "both mulligan rows must be dropped");
        assert_eq!(
            out[0].step, 0,
            "the survivor must be re-indexed to the origin"
        );
        assert_eq!(
            out[0].p0.security, 5,
            "and it must be the post-mulligan row"
        );
    }

    #[test]
    fn align_is_a_no_op_without_mulligan_rows() {
        let rec = concat!(
            r#"{"type":"action","step":0,"actor":0,"action_id":62,"source":"breeding"}"#,
            "
",
        );
        let side = concat!(
            r#"{"step":0,"turn":1,"phase":"Breeding","memory":0,"p0":{"security":5,"hand":[],"trash":[],"field":[]},"p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
            "
",
        );
        let out = align_to_scenario_origin(parse_sidecar(side).unwrap(), rec).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].step, 0);
    }

    // ── pairing by lowered step ─────────────────────────────────────────

    #[test]
    fn a_one_row_per_step_line_pairs_positionally() {
        // The common case must stay exactly what it was.
        let p = pair_by_wire_rows(&[1, 1, 1], 3);
        assert_eq!(p.pairs, vec![(0, 0), (1, 1), (2, 2)]);
        assert_eq!(p.ours_unpairable, 0);
        assert_eq!(p.dcgo_unpairable, 0);
    }

    #[test]
    fn a_two_row_step_pairs_with_its_first_row_and_drops_the_intermediate() {
        // Measured witness EX12-011#effect#0: step 1 is an OptionalSkill+pick
        // fold, so DCGO writes two rows for it. Our row 1 belongs opposite the
        // FIRST of them; the second is a mid-resolution state our trace never
        // materializes, and comparing it can only manufacture a divergence.
        let p = pair_by_wire_rows(&[1, 2, 1], 4);
        assert_eq!(p.pairs, vec![(0, 0), (1, 1), (2, 3)]);
        assert_eq!(
            p.dcgo_unpairable, 1,
            "the fold's second row is accounted, not erased"
        );
        assert_eq!(p.ours_unpairable, 0);
    }

    #[test]
    fn a_sim_only_step_drops_our_row_and_consumes_no_dcgo_row() {
        // Witness EX12-004#effect#2: the author balanced the row counts with a
        // sim-only step, which put our POST-attack row opposite DCGO's
        // PRE-attack one. Under the pairing rule the sim-only row simply has
        // no partner.
        let p = pair_by_wire_rows(&[1, 0, 1], 2);
        assert_eq!(p.pairs, vec![(0, 0), (2, 1)]);
        assert_eq!(p.ours_unpairable, 1);
        assert_eq!(p.dcgo_unpairable, 0);
    }

    #[test]
    fn a_short_dcgo_trace_leaves_our_rows_unpaired_rather_than_inventing_partners() {
        // Two steps predicted, one row delivered: the second step gets no
        // pair at all -- it must NOT slide onto a row belonging to another
        // step.
        let p = pair_by_wire_rows(&[1, 1, 1], 1);
        assert_eq!(p.pairs, vec![(0, 0)]);
        assert_eq!(
            p.ours_unpairable, 0,
            "an absent partner is a shortfall, not a deliberate drop"
        );
    }

    #[test]
    fn intermediate_rows_are_only_counted_when_they_exist() {
        // A 2-row step whose second row DCGO never wrote must not be counted
        // as a deliberately-dropped row -- that would launder a truncated
        // trace into an accounted one.
        let p = pair_by_wire_rows(&[2], 1);
        assert_eq!(p.pairs, vec![(0, 0)]);
        assert_eq!(p.dcgo_unpairable, 0);
    }

    #[test]
    fn parse_sidecar_reads_every_row() {
        let text = format!("{}\n{}\n", ROW.replace("\"step\":3", "\"step\":0"), ROW);
        let rows = parse_sidecar(&text).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].step, 0);
        assert_eq!(rows[1].step, 3);
    }

    #[test]
    fn a_malformed_row_fails_loudly_rather_than_being_skipped() {
        // Silently dropping an unparseable row would shorten one side of the
        // diff and misalign every step after it.
        let text = format!("{ROW}\nnot json\n");
        assert!(parse_sidecar(&text).is_err());
    }

    #[test]
    fn unmeasured_keywords_are_a_wildcard_not_an_empty_set() {
        // DCGO's keyword dump is O(22 x field x effects) and defaults OFF, so
        // an absent list means "nobody looked". Treating it as "no keywords"
        // would report a keyword divergence on every permanent of every run
        // made without the flag -- pure noise, and worse, noise that looks
        // exactly like a real engine bug.
        let measured = StateProjection::from_sidecar_line(ROW).unwrap();
        let unmeasured =
            StateProjection::from_sidecar_line(&ROW.replace(",\"keywords\":[\"Blocker\"]", ""))
                .unwrap();
        assert_eq!(
            unmeasured.p0.field[0].keywords, None,
            "an omitted keyword list must parse as None, not as an empty Vec"
        );
        assert_eq!(
            measured, unmeasured,
            "an unmeasured keyword list must never diff against a measured one"
        );

        // ...but two sides that BOTH measured must still diff when they
        // disagree, including when one measured an genuinely empty set.
        let other =
            StateProjection::from_sidecar_line(&ROW.replace("\"Blocker\"", "\"Rush\"")).unwrap();
        assert_ne!(measured, other, "measured-vs-measured keywords must diff");

        let measured_empty =
            StateProjection::from_sidecar_line(&ROW.replace("[\"Blocker\"]", "[]")).unwrap();
        assert_ne!(
            measured, measured_empty,
            "a measured empty set is not the same claim as an unmeasured one"
        );
        assert_eq!(
            measured_empty, unmeasured,
            "unmeasured still wildcards against a measured empty set"
        );
    }

    /// `from_sidecar_line` is only half the contract -- if `from_game`
    /// disagrees with the real engine API the whole comparison is against a
    /// board nobody is playing. This runs the actual constructor over a real
    /// `Game` built from `cards.json`.
    #[test]
    fn from_game_projects_a_real_engine_state() {
        use crate::exam::test_support::{load_card_data, st1_decks};
        use digimon_engine::rules::Rules;
        use digimon_engine::Game;

        let card_data = load_card_data();
        let (main, egg) = st1_decks();
        let deck: Vec<String> = main.iter().chain(egg.iter()).cloned().collect();
        let game = Game::new_with_ordered_decks(
            &[deck.clone(), deck],
            &card_data,
            Rules::default(),
            Some(7),
            0,
        )
        .expect("ST-1 mirror should construct");

        let p = StateProjection::from_game(&game, 0);
        assert_eq!(p.step, 0);
        assert_eq!(p.turn, u64::from(game.turn_count));
        assert_eq!(p.phase, game.current_phase.py_name());
        assert_eq!(p.p0.security, game.player(0).security.len());
        assert_eq!(p.p0.hand.len(), game.player(0).hand.len());
        assert!(!p.p0.hand.is_empty(), "an opening hand should be dealt");
        // Sorted on construction, so the multiset compare in `PartialEq` is
        // honest for engine-built projections too, not just parsed ones.
        assert!(p.p0.hand.windows(2).all(|w| w[0] <= w[1]));
        // Keywords default to NOT MEASURED on our side as well, matching the
        // DCGO dumper's default-off flag.
        assert!(p.p0.field.iter().all(|f| f.keywords.is_none()));

        // Memory is pinned to player 0's perspective on both sides.
        let expected_memory = if game.memory_pair.0 == 0 {
            i64::from(game.memory)
        } else {
            -i64::from(game.memory)
        };
        assert_eq!(p.memory, expected_memory);

        // The keyword-measuring variant actually measures.
        let kw = StateProjection::from_game_with_keywords(&game, 0);
        assert!(kw.p0.field.iter().all(|f| f.keywords.is_some()));
    }
}

#[cfg(test)]
mod legal_option_measurement_runtime_tests {
    use super::*;
    use digimon_engine::debug_runner::DebugRunner;
    use digimon_engine::enums::{EffectSourceKind, Expiry, GamePhase, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;

    fn runner() -> DebugRunner {
        let mut r = DebugRunner::builder()
            .with_card_data(crate::exam::test_support::load_card_data())
            .hand(0, &["ST1-16", "ST1-02", "ST1-16"])
            .memory(3)
            .build();
        r.game.mulligan_pending.clear();
        r.game.game_over = false;
        r.game.current_phase = GamePhase::Main;
        r
    }
    fn observe(r: &DebugRunner) -> StateProjection {
        let mut p = StateProjection::from_game(&r.game, 0);
        p.measure_legal_option_plays(&r.game, [true, true]);
        p
    }
    #[test]
    fn legal_option_measurement_uses_mask_for_breeding_color_memory_and_prohibition() {
        let mut r = runner();
        assert_eq!(observe(&r).p0.legal_option_plays, Some(vec![]));
        r.place_in_breeding(0, "ST1-01");
        assert_eq!(
            observe(&r).p0.legal_option_plays,
            Some(vec!["ST1-16".into(), "ST1-16".into()])
        );
        assert!(
            observe(&r).p1.legal_option_plays.is_none(),
            "opponent is not deciding"
        );
        r.game.memory = -3;
        assert_eq!(
            observe(&r).p0.legal_option_plays,
            Some(vec![]),
            "8-cost Option exceeds remaining memory"
        );
        r.game.memory = 3;
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(
                ModifierType::CannotPlayFromHand,
                1,
                Expiry::Permanent,
                None,
                0,
            ),
        );
        assert_eq!(observe(&r).p0.legal_option_plays, Some(vec![]));
    }
    #[test]
    fn legal_option_measurement_requires_opt_in_and_follows_actual_turn_seat() {
        let mut r = runner();
        r.add_to_hand(1, "ST1-16");
        r.place_in_breeding(1, "ST1-01");
        r.game.turn_player_idx = 1;
        r.game.memory_pair = (1, 0);
        let mut p = StateProjection::from_game(&r.game, 0);
        p.measure_legal_option_plays(&r.game, [false, false]);
        assert!(p.p0.legal_option_plays.is_none() && p.p1.legal_option_plays.is_none());
        p.measure_legal_option_plays(&r.game, [true, true]);
        assert!(p.p0.legal_option_plays.is_none());
        assert_eq!(p.p1.legal_option_plays, Some(vec!["ST1-16".into()]));
    }
    #[test]
    fn legal_option_measurement_never_samples_selection_even_when_phase_is_main() {
        use digimon_engine::selection::{PendingSelection, SelectionKind};
        let mut r = runner();
        r.place_in_breeding(0, "ST1-01");
        r.game.pending_selection = Some(PendingSelection { card_choice: None,
            kind: SelectionKind::Hand,
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids: vec![0],
            is_optional: false,
            prompt: "test".into(),
            effect_choices: None,
            source_card: r.game.player(0).hand[0].handle(),
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
            callback: Box::new(|_, _| {}),
            on_decline: None,
            zone_owner: Some(0),
        });
        assert!(observe(&r).p0.legal_option_plays.is_none());
        assert!(observe(&r).p1.legal_option_plays.is_none());
        r.game.pending_selection = None;
        r.game.current_phase = GamePhase::Breeding;
        assert!(observe(&r).p0.legal_option_plays.is_none());
        r.game.current_phase = GamePhase::Main;
        r.game.game_over = true;
        assert!(observe(&r).p0.legal_option_plays.is_none());
    }
}

#[cfg(test)]
mod selection_decline_observation_runtime_tests {
    use super::*;
    use digimon_engine::action::space::PASS;
    use digimon_engine::debug_runner::DebugRunner;
    use digimon_engine::enums::{EffectSourceKind, GamePhase};
    use digimon_engine::selection::{PendingSelection, SelectionKind};

    fn runner(optional: bool, explicit_pass: bool, actor: u8) -> DebugRunner {
        let mut r = DebugRunner::builder()
            .with_card_data(crate::exam::test_support::load_card_data())
            .hand(0, &["ST1-16"])
            .memory(3)
            .build();
        r.game.mulligan_pending.clear();
        r.game.game_over = false;
        r.game.current_phase = GamePhase::Main;
        r.game.pending_selection = Some(PendingSelection { card_choice: None,
            kind: SelectionKind::Hand,
            selecting_player: actor,
            previous_phase: GamePhase::Main,
            valid_action_ids: if explicit_pass {
                vec![0, PASS]
            } else {
                vec![0]
            },
            is_optional: optional,
            prompt: "live test choice".into(),
            effect_choices: None,
            source_card: r.game.player(0).hand[0].handle(),
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
            callback: Box::new(|_, _| {}),
            on_decline: None,
            zone_owner: Some(0),
        });
        r
    }
    fn observe(r: &DebugRunner, request: [bool; 2]) -> StateProjection {
        let mut p = StateProjection::from_game(&r.game, 0);
        p.measure_selection_can_decline(&r.game, request);
        p
    }
    #[test]
    fn selection_decline_observation_runtime_uses_actual_pass_mask() {
        for (optional, explicit_pass, expected) in [
            (false, false, false),
            (true, false, true),
            (false, true, true),
        ] {
            let r = runner(optional, explicit_pass, 0);
            let p = observe(&r, [true, true]);
            assert_eq!(p.p0.selection_can_decline, Some(expected));
            assert!(p.p1.selection_can_decline.is_none());
        }
    }
    #[test]
    fn selection_decline_observation_runtime_follows_selecting_actor_not_turn_player() {
        let r = runner(false, false, 1);
        assert_eq!(r.game.turn_player(), 0);
        let p = observe(&r, [true, true]);
        assert!(p.p0.selection_can_decline.is_none());
        assert_eq!(p.p1.selection_can_decline, Some(false));
        let p = observe(&r, [true, false]);
        assert!(p.p0.selection_can_decline.is_none() && p.p1.selection_can_decline.is_none());
    }
    #[test]
    fn selection_decline_observation_runtime_requires_live_choice_and_opt_in() {
        let mut r = runner(true, false, 0);
        let p = observe(&r, [false, false]);
        assert!(p.p0.selection_can_decline.is_none());
        r.game.game_over = true;
        assert!(observe(&r, [true, true]).p0.selection_can_decline.is_none());
        r.game.game_over = false;
        r.game.pending_selection = None;
        assert!(
            observe(&r, [true, true]).p0.selection_can_decline.is_none(),
            "Main alone is not a selection"
        );
    }
}

#[cfg(test)]
mod selection_candidate_runtime_tests {
    use super::*;
    use digimon_engine::action::space::PASS;
    use digimon_engine::debug_runner::DebugRunner;
    use digimon_engine::enums::{EffectSourceKind, GamePhase};
    use digimon_engine::selection::{PendingSelection, SelectionKind};

    fn runner(actor: u8) -> DebugRunner {
        let mut r = DebugRunner::builder()
            .with_card_data(crate::exam::test_support::load_card_data())
            .hand(0, &["ST1-05", "ST1-05", "ST1-09"])
            .hand(1, &["ST2-05"])
            .memory(3)
            .build();
        r.game.mulligan_pending.clear();
        r.game.game_over = false;
        r.game.current_phase = GamePhase::Main;
        r.game.pending_selection = Some(PendingSelection { card_choice: None,
            kind: SelectionKind::Hand,
            selecting_player: actor,
            previous_phase: GamePhase::Main,
            valid_action_ids: vec![0, 1, PASS],
            is_optional: true,
            prompt: "choose legal level4".into(),
            effect_choices: None,
            source_card: r.game.player(0).hand[0].handle(),
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
            callback: Box::new(|_, _| {}),
            on_decline: None,
            zone_owner: Some(0),
        });
        r
    }
    fn observe(r: &DebugRunner, request: [bool; 2]) -> StateProjection {
        let mut p = StateProjection::from_game(&r.game, 0);
        p.measure_selection_candidates(&r.game, request);
        p
    }
    #[test]
    fn selection_candidates_runtime_preserves_copies_excludes_unoffered_card_and_pass() {
        let r = runner(0);
        let p = observe(&r, [true, true]);
        assert_eq!(
            p.p0.selection_candidates,
            Some(vec!["ST1-05".into(), "ST1-05".into()])
        );
        assert!(p.p1.selection_candidates.is_none());
    }
    #[test]
    fn selection_candidates_runtime_missing_or_empty_card_identity_is_unknown() {
        let mut r = runner(0);
        // Construct the ordinary projection before deliberately corrupting identity;
        // exercise this observer's fail-closed boundary, not unrelated serializers.
        let mut p = StateProjection::from_game(&r.game, 0);
        let index = r.game.player(0).hand[0].data_index;
        r.game.player_mut(0).hand[0].data_index = usize::MAX;
        p.measure_selection_candidates(&r.game, [true, false]);
        assert!(p.p0.selection_candidates.is_none());
        r.game.player_mut(0).hand[0].data_index = index;
        r.game.card_data[index].card_id.clear();
        p.measure_selection_candidates(&r.game, [true, false]);
        assert!(p.p0.selection_candidates.is_none());
    }
    #[test]
    fn selection_candidates_runtime_uses_zone_owner_and_selecting_actor() {
        let r = runner(1);
        let p = observe(&r, [true, true]);
        assert!(p.p0.selection_candidates.is_none());
        assert_eq!(
            p.p1.selection_candidates,
            Some(vec!["ST1-05".into(), "ST1-05".into()])
        );
        assert!(observe(&r, [true, false]).p1.selection_candidates.is_none());
    }
    #[test]
    fn selection_candidates_runtime_empty_is_measured_but_invalid_or_unsupported_is_unknown() {
        let mut r = runner(0);
        r.game.pending_selection.as_mut().unwrap().valid_action_ids = vec![PASS];
        assert_eq!(
            observe(&r, [true, false]).p0.selection_candidates,
            Some(vec![])
        );
        r.game.pending_selection.as_mut().unwrap().valid_action_ids = vec![4];
        assert!(observe(&r, [true, false]).p0.selection_candidates.is_none());
        r.game.pending_selection.as_mut().unwrap().kind = SelectionKind::Reveal;
        assert!(observe(&r, [true, false]).p0.selection_candidates.is_none());
        r.game.pending_selection = None;
        assert!(observe(&r, [true, false]).p0.selection_candidates.is_none());
    }
    #[test]
    fn selection_candidates_runtime_not_requested_or_game_over_is_unknown() {
        let mut r = runner(0);
        assert!(observe(&r, [false, false])
            .p0
            .selection_candidates
            .is_none());
        r.game.game_over = true;
        assert!(observe(&r, [true, true]).p0.selection_candidates.is_none());
    }
}

#[cfg(test)]
mod source_candidate_runtime_tests {
    use super::*;
    use digimon_engine::action::{
        mask::build_action_mask,
        space::{FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS},
    };
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::enums::GamePhase;
    use digimon_engine::resume::{ResumeFrame, SourceMultiState};
    use digimon_engine::selection::SelectionKind;

    fn runner() -> DebugRunner { runner_with_sources(2) }
    fn runner_with_sources(count: usize) -> DebugRunner {
        let mut r = DebugRunner::builder()
            .dsl_card("BT4-072")
            .unwrap()
            .dsl_card("BT9-109")
            .unwrap()
            .add_card(make_test_card("ORDINARY", "Ordinary source"))
            .memory(3)
            .build();
        r.game.mulligan_pending.clear();
        r.game.game_over = false;
        r.game.current_phase = GamePhase::Main;
        r.set_first_player(0);
        let mut stack=vec!["BT9-109"];
        stack.extend(std::iter::repeat_n("ORDINARY",count));
        stack.push("BT4-072");
        r.place_stack(0, &stack);
        r.game.tick_declarative_effects();
        let action = FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN;
        assert!(
            build_action_mask(&r.game, 0)[action as usize] > 0.5,
            "real Digi-Burst Main is legal"
        );
        r.game.decode_action(action, 0);
        assert!(matches!(
            r.game.pending_selection.as_ref().unwrap().kind,
            SelectionKind::SourceMulti {
                min: 1,
                max: 1,
                picked: 0
            }
        ));
        r
    }
    fn state(game: &mut Game) -> &mut SourceMultiState {
        match game
            .pending_selection_resume
            .as_mut()
            .unwrap()
            .frames
            .last_mut()
            .unwrap()
        {
            ResumeFrame::SourceMultiStep(s) => s,
            _ => panic!("real source resume expected"),
        }
    }
    fn observe(game: &Game, requested: [bool; 2]) -> StateProjection {
        let mut p = StateProjection::from_game(game, 0);
        p.measure_selection_candidates(game, requested);
        p
    }
    fn expected() -> Option<Vec<String>> {
        Some(vec!["ORDINARY".into(), "ORDINARY".into()])
    }
    #[test]
    fn source_candidates_runtime_real_burst_excludes_protected_option_and_preserves_copies() {
        let r = runner();
        assert_eq!(
            observe(&r.game, [true, true]).p0.selection_candidates,
            expected()
        );
        assert!(observe(&r.game, [true, true])
            .p1
            .selection_candidates
            .is_none());
    }
    #[test]
    fn source_candidates_runtime_measures_every_page_without_changing_the_prompt() {
        let r=runner_with_sources(37);
        let before=serde_json::to_string(&r.pending_selection_view().unwrap().card_choice).unwrap();
        let view=r.pending_selection_view().unwrap();
        assert_eq!(view.card_choice.as_ref().unwrap().page_count,"2");
        assert_eq!(observe(&r.game,[true,false]).p0.selection_candidates,Some(vec!["ORDINARY".to_string();37]));
        assert_eq!(serde_json::to_string(&r.pending_selection_view().unwrap().card_choice).unwrap(),before,"measurement must not navigate the live prompt");
    }
    #[test]
    fn source_candidates_runtime_clone_preserves_physical_snapshot() {
        let r = runner();
        let cloned = r.game.clone();
        assert_eq!(
            observe(&cloned, [true, false]).p0.selection_candidates,
            expected()
        );
        assert_eq!(
            observe(&r.game, [true, false]).p0.selection_candidates,
            observe(&cloned, [true, false]).p0.selection_candidates
        );
    }
    #[test]
    fn source_candidates_runtime_actor_can_differ_from_source_owner() {
        let mut r = runner();
        r.game.pending_selection.as_mut().unwrap().selecting_player = 1;
        state(&mut r.game).selecting_player = 1;
        let p = observe(&r.game, [true, true]);
        assert!(p.p0.selection_candidates.is_none());
        assert_eq!(p.p1.selection_candidates, expected());
    }
    #[test]
    fn source_candidates_runtime_pass_is_not_a_source() {
        let mut r = runner();
        let candidates=state(&mut r.game).candidates.clone();
        let pins=candidates.iter().map(|source| digimon_engine::card_choice_page::SourceChoicePin::capture(&mut r.game,source.permanent,source.card).unwrap()).collect();
        let choice=digimon_engine::owned_card_choice::OwnedCardChoice::sources(&mut r.game,pins,true).unwrap();
        let actions=choice.valid_action_ids(&r.game);
        assert!(actions.contains(&PASS));
        let pending=r.game.pending_selection.as_mut().unwrap();
        pending.is_optional=true;
        pending.valid_action_ids=actions;
        pending.card_choice=Some(choice);
        assert_eq!(
            observe(&r.game, [true, false]).p0.selection_candidates,
            expected()
        );
    }
    #[test]
    fn source_candidates_runtime_requires_requested_live_supported_frame() {
        let mut r = runner();
        assert!(observe(&r.game, [false, false])
            .p0
            .selection_candidates
            .is_none());
        r.game.game_over = true;
        assert!(observe(&r.game, [true, false])
            .p0
            .selection_candidates
            .is_none());
        r.game.game_over = false;
        r.game.pending_selection.as_mut().unwrap().kind = SelectionKind::SourceMulti {
            min: 1,
            max: 2,
            picked: 0,
        };
        assert!(observe(&r.game, [true, false])
            .p0
            .selection_candidates
            .is_none());
        r.game.pending_selection.as_mut().unwrap().kind = SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0,
        };
        state(&mut r.game).selecting_player = 1;
        assert!(observe(&r.game, [true, true])
            .p0
            .selection_candidates
            .is_none());
        r.game.pending_selection_resume = None;
        assert!(observe(&r.game, [true, false])
            .p0
            .selection_candidates
            .is_none());
    }
    #[test]
    fn source_candidates_runtime_stale_physical_source_is_unknown() {
        let mut r = runner();
        r.game.players[0].battle_area[0].card_sources.remove(1);
        assert!(observe(&r.game, [true, false])
            .p0
            .selection_candidates
            .is_none());
    }
    #[test]
    fn source_candidates_runtime_unmapped_or_duplicate_action_is_unknown() {
        let mut r = runner();
        state(&mut r.game).candidates.pop();
        assert!(observe(&r.game, [true, false])
            .p0
            .selection_candidates
            .is_none());
        let mut r = runner();
        let x = state(&mut r.game).candidates[0];
        state(&mut r.game).candidates.push(x);
        assert!(observe(&r.game, [true, false])
            .p0
            .selection_candidates
            .is_none());
    }
    #[test]
    fn source_candidates_runtime_unknown_identity_is_not_empty() {
        let mut r = runner();
        let mut p = StateProjection::from_game(&r.game, 0);
        let index = r.game.players[0].battle_area[0].card_sources[1].data_index;
        r.game.players[0].battle_area[0].card_sources[1].data_index = usize::MAX;
        p.measure_selection_candidates(&r.game, [true, false]);
        assert!(p.p0.selection_candidates.is_none());
        r.game.players[0].battle_area[0].card_sources[1].data_index = index;
        r.game.card_data[index].card_id.clear();
        p.measure_selection_candidates(&r.game, [true, false]);
        assert!(p.p0.selection_candidates.is_none());
    }
}
