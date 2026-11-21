//! DCGO replay: an EFFECT-initiated DNA digivolve (Nyaromon (EX12-001)'s
//! inherited [End of Your Turn], and every `may_dna_digivolve_now` card) is
//! recorded as FOUR rows, none of them the engine's partner pick.
//!
//! DCGO `DNADigivolvePermanentsIntoHandOrTrashCard`
//! (`CardEffectCommons/DNADigivolveEffects.cs:456-640`) asks, in order:
//!   1. `SelectHandEffect` - the DNA card in hand;
//!   2. `SelectJogressEffect.SelectDigivolutionRoots` - the two material
//!      Digimon, recorded as TWO `SelectPermanentEffect` root rows (the order
//!      the player clicked them in);
//!   3. then, for the "may attack" follow-up, a `SelectAttackEffect`.
//!
//! The engine asks the PARTNER first (an `OwnField` pick), then the hand card
//! (`MayDnaPartnerSelection` -> `MayDnaResultSelection`), then the optional
//! attack. The replay driver therefore has to read the two root rows as the
//! engine's partner answer: the anchor is the carrier of the effect, so the
//! root pair must be exactly `{anchor, partner}` (CR 4.3 8-2 "digivolve
//! multiple Digimon into one": the effect names the pair, not an order).
//!
//! Without that fold the first root row met the optional "may attack" prompt
//! and the second a four-entry trigger order ("frame target (player 1, slot 1)
//! matches none of [30..33]"). All boards below are synthetic.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::{GameEnd, GameStart, RecordingV1, Row, SelectionRow};
use digimon_engine::debug_runner::{make_test_card, make_test_dna_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::runners::replay::{DivergenceKind, ReplaySession};
use digimon_engine::{CardColor, Game, TriggerSource};

fn shell_db() -> HashMap<String, CardData> {
    CardData::load_from_str(
        r#"{
        "ST1-03": {
            "card_id": "ST1-03", "card_name_eng": "Koromon",
            "card_effect_class_name": "ST1_03", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["Rookie"], "attribute_eng": ["Free"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }
    }"#,
    )
    .unwrap()
}

fn sel(step: u32, extra: serde_json::Value) -> Row {
    let mut v = serde_json::json!({ "step": step, "actor": 0, "phase": "Main" });
    for (k, val) in extra.as_object().unwrap() {
        v[k] = val.clone();
    }
    Row::Selection(serde_json::from_value::<SelectionRow>(v).unwrap())
}

fn session_over(game: Game, rows: Vec<Row>) -> ReplaySession {
    let deck = vec!["ST1-03".to_string(); 50];
    let total = rows.len() as u32;
    let rec = RecordingV1 {
        start: GameStart {
            v: 1,
            game_id: "effect-dna-root-rows".into(),
            timestamp: "t".into(),
            my_player_id: 0,
            first_player: Some(0),
            is_ai: true,
            my_deck_post_shuffle: deck.clone(),
            opp_deck_post_shuffle: Some(deck),
            opp_decklist_composition: None,
            my_egg_deck: None,
            opp_egg_deck: None,
        },
        rows,
        end: GameEnd { winner: -1, reason: "truncated: test".into(), total_steps: total },
    };
    let mut s = ReplaySession::from_dcgo(rec, &shell_db(), false).expect("recording constructs");
    s.game = game;
    s
}

fn vb_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.traits = vec!["VB".into()];
    c
}

/// Own field after `effect_dna_game()` (engine slots): 0 `VB-C`, 1 `VB-A`
/// (carrying Nyaromon (EX12-001) as its source, so it owns the inherited
/// effect), 2 `VB-B`. The DNA card `VB-DNA` (two Lv.4 materials, cost 0) and a
/// decoy `VB-DECOY` are in hand. [End of Your Turn] has fired and the engine is
/// parked on its partner pick.
fn effect_dna_game() -> Game {
    let mut dna = make_test_dna_card("VB-DNA", "VB-DNA", 4, 4, 0);
    dna.colors = vec![CardColor::Yellow];
    dna.level = Some(5);
    dna.dp = Some(20000); // survives the attack on the opponent's security
    dna.traits = vec!["VB".into()];
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-001")
        .expect("EX12-001 YAML")
        .add_card(vb_digimon("VB-A"))
        .add_card(vb_digimon("VB-B"))
        .add_card(vb_digimon("VB-C"))
        .add_card(dna)
        .add_card(vb_digimon("VB-DECOY"))
        .add_card(make_test_card("FILL", "FILL"))
        .hand(0, &["VB-DNA", "VB-DECOY"])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 3])
        .memory(10)
        .start();
    r.place_on_field(0, "VB-C", Some(0));
    let anchor = r.place_stack(0, &["EX12-001", "VB-A"]);
    r.place_on_field(0, "VB-B", Some(0));
    r.game.enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(anchor));
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the partner pick is parked");
    assert_eq!(p.valid_action_ids.len(), 2, "two partner candidates (VB-B, VB-C): {p:?}");
    r.game
}

fn hand_row(step: u32) -> Row {
    sel(step, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["VB-DNA"]}))
}

/// One DCGO material-root row. DCGO's own frame order is `[VB-B, VB-A, VB-C]`
/// (NOT the engine's `[VB-C, VB-A, VB-B]`), so the frame must be remapped
/// through the row's board.
fn root_row(step: u32, frame: i64) -> Row {
    sel(
        step,
        serde_json::json!({
            "prompt": "SelectPermanentEffect",
            "board_p0": ["VB-B", "VB-A", "VB-C"],
            "board_p1": [],
            "targets": [{"player": 0, "frame": frame}],
        }),
    )
}

fn attack_row(step: u32) -> Row {
    sel(step, serde_json::json!({"prompt": "SelectAttackEffect", "targets": [{"player": 1, "frame": -1}]}))
}

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(
            s.divergences().is_empty() && !s.is_paused(),
            "row {before} diverged: {:?}",
            s.divergences()
        );
        assert!(s.current_step() > before, "row {before} did not advance");
    }
}

/// The own field's top cards, sorted (the DNA Digimon's slot is not what these
/// tests pin).
fn field_tops(game: &Game) -> Vec<String> {
    let mut tops: Vec<String> = game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&game.card_data).to_string())
        .collect();
    tops.sort();
    tops
}

fn sources_of(game: &Game, top: &str) -> Vec<String> {
    let perm = game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&game.card_data) == top)
        .expect("permanent on the field");
    let mut ids: Vec<String> = perm.card_sources.iter().map(|c| c.card_id(&game.card_data).to_string()).collect();
    ids.sort();
    ids
}

/// Hand, root (anchor), root (partner), attack: the engine's partner is the
/// non-anchor root (`VB-B`, NOT the lone-prompt auto-answer's pick), the DNA
/// card is the hand row, and the attack row meets the "may attack" prompt.
#[test]
fn hand_then_two_roots_answer_the_partner_hand_and_attack_prompts() {
    let mut s = session_over(
        effect_dna_game(),
        vec![hand_row(0), root_row(1, 1), root_row(2, 0), attack_row(3)],
    );
    replay_all(&mut s);
    let g = &s.game;
    assert_eq!(field_tops(g), vec!["VB-C", "VB-DNA"], "VB-A and VB-B became one DNA Digimon: {:?}", field_tops(g));
    assert_eq!(
        sources_of(g, "VB-DNA"),
        vec!["EX12-001", "VB-A", "VB-B", "VB-DNA"],
        "both materials sit under the DNA card"
    );
    assert_eq!(g.players[1].security.len(), 2, "the attack row answered the may-attack prompt: the DNA Digimon broke a security");
    assert!(g.pending_selection.is_none(), "nothing left to answer: {:?}", g.pending_selection.as_ref().map(|p| &p.prompt));
}

/// The roots may be recorded in either click order: partner first, anchor
/// second names the same pair.
#[test]
fn root_order_does_not_matter() {
    let mut s = session_over(
        effect_dna_game(),
        vec![hand_row(0), root_row(2, 2), root_row(1, 1), attack_row(3)],
    );
    // frame 2 = VB-C in DCGO's board, so this names {VB-C, VB-A}: VB-C is the
    // partner.
    replay_all(&mut s);
    assert_eq!(field_tops(&s.game), vec!["VB-B", "VB-DNA"]);
    assert_eq!(sources_of(&s.game, "VB-DNA"), vec!["EX12-001", "VB-A", "VB-C", "VB-DNA"]);
}

/// A root pair that does not contain the anchor cannot be this effect's pair:
/// the replay stops with a precise reason instead of guessing.
#[test]
fn a_root_pair_without_the_anchor_is_an_honest_divergence() {
    let mut s = session_over(
        effect_dna_game(),
        vec![hand_row(0), root_row(1, 0), root_row(2, 2), attack_row(3)],
    );
    s.step();
    let d = s.divergences();
    assert_eq!(d.len(), 1, "{d:?}");
    match &d[0].kind {
        DivergenceKind::SelectionResolution { reason } => {
            assert!(reason.contains("anchor"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
    assert!(s.is_paused());
    assert_eq!(s.current_step(), 0, "nothing was consumed");
}
