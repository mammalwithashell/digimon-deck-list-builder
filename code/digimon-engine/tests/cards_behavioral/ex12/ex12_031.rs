use digimon_engine::enums::CardColor;

use super::support::{
    bottom_source_id, field_contains, hand_contains, hand_index, plain_digimon,
    select_first_non_pass, select_hand_card, source_count, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-031";

#[test]
fn ex12_031_on_play_places_matching_hand_card_as_bottom_source_then_bounces_low_source_digimon() {
    let aqua = super::support::digimon("AQUATIC-HAND", CardColor::Blue, 4, 4, 5000, &["Aquatic"]);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-031 YAML loads")
        .add_card(aqua)
        .add_card(plain_digimon("OPP-SRC", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("OPP-TOP", CardColor::Blue, 4, 5000))
        .hand(0, &[CARD_ID, "AQUATIC-HAND"])
        .memory(10)
        .start();
    runner.place_stack(1, &["OPP-SRC", "OPP-TOP"]);

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-031");
    select_hand_card(&mut runner, 0, "AQUATIC-HAND");
    assert_eq!(bottom_source_id(&runner, 0, 0), "AQUATIC-HAND");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish bounce");

    assert_eq!(source_count(&runner, 0, 0), 1);
    assert!(hand_contains(&runner, 1, "OPP-TOP"));
    assert_eq!(runner.battle_area_size(1), 0);
}

#[test]
fn ex12_031_decode_plays_matching_tb_source_on_non_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-031 YAML loads")
        .add_card(tb_digimon("TB-SRC", CardColor::Blue, 4, 5000))
        .memory(10)
        .start();
    let marine_bullmon = runner.place_stack(0, &["TB-SRC", CARD_ID]);

    runner.game.return_to_hand_from_effect(marine_bullmon, 1);
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish Decode");

    assert!(
        field_contains(&runner, 0, "TB-SRC"),
        "Decode plays the matching [TB] source"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "the original non-battle leave still returns EX12-031 to hand"
    );
}

/// TDD RED — task_69f10a66 ruling, EX12-031#effect#1 sub-finding: `<Decode>`
/// is SELF-scoped. Rule 16-35-1: "When the Digimon WITH THIS EFFECT would
/// leave the battle area other than by a battle, you may play 1 Digimon card
/// specified by this effect FROM THAT DIGIMON'S digivolution cards" —
/// another permanent leaving is not this Digimon's trigger. DCGO gates the
/// same way (`Decode`'s CanUse/CanActivate are keyed to the carrier) and
/// asks nothing when a different Digimon is bounced.
///
/// Today this FAILS: `keyword_to_auto_effect(Keyword::Decode)` synthesizes a
/// legacy "<Decode> (hand)"/"(deck)" redirect pair whose self-scope check
/// lives only INSIDE the post-accept process — `collect_candidates` has no
/// `replacement_condition` to filter it, so bouncing ANY permanent while a
/// Decode carrier sits on either field parks a vacuous
/// "May accept replacement: <Decode> (hand)" prompt — handed to the LEAVING
/// permanent's controller, not even the keyword carrier's. Exam witness:
/// qa/dcgo-exams/EX12/EX12-031-effect1.yaml step 9 (our engine asks, DCGO
/// does not — the scripted line desyncs at the spurious gate).
///
/// The carrier here is a synthetic card with `keywords = [Decode]` because
/// that is what PRODUCTION card data holds for EX12-031: `card_data.rs`
/// parses the printed "<Decode ...>" text into `CardData.keywords`, and
/// `build_effects_for_card` then synthesizes the auto-effect pair. The
/// DebugRunner's YAML-derived CardData carries no text-parsed keywords
/// (only `grant_keyword` clauses), which would silently mask the prod bug.
#[test]
fn ex12_031_decode_does_not_open_a_window_for_another_permanents_leave() {
    use digimon_engine::enums::Keyword;
    let decode_carrier = {
        let mut c = plain_digimon("DECODE-CARRIER", CardColor::Blue, 5, 7000);
        // Prod parity: EX12-031's cards.json text "<Decode (...)>" parses to
        // Keyword::Decode in CardData.keywords.
        c.keywords = vec![Keyword::Decode];
        c
    };
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-031 YAML loads")
        .add_card(decode_carrier)
        .add_card(plain_digimon("OPP-BOUNCE", CardColor::Blue, 3, 3000))
        .memory(10)
        .start();
    // Decode carrier on P0's field; a plain Digimon (no Decode) on P1's.
    let _carrier = runner.place_on_field(0, "DECODE-CARRIER", Some(0));
    let opp = runner.place_on_field(1, "OPP-BOUNCE", Some(0));

    // P0's effect bounces P1's Digimon — the exam line's shape.
    runner.game.return_to_hand_from_effect(opp, 0);

    assert!(
        runner.game.pending_selection.is_none(),
        "no replacement window may open: <Decode> is self-scoped (16-35-1); \
         the leaving Digimon has no Decode and MarineBullmon is not leaving \
         (got: {:?})",
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|s| s.prompt.clone())
    );
    assert!(
        hand_contains(&runner, 1, "OPP-BOUNCE"),
        "the bounce itself must have committed"
    );
}
