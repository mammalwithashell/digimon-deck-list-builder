use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "EX12-009";

fn red_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.play_cost = 3;
    card.dp = Some(3000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn option_with_trait(id: &str, trait_name: &str) -> CardData {
    let mut card = red_digimon(id, &[trait_name]);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn ex12_009_on_play_reveals_three_adds_shambala_and_tb_cards_rest_to_bottom() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-009 YAML loads")
        .add_card(option_with_trait("SHAMBALA-OPTION", "Shambala"))
        .add_card(red_digimon("TB-DIGIMON", &["TB"]))
        .add_card(red_digimon("FILLER", &[]))
        .add_card(red_digimon("PAD", &[]))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD", "SHAMBALA-OPTION", "TB-DIGIMON", "FILLER"])
        .memory(10)
        .start();
    let deck_before = runner.deck_size(0);

    assert!(runner.play(0, 0).is_some(), "EX12-009 should be playable");
    runner.auto_resolve().expect("resolve EX12-009 reveal");

    let hand = hand_ids(&runner, 0);
    assert!(
        hand.iter().any(|id| id == "SHAMBALA-OPTION"),
        "hand={hand:?}"
    );
    assert!(hand.iter().any(|id| id == "TB-DIGIMON"), "hand={hand:?}");
    assert!(
        !hand.iter().any(|id| id == "FILLER"),
        "non-matching revealed card must not be added: {hand:?}"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "two selected cards leave deck; the unselected card returns to bottom"
    );
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        "FILLER",
        "unpicked reveal remainder returns to deck bottom"
    );
}

#[test]
fn ex12_009_inherited_dp_aura_applies_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-009 YAML loads")
        .add_card(red_digimon("CARRIER", &[]))
        .memory(1)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(
        runner.effective_dp(carrier),
        Some(5000),
        "EX12-009 inherited +2000 DP applies on your turn"
    );

    runner.end_turn();
    assert_eq!(
        runner.effective_dp(carrier),
        Some(3000),
        "EX12-009 inherited +2000 DP turns off outside your turn"
    );
}

// ---------------------------------------------------------------------------
// CONTESTED REVEAL -- the [Shambala] pick and the [TB] pick compete
// ---------------------------------------------------------------------------
// The two tests above (and the DCGO exam scenario for this clause) both run a
// reveal where each bucket has at most ONE candidate, so "add 1 [Shambala]
// card and 1 [TB] card" never actually asks the player anything. That is not
// how this card plays in Toho Braves: every Digimon and Option in the pool
// prints BOTH [Shambala] and [TB], so in a real game the buckets are always
// competing for the same cards.
//
// The consequential case is a reveal holding a Shambala-only card, a
// dual-trait card, and a blank:
//
//   bucket 0 [Shambala] -> { A-SHAMBALA, B-DUAL }   <-- a REAL choice
//     pick B-DUAL     -> bucket 1 [TB] has nothing left  -> ONE card added
//     pick A-SHAMBALA -> bucket 1 [TB] still has B-DUAL  -> TWO cards added
//
// So the player's answer to bucket 0 decides whether the effect nets one card
// or two. An engine that auto-took the first candidate would silently rob the
// player of a card in exactly the reveal that matters most (rule 17). Both
// tests assert the choice was OFFERED before answering it, so a regression
// that collapses the prompt to a single option fails here rather than quietly
// picking one branch.

/// Resolve the pending reveal-bucket prompt by naming the card, not a slot --
/// a positional pick would silently follow a reordering of `revealed_cards`.
fn pick_revealed(runner: &mut DebugRunner, card_id: &str) {
    let view = runner
        .pending_selection_view()
        .expect("a reveal-bucket prompt must be pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&action| {
            action
                .checked_sub(SEL_REVEAL_START)
                .and_then(|idx| runner.game.revealed_cards.get(idx as usize))
                .is_some_and(|card| card.card_id(&runner.game.card_data) == card_id)
        })
        .unwrap_or_else(|| panic!("{card_id} must be a legal pick: {:?}", view.valid_action_ids));
    runner
        .execute_action(view.selecting_player, want)
        .unwrap_or_else(|err| panic!("select {card_id}: {err:?}"));
    runner.game.drain_effect_queue();
}

fn contested_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-009 YAML loads")
        .add_card(red_digimon("A-SHAMBALA", &["Shambala"]))
        .add_card(red_digimon("B-DUAL", &["Shambala", "TB"]))
        .add_card(red_digimon("C-BLANK", &[]))
        .add_card(red_digimon("PAD", &[]))
        .hand(0, &[CARD_ID])
        // deck is listed BOTTOM-first, so the revealed top 3 are the last three.
        .deck(0, &["PAD", "C-BLANK", "B-DUAL", "A-SHAMBALA"])
        .memory(10)
        .start()
}

/// Spending the dual-trait card on the [Shambala] bucket strands the [TB]
/// bucket with no candidate, and the effect adds only that one card.
#[test]
fn ex12_009_dual_trait_card_spent_on_shambala_leaves_the_tb_bucket_empty() {
    let mut runner = contested_runner();
    assert!(runner.play(0, 0).is_some(), "EX12-009 should be playable");
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("the [Shambala] bucket must prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both the Shambala-only and the dual-trait card must be offered, \
         so the player -- not the engine -- decides which fills the bucket: {:?}",
        view.valid_action_ids
    );

    pick_revealed(&mut runner, "B-DUAL");
    runner.auto_resolve().expect("drain remaining prompts");

    let hand = hand_ids(&runner, 0);
    assert!(hand.contains(&"B-DUAL".to_string()), "hand={hand:?}");
    assert!(
        !hand.contains(&"A-SHAMBALA".to_string()),
        "the [TB] bucket has no candidate once B-DUAL is consumed, so nothing \
         else may be added -- the buckets resolve in printed order and the \
         second is not rescued: hand={hand:?}"
    );
    assert_eq!(
        hand.len(),
        1,
        "exactly one card is added on this branch: hand={hand:?}"
    );
}

/// Filling [Shambala] from the single-trait card keeps the dual-trait card
/// available for [TB], and the effect adds two cards.
#[test]
fn ex12_009_shambala_only_pick_keeps_the_dual_trait_card_for_the_tb_bucket() {
    let mut runner = contested_runner();
    assert!(runner.play(0, 0).is_some(), "EX12-009 should be playable");
    runner.game.drain_effect_queue();

    pick_revealed(&mut runner, "A-SHAMBALA");
    pick_revealed(&mut runner, "B-DUAL");
    runner.auto_resolve().expect("drain remaining prompts");

    let mut hand = hand_ids(&runner, 0);
    hand.sort();
    assert_eq!(
        hand,
        vec!["A-SHAMBALA".to_string(), "B-DUAL".to_string()],
        "routing the dual-trait card to [TB] instead nets both cards"
    );
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        "C-BLANK",
        "the unpicked blank returns to the deck bottom"
    );
}
