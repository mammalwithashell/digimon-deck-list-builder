use digimon_dsl::{compile, CardSpec};
use digimon_engine::action::space::HAND_EFFECT_START;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use std::sync::Arc;

fn compile_yaml(yaml: &str) -> Arc<digimon_dsl::compiled::CompiledCard> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse card yaml");
    Arc::new(compile::compile(&spec).expect("compile card yaml"))
}

fn option_card(card_id: &str, cost: u16, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = cost;
    cd.colors = vec![color];
    cd
}

fn digimon_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd
}

fn register_dsl_yaml(r: &mut DebugRunner, yaml: &str) {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse card yaml");
    let card_id = spec.card.clone();
    let compiled = compile::compile(&spec).expect("compile card yaml");
    r.register_effect(&card_id, Arc::new(DslCardEffect::new(Arc::new(compiled))));
}

#[test]
fn link_requirement_clause_lowers_to_link_option_effect() {
    let yaml = r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - kind: link_requirement
    scope: inherited
    cost: 2
    filter: { level_gte: 3 }
"#;
    let card = compile_yaml(yaml);
    let dsl = DslCardEffect::new(card);
    let effects = dsl.effects(CardHandle(0));
    assert!(effects.iter().any(|e| {
        e.timing == EffectTiming::OptionMain && e.link_cost.is_some() && e.link_filter.is_some()
    }));
}

#[test]
fn linked_scope_lowers_to_linked_effect_flag() {
    let yaml = r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - scope: linked
    when: end_of_your_turn
    optional: true
    process:
      - gain_memory: 1
"#;
    let card = compile_yaml(yaml);
    let dsl = DslCardEffect::new(card);
    let effects = dsl.effects(CardHandle(0));
    assert!(effects
        .iter()
        .any(|e| e.linked && e.timing == EffectTiming::EndOfYourTurn));
}

#[test]
fn linked_scope_only_fires_from_linked_card_scan() {
    let yaml = r#"
card: LINKED-EFFECT
name: Linked Draw
kind: option
effects:
  - kind: link_requirement
    scope: inherited
    cost: 0
    filter: { kind: digimon }
  - scope: linked
    when: end_of_your_turn
    process:
      - draw: { of: you, count: 1 }
"#;

    let mut face_up = DebugRunner::builder()
        .add_card(option_card("LINKED-EFFECT", 0, CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .deck(0, &["FILLER"; 5])
        .memory(0)
        .start();
    register_dsl_yaml(&mut face_up, yaml);
    face_up.place_on_field(0, "LINKED-EFFECT", Some(0));

    face_up.end_turn();

    assert_eq!(
        face_up.hand_size(0),
        0,
        "linked-scope effect must not fire from normal face-up permanents"
    );

    let mut linked = DebugRunner::builder()
        .add_card(option_card("LINKED-EFFECT", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .hand(0, &["LINKED-EFFECT"])
        .deck(0, &["FILLER"; 5])
        .memory(0)
        .start();
    register_dsl_yaml(&mut linked, yaml);
    let host = linked.place_on_field(0, "HOST", Some(0));
    linked.game.enter_main_phase();

    // `G-ENGINE-OPTION-LINK-FROM-HAND`: the from-hand link is its OWN
    // main-phase action, not a mode of "use this Option". general_rule.pdf
    // (Ver.3.6) §6-5-1 lists "use an Option card from the hand" (§6-5-1-3) and
    // "link a card from the hand or battle area" (§6-5-1-4) separately, so the
    // declaration lives on the `HAND_EFFECT` bit — `play_option_from_hand` no
    // longer offers a Link mode and would refuse this card outright. Same
    // shape as the `declare_from_hand` helper in
    // `tests/option_flow/hand_declaration.rs`.
    assert!(
        linked.game.hand_effect_slot_is_link(0, 0),
        "the Plug-In Option's hand slot must carry the §6-5-1-4 link declaration"
    );
    linked.game.decode_action(HAND_EFFECT_START, 0);
    assert!(
        linked.game.pending_selection.is_some(),
        "the link declaration installs the host pick (§10-1-3-1 chooses the host first)"
    );
    let action = linked
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    let _ = linked.game.resolve_selection(0, action);
    assert_eq!(
        linked.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );

    linked.end_turn();

    assert_eq!(
        linked.hand_size(0),
        1,
        "linked-scope effect must fire through the host linked-card scan"
    );
}

// ── A `scope: linked` keyword grant applies to the host only (4-2-6) ──────
//
// A granted keyword with an auto-effect (here <Evade>'s optional replacement,
// 16-21) is synthesized by `Game::build_effects_for_card`. It must reach the
// Digimon the card is linked to, once, and never the card's own permanent.

const LINKED_EVADE: &str = r#"
card: TEST-LINK-EVADE
name: Link Evade
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - scope: linked
    kind: grant_keyword
    keyword: Evade
"#;

fn linked_evade_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(LINKED_EVADE)
        .expect("fixture compiles")
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("ALLY", CardColor::Red))
        .start()
}

#[test]
fn linked_scope_replacement_keyword_is_not_the_link_cards_own() {
    use digimon_engine::replacement::ReplacementCause;
    let mut r = linked_evade_runner();
    let card = r.place_on_field(0, "TEST-LINK-EVADE", Some(0));
    r.place_on_field(0, "ALLY", Some(0));

    r.game.delete_permanent_with_cause(card, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "the link card's own deletion offers no <Evade>: {:?}",
        r.pending_kind()
    );
    assert_eq!(r.battle_area_size(0), 1, "the link card is deleted");
}

#[test]
fn linked_scope_replacement_keyword_is_offered_to_the_host_once() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::replacement::ReplacementCause;
    let mut r = linked_evade_runner();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(0, "ALLY", Some(0));
    r.push_linked_owned(host, "TEST-LINK-EVADE", 0);
    r.game.tick_declarative_effects();

    r.game.delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.as_ref().is_some_and(|p| p.is_optional),
        "the host may use the linked <Evade>"
    );
    r.game.resolve_selection(0, PASS).expect("decline <Evade>");

    assert!(
        r.game.pending_selection.is_none(),
        "<Evade> is offered once: {:?}",
        r.pending_kind()
    );
    assert_eq!(r.battle_area_size(0), 1, "the host is deleted after the decline");
}
