//! BT16-029 Agumon — Digimon, Lv.3, Yellow, DP 1000, Cost 3.
//! Traits: Dinosaur/Light Fang. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB / data/card_bundles/BT16-029.md)
//!
//! Digivolve: Yellow Lv.2 / Cost 0 (standard circle)
//! Digivolve (special): [Digivolve] Lv.2 w/[Light Fang]/[Night Claw] trait: Cost 0
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Light Fang] trait
//! card and 1 [Night Claw] trait or 1 2-color card among them to the hand.
//! Return the rest to the bottom of the deck.
//!
//! Inherited Effect [Your Turn] All of your opponent's Security Digimon get
//! -3000 DP.
//!
//! Official Q&A: "No, you can't. You can only add 1 [Night Claw] trait or 1
//! 2-color card to the hand." — confirms bucket B is a single pick satisfying
//! EITHER condition, not both a Night Claw card AND a 2-color card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Yellow/BT16_029.cs
//!
//! `LightFangCondition`: trait contains "Light Fang" (or "LightFang").
//! `NightClawCondition`: trait contains "Night Claw" (or "NightClaw") OR
//! `cardSource.CardColors.Count == 2`.
//! `SimplifiedRevealDeckTopCardsAndSelect(revealCount: 3, buckets: [LightFang(max 1),
//! NightClawOr2Color(max 1)], remainingCardsPlace: DeckBottom)`.
//! Digivolution condition: `AddSelfDigivolutionRequirementStaticEffect` — Lv.2
//! top card with Light Fang OR Night Claw trait, cost 0 (any color — a second,
//! trait-gated alt-path alongside the standard Yellow Lv.2/cost-0 circle).
//! Inherited: `ChangeSecurityDigimonCardDPStaticEffect(-3000, isInheritedEffect:
//! true, condition: owner's turn)` — same PUPPETS-G008 idiom as ST19-03/EX7-024.
//!
//! # Patterns this test covers
//! - A1/A3 reveal-top-N, two-bucket add-to-hand (ST19-03 idiom)
//! - Bucket B `any_of` disjunction: trait_has(Night Claw) OR self_color_count_gte(2)
//! - Cross-bucket duplicate prevention (no_duplicate_cards)
//! - D4 declarative aura (inherited security DP debuff, PUPPETS-G008 idiom)
//! - Alt-path: standard color circle + trait-gated any-color alt-path (BT10-029 idiom)

use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

// ─── Structural ────────────────────────────────────────────────────────────

#[test]
fn bt16_029_has_on_play_clause() {
    use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};

    let runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .memory(10)
        .start();
    let compiled = runner.compiled_card("BT16-029").expect("compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let on_play = triggered
        .iter()
        .find(|c| c.when == vec![CompiledTiming::OnPlay] && c.scope == CompiledScope::FaceUp)
        .expect("own OnPlay clause exists");
    assert!(
        !on_play.optional,
        "reveal-and-add is mandatory, not a 'you may'"
    );
}

#[test]
fn bt16_029_inherited_effect_is_declarative_aura() {
    use digimon_dsl::compiled::CompiledClause;

    let runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .memory(10)
        .start();
    let compiled = runner.compiled_card("BT16-029").expect("compiled card");

    let has_aura = compiled
        .effects
        .iter()
        .any(|c| matches!(c, CompiledClause::Declarative(_)));
    assert!(
        has_aura,
        "inherited -3000 DP to opponent security Digimon is a declarative aura clause"
    );
}

#[test]
fn bt16_029_has_standard_and_trait_alt_paths() {
    use digimon_dsl::compiled::CompiledAltPathKind;

    let runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .memory(10)
        .start();
    let compiled = runner.compiled_card("BT16-029").expect("compiled card");

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        2,
        "BT16-029 has the standard yellow Lv.2/cost-0 circle plus the \
         [Light Fang]/[Night Claw]-trait any-color Lv.2/cost-0 alt-path"
    );
}

// ─── On Play: bucket helpers ────────────────────────────────────────────────

fn light_fang_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = vec!["Light Fang".to_string()];
    card
}

fn night_claw_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = vec!["Night Claw".to_string()];
    card
}

fn two_color_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![CardColor::Red, CardColor::Yellow];
    card
}

fn one_color_neutral_card(id: &str) -> digimon_engine::card_data::CardData {
    // No traits, single color — must not satisfy either bucket.
    make_test_card(id, id)
}

fn dual_light_fang_and_night_claw_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = vec!["Light Fang".to_string(), "Night Claw".to_string()];
    card
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let action = revealed_action_for_id(runner, id);
    let view = runner.pending_selection_view().expect(label);
    assert!(view.valid_action_ids.contains(&action), "{label}");
    let player = runner.pending_selection().expect(label).selecting_player;
    runner.execute_action(player, action).expect(label);
}

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: bucket pick is required (mandatory 'Add' clause)"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS is not legal on a mandatory bucket"
    );
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

// ─── On Play: behavioral outcomes ───────────────────────────────────────────

#[test]
fn bt16_029_on_play_adds_light_fang_and_night_claw() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(light_fang_card("FANG"))
        .add_card(night_claw_card("CLAW"))
        .add_card(one_color_neutral_card("FILLER"))
        .deck(0, &["FILLER", "CLAW", "FANG"])
        .hand(0, &["BT16-029"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon");

    assert_required_pending_pick(&runner, "Light Fang bucket");
    pick_revealed_by_id(&mut runner, "FANG", "pick Light Fang");

    assert_required_pending_pick(&runner, "Night Claw / 2-color bucket");
    pick_revealed_by_id(&mut runner, "CLAW", "pick Night Claw");

    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"FANG".to_string()));
    assert!(hand_ids.contains(&"CLAW".to_string()));
    assert!(!hand_ids.contains(&"FILLER".to_string()));

    // Remainder goes to the bottom of the deck, not the top.
    let deck_ids = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(deck_ids, vec!["FILLER".to_string()]);
}

#[test]
fn bt16_029_on_play_bucket_b_accepts_two_color_card_without_night_claw_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(light_fang_card("FANG"))
        .add_card(two_color_card("DUOCOLOR"))
        .add_card(one_color_neutral_card("FILLER"))
        .deck(0, &["FILLER", "DUOCOLOR", "FANG"])
        .hand(0, &["BT16-029"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon");
    assert_required_pending_pick(&runner, "Light Fang bucket");
    pick_revealed_by_id(&mut runner, "FANG", "pick Light Fang");

    assert_required_pending_pick(&runner, "Night Claw / 2-color bucket");
    let view = runner
        .pending_selection_view()
        .expect("Night Claw / 2-color bucket");
    let duocolor_action = revealed_action_for_id(&runner, "DUOCOLOR");
    assert!(
        view.valid_action_ids.contains(&duocolor_action),
        "a 2-color card (no Night Claw trait) must be a legal pick for bucket B"
    );
    pick_revealed_by_id(&mut runner, "DUOCOLOR", "pick 2-color card");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"FANG".to_string()));
    assert!(hand_ids.contains(&"DUOCOLOR".to_string()));
}

#[test]
fn bt16_029_on_play_bucket_b_rejects_single_color_non_night_claw_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(light_fang_card("FANG"))
        .add_card(one_color_neutral_card("PLAIN"))
        .add_card(one_color_neutral_card("FILLER"))
        .deck(0, &["FILLER", "PLAIN", "FANG"])
        .hand(0, &["BT16-029"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon");
    assert_required_pending_pick(&runner, "Light Fang bucket");
    pick_revealed_by_id(&mut runner, "FANG", "pick Light Fang");

    // Bucket B has no candidate (PLAIN and FILLER each have 1 color, no
    // Night Claw trait). Per DCGO's `SimplifiedRevealDeckTopCardsAndSelect`
    // with maxCount 1 and no qualifying candidate, the bucket must finalize
    // silently with 0 picks — the flow advances straight to the mandatory
    // remainder-ordering prompt over the 2 leftover cards, never parking a
    // bucket-B selection over an ineligible card.
    let post_bucket_a_view = runner
        .pending_selection_view()
        .expect("remainder-ordering prompt installs once bucket B auto-finalizes empty");
    assert!(
        matches!(
            post_bucket_a_view.kind,
            SelectionKind::OrderedPermutation { remaining: 2 }
        ),
        "bucket B must finalize with 0 picks (no valid candidates) and hand off directly to \
         remainder ordering over PLAIN + FILLER, not park a phantom pick over an ineligible card; \
         got {:?}",
        post_bucket_a_view.kind
    );

    runner.auto_resolve().expect("finalize remaining flow");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"FANG".to_string()));
    assert!(
        !hand_ids.contains(&"PLAIN".to_string()),
        "PLAIN does not satisfy bucket B and must not be added to hand"
    );
    assert!(
        !hand_ids.contains(&"FILLER".to_string()),
        "FILLER does not satisfy bucket B and must not be added to hand"
    );
}

#[test]
fn bt16_029_on_play_no_light_fang_candidate_leaves_bucket_a_empty() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(night_claw_card("CLAW"))
        .add_card(one_color_neutral_card("FILLER1"))
        .add_card(one_color_neutral_card("FILLER2"))
        .deck(0, &["FILLER2", "FILLER1", "CLAW"])
        .hand(0, &["BT16-029"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon");

    // Bucket A (Light Fang) has no candidate among the 3 revealed cards, so
    // it auto-finalizes with 0 picks and the flow hands off directly to
    // bucket B (Night Claw / 2-color), whose only qualifying candidate is
    // CLAW.
    let bucket_b_view = runner
        .pending_selection_view()
        .expect("bucket B prompt installs once bucket A auto-finalizes empty");
    let claw_action = revealed_action_for_id(&runner, "CLAW");
    assert_eq!(
        bucket_b_view.valid_action_ids,
        vec![claw_action],
        "bucket B's only legal pick is CLAW; bucket A never parked a prompt"
    );
    pick_revealed_by_id(&mut runner, "CLAW", "pick Night Claw");
    runner.auto_resolve().expect("resolve remaining buckets");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"CLAW".to_string()));
    assert_eq!(
        hand_ids.len(),
        1,
        "only the Night Claw pick lands in hand; bucket A stayed empty"
    );
}

#[test]
fn bt16_029_on_play_same_revealed_card_cannot_fill_both_buckets() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(dual_light_fang_and_night_claw_card("DUAL"))
        .add_card(night_claw_card("CLAW"))
        .add_card(one_color_neutral_card("FILLER"))
        .deck(0, &["FILLER", "CLAW", "DUAL"])
        .hand(0, &["BT16-029"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon");
    assert_required_pending_pick(&runner, "Light Fang bucket");
    pick_revealed_by_id(&mut runner, "DUAL", "pick dual-trait card for bucket A");

    // DUAL already spent on bucket A — it cannot also satisfy bucket B, even
    // though it carries the Night Claw trait too (official Q&A: only one pick
    // per bucket, no double-dipping the same revealed card). CLAW is still a
    // live, distinct candidate for bucket B.
    let bucket_b_view = runner
        .pending_selection_view()
        .expect("bucket B prompt installs after bucket A's pick");
    let dual_action = revealed_action_for_id(&runner, "DUAL");
    let claw_action = revealed_action_for_id(&runner, "CLAW");
    assert!(
        !bucket_b_view.valid_action_ids.contains(&dual_action),
        "the same revealed card cannot satisfy both printed buckets"
    );
    assert!(
        bucket_b_view.valid_action_ids.contains(&claw_action),
        "CLAW (a distinct Night Claw card) remains a legal bucket-B pick"
    );
    pick_revealed_by_id(&mut runner, "CLAW", "pick Night Claw");
    runner.auto_resolve().expect("resolve remaining buckets");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"DUAL".to_string()));
    assert!(hand_ids.contains(&"CLAW".to_string()));
    assert_eq!(
        hand_ids.len(),
        2,
        "DUAL fills bucket A; CLAW fills bucket B — no double-dipping DUAL"
    );
}

#[test]
fn bt16_029_on_play_bottoms_all_three_when_no_bucket_candidates() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(one_color_neutral_card("PLAIN1"))
        .add_card(one_color_neutral_card("PLAIN2"))
        .add_card(one_color_neutral_card("PLAIN3"))
        .deck(0, &["PLAIN3", "PLAIN2", "PLAIN1"])
        .hand(0, &["BT16-029"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.game.players[0].deck.len();

    runner.play(0, 0).expect("play Agumon");

    // Neither bucket has a qualifying candidate (all 3 revealed cards are
    // untraited/1-color): both buckets should finalize with 0 picks, then
    // `place_remainder_on_deck` parks a no-approximations
    // `select_ordered_permutation` prompt over the 3-card remainder — the
    // bottoming order is a genuine player/RL-agent choice, not auto-placed.
    runner
        .pending_selection_view()
        .expect("remainder-ordering prompt must install for a 3-card remainder");
    assert!(
        !runner.pending_is_optional(),
        "ordering the remainder is not a 'you may' — it always installs when the pool is non-empty"
    );

    runner.auto_resolve().expect("resolve remainder ordering");

    // Agumon itself left the hand when played; no bucket adds happened.
    assert_eq!(runner.hand_size(0), hand_before - 1);
    let deck_ids = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(
        deck_ids.len(),
        deck_before,
        "all 3 revealed cards return to the deck (bottomed), none stay in the reveal pool"
    );
    for id in ["PLAIN1", "PLAIN2", "PLAIN3"] {
        assert!(
            deck_ids.contains(&id.to_string()),
            "{id} must be back in the deck after bottoming the remainder"
        );
    }
    assert!(
        runner.game.revealed_cards.is_empty(),
        "reveal pool must be empty once the remainder is placed"
    );
}

#[test]
fn bt16_029_on_play_fires_with_fewer_than_three_cards_in_deck() {
    // DCGO CanActivateCondition only requires LibraryCards.Count >= 1.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(light_fang_card("FANG"))
        .deck(0, &["FANG"])
        .hand(0, &["BT16-029"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon");
    runner
        .auto_resolve()
        .expect("effect resolves even with only 1 card in deck");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"FANG".to_string()));
}

// ─── Inherited [Your Turn] security DP debuff ───────────────────────────────

fn attacker_lv4(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 5;
    card
}

fn digimon_security_card(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card
}

#[test]
fn bt16_029_inherited_security_dp_debuff_lets_8000_attacker_survive_9000_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT16-029")
            .expect("BT16-029 registered");
        let next = game.next_card_index();
        let perm = &mut game.players[atk.player as usize].battle_area[atk.index as usize];
        let mut src = CardSource::new(data_idx, atk.player, next);
        src.card_index = next;
        perm.card_sources.insert(0, src);
    }

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "8000 DP attacker survives after Agumon's -3000 drops 9000 security Digimon to 6000"
    );
    assert_eq!(runner.battle_area_size(0), 1);
}

#[test]
fn bt16_029_inherited_security_dp_debuff_without_agumon_in_stack_loses_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "without BT16-029 in the stack, an 8000 attacker loses to 9000 security Digimon"
    );
    assert_eq!(runner.battle_area_size(0), 0);
}

#[test]
fn bt16_029_inherited_security_dp_debuff_does_not_apply_when_not_attackers_turn() {
    // The aura is scoped `active_when: your_turn`, evaluated with `rctx.player
    // == attacker.player` (`Game::attacker_security_dp_adjustment` always
    // reads the attacking permanent's own digivolution stack). Placing
    // BT16-029 in PLAYER 1's attacking stack while `turn_player()` is still
    // player 0 (the harness default after `start()`, since nothing advanced
    // the turn) means it is NOT the attacker's turn, so the -3000 DP debuff
    // must not apply — the 9000 DP security Digimon stops the 8000 DP
    // attacker exactly like the "without Agumon in the stack" case.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-029")
        .expect("BT16-029 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(0, &["SEC"])
        .start();

    assert_eq!(
        runner.game.turn_player(),
        0,
        "default turn player after start() is player 0"
    );

    let atk = runner.place_on_field(1, "ATK", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT16-029")
            .expect("BT16-029 registered");
        let next = game.next_card_index();
        let perm = &mut game.players[atk.player as usize].battle_area[atk.index as usize];
        let mut src = CardSource::new(data_idx, atk.player, next);
        src.card_index = next;
        perm.card_sources.insert(0, src);
    }

    // Player 1 attacks while it is still player 0's turn (`turn_player() ==
    // 0 != attacker.player == 1`), so the "your turn" gate must fail.
    let result = runner.attack_player(atk, 0, false);
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "aura must NOT apply when it is not the attacker's (Agumon controller's) turn"
    );
}
