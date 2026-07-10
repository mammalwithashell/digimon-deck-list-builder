use digimon_engine::action::space::PASS;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectSourceKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::selection::{PendingSelection, SelectionKind};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("VERIFY-001", "Verifier"))
        .add_card(make_test_card("VERIFY-002", "Comparer"))
        .start()
}

#[test]
fn verification_digest_is_stable_for_identical_gameplay_state() {
    let mut a = runner();
    let mut b = runner();

    a.place_on_field(0, "VERIFY-001", Some(0));
    b.place_on_field(0, "VERIFY-001", Some(0));

    assert_eq!(
        a.game.verification_digest(),
        b.game.verification_digest(),
        "identical gameplay state must produce an identical verification digest"
    );
}

#[test]
fn verification_digest_changes_for_zone_order_and_memory_changes() {
    let mut base = runner();
    let mut reordered = runner();

    push_card_to_hand(&mut base, 0, "VERIFY-001");
    push_card_to_hand(&mut base, 0, "VERIFY-002");

    push_card_to_hand(&mut reordered, 0, "VERIFY-001");
    push_card_to_hand(&mut reordered, 0, "VERIFY-002");
    reordered.game.players[0].hand.swap(0, 1);

    assert_ne!(
        base.game.verification_digest(),
        reordered.game.verification_digest(),
        "zone order is gameplay state and must be included"
    );

    reordered.game.players[0].hand.swap(0, 1);
    reordered.game.set_memory(3);
    assert_ne!(
        base.game.verification_digest(),
        reordered.game.verification_digest(),
        "memory is gameplay state and must be included"
    );
}

#[test]
fn verification_digest_includes_pending_selection_kind() {
    let mut target_selection = runner();
    let mut hand_selection = runner();

    target_selection.game.pending_selection = Some(selection(SelectionKind::Target));
    hand_selection.game.pending_selection = Some(selection(SelectionKind::Hand));

    assert_ne!(
        target_selection.game.verification_digest(),
        hand_selection.game.verification_digest(),
        "the pending-selection kind must be visible to replay divergence checks"
    );
}

#[test]
fn verification_digest_sorts_modifier_summary_by_handle_not_hashmap_order() {
    let mut a = runner();
    let mut b = runner();
    let a0 = a.place_on_field(0, "VERIFY-001", Some(0));
    let a1 = a.place_on_field(1, "VERIFY-002", Some(0));
    let b0 = b.place_on_field(0, "VERIFY-001", Some(0));
    let b1 = b.place_on_field(1, "VERIFY-002", Some(0));

    a.game.modifiers.add(
        a0,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, 0),
    );
    a.game.modifiers.add(
        a1,
        ModifierEntry::simple(ModifierType::ChangeDp, 2000, Expiry::EndOfTurn, 1),
    );
    b.game.modifiers.add(
        b1,
        ModifierEntry::simple(ModifierType::ChangeDp, 2000, Expiry::EndOfTurn, 1),
    );
    b.game.modifiers.add(
        b0,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, 0),
    );

    assert_eq!(
        a.game.verification_digest(),
        b.game.verification_digest(),
        "equivalent modifier state must not depend on HashMap insertion order"
    );
}

#[test]
#[ignore = "diagnostic micro-benchmark; run with --ignored --nocapture when changing digest fields"]
fn verification_digest_micro_benchmark() {
    let mut r = runner();
    let p0 = r.place_on_field(0, "VERIFY-001", Some(0));
    let p1 = r.place_on_field(1, "VERIFY-002", Some(0));
    r.game.modifiers.add(
        p0,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, 0),
    );
    r.game.modifiers.add(
        p1,
        ModifierEntry::simple(ModifierType::ChangeDp, 2000, Expiry::EndOfTurn, 1),
    );

    let iterations = 10_000;
    let start = std::time::Instant::now();
    let mut digest = 0;
    for _ in 0..iterations {
        digest = std::hint::black_box(r.game.verification_digest());
    }
    let elapsed = start.elapsed();
    eprintln!(
        "verification_digest: {iterations} iterations in {elapsed:?} ({:?}/digest), last={digest}",
        elapsed / iterations
    );
    assert_ne!(
        digest, 0,
        "digest should produce a non-zero hash for this state"
    );
}

fn selection(kind: SelectionKind) -> PendingSelection {
    PendingSelection {
        kind,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![PASS],
        is_optional: true,
        prompt: "digest test".to_string(),
        effect_choices: None,
        source_card: CardHandle(0),
        source_permanent: None,
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(|_, _| {}),
        on_decline: None,
        zone_owner: None,
    }
}

fn push_card_to_hand(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("test card exists");
    let card_index = runner.game.next_card_index();
    runner.game.players[player]
        .hand
        .push(CardSource::new(data_idx, player as u8, card_index));
}
