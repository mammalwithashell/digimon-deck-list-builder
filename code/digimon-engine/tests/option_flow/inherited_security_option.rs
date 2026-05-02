use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;

fn option_source(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.colors = vec![CardColor::Red];
    cd
}

fn digimon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![CardColor::Red];
    cd
}

struct SecurityPlacesSelfAsDelay;

impl CardEffect for SecurityPlacesSelfAsDelay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("Security place this card as Delay")
            .process(|ctx| {
                ctx.place_self_as_delay_option_permanent();
            })
            .build()]
    }
}

struct OptionPlacedWitness(Arc<Mutex<u32>>);

impl CardEffect for OptionPlacedWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_play(card)
            .name("OnOptionPlaced witness")
            .timing(EffectTiming::OnOptionPlaced)
            .process(move |_ctx| {
                *witness.lock().unwrap() += 1;
            })
            .build()]
    }
}

struct SecurityPlacesSelfAsStartDelay;

impl CardEffect for SecurityPlacesSelfAsStartDelay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::security(card)
                .name("Security place this card as start Delay")
                .process(|ctx| {
                    ctx.place_self_as_delay_option_permanent();
                })
                .build(),
            Effect::on_play(card)
                .name("Start Delay body")
                .delay(DelayTrigger::StartOfYourNextTurn)
                .process(|_| {})
                .build(),
        ]
    }
}

#[test]
fn inherited_security_places_source_option_as_delay_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST"))
        .add_card(option_source("P-035"))
        .memory(0)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.push_source(host, "P-035");

    r.run_inherited_security_effect(host, "P-035", |ctx| {
        ctx.place_self_as_delay_option_permanent();
    });

    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .card_sources
            .len(),
        1
    );
    let placed = r
        .game
        .player(0)
        .battle_area
        .last()
        .expect("placed option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { owner: 0, .. }
    ));
    assert_eq!(placed.top_card().card_id(&r.game.card_data), "P-035");
}

#[test]
fn inherited_security_does_not_place_non_option_source() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST"))
        .add_card(digimon_card("SOURCE-DIGIMON"))
        .memory(0)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.push_source(host, "SOURCE-DIGIMON");

    r.run_inherited_security_effect(host, "SOURCE-DIGIMON", |ctx| {
        ctx.place_self_as_delay_option_permanent();
    });

    assert_eq!(r.game.player(0).battle_area.len(), 1);
    let host_stack = &r.game.player(0).battle_area[host.index as usize].card_sources;
    assert_eq!(host_stack.len(), 2);
    assert_eq!(host_stack[0].card_id(&r.game.card_data), "SOURCE-DIGIMON");
    assert!(!r.game.player(0).battle_area.iter().any(|permanent| {
        matches!(permanent.option_state, OptionState::Delayed { .. })
            && permanent.top_card().card_id(&r.game.card_data) == "SOURCE-DIGIMON"
    }));
}

#[test]
fn security_check_places_revealed_option_as_delay_permanent_instead_of_trash() {
    let witness = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("ATTACKER"))
        .add_card(digimon_card("OBSERVER"))
        .add_card(option_source("SEC-DELAY"))
        .security(1, &["SEC-DELAY"])
        .memory(0)
        .start();
    r.register_effect("SEC-DELAY", std::sync::Arc::new(SecurityPlacesSelfAsDelay));
    r.register_effect("OBSERVER", Arc::new(OptionPlacedWitness(witness.clone())));
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(1, "OBSERVER", Some(0));

    let result = r.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(r.security_count(1), 0, "revealed option left security");
    assert_eq!(
        r.trash_size(1),
        0,
        "revealed option must not fall through to security dispose trash"
    );
    let placed = r
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&r.game.card_data) == "SEC-DELAY")
        .expect("revealed option became a battle-area permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { owner: 1, .. }
    ));
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "security placement must dispatch OnOptionPlaced"
    );
}

#[test]
fn security_check_placement_preserves_card_delay_trigger() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("ATTACKER"))
        .add_card(option_source("SEC-START-DELAY"))
        .security(1, &["SEC-START-DELAY"])
        .memory(0)
        .start();
    r.register_effect("SEC-START-DELAY", Arc::new(SecurityPlacesSelfAsStartDelay));
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));

    let result = r.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    let placed = r
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&r.game.card_data) == "SEC-START-DELAY")
        .expect("revealed option became a battle-area permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::StartOfYourNextTurn,
            ..
        }
    ));
}
