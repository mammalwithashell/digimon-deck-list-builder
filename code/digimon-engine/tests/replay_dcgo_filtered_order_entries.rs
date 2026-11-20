//! DCGO replay: a card-less `MultipleSkills` index against an engine trigger
//! order that offers ONE entry because the engine filtered the others.
//!
//! DCGO's `MultipleSkills` lists every stacked effect whose `CanActivate`
//! holds (MultipleSkills.cs:217-221) and asks the order only when 2+ remain
//! (`skillInfos_active.Count == 1` activates the lone one silently,
//! :273-278). Karakurumon (EX9-032)'s "[On Play] [When Digivolving] By deleting
//! 1 of your Tokens or other [Puppet] trait Digimon, ..." has a
//! `CanActivateCondition` that only checks the card is on the field
//! (EX9_032.cs), so DCGO lists it next to Mirai Kinosaki (EX9-067)'s
//! "[Your Turn] When any of your Digimon digivolve into a [Puppet]..." even
//! with no Token or other Puppet to delete. The engine follows CR 4.3 15-7-4
//! (an optional processing condition that cannot be executed cannot be
//! chosen): Karakurumon's trigger is not offered, the order prompt holds ONE
//! entry, and DCGO's index (1) names no entry of it.
//!
//! The index is not read positionally. The rows after it decide: the actor's
//! "use it?" yes and a hand pick that only the OFFERED trigger asks for prove
//! the offered trigger is the one DCGO activated (a Karakurumon activation
//! would ask "Select 1 Digimon to delete" instead). With nothing to prove it,
//! or with a pick the offered trigger cannot answer, the replay stops.
//!
//! The board is synthetic (real EX9-032 / EX9-067 YAML on a staged game); no
//! recording data is used. Not covered here: the converter that writes rows.

use std::collections::HashMap;

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, level: u8, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    if level == 4 {
        c.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 2 }];
    }
    c
}

/// Own turn: Mirai Kinosaki (EX9-067) and a level 4 [Puppet] base on the
/// field, Karakurumon (EX9-032) in hand next to a [Puppet] Digimon to play.
/// The base digivolves into Karakurumon through its printed `Lv.4 w/[Puppet]`
/// route; no OTHER Token or [Puppet] Digimon stands, so Karakurumon's costed
/// [When Digivolving] cannot be paid.
fn digivolved_into_karakurumon(other_puppet: bool) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("EX9-032")
        .expect("EX9-032 YAML loads")
        .dsl_card("EX9-067")
        .expect("EX9-067 YAML loads")
        .add_card(digimon("PUP-BASE", 4, 4, &["Puppet"]))
        .add_card(digimon("PUP-OTHER", 4, 4, &["Puppet"]))
        .add_card(digimon("PUP-HAND", 4, 6, &["Puppet"]))
        .add_card(digimon("PLAIN-HAND", 4, 6, &[]))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["EX9-032", "PUP-HAND", "PLAIN-HAND"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.place_on_field(0, "EX9-067", Some(0));
    r.place_on_field(0, "PUP-BASE", Some(0));
    if other_puppet {
        r.place_on_field(0, "PUP-OTHER", Some(0));
    }
    let base = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "PUP-BASE")
        .expect("base on field");
    assert!(r.game.digivolve_from_hand(0, 0, base, PlaySource::ByEffect));
    r
}

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
}

impl RecordingSource for StagedDcgo {
    fn build_initial_game(&self, _: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
        Ok(self.game.clone())
    }
    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        *game = self.game.clone();
        Ok(())
    }
    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }
    fn default_policy(&self) -> StepPolicy {
        StepPolicy::CheckThenApply
    }
    fn omits_dcgo_silent_prompts(&self) -> bool {
        true
    }
}

fn row(json: serde_json::Value) -> StepSpec {
    let selection: SelectionRow = serde_json::from_value(json).expect("selection row");
    StepSpec {
        actor: selection.actor,
        action_id: 0,
        phase: "Main".into(),
        source: "selection".into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection: Some(selection),
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

fn session(runner: DebugRunner, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game: runner.game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

fn index(i: i64) -> StepSpec {
    row(serde_json::json!({"step":0,"actor":0,"prompt":"MultipleSkills","int_value":i}))
}

fn use_it(yes: bool) -> StepSpec {
    row(serde_json::json!({"step":1,"actor":0,"prompt":"generic_bool","bool_value":yes,"unstamped":true}))
}

fn hand_pick(card: &str) -> StepSpec {
    row(serde_json::json!({"step":2,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","card_ids":[card]}))
}

fn on_field(game: &Game, card_id: &str) -> bool {
    game.players[0].battle_area.iter().any(|p| p.top_card().card_id(&game.card_data) == card_id)
}

fn stop_reason(s: &ReplaySession) -> String {
    assert!(s.is_paused(), "the replay must stop");
    match s.divergences() {
        [d] => match &d.kind {
            DivergenceKind::SelectionResolution { reason } => reason.clone(),
            other => panic!("expected SelectionResolution, got {other:?}"),
        },
        other => panic!("expected one divergence, got {other:?}"),
    }
}

/// The engine offers ONE entry (Mirai's) and Karakurumon's costed trigger
/// stays queued, filtered.
fn assert_lone_mirai_entry(game: &Game) {
    let p = game.pending_selection.as_ref().expect("a trigger order is parked");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder), "{:?}", p.kind);
    let offered: Vec<_> = p
        .effect_choices
        .as_ref()
        .expect("entries")
        .iter()
        .filter(|e| p.valid_action_ids.contains(&e.action_id))
        .map(|e| e.label.clone())
        .collect();
    assert_eq!(offered.len(), 1, "{offered:?}");
    assert!(offered[0].starts_with("EX9-067"), "{offered:?}");
}

/// The red case, in miniature: index 1 against the lone offered entry, then
/// "use it?" yes and the hand pick only Mirai's trigger asks for.
#[test]
fn an_index_past_the_lone_offered_entry_is_proved_by_the_pick_it_leads_to() {
    let r = digivolved_into_karakurumon(false);
    assert_lone_mirai_entry(&r.game);
    let mut s = session(r, vec![index(1), use_it(true), hand_pick("PUP-HAND")]);
    for _ in 0..3 {
        s.step();
    }
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(on_field(&s.game, "PUP-HAND"), "Mirai's trigger played the picked Puppet");
    assert!(!on_field(&s.game, "EX9-067"), "Mirai was returned to the deck as the cost");
}

/// The index may also be 0 (DCGO's own order is not ours): the pick proves it
/// the same way.
#[test]
fn index_zero_with_the_same_proof_is_accepted_too() {
    let r = digivolved_into_karakurumon(false);
    let mut s = session(r, vec![index(0), use_it(true), hand_pick("PUP-HAND")]);
    for _ in 0..3 {
        s.step();
    }
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(on_field(&s.game, "PUP-HAND"));
}

/// Nothing after the index names a card: it cannot tell Mirai's trigger from
/// the filtered Karakurumon one (both start with "use it?"), so the replay
/// stops with nothing applied.
#[test]
fn an_index_with_no_pick_to_prove_it_stops() {
    let r = digivolved_into_karakurumon(false);
    let before = r.game.verification_digest();
    let mut s = session(r, vec![index(1), use_it(true)]);
    s.step();
    let reason = stop_reason(&s);
    assert!(reason.contains("nothing proves DCGO chose the offered trigger"), "{reason}");
    assert_eq!(s.game.verification_digest(), before, "nothing was applied");
}

/// A cancel after the "use it?" yes does not discriminate: Mirai's optional hand
/// pick and the filtered Karakurumon's cost pick can both be cancelled, so it is
/// no proof DCGO chose the offered trigger. The replay stops with nothing applied
/// (paying Mirai's cost would be a guess).
#[test]
fn an_index_followed_by_a_cancel_stops() {
    let r = digivolved_into_karakurumon(false);
    let before = r.game.verification_digest();
    let cancel = row(serde_json::json!({"step":2,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","cancel":true}));
    let mut s = session(r, vec![index(1), use_it(true), cancel]);
    s.step();
    let reason = stop_reason(&s);
    assert!(reason.contains("nothing proves DCGO chose the offered trigger"), "{reason}");
    assert_eq!(s.game.verification_digest(), before, "nothing was applied");
    assert!(on_field(&s.game, "EX9-067"), "Mirai was not returned to the deck");
}

/// A pick the offered trigger cannot answer (PLAIN-HAND is no [Puppet]; only a
/// Puppet is a legal pick) is not evidence for it.
#[test]
fn an_index_followed_by_a_pick_the_offered_trigger_cannot_take_stops() {
    let r = digivolved_into_karakurumon(false);
    let before = r.game.verification_digest();
    let mut s = session(r, vec![index(1), use_it(true), hand_pick("PLAIN-HAND")]);
    s.step();
    let reason = stop_reason(&s);
    assert!(reason.contains("no entry of the"), "{reason}");
    assert_eq!(s.game.verification_digest(), before, "nothing was applied");
}

/// "Don't use it" after the index cannot say WHICH effect DCGO declined: the
/// replay stops instead of declining the offered trigger on a guess.
#[test]
fn an_index_followed_by_a_decline_stops() {
    let r = digivolved_into_karakurumon(false);
    let mut s = session(r, vec![index(1), use_it(false), hand_pick("PUP-HAND")]);
    s.step();
    assert!(s.is_paused());
    assert!(!on_field(&s.game, "PUP-HAND"));
}

/// DCGO's list holds the offered entry plus the one the engine filtered (2),
/// so its index cannot be 2.
#[test]
fn an_index_beyond_every_effect_dcgo_could_have_listed_stops() {
    let r = digivolved_into_karakurumon(false);
    let mut s = session(r, vec![index(2), use_it(true), hand_pick("PUP-HAND")]);
    s.step();
    let reason = stop_reason(&s);
    assert!(reason.contains("cannot have an index 2"), "{reason}");
}


// ── Several entries of ONE card in front of a card-named index ──────────────
//
// Angewomon (EX12-044) attacks with Virus Busters (EX12-069) face up in
// security. The engine's order prompt lists several EX12-044 entries (the
// shared -4000 effect and the optional hand digivolve) plus EX12-069's.

fn vb(id: &str, level: u8, evo_from: Option<u8>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(6000);
    c.play_cost = 5;
    c.traits = vec!["VB".into()];
    if let Some(from) = evo_from {
        c.evo_costs = vec![EvoCost { card_color: 2, level: from, memory_cost: 3 }];
    }
    c
}

fn angewomon_attacks() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut opp = vb("OPP", 4, None);
    opp.traits.clear();
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-044")
        .expect("EX12-044 YAML loads")
        .dsl_card("EX12-069")
        .expect("EX12-069 YAML loads")
        .add_card(vb("SRC-A", 4, None))
        .add_card(vb("SRC-B", 4, None))
        .add_card(vb("VB-L5", 5, Some(4)))
        .add_card(vb("VB-L6", 6, Some(5)))
        .add_card(opp)
        .add_card(make_test_card("SEC", "Security"))
        .hand(0, &["VB-L6", "VB-L5"])
        .security(0, &["EX12-069"])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    r.set_turn(1);
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    r.place_stack(1, &["OPP"]);
    let angewomon = r.place_stack(0, &["SRC-A", "SRC-B", "EX12-044"]);
    (r, angewomon)
}

fn same_card_index(index: i64) -> StepSpec {
    row(serde_json::json!({"step":0,"actor":0,"prompt":"MultipleSkills","int_value":index,"chosen_effect_card":"EX12-044"}))
}

fn dp_of_opponent(game: &Game) -> i32 {
    let handle = digimon_engine::permanent::PermanentHandle { player: 1, index: 0 };
    game.effective_dp(handle).expect("the opponent's Digimon stands")
}

/// The pick right after the index is the chosen effect's own: only the
/// optional hand digivolve asks for a hand card. The mandatory -4000 effects
/// run silently on the lone opponent Digimon and lead back to the order, so
/// they are not the effect DCGO chose (it would have asked `MultipleSkills`
/// again with 2+ effects left). The digivolve happens first, which drops the
/// -4000 effects with their source (no longer the top card).
#[test]
fn a_card_named_index_over_same_card_entries_is_the_one_whose_own_prompt_takes_the_pick() {
    let (mut r, a) = angewomon_attacks();
    r.attack_player(a, 1, false);
    let before_dp = dp_of_opponent(&r.game);
    let mut s = session(r, vec![same_card_index(2), hand_pick("VB-L6")]);
    s.step();
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 2);
    assert!(
        s.game.players[0].battle_area.iter().any(|p| p.top_card().card_id(&s.game.card_data) == "VB-L6"),
        "the digivolve resolved first"
    );
    assert_eq!(dp_of_opponent(&s.game), before_dp, "the -4000 effects did not run first");
}
