//! DCGO replay: round-4 review findings 11-13 (a `MultipleSkills` row read
//! against the wrong prompt, the remaining positional readings, and the
//! leniency towards a remote seat's unstamped "use this effect?" answer).
//!
//! Every game is staged with `DebugRunner` (synthetic cards, and real card
//! YAML where the engine path is the subject) and served through a DCGO-policy
//! source whose rows are the rows under test. No recording content is used.
//!
//! 11. A `MultipleSkills` row meets the engine's TRIGGER ORDER, never an
//!     unrelated prompt: the prompts DCGO skips in front of it (a one-candidate
//!     field pick, a one-card deck order) are answered first, and the
//!     "ordering index against a prompt that is not an order" fold consumes the
//!     row only against the chosen effect's own prompt.
//! 12. No positional guess survives: a two-branch tie is broken by a trailing
//!     bool only for a card whose DCGO script asks that bool; an int is read
//!     only through the card's own DCGO menu values; a past-the-end int is the
//!     decline only when it is that card's DCGO "Don't" value; and
//!     `MultipleSkills` -1 against a non-optional trigger order is applied only
//!     when every order leads to the same game.
//! 13. An unstamped "use this optional effect?" answer with nothing parked is
//!     a note only while the engine's event log is consistent with it.

use std::collections::HashMap;

use digimon_engine::action::space::{self, MOVE_FROM_BREEDING, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::{GameEnd, GameStart, RecordingV1, Row, SelectionRow};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, GamePhase};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::{EffectChoiceEntry, PendingSelection, SelectionKind, TriggerSource};

// ── Scaffolding ──────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A DCGO-policy source over a staged (clone-safe) game.
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

fn spec(actor: u8, action_id: u16, phase: &str, source: &str, selection: Option<SelectionRow>) -> StepSpec {
    StepSpec {
        actor,
        action_id,
        phase: phase.into(),
        source: source.into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection,
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

/// A P0 selection row in the main phase with `json`'s fields.
fn sel(json: serde_json::Value) -> StepSpec {
    let mut v = serde_json::json!({ "step": 0, "actor": 0, "phase": "Main" });
    for (k, val) in json.as_object().unwrap() {
        v[k] = val.clone();
    }
    let row: SelectionRow = serde_json::from_value(v).expect("selection row");
    spec(row.actor, 0, "Main", "selection", Some(row))
}

/// A staged, clone-safe game (real engine prompts only) served as a DCGO recording.
fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

/// A DCGO recording of selection `rows` over a game whose parked prompt is
/// hand-built (its closure callback does not survive `Game::clone`, so the game
/// is moved in, not cloned).
fn recorded(game: Game, rows: Vec<serde_json::Value>) -> ReplaySession {
    let rows: Vec<Row> = rows
        .into_iter()
        .enumerate()
        .map(|(i, json)| {
            let mut v = serde_json::json!({ "actor": 0, "phase": "Main" });
            for (k, val) in json.as_object().unwrap() {
                v[k] = val.clone();
            }
            v["step"] = serde_json::json!(i);
            Row::Selection(serde_json::from_value::<SelectionRow>(v).expect("selection row"))
        })
        .collect();
    let deck = vec!["ST1-03".to_string(); 50];
    let total = rows.len() as u32;
    let rec = RecordingV1 {
        start: GameStart {
            v: 1,
            game_id: "review-r4b".into(),
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
    let db = CardData::load_from_str(
        r#"{ "ST1-03": {
            "card_id": "ST1-03", "card_name_eng": "Koromon", "card_effect_class_name": "ST1_03",
            "play_cost": 3, "dp": 2000, "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["Rookie"], "attribute_eng": ["Free"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": [] } }"#,
    )
    .unwrap();
    let mut s = ReplaySession::from_dcgo(rec, &db, false).expect("recording constructs");
    s.game = game;
    s
}

/// Every row applies without a divergence and advances the cursor.
fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {before} diverged: {:?}", s.divergences());
        assert!(s.current_step() > before, "row {before} did not advance the cursor");
    }
}

/// Exactly one selection-resolution divergence, at row `at`, which halted the
/// replay there; returns its reason.
fn halted_at(s: &ReplaySession, at: u32) -> String {
    let reason = match s.divergences() {
        [d] => {
            assert_eq!(d.step, at, "{d:?}");
            match &d.kind {
                DivergenceKind::SelectionResolution { reason } => reason.clone(),
                other => panic!("expected a selection-resolution divergence, got {other:?}"),
            }
        }
        other => panic!("expected one divergence at row {at}, got {other:?}"),
    };
    assert!(s.is_paused(), "the divergence halts the replay");
    assert_eq!(s.current_step(), at, "the row is not consumed");
    reason
}

fn source_card_id(game: &Game) -> Option<String> {
    let p = game.pending_selection.as_ref()?;
    game.card_data_for_handle(p.source_card).map(|d| d.card_id.clone())
}

/// Main phase, P0 to act, `source` (and MON) on P0's field, and a hand-built
/// prompt of `source` parked for P0. EffectChoice entries come from `labels`
/// (`label`, its action id); otherwise `ids` are the valid ids.
fn parked(source: CardData, kind: SelectionKind, optional: bool, ids: Vec<u16>, labels: &[(&str, u16)]) -> Game {
    let id = source.card_id.clone();
    let mut r = DebugRunner::builder()
        .add_card(source)
        .add_card(make_test_card("MON", "MON"))
        .deck(0, &["MON"; 5])
        .deck(1, &["MON"; 5])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, &id, Some(0));
    r.place_on_field(0, "MON", Some(0));
    r.place_on_field(1, "MON", Some(0));
    let source_card = r.top_card(h);
    let effect_choices = (!labels.is_empty()).then(|| {
        labels
            .iter()
            .map(|(l, a)| EffectChoiceEntry {
                label: (*l).into(),
                action_id: *a,
                source_card: Some(source_card),
                source_kind: Some(EffectSourceKind::Digimon),
                timing: None,
                is_optional: false,
                keyword: None,
                observation_metadata: Default::default(),
                count_choice: None,
            })
            .collect::<Vec<_>>()
    });
    let valid = if labels.is_empty() { ids } else { labels.iter().map(|(_, a)| *a).collect() };
    r.game.pending_selection = Some(PendingSelection {
        card_choice: None,
        zone_owner: None,
        kind,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: valid,
        is_optional: optional,
        prompt: "test prompt".into(),
        effect_choices,
        source_card,
        source_permanent: Some(h),
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(|_, _| {}),
        on_decline: None,
    });
    r.game
}

fn choice_of(card_id: &str, labels: &[(&str, u16)]) -> Game {
    parked(make_test_card(card_id, card_id), SelectionKind::EffectChoice, false, vec![], labels)
}

fn int_row(v: i64) -> serde_json::Value {
    serde_json::json!({"prompt": "generic_int", "int_value": v})
}

// ── (11) a MultipleSkills row meets the trigger order ────────────────────────

const SOMP_GAIN: &str = r#"
card: TEST-SOMP-GAIN
name: Test SOMP Gain
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Gain 1 memory"
    process:
      - gain_memory: 1
"#;

const SOMP_DRAW: &str = r#"
card: TEST-SOMP-DRAW
name: Test SOMP Draw
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    optional: true
    summary: "[Start of Your Main Phase] You may draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;

/// P0's breeding phase: Kudamon (EX13-026) in the breeding area, ONE
/// [DATA SQUAD] Tamer and the two Start-of-Main-Phase tamers on the field, the
/// deck top HB / DS / FILL. Moving Kudamon reveals 3: HB to hand, DS under the
/// lone Tamer (a one-candidate pick DCGO skips), FILL to the bottom (a one-card
/// order DCGO skips); then the main phase starts and both tamers trigger.
fn kudamon_moves_before_two_main_phase_triggers() -> DebugRunner {
    let mut deck: Vec<&str> = vec!["FILL"; 6];
    deck.extend(["FILL", "DS", "HB"]);
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-026")
        .expect("EX13-026 YAML loads")
        .from_dsl_yaml(SOMP_GAIN)
        .expect("gain fixture compiles")
        .from_dsl_yaml(SOMP_DRAW)
        .expect("draw fixture compiles")
        .add_card(digimon("HB", 4, &["Holy Beast"]))
        .add_card(digimon("DS", 4, &["DATA SQUAD"]))
        .add_card(digimon("FILL", 3, &[]))
        .add_card(tamer("DS-TAMER", &["DATA SQUAD"]))
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.set_turn(3);
    r.set_phase(GamePhase::Breeding);
    r.place_in_breeding(0, "EX13-026");
    r.place_on_field(0, "DS-TAMER", Some(0));
    r.place_on_field(0, "TEST-SOMP-GAIN", Some(0));
    r.place_on_field(0, "TEST-SOMP-DRAW", Some(0));
    r
}

/// raw-20261113T025510Z#g6 file step 28 in miniature: DCGO records the two
/// reveal picks and then its `MultipleSkills` choice of the second trigger;
/// the Tamer pick and the one-card order have no row. The `MultipleSkills`
/// row must meet the engine's trigger order and run the chosen trigger - it
/// was consumed against the one-card order instead, with no note, leaving the
/// order to a later probe.
#[test]
fn multipleskills_row_after_a_skipped_lone_pick_and_order_meets_the_trigger_order() {
    let r = kudamon_moves_before_two_main_phase_triggers();
    let memory_before = r.game.memory;
    let mut s = staged(
        r.game,
        vec![
            spec(0, MOVE_FROM_BREEDING, "Breeding", "breeding_action", None),
            sel(serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["HB"]})),
            sel(serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["DS"]})),
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": 1, "chosen_effect_card": "TEST-SOMP-DRAW"})),
        ],
    );
    replay_all(&mut s);
    assert_eq!(
        source_card_id(&s.game).as_deref(),
        Some("TEST-SOMP-DRAW"),
        "the chosen trigger's own prompt is parked: {:?}",
        s.game.pending_selection.as_ref().map(|p| (&p.kind, &p.prompt))
    );
    assert_eq!(s.game.memory, memory_before, "the other trigger has not resolved yet");
    assert!(
        s.replay_notes().iter().all(|n| !n.contains("MultipleSkills")),
        "the row was not consumed as a no-op: {:?}",
        s.replay_notes()
    );
}

/// An ordering index never answers a prompt of ANOTHER effect: the parked pick
/// belongs to SRC, the row chose OTHER's effect. The replay stops; it used to
/// consume the row with no note.
#[test]
fn multipleskills_index_does_not_consume_an_unrelated_prompt() {
    let game = parked(make_test_card("SRC", "SRC"), SelectionKind::OwnField, false, vec![100, 101], &[]);
    let mut s = recorded(
        game,
        vec![serde_json::json!({"prompt": "MultipleSkills", "int_value": 1, "chosen_effect_card": "OTHER"})],
    );
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("MultipleSkills"), "{reason}");
    assert!(s.game.pending_selection.is_some());
}

/// Regression guard: the chosen effect's OWN prompt (the engine queued only
/// that trigger) still consumes the ordering index, and the prompt stays for
/// the row that answers it.
#[test]
fn multipleskills_index_is_consumed_by_the_chosen_effects_own_prompt() {
    let game = parked(make_test_card("SRC", "SRC"), SelectionKind::OwnField, true, vec![100, 101], &[]);
    let mut s = recorded(
        game,
        vec![serde_json::json!({"prompt": "MultipleSkills", "int_value": 1, "chosen_effect_card": "SRC"})],
    );
    replay_all(&mut s);
    assert!(s.game.pending_selection.is_some(), "the pick waits for its own row");
}

/// A card-less ordering index (a seat without a recorder) in front of the
/// prompt the actor's NEXT row answers: that prompt is the chosen effect's
/// first decision (Hiro Amanokawa (EX12-066)'s "Which effect will you
/// activate?" met by its own int, raw-20261113T025510Z#g3 file step 38), so the
/// index is consumed and the next row answers the prompt. Regression guard.
#[test]
fn cardless_index_is_consumed_by_the_prompt_the_next_row_answers() {
    let game = choice_of("AD1-016", &[("From hand", A), ("From trash", A + 1)]);
    let mut s = recorded(game, vec![serde_json::json!({"prompt": "MultipleSkills", "int_value": 1}), int_row(2)]);
    replay_all(&mut s);
    assert!(s.game.pending_selection.is_none(), "the int answered the choice");
}

/// ... but a card-less index in front of a prompt its next row does NOT
/// answer proves no tie to it: the replay stops instead of consuming it.
#[test]
fn cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer() {
    let game = choice_of("AD1-016", &[("From hand", A), ("From trash", A + 1)]);
    let mut s = recorded(
        game,
        vec![
            serde_json::json!({"prompt": "MultipleSkills", "int_value": 1}),
            serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["NOWHERE"]}),
        ],
    );
    s.step();
    halted_at(&s, 0);
}

const SELF_BUFF: &str = r#"
card: TEST-SELF-BUFF
name: Test Self Buff
kind: digimon
level: 4
color: [yellow]
cost: 4
dp: 4000
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] If you have 2 or more cards in hand, this Digimon gets +1000 DP for the turn. Then draw 1."
    process:
      - if:
          condition:
            count_gte:
              filter: { of: you, zone: [hand] }
              n: 2
          then:
            - add_dp_modifier: { target: source, value: 1000, expiry: end_of_turn }
      - draw: { of: you, count: 1 }
"#;

/// Two copies of one card trigger together, and which copy resolves first
/// decides which of them is buffed. DCGO's row names the card, not the copy,
/// and nothing after it tells them apart: the replay stops. It used to take
/// the entry at DCGO's index, read as a position.
#[test]
fn chosen_card_on_two_order_entries_that_lead_to_different_games_halts() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SELF_BUFF)
        .expect("self-buff fixture compiles")
        .add_card(digimon("FILL", 3, &[]))
        .hand(0, &["FILL"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let a = r.place_on_field(0, "TEST-SELF-BUFF", Some(0));
    let b = r.place_on_field(0, "TEST-SELF-BUFF", Some(0));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(a));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(b));
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("an order prompt is parked");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder), "{:?}", p.kind);
    let mut s = staged(
        r.game,
        vec![sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": 0, "chosen_effect_card": "TEST-SELF-BUFF"}))],
    );
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("is ambiguous"), "{reason}");
}

// ── (12a) a two-branch tie broken by a trailing bool ─────────────────────────

/// Gazimon (BT25-078)'s [On Play] effect body, under another card id: DCGO has
/// no such card, so no DCGO bool can name its branch.
const TWO_WAY: &str = r#"
card: TEST-TWO-WAY
name: Test Two Way
kind: digimon
level: 3
color: [purple]
cost: 3
dp: 1000
traits: [Test]
effects:
  - when: on_play
    summary: "[On Play] Reveal 3; add 1 to hand OR place 1 as bottom source; rest to bottom"
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - select_effect_choice:
          labels:
            - "Add a [Three Musketeers] card to hand"
            - "Place a [Three Musketeers] trait card as bottom digivolution card"
          bind_as: branch
          prompt: "Add or place?"
      - if:
          condition: { equals: [branch, 0] }
          then:
            - choose_from_reveal:
                of: you
                filter: { in_text_contains: "Three Musketeers" }
                destination: hand
                bind_as: hand_pick
                prompt: "Add 1 card with [Three Musketeers] in its text to your hand"
      - if:
          condition: { equals: [branch, 1] }
          then:
            - choose_from_reveal:
                of: you
                optional: true
                filter: { trait_has: "Three Musketeers" }
                destination:
                  bottom_source_of: { target: this }
                bind_as: source_pick
                prompt: "Place 1 [Three Musketeers] trait card as this Digimon's bottom digivolution card"
      - place_remainder_on_deck: { of: you, position: bottom }
"#;

/// `card` played from P0's hand over a deck whose top 3 are TM (a [Three
/// Musketeers] trait card, which both branches can take), FILL, FILL. Its
/// branch choice is parked.
fn two_way_played(card: &str) -> DebugRunner {
    let mut deck: Vec<&str> = vec!["FILL"; 6];
    deck.extend(["FILL", "FILL", "TM"]);
    let b = DebugRunner::builder()
        .add_card(digimon("TM", 4, &["Three Musketeers"]))
        .add_card(digimon("FILL", 3, &[]));
    let b = if card == "BT25-078" {
        b.dsl_card(card).expect("BT25-078 YAML loads")
    } else {
        b.from_dsl_yaml(TWO_WAY).expect("two-way fixture compiles")
    };
    let mut r = b.hand(0, &[card]).deck(0, &deck).deck(1, &["FILL"; 6]).memory(5).start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.play(0, 0);
    let p = r.game.pending_selection.as_ref().expect("the branch choice is parked");
    assert!(matches!(p.kind, SelectionKind::EffectChoice), "{:?} '{}'", p.kind, p.prompt);
    r
}

fn pick_then_bool(value: bool) -> Vec<StepSpec> {
    vec![
        sel(serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["TM"]})),
        sel(serde_json::json!({"prompt": "generic_bool", "bool_value": value})),
    ]
}

fn under(game: &Game, card: &str, source: &str) -> bool {
    game.players[0].battle_area.iter().any(|p| {
        p.top_card().card_id(&game.card_data) == card
            && p.digivolution_cards().iter().any(|c| c.card_id(&game.card_data) == source)
    })
}

/// A card whose DCGO script asks no such bool: both branches take TM, so the
/// pick row proves no branch, and the bool two rows on is not the answer to
/// anything this card asks. The replay stops; it used to read true / false as
/// first / second branch.
#[test]
fn trailing_bool_does_not_break_a_branch_tie_for_a_card_dcgo_does_not_ask_it_of() {
    let r = two_way_played("TEST-TWO-WAY");
    let mut s = staged(r.game, pick_then_bool(false));
    s.step();
    let reason = halted_at(&s, 0);
    assert!(!under(&s.game, "TEST-TWO-WAY", "TM"), "{reason}");
}

/// Regression guard: Gazimon (BT25-078) asks exactly that bool after the pick
/// (BT25_078.cs:67-84: "Add to hand" = true, "Place on digivolution cards" =
/// false), so its false names the place branch.
#[test]
fn gazimon_trailing_bool_false_names_the_place_branch() {
    let r = two_way_played("BT25-078");
    let mut s = staged(r.game, pick_then_bool(false));
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(under(&s.game, "BT25-078", "TM"), "TM went under Gazimon");
}

// ── (12b) an int read through the card's own DCGO menu ───────────────────────

/// An `EffectChoice` entry's id is 30 + its label index (`SEL_REVEAL_START`).
const A: u16 = space::SEL_REVEAL_START;

/// A card with no DCGO int menu at all: the int is not one of its constants and
/// not a position. It used to pick the second entry.
#[test]
fn int_from_a_card_with_no_dcgo_int_menu_halts() {
    let mut s = recorded(choice_of("SRC", &[("Alpha", A), ("Beta", A + 1)]), vec![int_row(1)]);
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("SRC"), "{reason}");
}

/// A zero-based DCGO menu (EX11-022: values 0 / 1 / 2, EX11_022.cs:72-76)
/// whose middle option the engine dropped: 1 names that dropped option, not
/// the entry that now sits at position 1. It used to pick that entry.
#[test]
fn zero_based_int_naming_a_dropped_branch_halts() {
    let mut s = recorded(choice_of("EX11-022", &[("Alpha", A), ("Gamma", A + 2)]), vec![int_row(1)]);
    s.step();
    halted_at(&s, 0);
}

/// Regression guard: a zero-based menu value that names a surviving branch
/// resolves to it.
#[test]
fn zero_based_int_names_its_branch() {
    let mut s = recorded(choice_of("EX11-022", &[("Alpha", A), ("Beta", A + 1)]), vec![int_row(1)]);
    replay_all(&mut s);
    assert!(s.game.pending_selection.is_none());
}

/// A one-based menu past the engine's last entry is the decline only when it
/// is the card's DCGO "Don't" value. BT26-102's 3 is "Trash" in one menu and
/// "Don't play a card" in the other (BT26_102.cs:112, :324): no grounded
/// reading. It used to take the decline-shaped last entry.
#[test]
fn past_the_end_one_based_int_that_is_not_the_cards_decline_halts() {
    let mut s = recorded(choice_of("BT26-102", &[("Battle area", A), ("Don't", A + 1)]), vec![int_row(3)]);
    s.step();
    halted_at(&s, 0);
}

/// Regression guard: AD1-016's 3 is its "Don't" (AD1_016.cs:80), so past the
/// engine's last entry it is the decline.
#[test]
fn past_the_end_one_based_int_that_is_the_cards_decline_is_the_decline() {
    let mut s = recorded(choice_of("AD1-016", &[("From hand", A), ("Don't play", A + 1)]), vec![int_row(3)]);
    replay_all(&mut s);
}

// ── (12c) MultipleSkills -1 against a non-optional trigger order ─────────────

const IF_HAND_GAIN: &str = r#"
card: TEST-HAND-GAIN
name: Test Hand Gain
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] If you have 2 or more cards in hand, gain 1 memory"
    process:
      - if:
          condition:
            count_gte:
              filter: { of: you, zone: [hand] }
              n: 2
          then:
            - gain_memory: 1
"#;

const DRAW_ONE: &str = r#"
card: TEST-DRAW-ONE
name: Test Draw One
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;

/// Two simultaneous mandatory triggers whose result depends on their order
/// (draw first, and the other gains memory). One card in hand.
fn order_dependent_triggers() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(IF_HAND_GAIN)
        .expect("hand-gain fixture compiles")
        .from_dsl_yaml(DRAW_ONE)
        .expect("draw fixture compiles")
        .add_card(digimon("FILL", 3, &[]))
        .hand(0, &["FILL"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let a = r.place_on_field(0, "TEST-HAND-GAIN", Some(0));
    let b = r.place_on_field(0, "TEST-DRAW-ONE", Some(0));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(a));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(b));
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("an order prompt is parked");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder) && !p.is_optional, "{:?}", p.kind);
    r
}

/// DCGO offers "Don't activate these effects" only when every stacked effect
/// is skippable (MultipleSkills.cs:393); the engine holds two mandatory ones
/// whose order changes the game. Which order would run them is not recorded,
/// so the replay stops. It used to take the engine's first entry.
#[test]
fn multipleskills_decline_all_against_an_order_that_matters_halts() {
    let r = order_dependent_triggers();
    let mut s = staged(r.game, vec![sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1}))]);
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("-1"), "{reason}");
}

/// Regression guard: two OPTIONAL triggers - the order prompt's own PASS is
/// DCGO's -1, and nothing resolves.
#[test]
fn multipleskills_decline_all_against_an_optional_order_declines_it() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SOMP_DRAW)
        .expect("draw fixture compiles")
        .add_card(digimon("FILL", 3, &[]))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let a = r.place_on_field(0, "TEST-SOMP-DRAW", Some(0));
    let b = r.place_on_field(0, "TEST-SOMP-DRAW", Some(0));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(a));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(b));
    r.game.drain_effect_queue();
    let hand = r.game.players[0].hand.len();
    let mut s = staged(r.game, vec![sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1}))]);
    replay_all(&mut s);
    assert_eq!(s.game.players[0].hand.len(), hand, "neither draw ran");
    assert!(s.game.pending_selection.is_none());
    let _ = PASS;
}

// ── (13) an unstamped answer with nothing parked ─────────────────────────────

fn unstamped(value: bool) -> StepSpec {
    sel(serde_json::json!({"prompt": "generic_bool", "bool_value": value, "unstamped": true}))
}

/// P0's main phase, nothing parked. With `gained`, a P0 effect
/// (TEST-SOMP-GAIN) has just gained 1 memory with no recorded answer.
fn nothing_parked(gained: bool) -> Game {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SOMP_GAIN)
        .expect("gain fixture compiles")
        .add_card(digimon("FILL", 3, &[]))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    if gained {
        let a = r.place_on_field(0, "TEST-SOMP-GAIN", Some(0));
        r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(a));
        r.game.drain_effect_queue();
    }
    assert!(r.game.pending_selection.is_none());
    r.game
}

/// DCGO used an optional effect ("may", no picks after it), and the engine
/// resolved no effect at all since the last decision: it never offered the
/// effect DCGO used (a "you may gain 1 memory" the engine never triggered).
/// It used to be a note.
#[test]
fn unstamped_yes_with_no_engine_effect_since_the_last_decision_halts() {
    let mut s = staged(nothing_parked(false), vec![unstamped(true)]);
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("never offered"), "{reason}");
}

/// DCGO declined an optional effect, but the engine gained memory through an
/// effect with no recorded answer: the decline names no card, so whether the
/// engine ran the declined effect cannot be told. It used to be a note.
#[test]
fn unstamped_no_after_an_unanswered_engine_memory_gain_halts() {
    let mut s = staged(nothing_parked(true), vec![unstamped(false)]);
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("memory +1 from TEST-SOMP-GAIN"), "{reason}");
}

/// Regression guard: a yes after the engine resolved an effect stays a note
/// (the engine may have run DCGO's "may" as its own mandatory effect).
#[test]
fn unstamped_yes_after_an_engine_effect_is_a_note() {
    let mut s = staged(nothing_parked(true), vec![unstamped(true)]);
    replay_all(&mut s);
    assert!(s.replay_notes().iter().any(|n| n.contains("unstamped")), "{:?}", s.replay_notes());
}

/// Regression guard: a decline with no engine effect since the last decision
/// stays a note (nothing ran that DCGO declined).
#[test]
fn unstamped_no_with_no_engine_effect_is_a_note() {
    let mut s = staged(nothing_parked(false), vec![unstamped(false)]);
    replay_all(&mut s);
    assert!(s.replay_notes().iter().any(|n| n.contains("unstamped")), "{:?}", s.replay_notes());
}

/// A run of unstamped answers shares one window: the second yes is judged
/// against the effects of the decision in front of the run, not against the
/// (empty) step of the first answer.
#[test]
fn a_run_of_unstamped_answers_shares_the_window_of_the_decision_before_it() {
    let mut s = staged(nothing_parked(true), vec![unstamped(true), unstamped(true)]);
    replay_all(&mut s);
}
