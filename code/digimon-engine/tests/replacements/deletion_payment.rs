//! CR 4.3 15-7-2/3/4: an impossible or uncompleted deletion cannot pay a
//! "by deleting" protection condition. Keep ordinary effect targeting legal.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, REPLACEMENT_ACCEPT},
    },
    card_data::CardData,
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect},
    effect_context::EffectContext,
    enums::{EffectTiming, Expiry, GamePhase, ModifierType, Zone},
    game::Game,
    modifiers::ModifierEntry,
    permanent::PermanentHandle,
    replacement::ReplacementCause,
    runners::replay::ReplayRunner,
    serialization::to_ui_json,
};
use std::sync::Arc;

fn runner() -> DebugRunner {
    let mut cards = CardData::load_from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/cards.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut body = make_test_card("BODY", "Reptile subject");
    body.traits = vec!["Reptile".into()];
    body.dp = Some(6000);
    let mut low = make_test_card("LOW", "Low DP payer");
    low.dp = Some(2000);
    let mut high = make_test_card("HIGH", "High DP payer");
    high.dp = Some(5000);
    let mut b = DebugRunner::builder()
        .add_card(body)
        .add_card(low)
        .add_card(high);
    for id in ["EX11-012", "BT24-018"] {
        b = b.add_card(cards.remove(id).unwrap()).dsl_card(id).unwrap();
    }
    let mut r = b
        .deck(0, &["BODY"; 16])
        .deck(1, &["BODY"; 16])
        .security(0, &["BODY"; 3])
        .security(1, &["BODY"; 3])
        .memory(3)
        .start();
    r.set_phase(GamePhase::Main);
    r
}

fn token(r: &mut DebugRunner, player: u8) -> PermanentHandle {
    let index = r.game.player(player).battle_area.len() as u8;
    EffectContext::new(&mut r.game, CardHandle(0), None, player)
        .play_token(player, "petrification");
    PermanentHandle { player, index }
}

fn immune(r: &mut DebugRunner, handle: PermanentHandle, modifier: ModifierType) {
    r.game.modifiers.add(
        handle,
        ModifierEntry::simple(modifier, 0, Expiry::Permanent, handle.player),
    );
}

fn contains(game: &Game, player: u8, id: &str) -> bool {
    game.player(player)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&game.card_data) == id)
}

fn choose(game: &mut Game, action: u16) {
    let player = game
        .pending_selection
        .as_ref()
        .expect("pending choice")
        .selecting_player;
    assert_eq!(build_action_mask(game, player)[action as usize], 1.0);
    game.decode_action(action, player);
}

#[test]
fn medusamon_impossible_token_payment_is_not_offered() {
    for modifier in [
        ModifierType::CannotBeAffected,
        ModifierType::CannotBeDestroyed,
    ] {
        let mut r = runner();
        let medusa = r.place_on_field(0, "EX11-012", Some(0));
        let payer = token(&mut r, 1);
        immune(&mut r, payer, modifier);
        r.game
            .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);
        assert!(
            r.game.pending_selection.is_none(),
            "impossible payment: {modifier:?}"
        );
        assert!(!contains(&r.game, 0, "EX11-012"));
        assert!(contains(&r.game, 1, "TOKEN_PETRIFICATION"));
    }
}

#[test]
fn medusamon_filters_only_impossible_payers_and_can_pay_or_decline() {
    for decline in [false, true] {
        let mut r = runner();
        let medusa = r.place_on_field(0, "EX11-012", Some(0));
        let blocked = token(&mut r, 1);
        immune(&mut r, blocked, ModifierType::CannotBeAffected);
        let payable = token(&mut r, 1);
        r.game
            .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);
        let action = encode_attack(payable.player as u16, payable.index as u16);
        assert_eq!(
            r.game.pending_selection.as_ref().unwrap().valid_action_ids,
            vec![action]
        );
        let mut clone = r.game.clone();
        choose(&mut r.game, if decline { PASS } else { action });
        choose(&mut clone, if decline { PASS } else { action });
        assert_eq!(to_ui_json(&r.game), to_ui_json(&clone));
        assert_eq!(contains(&r.game, 0, "EX11-012"), !decline);
        assert_eq!(
            r.game.player(1).battle_area.len(),
            if decline { 2 } else { 1 }
        );
        assert!(r.game.pending_selection.is_none());
    }
}

struct ProtectPayer {
    mode: u8,
}
impl CardEffect for ProtectPayer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let mode = self.mode;
        vec![Effect::when_would_be_deleted(card)
            .optional()
            .replacement_condition(|ctx, subject| {
                subject.permanent() == ctx.source_permanent
                    && ctx.source_permanent.is_some_and(|h| h.index == 0)
            })
            .replacement_process(move |ctx| match mode {
                0 => ctx.cancel(),
                1 => ctx.redirect_to(Zone::Hand),
                _ => ctx.substitute(digimon_engine::replacement::ReplacementSubject::Permanent(
                    PermanentHandle {
                        player: 1,
                        index: 1,
                    },
                )),
            })
            .build()]
    }
}

#[test]
fn medusamon_protected_or_redirected_payer_does_not_pay_and_clone_resumes() {
    for mode in [0, 1, 2] {
        let mut r = runner();
        r.register_effect("TOKEN_PETRIFICATION", Arc::new(ProtectPayer { mode }));
        let medusa = r.place_on_field(0, "EX11-012", Some(0));
        let payer = token(&mut r, 1);
        if mode == 2 {
            token(&mut r, 1);
        }
        r.game
            .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);
        choose(
            &mut r.game,
            encode_attack(payer.player as u16, payer.index as u16),
        );
        assert_eq!(
            r.game.pending_selection.as_ref().unwrap().selecting_player,
            1
        );
        let mut clone = r.game.clone();
        choose(&mut r.game, REPLACEMENT_ACCEPT);
        choose(&mut clone, REPLACEMENT_ACCEPT);
        assert_eq!(to_ui_json(&r.game), to_ui_json(&clone));
        assert!(
            !contains(&r.game, 0, "EX11-012"),
            "unpaid cancellation, mode={mode}"
        );
        assert_eq!(
            r.game.player(1).battle_area.len(),
            if mode == 1 { 0 } else { 1 }
        );
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn medusamon_declining_payer_replacement_completes_payment_on_both_clones() {
    let mut r = runner();
    r.register_effect("TOKEN_PETRIFICATION", Arc::new(ProtectPayer { mode: 0 }));
    let medusa = r.place_on_field(0, "EX11-012", Some(0));
    let payer = token(&mut r, 1);
    r.game
        .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);
    choose(
        &mut r.game,
        encode_attack(payer.player as u16, payer.index as u16),
    );
    let mut clone = r.game.clone();
    choose(&mut r.game, PASS);
    choose(&mut clone, PASS);
    assert_eq!(to_ui_json(&r.game), to_ui_json(&clone));
    assert!(contains(&r.game, 0, "EX11-012"));
    assert!(r.game.player(1).battle_area.is_empty());
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn medusamon_payment_preflight_uses_its_controller_not_the_original_deleter() {
    let mut r = runner();
    let medusa = r.place_on_field(0, "EX11-012", Some(0));
    let payer = token(&mut r, 1);
    r.game.modifiers.add(
        payer,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeDestroyedByEffect,
            Expiry::Permanent,
            1,
        )
        .opponent_only()
        .with_condition(Arc::new(|ctx, _| {
            ctx.replacement_source_controller() == Some(0)
        })),
    );
    EffectContext::new(&mut r.game, CardHandle(0), None, 1).delete_permanent(medusa);
    assert!(r.game.pending_selection.is_none());
    assert!(!contains(&r.game, 0, "EX11-012"));
    assert!(contains(&r.game, 1, "TOKEN_PETRIFICATION"));
}

#[test]
fn styracomon_lowest_dp_is_computed_before_payment_viability() {
    let mut r = runner();
    r.place_on_field(0, "BT24-018", Some(0));
    let subject = r.place_on_field(0, "BODY", Some(0));
    let low = r.place_on_field(1, "LOW", Some(0));
    r.place_on_field(1, "HIGH", Some(0));
    immune(&mut r, low, ModifierType::CannotBeAffected);
    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "higher DP target cannot replace immune lowest DP payer"
    );
    assert!(!contains(&r.game, 0, "BODY"));
    assert_eq!(r.game.player(1).battle_area.len(), 2);
}

#[test]
fn ordinary_medusamon_deletion_keeps_immune_targets_selectable() {
    let mut r = runner();
    let medusa = r.place_on_field(0, "EX11-012", Some(0));
    let target = r.place_on_field(1, "LOW", Some(0));
    immune(&mut r, target, ModifierType::CannotBeAffected);
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(medusa),
    );
    r.game.drain_effect_queue();
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    choose(&mut r.game, action);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    choose(&mut r.game, action);
    assert!(contains(&r.game, 1, "LOW"));
}

#[test]
fn styracomon_tied_lowest_dp_filters_immunity_and_pays_exact_payer() {
    let mut r = runner();
    r.place_on_field(0, "BT24-018", Some(0));
    let subject = r.place_on_field(0, "BODY", Some(0));
    let blocked = r.place_on_field(1, "LOW", Some(0));
    r.place_on_field(1, "LOW", Some(0));
    r.place_on_field(1, "HIGH", Some(0));
    immune(&mut r, blocked, ModifierType::CannotBeAffected);
    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);
    choose(&mut r.game, REPLACEMENT_ACCEPT);
    let s = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(
        s.valid_action_ids.len(),
        1,
        "only the payable lowest-DP tie is legal"
    );
    let action = s.valid_action_ids[0];
    let mut clone = r.game.clone();
    choose(&mut r.game, action);
    choose(&mut clone, action);
    assert_eq!(to_ui_json(&r.game), to_ui_json(&clone));
    assert!(contains(&r.game, 0, "BODY"));
    assert_eq!(r.game.player(1).battle_area.len(), 2);
}

#[test]
fn recorded_medusamon_immune_token_payment_does_not_loop() {
    let recording: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/training/medusamon-immune-token-payment.json"
    ))
    .unwrap();
    let cards = CardData::load_from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/cards.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut replay = ReplayRunner::new(recording.clone(), &cards, true).unwrap();
    for row in recording["actions"].as_array().unwrap() {
        if row["phase"] == "Mulligan" {
            continue;
        }
        let action = row["action_id"].as_u64().unwrap() as u16;
        let player = row["player_id"].as_u64().unwrap() as u8 - 1;
        assert_eq!(
            build_action_mask(&replay.game, player)[action as usize],
            1.0,
            "row {}",
            row["step"]
        );
        if row["step"] == 93 {
            let mut clone = replay.game.clone();
            replay.game.decode_action(action, player);
            clone.decode_action(action, player);
            assert_eq!(to_ui_json(&replay.game), to_ui_json(&clone));
            assert!(!contains(&replay.game, 0, "EX11-012"));
            assert!(contains(&replay.game, 1, "TOKEN_PETRIFICATION"));
            assert!(!replay
                .game
                .pending_selection
                .as_ref()
                .is_some_and(|s| s.prompt.contains("Delete 1 Token")));
            return;
        }
        let step = replay.step();
        assert!(
            step.divergences.is_empty(),
            "row {}: {:?}",
            row["step"],
            step.divergences
        );
    }
    panic!("missing recorded payment boundary");
}
