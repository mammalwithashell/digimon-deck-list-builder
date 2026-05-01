use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, Keyword};
use digimon_engine::selection::{OptionPlayResult, PendingSecurity, TriggerSource};

fn test_card(card_id: &str, kind: CardKind) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = kind;
    card.colors = vec![CardColor::Red];
    match kind {
        CardKind::Digimon => {
            card.level = Some(3);
            card.dp = Some(3000);
            card.play_cost = 3;
        }
        CardKind::Tamer | CardKind::Option => {
            card.level = None;
            card.dp = None;
            card.play_cost = 3;
        }
        _ => {}
    }
    card
}

struct OnPlayNoop;

impl CardEffect for OnPlayNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card).name("source witness").build()]
    }
}

struct SecurityNoop;

impl CardEffect for SecurityNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("security source witness")
            .build()]
    }
}

struct OptionSelectWitness(Arc<Mutex<Option<EffectSourceKind>>>);

impl CardEffect for OptionSelectWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let witness = self.0.clone();
        vec![Effect::on_play(card)
            .option_main()
            .name("option source witness")
            .process(move |ctx| {
                let witness = witness.clone();
                ctx.select_opponent_permanent(
                    "choose target",
                    false,
                    |_game, _target| true,
                    move |ctx, _target| {
                        *witness.lock().unwrap() = Some(ctx.source_kind());
                    },
                );
            })
            .build()]
    }
}

struct DualOptionAndArtsWitness {
    option: Arc<Mutex<Option<EffectSourceKind>>>,
    digivolve: Arc<Mutex<Option<EffectSourceKind>>>,
}

impl CardEffect for DualOptionAndArtsWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let option = self.option.clone();
        let digivolve = self.digivolve.clone();
        vec![
            Effect::on_play(card)
                .option_main()
                .name("dual option source witness")
                .process(move |ctx| {
                    *option.lock().unwrap() = Some(ctx.source_kind());
                })
                .build(),
            Effect::when_digivolving(card)
                .name("dual digivolve source witness")
                .process(move |ctx| {
                    *digivolve.lock().unwrap() = Some(ctx.source_kind());
                })
                .build(),
        ]
    }
}

fn dual_arts_card() -> CardData {
    let mut card = make_test_card("DUAL-SRC", "Dual Source");
    card.card_kind = CardKind::Dual;
    card.level = Some(6);
    card.dp = Some(12000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Red];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 5,
        memory_cost: 3,
    }];
    card.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 6,
            dp: 12000,
            colors: vec![CardColor::Red],
            traits: Vec::new(),
            evo_costs: card.evo_costs.clone(),
            effect_text: "[When Digivolving] source witness.".to_string(),
            inherited_text: String::new(),
            keywords: vec![Keyword::ArtsDigivolve],
        },
        option: DualOptionFace {
            use_cost: 5,
            colors: vec![CardColor::Purple],
            effect_text: "[Main] source witness.".to_string(),
            security_text: String::new(),
            keywords: vec![Keyword::ArtsDigivolve],
        },
    });
    card
}

#[test]
fn field_digimon_effect_queues_as_digimon_source() {
    let mut r = DebugRunner::builder()
        .add_card(test_card("DIG-SRC", CardKind::Digimon))
        .start();
    r.register_effect("DIG-SRC", Arc::new(OnPlayNoop));
    let h = r.place_on_field(0, "DIG-SRC", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));

    let queued = r.game.effect_queue.front().expect("queued effect");
    assert_eq!(queued.source_kind, EffectSourceKind::Digimon);
}

#[test]
fn field_tamer_effect_queues_as_tamer_source() {
    let mut r = DebugRunner::builder()
        .add_card(test_card("TAMER-SRC", CardKind::Tamer))
        .start();
    r.register_effect("TAMER-SRC", Arc::new(OnPlayNoop));
    let h = r.place_on_field(0, "TAMER-SRC", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));

    let queued = r.game.effect_queue.front().expect("queued effect");
    assert_eq!(queued.source_kind, EffectSourceKind::Tamer);
}

#[test]
fn digimon_security_effect_queues_as_digimon_source() {
    let mut r = DebugRunner::builder()
        .add_card(test_card("SEC-DIG", CardKind::Digimon))
        .security(0, &["SEC-DIG"])
        .start();
    r.register_effect("SEC-DIG", Arc::new(SecurityNoop));

    let revealed = r.game.player(0).security.last().unwrap().clone();
    let handle = revealed.handle();
    r.game.pending_security = Some(PendingSecurity {
        defender: 0,
        card: revealed,
        played: false,
    });
    r.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::SecurityRevealed {
            defender: 0,
            card: handle,
        },
    );

    let queued = r.game.effect_queue.front().expect("queued security effect");
    assert_eq!(queued.source_kind, EffectSourceKind::Digimon);
}

#[test]
fn option_selection_callback_preserves_option_source_kind() {
    let witness = Arc::new(Mutex::new(None));
    let mut r = DebugRunner::builder()
        .add_card(test_card("OPT-SRC", CardKind::Option))
        .add_card(test_card("ANCHOR", CardKind::Digimon))
        .add_card(test_card("TARGET", CardKind::Digimon))
        .hand(0, &["OPT-SRC"])
        .memory(3)
        .start();
    r.register_effect("OPT-SRC", Arc::new(OptionSelectWitness(witness.clone())));
    r.place_on_field(0, "ANCHOR", Some(0));
    r.place_on_field(1, "TARGET", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    r.game
        .resolve_selection(0, encode_attack(0, 0))
        .expect("resolve target");

    assert_eq!(*witness.lock().unwrap(), Some(EffectSourceKind::Option));
}

#[test]
fn dual_option_use_resolves_as_option_and_arts_followup_resolves_as_digimon() {
    let option_witness = Arc::new(Mutex::new(None));
    let digivolve_witness = Arc::new(Mutex::new(None));
    let mut purple_anchor = test_card("PURPLE-ANCHOR", CardKind::Digimon);
    purple_anchor.colors = vec![CardColor::Purple];
    let mut base_red = test_card("BASE-RED", CardKind::Digimon);
    base_red.level = Some(5);
    base_red.dp = Some(7000);
    let mut r = DebugRunner::builder()
        .add_card(dual_arts_card())
        .add_card(purple_anchor)
        .add_card(base_red)
        .deck(0, &["PURPLE-ANCHOR"])
        .hand(0, &["DUAL-SRC"])
        .memory(5)
        .start();
    r.register_effect(
        "DUAL-SRC",
        Arc::new(DualOptionAndArtsWitness {
            option: option_witness.clone(),
            digivolve: digivolve_witness.clone(),
        }),
    );
    r.place_on_field(0, "PURPLE-ANCHOR", Some(0));
    r.place_on_field(0, "BASE-RED", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    assert_eq!(
        *option_witness.lock().unwrap(),
        Some(EffectSourceKind::Option)
    );

    r.game
        .resolve_selection(0, encode_attack(0, 1))
        .expect("accept Arts");

    assert_eq!(
        *digivolve_witness.lock().unwrap(),
        Some(EffectSourceKind::Digimon)
    );
}
