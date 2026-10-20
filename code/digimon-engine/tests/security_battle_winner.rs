//! Official general_rule.pdf p21, 13-1-7-3 and 14-2: security Digimon
//! participate in battles, but losing security Digimon are never deleted.
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardKind, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
struct Seen {
    winner: Option<PermanentHandle>,
    deleters: usize,
    revealed: bool,
    security_in_trash: bool,
}
struct Observer(Arc<Mutex<Vec<Seen>>>, bool);
impl CardEffect for Observer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        let mut effect = Effect::end_of_battle(card).name("security battle observer");
        if self.1 {
            effect = effect.optional().needs_outer_optional_prompt();
        }
        vec![effect
            .process(move |ctx| {
                seen.lock().unwrap().push(Seen {
                    winner: ctx.battle_winner(),
                    deleters: ctx
                        .game
                        .current_trigger_context
                        .as_ref()
                        .unwrap()
                        .battle_deleters
                        .len(),
                    revealed: ctx.game.pending_security.is_some(),
                    security_in_trash: ctx.game.players[1]
                        .trash
                        .iter()
                        .any(|c| c.card_id(&ctx.game.card_data) == "SEC"),
                });
            })
            .build()]
    }
}
fn card(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c.level = Some(5);
    c
}
fn setup(
    dp: i32,
    kind: CardKind,
    optional: bool,
    checks: usize,
) -> (DebugRunner, PermanentHandle, Arc<Mutex<Vec<Seen>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut sec = card("SEC", 8000);
    sec.card_kind = kind;
    let mut r = DebugRunner::builder()
        .add_card(card("ATK", dp))
        .add_card(card("OBS", 1000))
        .add_card(sec)
        .deck(0, &["OBS"; 10])
        .deck(1, &["OBS"; 10])
        .security(0, &["OBS"])
        .security(1, &vec!["SEC"; checks])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(Observer(seen.clone(), optional)));
    r.place_on_field(0, "OBS", Some(0));
    let atk = r.place_on_field(0, "ATK", Some(0));
    (r, atk, seen)
}
fn choose(r: &mut DebugRunner, decline: bool) {
    use digimon_engine::action::{build_action_mask, space::PASS};
    let view = r.pending_selection_view().expect("legal optional decision");
    let action = if decline {
        PASS
    } else {
        *view.valid_action_ids.iter().find(|&&a| a != PASS).unwrap()
    };
    assert!(build_action_mask(&r.game, view.selecting_player)[action as usize] > 0.5);
    r.execute_action(view.selecting_player, action).unwrap();
}

#[test]
fn strict_security_win_notifies_before_disposal_without_battle_deleter() {
    let (mut r, atk, seen) = setup(12000, CardKind::Digimon, false, 1);
    r.attack_player(atk, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        vec![Seen {
            winner: Some(atk),
            deleters: 0,
            revealed: true,
            security_in_trash: false
        }]
    );
    assert_eq!(r.trash_size(1), 1);
}

#[test]
fn tied_and_lost_security_battles_have_no_permanent_winner_even_with_jamming() {
    for dp in [8000, 3000] {
        for jamming in [false, true] {
            let (mut r, atk, seen) = setup(dp, CardKind::Digimon, false, 1);
            if jamming {
                r.game
                    .modifiers
                    .grant_keyword(atk, Keyword::Jamming, Expiry::Permanent, 0);
            }
            r.attack_player(atk, 1, false);
            r.auto_resolve().unwrap();
            let events = seen.lock().unwrap();
            assert_eq!(
                events.len(),
                1,
                "one battle event for dp={dp}, jamming={jamming}"
            );
            assert_eq!(events[0].winner, None);
            assert_eq!(events[0].deleters, 0);
            assert_eq!(r.battle_area_size(0), if jamming { 2 } else { 1 });
        }
    }
}

#[test]
fn option_and_skipped_security_battles_do_not_emit_end_of_battle() {
    for (kind, skip) in [(CardKind::Option, false), (CardKind::Digimon, true)] {
        let (mut r, atk, seen) = setup(12000, kind, false, 1);
        if skip {
            r.game.modifiers.add(
                atk,
                ModifierEntry::simple(
                    ModifierType::DontBattleSecurityDigimon,
                    0,
                    Expiry::Permanent,
                    0,
                ),
            );
        }
        r.attack_player(atk, 1, false);
        r.auto_resolve().unwrap();
        assert!(seen.lock().unwrap().is_empty());
    }
}

#[test]
fn optional_battle_event_parks_before_next_check_and_clone_does_not_refire() {
    let (mut r, atk, seen) = setup(12000, CardKind::Digimon, true, 2);
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(ModifierType::SecurityAttackChange, 1, Expiry::Permanent, 0),
    );
    r.attack_player(atk, 1, false);
    assert!(
        r.pending_selection().is_some(),
        "first battle event must park"
    );
    assert_eq!(r.security_count(1), 1, "second check has not started");
    assert_eq!(r.trash_size(1), 0);
    let mut clone = DebugRunner::wrap(r.game.clone());
    choose(&mut r, false);
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert!(
        r.pending_selection().is_some(),
        "second check has its own event"
    );
    choose(&mut r, false);
    assert_eq!(seen.lock().unwrap().len(), 2);
    choose(&mut clone, true);
    choose(&mut clone, true);
    assert_eq!(
        seen.lock().unwrap().len(),
        2,
        "declined cloned events never execute"
    );
    for game in [&r, &clone] {
        assert!(game.game.security_resolution.is_none());
        assert!(game.game.pending_attack.is_none());
        assert_eq!(game.trash_size(1), 2);
    }
}

#[test]
fn losing_attacker_barrier_choice_finishes_before_battle_event_after_clone() {
    let (mut r, atk, seen) = setup(3000, CardKind::Digimon, false, 1);
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Barrier, Expiry::Permanent, 0);
    r.attack_player(atk, 1, false);
    assert!(r.pending_selection().is_some(), "Barrier must be offered");
    assert!(
        seen.lock().unwrap().is_empty(),
        "battle event waits for replacement"
    );
    let mut clone = DebugRunner::wrap(r.game.clone());
    choose(&mut r, false);
    r.auto_resolve().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert_eq!(r.battle_area_size(0), 2);
    choose(&mut clone, true);
    clone.auto_resolve().unwrap();
    assert_eq!(clone.battle_area_size(0), 1);
    let events = seen.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert!(events
        .iter()
        .all(|e| e.winner.is_none() && e.deleters == 0 && e.revealed));
    assert!(r.game.pending_attack.is_none());
    assert!(clone.game.pending_attack.is_none());
}

struct DeleteAttacker(PermanentHandle);
impl CardEffect for DeleteAttacker {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let attacker = self.0;
        vec![Effect::security(card)
            .process(move |ctx| {
                ctx.game.delete_permanent_with_effects(attacker);
            })
            .build()]
    }
}

#[test]
fn attacker_removed_by_security_effect_does_not_enter_security_battle() {
    let (mut r, atk, seen) = setup(12000, CardKind::Digimon, false, 1);
    r.register_effect("SEC", Arc::new(DeleteAttacker(atk)));
    r.attack_player(atk, 1, false);
    r.auto_resolve().unwrap();
    assert!(seen.lock().unwrap().is_empty());
    assert_eq!(r.battle_area_size(0), 1);
}

struct DyingAttacker;
impl CardEffect for DyingAttacker {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .optional()
            .needs_outer_optional_prompt()
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn on_deletion_decision_finishes_before_security_battle_observers() {
    let (mut r, atk, seen) = setup(3000, CardKind::Digimon, false, 1);
    r.register_effect("ATK", Arc::new(DyingAttacker));
    r.attack_player(atk, 1, false);
    assert_eq!(r.battle_area_size(0), 1);
    assert!(r.pending_selection().is_some());
    assert!(seen.lock().unwrap().is_empty());
    let mut clone = DebugRunner::wrap(r.game.clone());
    choose(&mut r, false);
    choose(&mut clone, true);
    assert_eq!(seen.lock().unwrap().len(), 2);
    for game in [&r, &clone] {
        assert!(game.game.security_resolution.is_none());
        assert!(game.game.pending_attack.is_none());
        assert_eq!(game.trash_size(1), 1);
    }
}

#[test]
fn examon_security_win_exposes_source_card_free_play() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-045")
        .unwrap()
        .dsl_card("BT20-040")
        .unwrap()
        .add_card(card("SEC", 8000))
        .deck(0, &["SEC"; 10])
        .deck(1, &["SEC"; 10])
        .security(1, &["SEC"; 2])
        .memory(10)
        .start();
    // The DSL test loader does not hydrate printed CardData text.
    r.game.card_data.iter_mut().find(|c| c.card_id == "BT20-040").unwrap().effect_text =
        "[Your Turn] When any of your blue Digimon with [Dracomon] or [Examon] in their texts are played".into();
    r.set_first_player(0);
    let atk = r.place_stack(0, &["BT20-040", "EX13-045"]);
    r.attack_player(atk, 1, false);
    assert!(
        r.pending_selection().is_some(),
        "winning security battle exposes Examon's optional source play"
    );
    choose(&mut r, false);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.battle_area_size(0),
        2,
        "Coredramon played from Examon's sources"
    );
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BT20-040"));
    assert_eq!(
        r.security_count(1),
        0,
        "both SA+1 checks complete after the free play"
    );
}

#[test]
fn scheduled_battle_effects_wait_for_ordinary_choice_and_resume_once_after_clone() {
    use digimon_dsl::compiled::CompiledStep;
    use digimon_engine::dsl_cards::bindings::Bindings;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::EffectTiming;
    let (mut r, atk, seen) = setup(12000, CardKind::Digimon, true, 1);
    r.game.memory = 0;
    let source = r.game.players[0].battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(&mut r.game, source, None, 0);
    ctx.schedule_delayed(
        EffectTiming::EndOfBattle,
        vec![
            CompiledStep::SelectEffectChoice {
                labels: vec!["first".into(), "second".into()],
                legal_when: None,
                bind_as: None,
                prompt: "Scheduled choice".into(),
                prompt_key: None,
            },
            CompiledStep::GainMemory(1),
        ],
        Bindings::new(),
    );
    ctx.schedule_delayed(
        EffectTiming::EndOfBattle,
        vec![CompiledStep::GainMemory(2)],
        Bindings::new(),
    );
    r.attack_player(atk, 1, false);
    assert!(r.pending_selection().is_some());
    assert_eq!(
        r.game.scheduled_effects.len(),
        2,
        "scheduled effects must not replace the ordinary optional choice"
    );
    let mut clone = DebugRunner::wrap(r.game.clone());
    for (game, decline) in [(&mut r, false), (&mut clone, true)] {
        choose(game, decline);
        assert!(
            game.pending_selection().is_some(),
            "scheduled choice runs only after ordinary observer"
        );
        assert_eq!(game.trash_size(1), 0);
        choose(game, false);
        assert_eq!(game.memory(), 3, "both scheduled effects run exactly once");
        assert_eq!(game.trash_size(1), 1);
        assert!(game.game.scheduled_drain_tail.is_none());
        assert!(game.game.scheduled_effects.is_empty());
        assert!(game.game.security_resolution.is_none());
    }
    assert_eq!(seen.lock().unwrap().len(), 1);
}

// Integration regression: captured check observers from the campaign precede
// main's security battle completion, including a parked end-of-battle choice.
struct OrderedSecurityObserver(Arc<Mutex<Vec<&'static str>>>);
impl CardEffect for OrderedSecurityObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let checks = self.0.clone();
        let battles = self.0.clone();
        vec![
            Effect::on_security_check(card)
                .process(move |_| checks.lock().unwrap().push("check"))
                .build(),
            Effect::end_of_battle(card)
                .optional()
                .needs_outer_optional_prompt()
                .process(move |ctx| {
                    assert!(ctx.game.pending_security.is_some());
                    assert!(ctx.game.players[1].trash.is_empty());
                    battles.lock().unwrap().push("battle");
                })
                .build(),
        ]
    }
}

#[test]
fn captured_check_then_optional_security_battle_finishes_once() {
    for decline in [false, true] {
        let (mut r, atk, _) = setup(12000, CardKind::Digimon, false, 1);
        let trace = Arc::new(Mutex::new(Vec::new()));
        r.register_effect("OBS", Arc::new(OrderedSecurityObserver(trace.clone())));
        r.attack_player(atk, 1, false);
        assert_eq!(*trace.lock().unwrap(), vec!["check"]);
        assert!(r.game.pending_security.is_some());
        choose(&mut r, decline);
        r.auto_resolve().unwrap();
        let expected = if decline { vec!["check"] } else { vec!["check", "battle"] };
        assert_eq!(*trace.lock().unwrap(), expected);
        assert_eq!(r.trash_size(1), 1);
        assert!(r.game.pending_security.is_none());
        assert!(r.pending_selection_view().is_none());
    }
}
