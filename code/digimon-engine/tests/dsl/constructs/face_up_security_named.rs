//! `no_face_up_security_named` predicate leaf. Card-text idioms: "While you
//! have no face-up [Island of Adventure] security cards, you can ignore this
//! card's color requirements" (ST20-15), "... no face-up [Gennai's House]
//! security cards ..." (ST21-15), "If you have no green face-up security
//! cards" (EX10-020).
//!
//! The leaf is TRUE unless `of`'s security stack holds a FACE-UP card that
//! matches the identity filter (`card_number_is` | `name_is` | `color_is`).
//! Face-down matches never close it (DCGO `!c.IsFlipped`).

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledFaceUpSecurityNamed, CompiledPlayerRef,
    CompiledPredicate, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::CardData;

fn parse_compile(yaml: &str) -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    digimon_dsl::compile::compile(&spec).expect("compile yaml")
}

fn option(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Option;
    c.colors = vec![color];
    c
}

fn runner(sec0: &[&str], sec1: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("PRED-SRC", "Pred Src"))
        .add_card(option("HOUSE-A", "Gennai's House", CardColor::White))
        .add_card(option("HOUSE-B", "Gennai's House", CardColor::White))
        .add_card(option("GREEN-OPT", "Green Thing", CardColor::Green))
        .add_card(option("OTHER", "Other", CardColor::White))
        .hand(0, &["PRED-SRC"])
        .security(0, sec0)
        .security(1, sec1)
        .build()
}

fn flip_up(r: &mut DebugRunner, player: usize, id: &str) {
    let idx = r.game.players[player]
        .security
        .iter()
        .find(|c| c.card_id(&r.game.card_data) == id)
        .expect("in security")
        .card_index;
    r.game.players[player].face_up_security.insert(idx);
}

fn holds(r: &DebugRunner, leaf: CompiledFaceUpSecurityNamed) -> bool {
    let pred = CompiledPredicate { no_face_up_security_named: Some(leaf), ..Default::default() };
    let source_card = r.game.players[0].hand[0].handle();
    let rctx = EffectReadContext::new(&r.game, source_card, None, 0);
    eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, None)
}

fn by_id(of: CompiledPlayerRef, id: &str) -> CompiledFaceUpSecurityNamed {
    CompiledFaceUpSecurityNamed { of, card_number_is: Some(id.into()), name_is: None, color_is: None }
}

fn by_name(of: CompiledPlayerRef, name: &str) -> CompiledFaceUpSecurityNamed {
    CompiledFaceUpSecurityNamed { of, card_number_is: None, name_is: Some(name.into()), color_is: None }
}

fn by_color(of: CompiledPlayerRef, c: CompiledColor) -> CompiledFaceUpSecurityNamed {
    CompiledFaceUpSecurityNamed { of, card_number_is: None, name_is: None, color_is: Some(c) }
}

#[test]
fn no_face_up_security_named_compiles_from_yaml_in_use_requirement_and_condition() {
    let compiled = parse_compile(
        r#"
card: T-NFUS
name: Face-up Security Gate
kind: option
color: [white]
cost: 2
use_requirement:
  no_face_up_security_named:
    of: you
    name_is: "Gennai's House"
effects:
  - when: main_from_hand
    process:
      - if:
          condition:
            no_face_up_security_named: { of: opponent, card_number_is: "ST20-15" }
          then:
            - gain_memory: 1
"#,
    );
    let req = compiled
        .use_requirement
        .as_ref()
        .and_then(|p| p.no_face_up_security_named.clone())
        .expect("use_requirement carries the leaf");
    assert_eq!(req, by_name(CompiledPlayerRef::You, "Gennai's House"));
    let CompiledClause::Triggered(t) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::If { condition, .. } = &t.process[0] else {
        panic!("expected if step");
    };
    assert_eq!(
        condition.no_face_up_security_named,
        Some(by_id(CompiledPlayerRef::Opponent, "ST20-15"))
    );
}

#[test]
fn rejects_a_leaf_with_no_identity_filter() {
    let spec: Result<digimon_dsl::CardSpec, _> = serde_yml::from_str(
        r#"
card: T-NFUS2
name: Bad Gate
kind: option
color: [white]
cost: 2
use_requirement:
  no_face_up_security_named: { of: you }
effects: []
"#,
    );
    let rejected = match spec {
        Err(_) => true,
        Ok(spec) => digimon_dsl::compile::compile(&spec).is_err(),
    };
    assert!(rejected, "a leaf with no identity filter must fail to parse or compile");
}

#[test]
fn face_up_match_by_card_number_closes_the_gate_face_down_does_not() {
    let mut r = runner(&["HOUSE-A", "OTHER"], &[]);
    assert!(holds(&r, by_id(CompiledPlayerRef::You, "HOUSE-A")), "face-down copy keeps the gate open");
    flip_up(&mut r, 0, "OTHER");
    assert!(holds(&r, by_id(CompiledPlayerRef::You, "HOUSE-A")), "a face-up non-match keeps it open");
    flip_up(&mut r, 0, "HOUSE-A");
    assert!(!holds(&r, by_id(CompiledPlayerRef::You, "HOUSE-A")), "face-up match closes it");
    assert!(!holds(&r, by_id(CompiledPlayerRef::You, "house-a")), "card number match is case-insensitive");
    assert!(holds(&r, by_id(CompiledPlayerRef::You, "HOUSE-B")), "a different card number does not match");
}

#[test]
fn face_up_match_by_name_spans_printings() {
    let mut r = runner(&["HOUSE-B"], &[]);
    assert!(holds(&r, by_name(CompiledPlayerRef::You, "Gennai's House")));
    flip_up(&mut r, 0, "HOUSE-B");
    assert!(
        !holds(&r, by_name(CompiledPlayerRef::You, "Gennai's House")),
        "any card NAMED [Gennai's House] face-up closes it, whatever its number"
    );
    assert!(holds(&r, by_name(CompiledPlayerRef::You, "Island of Adventure")));
}

#[test]
fn face_up_match_by_color() {
    let mut r = runner(&["GREEN-OPT", "OTHER"], &[]);
    flip_up(&mut r, 0, "OTHER");
    assert!(holds(&r, by_color(CompiledPlayerRef::You, CompiledColor::Green)), "white face-up is not green");
    flip_up(&mut r, 0, "GREEN-OPT");
    assert!(!holds(&r, by_color(CompiledPlayerRef::You, CompiledColor::Green)));
}

#[test]
fn player_scope_only_reads_the_named_players_security() {
    let mut r = runner(&[], &["HOUSE-A"]);
    flip_up(&mut r, 1, "HOUSE-A");
    assert!(holds(&r, by_id(CompiledPlayerRef::You, "HOUSE-A")), "opponent's face-up card is not yours");
    assert!(!holds(&r, by_id(CompiledPlayerRef::Opponent, "HOUSE-A")));
    assert!(!holds(&r, by_id(CompiledPlayerRef::Any, "HOUSE-A")), "Any reads both stacks");
}
