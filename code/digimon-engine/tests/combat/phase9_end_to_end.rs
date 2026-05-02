//! Phase 9 Task 10 — Integrated behavioral end-to-end.
//!
//! A single scenario composes **five** Phase 9 mechanics in one attack
//! to catch integration bugs that unit tests miss:
//!
//!   1. **Counter window (Task 3)** — defender plays a hand Counter Option.
//!   2. **`ctx.redirect_attack` (Task 2)** — the Counter Option body rewrites
//!      `effective_target` from the original Digimon to a sibling Digimon.
//!   3. **`OnAttackTargetChange` observer (Task 2)** — fires globally on the
//!      redirect.
//!   4. **Collision MUST-block (Task 5)** — attacker carries `<Collision>`,
//!      so after the Counter window closes the state machine installs a
//!      MANDATORY BlockTiming selection (PASS bit dropped).
//!   5. **`OnBlock` observer (Task 8)** — fires when the defender declares
//!      the forced blocker.
//!
//! Final-state guarantees:
//!   - Counter Option ended up in trash (consumed by Phase 8's Option
//!     pipeline with the CounterEffect overlay).
//!   - `pending_attack`, `pending_option`, `pending_selection` all `None`
//!     — no state leaks across the integrated flow.

use std::sync::{Arc, Mutex};

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::AttackTarget;

fn dgmn(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Red];
    c
}

fn option_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 0;
    c.colors = vec![CardColor::Red];
    c
}

/// Counter Option whose body redirects the active attack from its original
/// target to the captured sibling handle. The OptionMain half is a no-op
/// sentinel so Phase 8's pipeline still trashes the card.
struct CounterOptionRedirect {
    new_target: PermanentHandle,
    counter_fired: Arc<Mutex<u32>>,
}
impl CardEffect for CounterOptionRedirect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let new_target = self.new_target;
        let fired = self.counter_fired.clone();
        vec![
            Effect::on_play(card)
                .name("Counter body — redirect attack")
                .counter()
                .timing(EffectTiming::CounterEffect)
                .process(move |ctx| {
                    *fired.lock().unwrap() += 1;
                    let _ = ctx.redirect_attack(AttackTarget::Digimon(new_target));
                })
                .build(),
            Effect::on_play(card)
                .name("OptionMain — no-op")
                .option_main()
                .process(|_ctx| {})
                .build(),
        ]
    }
}

/// `OnAttackTargetChange` observer — logs the post-change `effective_target`
/// each fire. Lets the test assert the redirect was visible to observers.
struct WitnessTargetChange(Arc<Mutex<Vec<AttackTarget>>>);
impl CardEffect for WitnessTargetChange {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_attack_target_change(c)
            .name("Witness target change")
            .process(move |ctx| {
                if let Some(pa) = ctx.game.pending_attack.as_ref() {
                    log.lock().unwrap().push(pa.effective_target);
                }
            })
            .build()]
    }
}

/// `OnBlock` observer — increments on each block declaration.
struct WitnessOnBlock(Arc<Mutex<u32>>);
impl CardEffect for WitnessOnBlock {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let counter = self.0.clone();
        vec![Effect::on_block(c)
            .name("Witness OnBlock")
            .process(move |_ctx| {
                *counter.lock().unwrap() += 1;
            })
            .build()]
    }
}

#[test]
fn counter_redirect_into_collision_forced_block_end_to_end() {
    // Witnesses for the five interrupts we're composing.
    let counter_fired = Arc::new(Mutex::new(0u32));
    let target_change_log: Arc<Mutex<Vec<AttackTarget>>> = Arc::new(Mutex::new(Vec::new()));
    let on_block_fired = Arc::new(Mutex::new(0u32));

    // Fixtures:
    //   - ATK: attacker, will carry <Collision> via modifier.
    //   - DEF_A: original declared target (P1 slot 0).
    //   - DEF_B: sibling — redirect destination from the Counter body
    //     (P1 slot 1).
    //   - OPT-CTR: Counter Option in the defender's hand.
    //   - W-TC: OnAttackTargetChange witness on P0 (observer fan-out is
    //     global, so placement side is orthogonal).
    //   - W-OB: OnBlock witness on P0.
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 5000))
        .add_card(dgmn("DEF-A", 3000))
        .add_card(dgmn("DEF-B", 3000))
        .add_card(option_card("OPT-CTR"))
        .add_card(dgmn("W-TC", 1000))
        .add_card(dgmn("W-OB", 1000))
        .hand(1, &["OPT-CTR"])
        .memory(0)
        .start();

    // Place witnesses first — handles fixed before we register effects
    // that reference them.
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def_a = r.place_on_field(1, "DEF-A", Some(0));
    let def_b = r.place_on_field(1, "DEF-B", Some(0));
    assert_eq!(
        def_b,
        PermanentHandle {
            player: 1,
            index: 1
        }
    );

    // Counter Option redirects from DEF_A to DEF_B.
    r.register_effect(
        "OPT-CTR",
        Arc::new(CounterOptionRedirect {
            new_target: def_b,
            counter_fired: counter_fired.clone(),
        }),
    );
    r.register_effect(
        "W-TC",
        Arc::new(WitnessTargetChange(target_change_log.clone())),
    );
    r.register_effect("W-OB", Arc::new(WitnessOnBlock(on_block_fired.clone())));
    let _w_tc = r.place_on_field(0, "W-TC", Some(0));
    let _w_ob = r.place_on_field(0, "W-OB", Some(0));

    // Attacker carries <Collision> — makes the post-Counter Block selection
    // mandatory (PASS bit dropped).
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);

    // ─── 1. Declare attack against DEF_A ─────────────────────────────
    let hand_before = r.hand_size(1);
    let trash_before = r.trash_size(1);
    let memory_before = r.memory();
    let _ = (hand_before, trash_before, memory_before);

    r.attack_digimon(atk, def_a, false);

    // Counter window is open — defender owns a hand Counter Option.
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Counter window installs a selection");
    assert_eq!(sel.selecting_player, 1);
    assert!(
        sel.valid_action_ids.contains(&(PLAY_HAND_START + 0)),
        "hand Counter Option candidate missing from {:?}",
        sel.valid_action_ids
    );

    // ─── 2. Defender plays the Counter Option ────────────────────────
    //
    // Its body calls ctx.redirect_attack(DEF_B). That triggers
    // OnAttackTargetChange, after which Phase 8's Option pipeline
    // continues with OptionMain and trashes the card.
    r.game
        .resolve_selection(1, PLAY_HAND_START + 0)
        .expect("defender resolves hand Counter Option");

    // Counter body fired exactly once.
    assert_eq!(*counter_fired.lock().unwrap(), 1, "Counter body fired");

    // OnAttackTargetChange fired and observed effective_target == DEF_B.
    let tc_log = target_change_log.lock().unwrap().clone();
    assert!(
        tc_log
            .iter()
            .any(|t| matches!(t, AttackTarget::Digimon(h) if *h == def_b)),
        "OnAttackTargetChange must see effective_target == DEF_B post-redirect \
         (saw {:?})",
        tc_log
    );

    // Counter Option card ended up in trash (consumed by Phase 8's pipeline).
    assert_eq!(
        r.hand_size(1),
        hand_before - 1,
        "Counter Option left defender's hand"
    );
    assert_eq!(
        r.trash_size(1),
        trash_before + 1,
        "Counter Option moved into trash"
    );

    // ─── 3. Collision MUST-block kicks in ────────────────────────────
    //
    // After the Counter window closes the state machine advances to
    // BlockOpen. Attacker has <Collision> AND the defender has DEF_A /
    // DEF_B as candidates, so the selection is MANDATORY.
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Collision installs a mandatory BlockTiming selection");
    assert!(
        !sel.is_optional,
        "Collision + non-empty candidate pool ⇒ MUST-block"
    );
    assert_eq!(sel.selecting_player, 1);
    // Defender has DEF_A @ slot 0 and DEF_B @ slot 1 — both are candidates.
    assert!(
        sel.valid_action_ids.contains(&encode_attack(0, 0)),
        "DEF_A must be a blocker candidate (got {:?})",
        sel.valid_action_ids
    );
    assert!(
        sel.valid_action_ids.contains(&encode_attack(0, 1)),
        "DEF_B must be a blocker candidate (got {:?})",
        sel.valid_action_ids
    );
    // Defender's action mask drops PASS.
    let mask = build_action_mask(&r.game, 1);
    assert_eq!(
        mask[PASS as usize], 0.0,
        "MUST-block selection drops the PASS bit"
    );

    // ─── 4. Defender declares a blocker — OnBlock fires ──────────────
    //
    // Pick DEF_A as the blocker. (DEF_B is the current effective target
    // post-redirect; DEF_A is still a legal blocker candidate.)
    r.game
        .resolve_selection(1, encode_attack(0, 0))
        .expect("defender declares forced block");

    assert_eq!(
        *on_block_fired.lock().unwrap(),
        1,
        "OnBlock fires on the forced-block declaration"
    );

    // ─── 5. Final state: no leaks ────────────────────────────────────
    //
    // ATK (5000) resolves against DEF_A (3000) after the block substitution,
    // so the attack finishes synchronously past Block. Either way, all
    // transient combat/selection state must be cleared.
    assert!(
        r.game.pending_attack.is_none(),
        "pending_attack cleared post-resolution"
    );
    assert!(
        r.game.pending_option.is_none(),
        "pending_option cleared post-resolution"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "pending_selection cleared post-resolution"
    );
}
