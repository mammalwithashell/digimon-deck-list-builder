//! Triage of replay failures that are DCGO bugs, not engine bugs.
//!
//! A failing replay is only a finding against OUR engine when the recorded move
//! was legal. When DCGO itself did something the rules forbid (and our engine
//! rightly has no such move), the divergence cannot be fixed on our side, and
//! leaving it as a bare FAIL hides it among real engine bugs. Each known DCGO
//! bug is recorded in `qa/rules/dcgo_quirks.md` under a `Q-DCGO-*` id; this
//! module recognises its SIGNATURE in the recording so the report can count the
//! game separately ("quirk"), name the id, and print the evidence.
//!
//! The signatures are deliberately narrow and falsifiable: each one reads
//! rows the recorder wrote, plus card data, and requires every element to
//! hold. A failure that merely happens near a quirk, or whose numbers do not
//! line up exactly, stays a FAIL. A triaged game is NOT a pass: the report
//! lists it separately and the exit code still treats only untriaged failures
//! as errors.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::enums::CardKind;

use crate::recording::{RecordingV1, Row};
use crate::replay::ReplayFail;

/// A recognised DCGO bug behind a replay failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triage {
    /// `Q-DCGO-*` id in `qa/rules/dcgo_quirks.md`.
    pub quirk_id: String,
    /// Human-readable proof, quoting the recorded rows that matched.
    pub evidence: String,
}

pub const TAMER_PHANTOM_DIGIVOLVE: &str = "Q-DCGO-TAMER-PHANTOM-DIGIVOLVE";

/// Classify a failure, or `None` when it is not a known DCGO bug.
pub fn triage_failure(
    recording: &RecordingV1,
    fail: &ReplayFail,
    cards: &HashMap<String, CardData>,
) -> Option<Triage> {
    tamer_phantom_digivolve(recording, fail, cards)
        .or_else(|| missing_activation_action(recording, fail))
        .or_else(|| stale_multipleskills_index(recording, fail))
        .or_else(|| attacker_index_not_digimon(recording, fail, cards))
        .or_else(|| option_without_color_source(recording, fail, cards))
}

pub const RECORDER_ATTACKER_INDEX_NOT_DIGIMON: &str = "R-RECORDER-ATTACKER-INDEX-NOT-DIGIMON";
pub const DCGO_OPTION_WITHOUT_COLOR_SOURCE: &str = "Q-DCGO-OPTION-WITHOUT-COLOR-SOURCE";

/// A human-reviewed list of recordings known to be invalid as replay input, for
/// cases no signature can prove from the recording alone (for example a scripted
/// exam whose raw action ids were passed to DCGO unchecked). Loaded from
/// `qa/rules/dcgo_recording_quarantine.json`. An entry only triages a game that
/// STILL FAILS: if the engine later replays it, it counts as a pass.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Quarantine {
    #[serde(default)]
    pub quarantined: Vec<QuarantineEntry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct QuarantineEntry {
    pub game_id: String,
    pub id: String,
    pub reason: String,
}

impl Quarantine {
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The triage for `recording` when it is quarantined (only call for failures).
    pub fn triage(&self, recording: &RecordingV1) -> Option<(String, String)> {
        self.quarantined
            .iter()
            .find(|e| e.game_id == recording.start.game_id)
            .map(|e| (e.id.clone(), e.reason.clone()))
    }
}

pub const RECORDER_MISSING_ACTIVATION_ACTION: &str = "R-RECORDER-MISSING-ACTIVATION-ACTION";
pub const RECORDER_STALE_MULTIPLESKILLS_INDEX: &str = "R-RECORDER-STALE-MULTIPLESKILLS-INDEX";

fn board_of(row: &Row, player: u8) -> Option<&Vec<String>> {
    match row {
        Row::Action(a) => if player == 0 { a.board_p0.as_ref() } else { a.board_p1.as_ref() },
        Row::Selection(s) => if player == 0 { s.board_p0.as_ref() } else { s.board_p1.as_ref() },
        _ => None,
    }
}

/// `Q-DCGO-TAMER-PHANTOM-DIGIVOLVE`: DCGO's AI digivolves a level-less Tamer
/// into a Digimon and is charged 0 (`alt_path: cost_modifier`), with no effect
/// reducing the printed cost. Every element below must hold:
///   1. the failure is a `memory_mismatch`;
///   2. a digivolve-range `action` row has a correlated `action_detail` with
///      `cost_paid == 0`, `alt_path == "cost_modifier"`;
///   3. the played card has a printed digivolution cost > 0 (so 0 is not the
///      printed price, and the engine's charge is the legal one);
///   4. across that action, exactly one card id left the actor's board, and
///      either it is a Tamer (not a legal digivolution target, CR 4.3 §6-5) or
///      the engine's own legality rejected that target (the action replayed
///      only through the recorded-operand fallback);
///   5. the mismatch's recorded memory equals that `action_detail`'s
///      `memory_after` (the failing comparison is that very payment), and the
///      engine's value differs.
fn tamer_phantom_digivolve(
    recording: &RecordingV1,
    fail: &ReplayFail,
    cards: &HashMap<String, CardData>,
) -> Option<Triage> {
    use digimon_engine::action::space;
    let ReplayFail::MemoryMismatch(mm) = fail else { return None };
    if mm.recorded == mm.replayed {
        return None;
    }
    for (i, row) in recording.rows.iter().enumerate() {
        let Row::Action(act) = row else { continue };
        if !(space::DIGIVOLVE_START..space::DIGIVOLVE_END).contains(&act.action_id) {
            continue;
        }
        let Some(detail) = recording.rows[i + 1..].iter().find_map(|r| match r {
            Row::ActionDetail(d) if d.step == act.step && d.actor == act.actor => Some(d),
            _ => None,
        }) else { continue };
        if detail.cost_paid != 0 || detail.alt_path.as_deref() != Some("cost_modifier") {
            continue;
        }
        let Some(played) = detail.card_id.as_deref().and_then(|id| cards.get(id)) else { continue };
        if !played.evo_costs.iter().any(|c| c.memory_cost > 0) {
            continue;
        }
        let Some(before) = board_of(row, act.actor) else { continue };
        let Some(after) = recording.rows[i + 1..]
            .iter()
            .filter(|r| !matches!(r, Row::ActionDetail(_)))
            .find_map(|r| board_of(r, act.actor))
        else { continue };
        let mut gone: Vec<&String> = Vec::new();
        let mut remaining: Vec<&String> = after.iter().collect();
        for id in before {
            if let Some(pos) = remaining.iter().position(|r| *r == id) {
                remaining.remove(pos);
            } else {
                gone.push(id);
            }
        }
        let [only] = gone.as_slice() else { continue };
        // The digivolve target must be one the engine itself rejects: a Tamer
        // (no level, never a digivolution target), or a Digimon for which the
        // engine's remapped target was ILLEGAL and only the recorded operand
        // replayed (`digivolve_operand_fallbacks`). The mask is the proof, not a
        // re-derivation of the card's requirements here.
        let target_is_tamer = cards.get(*only).is_some_and(|c| c.card_kind == CardKind::Tamer);
        let engine_rejected_target = mm
            .digivolve_operand_fallbacks
            .iter()
            .any(|card| card == &played.card_id);
        if !(target_is_tamer || engine_rejected_target) {
            continue;
        }
        if detail.memory_after.map(i64::from) != Some(mm.recorded) {
            continue;
        }
        return Some(Triage {
            quirk_id: TAMER_PHANTOM_DIGIVOLVE.to_string(),
            evidence: format!(
                "step {}: P{} digivolved {} onto {} {} at cost_paid=0 \
                 (printed digivolution cost > 0, alt_path=cost_modifier); recorded memory {} \
                 vs engine {} at the next check",
                act.step,
                act.actor,
                played.card_id,
                if target_is_tamer { "the level-less Tamer" } else { "the ineligible Digimon" },
                only,
                mm.recorded,
                mm.replayed
            ),
        });
    }
    None
}

/// `R-RECORDER-MISSING-ACTIVATION-ACTION`: a `[Main]` activated ability was
/// executed (`effect_activation`) but the recording has no `action` row for the
/// activation itself, so the replay never activates it and the gauge it moved
/// diverges. Recorders before DCGO `e69f10a38` (2026-08-24) did not log the
/// `UseCardEffect` decision at all (CLAUDE.md rule 27, `LogAction`).
///
/// Signature, all required: the failure is a `memory_mismatch`; some executed
/// `effect_activation` row's description starts with `[Main]`; and the nearest
/// preceding `action` row of that actor is not a field-effect or hand-effect
/// activation (a post-mirror recording always has one).
fn missing_activation_action(recording: &RecordingV1, fail: &ReplayFail) -> Option<Triage> {
    use digimon_engine::action::space;
    let ReplayFail::MemoryMismatch(_) = fail else { return None };
    let is_activation_id = |id: u16| {
        (space::HAND_EFFECT_START..space::HAND_EFFECT_END).contains(&id)
            || (space::FIELD_EFFECT_START..space::FIELD_EFFECT_END).contains(&id)
    };
    for (i, row) in recording.rows.iter().enumerate() {
        let Row::EffectActivation(e) = row else { continue };
        if e.executed != Some(true)
            || !e
                .effect_description
                .as_deref()
                .is_some_and(|d| d.trim_start().starts_with("[Main]"))
        {
            continue;
        }
        // The activation decision, if recorded, is the nearest preceding
        // `action` row of the same actor (it is logged before the effect runs).
        let nearest_action = recording.rows[..i].iter().rev().find_map(|r| match r {
            Row::Action(a) if a.actor == e.actor => Some(a),
            _ => None,
        });
        if nearest_action.is_some_and(|a| is_activation_id(a.action_id)) {
            continue;
        }
        return Some(Triage {
            quirk_id: RECORDER_MISSING_ACTIVATION_ACTION.to_string(),
            evidence: format!(
                "step {}: P{} executed {}'s [Main] ability ({}) with no preceding activation \
                 action row (nearest action: {:?}); recorder predates the UseCardEffect mirror",
                e.step,
                e.actor,
                e.card_id.as_deref().unwrap_or("?"),
                e.effect_name.as_deref().unwrap_or("?"),
                nearest_action.map(|a| a.action_id)
            ),
        });
    }
    None
}

/// `R-RECORDER-STALE-MULTIPLESKILLS-INDEX`: the recording ends on a
/// `MultipleSkills` row whose index is not even in DCGO's own list (an exam
/// harness passed the sim engine's slot id as a raw value), followed by no
/// `effect_activation` and a `disconnect` end. DCGO's own commit `617cf381b`
/// (2026-08-23) made such an index refused; before it the stack was cleared and
/// the game aborted.
///
/// Signature, all required: an `EngineError` saying the int payload is out of
/// range for a TriggerOrder prompt; the LAST decision row is a `MultipleSkills`
/// selection with that non-negative index (so no `effect_activation` follows
/// it); and the game ended with reason `disconnect`.
fn stale_multipleskills_index(recording: &RecordingV1, fail: &ReplayFail) -> Option<Triage> {
    let ReplayFail::EngineError { message, .. } = fail else { return None };
    if !message.contains("out of range for TriggerOrder") {
        return None;
    }
    if recording.end.reason != "disconnect" {
        return None;
    }
    let last_decision = recording.rows.iter().rposition(|r| {
        matches!(r, Row::Action(_) | Row::Selection(_) | Row::EffectActivation(_))
    })?;
    let Row::Selection(sel) = &recording.rows[last_decision] else { return None };
    if sel.prompt != "MultipleSkills" || !sel.int_value.is_some_and(|v| v >= 0) {
        return None;
    }
    Some(Triage {
        quirk_id: RECORDER_STALE_MULTIPLESKILLS_INDEX.to_string(),
        evidence: format!(
            "step {}: final row is MultipleSkills index {} (out of range for the offered \
             effects) with no activation after it and a `disconnect` end",
            sel.step,
            sel.int_value.unwrap_or(-1)
        ),
    })
}

/// `R-RECORDER-ATTACKER-INDEX-NOT-DIGIMON`: the recording's LAST decision is an
/// attack whose attacker index, read against that same row's own `board_pX`
/// snapshot, names a card that cannot attack (an Option or a Tamer). The
/// recorder's index space and its board snapshot disagree for that row (seen
/// with a 4-Digimon, 1-Digimon, 4-Delay-option board), so the operand cannot be
/// trusted and engine legality cannot repair it: several Digimon could attack.
///
/// Signature, all required: an `illegal_action` on an attack-range action; the
/// last `action` row of the recording is an attack-range action of the same
/// actor; its attacker slot, in its own board snapshot, is a card whose kind is
/// Option or Tamer.
fn attacker_index_not_digimon(
    recording: &RecordingV1,
    fail: &ReplayFail,
    cards: &HashMap<String, CardData>,
) -> Option<Triage> {
    use digimon_engine::action::space;
    let ReplayFail::IllegalAction(ia) = fail else { return None };
    if !(space::ATTACK_START..space::ATTACK_END).contains(&ia.action_id) {
        return None;
    }
    let last = recording.rows.iter().rev().find_map(|r| match r {
        Row::Action(a) => Some(a),
        _ => None,
    })?;
    if last.actor != ia.actor || !(space::ATTACK_START..space::ATTACK_END).contains(&last.action_id) {
        return None;
    }
    let (attacker, _target) = space::decode_attack(last.action_id);
    let board = if last.actor == 0 { last.board_p0.as_ref() } else { last.board_p1.as_ref() }?;
    let named = board.get(attacker as usize)?;
    let kind = cards.get(named)?.card_kind;
    if !matches!(kind, CardKind::Option | CardKind::Tamer) {
        return None;
    }
    Some(Triage {
        quirk_id: RECORDER_ATTACKER_INDEX_NOT_DIGIMON.to_string(),
        evidence: format!(
            "step {}: final decision is attack_{}_to_{} by P{}, but slot {} of the row's own board \
             snapshot is {} ({:?}), which cannot attack",
            last.step,
            attacker,
            _target,
            last.actor,
            attacker,
            named,
            kind
        ),
    })
}

/// `Q-DCGO-OPTION-WITHOUT-COLOR-SOURCE`: DCGO completed the play of an Option
/// (a plain Option, or a DUAL card's Option face) for a player whose battle area
/// was EMPTY. An Option's colour requirement needs a Digimon or Tamer of each of
/// its colours on your field (CR 4.3 4-22-2/4-22-3) and `<Use Req.>` only
/// waives it "with specific Digimon and/or Tamers on the field" (16-42-3), so no
/// card can satisfy either on an empty field.
///
/// Signature, all required: an `illegal_action` on a play-hand action; some
/// main-phase `action` row with that exact id and a `card_id` of kind Option or
/// Dual, whose actor's `board_pX` snapshot is empty, and for which DCGO
/// RESOLVED the play (a correlated `action_detail` with `cost_paid > 0`).
fn option_without_color_source(
    recording: &RecordingV1,
    fail: &ReplayFail,
    cards: &HashMap<String, CardData>,
) -> Option<Triage> {
    use digimon_engine::action::space;
    let ReplayFail::IllegalAction(ia) = fail else { return None };
    if !(space::PLAY_HAND_START..space::PLAY_HAND_END).contains(&ia.action_id) {
        return None;
    }
    for (i, row) in recording.rows.iter().enumerate() {
        let Row::Action(a) = row else { continue };
        if a.actor != ia.actor || a.action_id != ia.action_id || a.source != "main_phase" {
            continue;
        }
        let Some(card) = a.card_id.as_deref().and_then(|id| cards.get(id)) else { continue };
        if !matches!(card.card_kind, CardKind::Option | CardKind::Dual) {
            continue;
        }
        let board = if a.actor == 0 { a.board_p0.as_ref() } else { a.board_p1.as_ref() };
        if !board.is_some_and(|b| b.is_empty()) {
            continue;
        }
        let resolved = recording.rows[i + 1..].iter().any(|r| {
            matches!(r, Row::ActionDetail(d)
                if d.step == a.step && d.actor == a.actor && d.cost_paid > 0)
        });
        if !resolved {
            continue;
        }
        return Some(Triage {
            quirk_id: DCGO_OPTION_WITHOUT_COLOR_SOURCE.to_string(),
            evidence: format!(
                "step {}: P{} played {} ({:?}) from hand with an EMPTY battle area and DCGO \
                 resolved it; an Option's colour requirement needs a Digimon or Tamer on the field",
                a.step, a.actor, card.card_id, card.card_kind
            ),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording::parse_jsonl;
    use crate::replay::MemoryMismatch;

    fn card(id: &str, kind: CardKind, evo: &[(u8, u16)]) -> CardData {
        let mut c = digimon_engine::debug_runner::make_test_card(id, id);
        c.card_kind = kind;
        c.evo_costs = evo
            .iter()
            .map(|(level, memory_cost)| digimon_engine::card_data::EvoCost {
                card_color: 0,
                level: *level,
                memory_cost: *memory_cost,
            })
            .collect();
        c
    }

    const HEADER: &str = r#"{"v":1,"type":"game_start","game_id":"g","timestamp":"t","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":[],"opp_deck_post_shuffle":[]}"#;
    const END: &str = r#"{"type":"game_end","winner":-1,"reason":"interrupted","total_steps":9}"#;

    fn recording(rows: &[&str]) -> RecordingV1 {
        let mut text = String::from(HEADER);
        for r in rows {
            text.push('\n');
            text.push_str(r);
        }
        text.push('\n');
        text.push_str(END);
        parse_jsonl(&text).expect("parse")
    }

    fn cards() -> HashMap<String, CardData> {
        [
            card("TAMER", CardKind::Tamer, &[]),
            card("MON", CardKind::Digimon, &[(3, 2)]),
            card("HOST", CardKind::Digimon, &[]),
        ]
        .into_iter()
        .map(|c| (c.card_id.clone(), c))
        .collect()
    }

    fn mismatch(recorded: i64, replayed: i64) -> ReplayFail {
        ReplayFail::MemoryMismatch(MemoryMismatch {
            digivolve_operand_fallbacks: Vec::new(),
            step: 9,
            recorded,
            replayed,
            my_player_id: 0,
        })
    }

    const ACT: &str = r#"{"type":"action","step":12,"actor":0,"action_id":401,"phase":"Main","source":"main_phase","card_id":"MON","memory":8,"board_p0":["HOST","TAMER"],"board_p1":[]}"#;
    const DETAIL: &str = r#"{"type":"action_detail","step":12,"actor":0,"card_id":"MON","cost_paid":0,"memory_after":8,"alt_path":"cost_modifier"}"#;
    const AFTER: &str = r#"{"type":"selection","step":13,"actor":0,"prompt":"SelectPermanentEffect","phase":"Main","board_p0":["MON","HOST"],"board_p1":[]}"#;

    #[test]
    fn recognises_the_documented_signature() {
        let rec = recording(&[ACT, DETAIL, AFTER]);
        let t = triage_failure(&rec, &mismatch(8, 6), &cards()).expect("quirk");
        assert_eq!(t.quirk_id, TAMER_PHANTOM_DIGIVOLVE);
        assert!(t.evidence.contains("TAMER"), "{}", t.evidence);
    }

    #[test]
    fn a_legal_digivolve_onto_a_digimon_is_not_a_quirk() {
        // The card that left the board is a Digimon, not a Tamer.
        let after = AFTER.replace(r#"["MON","HOST"]"#, r#"["MON","TAMER"]"#);
        let rec = recording(&[ACT, DETAIL, &after]);
        assert!(triage_failure(&rec, &mismatch(8, 6), &cards()).is_none());
    }

    #[test]
    fn a_digimon_target_the_engine_rejected_is_a_quirk() {
        // HOST (a Digimon, not a Tamer) is the card that left the board.
        let after = AFTER.replace(r#"["MON","HOST"]"#, r#"["MON","TAMER"]"#);
        let rec = recording(&[ACT, DETAIL, &after]);
        let with_fallback = |cards_fell_back: &[&str]| {
            let mut fail = mismatch(8, 6);
            if let ReplayFail::MemoryMismatch(mm) = &mut fail {
                mm.digivolve_operand_fallbacks =
                    cards_fell_back.iter().map(|c| c.to_string()).collect();
            }
            fail
        };
        // The engine rejected the target: the digivolve only replayed through the
        // recorded-operand fallback.
        let t = triage_failure(&rec, &with_fallback(&["MON"]), &cards()).expect("quirk");
        assert_eq!(t.quirk_id, TAMER_PHANTOM_DIGIVOLVE);
        assert!(t.evidence.contains("ineligible Digimon"), "{}", t.evidence);
        // No fallback evidence: the departed Digimon could have been a legal
        // target, so this stays an ordinary failure.
        assert!(triage_failure(&rec, &mismatch(8, 6), &cards()).is_none());
        // A fallback for some OTHER card proves nothing about this one.
        assert!(triage_failure(&rec, &with_fallback(&["HOST"]), &cards()).is_none());
    }

    #[test]
    fn a_paid_digivolve_is_not_a_quirk() {
        let detail = DETAIL.replace(r#""cost_paid":0"#, r#""cost_paid":2"#);
        let rec = recording(&[ACT, &detail, AFTER]);
        assert!(triage_failure(&rec, &mismatch(8, 6), &cards()).is_none());
    }

    #[test]
    fn a_mismatch_at_some_other_value_is_not_a_quirk() {
        let rec = recording(&[ACT, DETAIL, AFTER]);
        assert!(triage_failure(&rec, &mismatch(5, 3), &cards()).is_none());
    }

    #[test]
    fn other_failure_kinds_are_never_triaged() {
        let rec = recording(&[ACT, DETAIL, AFTER]);
        let fail = ReplayFail::EngineError { step: Some(9), message: "x".into() };
        assert!(triage_failure(&rec, &fail, &cards()).is_none());
    }

    const MAIN_ACT: &str = r#"{"type":"effect_activation","step":6,"actor":0,"card_id":"MON","effect_name":"play with discount","effect_description":"[Main] [Once Per Turn] Play 1 card.","is_optional":true,"executed":true,"phase":"Main"}"#;
    const PASS_ROW: &str = r#"{"type":"action","step":5,"actor":0,"action_id":62,"phase":"Breeding","source":"breeding","memory":3,"board_p0":[],"board_p1":[]}"#;
    const FIELD_ACT: &str = r#"{"type":"action","step":5,"actor":0,"action_id":1005,"phase":"Main","source":"main_phase","memory":3,"board_p0":["MON"],"board_p1":[]}"#;

    #[test]
    fn an_executed_main_ability_with_no_activation_action_is_a_recorder_gap() {
        let rec = recording(&[PASS_ROW, MAIN_ACT]);
        let t = triage_failure(&rec, &mismatch(2, 3), &cards()).expect("recorder gap");
        assert_eq!(t.quirk_id, RECORDER_MISSING_ACTIVATION_ACTION);
        // A recorded activation action means the replay can run it: not a gap.
        let rec = recording(&[FIELD_ACT, MAIN_ACT]);
        assert!(triage_failure(&rec, &mismatch(2, 3), &cards()).is_none());
        // A non-[Main] effect needs no activation action.
        let other = MAIN_ACT.replace("[Main] [Once Per Turn]", "[On Play]");
        let rec = recording(&[PASS_ROW, &other]);
        assert!(triage_failure(&rec, &mismatch(2, 3), &cards()).is_none());
    }

    const MS_ROW: &str = r#"{"type":"selection","step":26,"actor":0,"prompt":"MultipleSkills","phase":"Main","int_value":5}"#;

    fn engine_err(msg: &str) -> ReplayFail {
        ReplayFail::EngineError { step: Some(20), message: msg.to_string() }
    }

    fn recording_ending(rows: &[&str], reason: &str) -> RecordingV1 {
        let mut text = String::from(HEADER);
        for r in rows {
            text.push('\n');
            text.push_str(r);
        }
        text.push('\n');
        text.push_str(&END.replace("interrupted", reason));
        parse_jsonl(&text).expect("parse")
    }

    #[test]
    fn a_final_out_of_range_multipleskills_index_is_a_recorder_gap() {
        let msg = "selection resolution: int payload 5 out of range for TriggerOrder prompt 'x' (2 valid ids)";
        let rec = recording_ending(&[MS_ROW], "disconnect");
        let t = triage_failure(&rec, &engine_err(msg), &cards()).expect("recorder gap");
        assert_eq!(t.quirk_id, RECORDER_STALE_MULTIPLESKILLS_INDEX);
        // Any other end reason keeps it a failure.
        let rec = recording_ending(&[MS_ROW], "interrupted");
        assert!(triage_failure(&rec, &engine_err(msg), &cards()).is_none());
        // So does a different error.
        let rec = recording_ending(&[MS_ROW], "disconnect");
        assert!(triage_failure(&rec, &engine_err("something else"), &cards()).is_none());
        // And an index that is followed by an activation (the game went on).
        let act = r#"{"type":"effect_activation","step":27,"actor":0,"card_id":"MON","executed":true,"phase":"Main"}"#;
        let rec = recording_ending(&[MS_ROW, act], "disconnect");
        assert!(triage_failure(&rec, &engine_err(msg), &cards()).is_none());
    }

    fn illegal(actor: u8, action_id: u16) -> ReplayFail {
        ReplayFail::IllegalAction(crate::replay::IllegalAction {
            step: 36,
            actor,
            action_id,
            phase: "Main".into(),
            source: "Main".into(),
            sample_legal_ids: vec![62],
            board: String::new(),
        })
    }

    fn more_cards() -> HashMap<String, CardData> {
        let mut m = cards();
        for c in [
            card("OPT", CardKind::Option, &[]),
            card("DUAL", CardKind::Dual, &[]),
        ] {
            m.insert(c.card_id.clone(), c);
        }
        m
    }

    // attack_4_to_14: ATTACK_START(100) + 4 * 15 + 14 = 174
    const ATTACK_ROW: &str = r#"{"type":"action","step":47,"actor":0,"action_id":174,"phase":"Main","source":"main_phase","memory":3,"board_p0":["MON","MON","MON","MON","HOST","OPT","OPT"],"board_p1":[]}"#;

    #[test]
    fn a_final_attack_by_a_non_digimon_slot_is_a_recorder_gap() {
        // Slot 4 of the row's own board is HOST (a Digimon): not a gap.
        let rec = recording(&[ATTACK_ROW]);
        assert!(triage_failure(&rec, &illegal(0, 129), &more_cards()).is_none());
        // Slot 4 is an Option: the recorded operand contradicts its own snapshot.
        let bad = ATTACK_ROW.replace(r#""HOST","OPT","OPT""#, r#""OPT","HOST","OPT""#);
        let rec = recording(&[&bad]);
        let t = triage_failure(&rec, &illegal(0, 129), &more_cards()).expect("recorder gap");
        assert_eq!(t.quirk_id, RECORDER_ATTACKER_INDEX_NOT_DIGIMON);
        // Another actor, a non-attack failure, or a row that is not the last
        // action all keep it a failure.
        assert!(triage_failure(&rec, &illegal(1, 129), &more_cards()).is_none());
        assert!(triage_failure(&rec, &illegal(0, 5), &more_cards()).is_none());
        let after = r#"{"type":"action","step":48,"actor":0,"action_id":62,"phase":"Main","source":"main_phase","memory":3,"board_p0":[],"board_p1":[]}"#;
        let rec = recording(&[&bad, after]);
        assert!(triage_failure(&rec, &illegal(0, 129), &more_cards()).is_none());
    }

    const OPT_PLAY: &str = r#"{"type":"action","step":7,"actor":0,"action_id":0,"phase":"Main","source":"main_phase","card_id":"OPT","memory":3,"board_p0":[],"board_p1":[]}"#;
    const OPT_DETAIL: &str = r#"{"type":"action_detail","step":7,"actor":0,"card_id":"OPT","cost_paid":6,"memory_after":-3}"#;

    #[test]
    fn an_option_played_onto_an_empty_field_is_a_dcgo_quirk() {
        let rec = recording(&[OPT_PLAY, OPT_DETAIL]);
        let t = triage_failure(&rec, &illegal(0, 0), &more_cards()).expect("quirk");
        assert_eq!(t.quirk_id, DCGO_OPTION_WITHOUT_COLOR_SOURCE);
        // A DUAL card's Option face counts too.
        let dual = OPT_PLAY.replace(r#""card_id":"OPT""#, r#""card_id":"DUAL""#);
        let dual_detail = OPT_DETAIL.replace(r#""card_id":"OPT""#, r#""card_id":"DUAL""#);
        let rec = recording(&[&dual, &dual_detail]);
        assert!(triage_failure(&rec, &illegal(0, 0), &more_cards()).is_some());
        // A Digimon on the field is a possible colour source: not a quirk.
        let occupied = OPT_PLAY.replace(r#""board_p0":[]"#, r#""board_p0":["HOST"]"#);
        let rec = recording(&[&occupied, OPT_DETAIL]);
        assert!(triage_failure(&rec, &illegal(0, 0), &more_cards()).is_none());
        // DCGO must have RESOLVED the play (a paid action_detail).
        let rec = recording(&[OPT_PLAY]);
        assert!(triage_failure(&rec, &illegal(0, 0), &more_cards()).is_none());
        // A Digimon card is not an Option.
        let mon = OPT_PLAY.replace(r#""card_id":"OPT""#, r#""card_id":"MON""#);
        let mon_detail = OPT_DETAIL.replace(r#""card_id":"OPT""#, r#""card_id":"MON""#);
        let rec = recording(&[&mon, &mon_detail]);
        assert!(triage_failure(&rec, &illegal(0, 0), &more_cards()).is_none());
    }

    #[test]
    fn the_quarantine_manifest_matches_only_by_game_id() {
        let q: Quarantine = serde_json::from_str(
            r#"{"quarantined":[{"game_id":"g","id":"R-QUARANTINE-X","reason":"why"}]}"#,
        )
        .unwrap();
        let rec = recording(&[PASS_ROW]);
        assert_eq!(q.triage(&rec), Some(("R-QUARANTINE-X".to_string(), "why".to_string())));
        let other: Quarantine = serde_json::from_str(
            r#"{"quarantined":[{"game_id":"someone-else","id":"R","reason":"r"}]}"#,
        )
        .unwrap();
        assert!(other.triage(&rec).is_none());
    }
}
