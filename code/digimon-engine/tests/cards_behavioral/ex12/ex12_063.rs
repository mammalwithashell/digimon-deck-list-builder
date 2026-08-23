use digimon_engine::enums::{CardColor, ModifierType};
use digimon_engine::replacement::ReplacementCause;

use super::support::{
    field_contains, hand_index, is_suspended, plain_digimon, puppet_tb, select_first_non_pass,
    DebugRunner,
};

const CARD_ID: &str = "EX12-063";

#[test]
fn ex12_063_on_play_suspends_opponent_then_locks_unsuspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .add_card(plain_digimon("OPP", CardColor::Green, 4, 5000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-063");
    select_first_non_pass(&mut runner);
    assert!(is_suspended(&runner, 1, opp.index as usize));

    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish lock");

    assert!(
        runner.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "target cannot unsuspend until their turn ends"
    );
}

#[test]
fn ex12_063_on_deletion_may_play_level4_or_lower_puppet_or_tb_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .add_card(puppet_tb("PUPPET-L4", 4))
        .deck(0, &["PUPPET-L4"])
        .memory(10)
        .start();
    let trash_card = runner.game.players[0].deck.pop().expect("seed trash card");
    runner.game.players[0].trash.push(trash_card);
    let karakurumon = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(karakurumon, ReplacementCause::OpponentEffect);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish on-deletion play");

    assert!(field_contains(&runner, 0, "PUPPET-L4"));
}

#[test]
fn ex12_063_inherited_on_deletion_plays_level4_or_lower_tb_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Purple, 4, 5000))
        .add_card(puppet_tb("TB-L4", 4))
        .deck(0, &["TB-L4"])
        .memory(10)
        .start();
    let trash_card = runner.game.players[0].deck.pop().expect("seed trash card");
    runner.game.players[0].trash.push(trash_card);
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("finish inherited on-deletion play");

    assert!(field_contains(&runner, 0, "TB-L4"));
}

/// task_69f10a66 / EX12-063#inherited#0 differential: the exam scenario
/// (qa/dcgo-exams/EX12/EX12-063-inherited0.yaml, "SIM-SIDE GAP" header)
/// reports that after a DECLINED optional deletion-replacement window our
/// engine never fires Karakurumon's inherited [On Deletion], while the plain
/// no-window deletion (test above) fires it. Rules: the trigger itself is
/// mandatory (15-8-3-1, 15-8-3-5 — pending activation for the card that was
/// the top card); only the "You may play ..." CONTENT is optional (15-9-2).
/// Dropping the trigger silently removes that choice (rule 17).
///
/// This test reproduces the decline-path shape without combat: the carrier's
/// top card has printed <Armor Purge> (optional when-would-be-deleted, not
/// cause-gated), the window parks, the player declines, the deletion commits
/// through `commit_permanent_deletion_no_replace` →
/// `commit_post_replacement_single` — and the inherited [On Deletion] trash
/// pick must still park afterward.
#[test]
fn ex12_063_inherited_on_deletion_still_fires_after_declined_replacement_window() {
    use digimon_engine::enums::Keyword;
    let ap_top = {
        let mut c = plain_digimon("AP-CARRIER", CardColor::Purple, 4, 5000);
        c.keywords = vec![Keyword::ArmorPurge];
        c
    };
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .add_card(ap_top)
        .add_card(puppet_tb("TB-L4", 4))
        .deck(0, &["TB-L4"])
        .memory(10)
        .start();
    let trash_card = runner.game.players[0].deck.pop().expect("seed trash card");
    runner.game.players[0].trash.push(trash_card);
    let carrier = runner.place_stack(0, &[CARD_ID, "AP-CARRIER"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    // The optional <Armor Purge> replacement window parks; decline it.
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Armor Purge accept window parks");
        assert!(
            sel.prompt.contains("Armor Purge"),
            "expected the Armor Purge accept gate, got {:?}",
            sel.prompt
        );
    }
    runner
        .decline_optional_trigger()
        .expect("decline Armor Purge");

    // The deletion has now committed through the decline path. The inherited
    // [On Deletion] must still offer its optional trash play.
    assert!(
        runner.game.pending_selection.is_some(),
        "inherited [On Deletion] must fire after the declined replacement \
         window (15-8-3-1/15-8-3-5); its 'you may play' choice must reach \
         the player (15-9-2, rule 17)"
    );
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("finish inherited on-deletion play after declined window");
    assert!(field_contains(&runner, 0, "TB-L4"));
}

/// task_69f10a66 Family 3a — the BATTLE-context shape of the drop the exam
/// scenario reports (EX12-063-inherited0.yaml, "SIM-SIDE GAP"): the carrier
/// [EX12-063 under, <Barrier>-keyword top] attacks the opponent player, the
/// flipped security Digimon wins the security battle, the mid-battle
/// `WhenWouldBeDeleted(Battle)` <Barrier> window parks during
/// `resolve_pending_battle` / `advance_security_resolution`, the player
/// DECLINES — and the inherited [On Deletion] trash pick must still park.
///
/// The generic (non-battle) decline path is proven sound by the test above;
/// this differential isolates the battle-resume path
/// (`make_decline_callback` → `commit_post_replacement_single` reached while
/// `security_resolution` is live, then `advance_security_resolution`).
/// Rules: 15-8-3-1 (a trigger-type effect ALWAYS triggers when its
/// conditions are met) + 15-8-3-5 (on deletion the pending activation
/// belongs to the card that was the top card — explicitly covering
/// inherited [On Deletion]) + 15-9-2 (the "You may play" content is the
/// player's choice; rule 17 forbids dropping it).
#[test]
fn ex12_063_inherited_on_deletion_fires_after_declined_mid_battle_barrier_window() {
    use digimon_engine::enums::Keyword;
    let barrier_top = {
        let mut c = plain_digimon("BARRIER-TOP", CardColor::Purple, 4, 5000);
        c.keywords = vec![Keyword::Barrier];
        c
    };
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .add_card(barrier_top)
        .add_card(puppet_tb("TB-L4", 4))
        .add_card(plain_digimon("SEC-WALL", CardColor::Green, 6, 12000))
        .add_card(plain_digimon("FILLER", CardColor::Green, 3, 1000))
        .deck(0, &["TB-L4"])
        .security(0, &["FILLER", "FILLER"])
        .security(1, &["SEC-WALL"])
        .memory(10)
        .start();
    let trash_card = runner.game.players[0].deck.pop().expect("seed trash card");
    runner.game.players[0].trash.push(trash_card);
    let carrier = runner.place_stack(0, &[CARD_ID, "BARRIER-TOP"]);

    // Security attack: flips SEC-WALL (12000 DP) — the 5000 DP carrier loses
    // the security battle → WhenWouldBeDeleted(cause=Battle) parks the
    // optional <Barrier> window mid-battle.
    let _ = runner.attack_player(carrier, 1, false);
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("mid-battle <Barrier> accept window parks");
        assert!(
            sel.prompt.contains("Barrier"),
            "expected the Barrier accept gate, got {:?}",
            sel.prompt
        );
    }
    runner
        .decline_optional_trigger()
        .expect("decline mid-battle Barrier");

    // The battle deletion has now committed through the decline path. The
    // inherited [On Deletion] must still offer its optional trash play —
    // identical to the generic-path test above.
    assert!(
        runner.game.pending_selection.is_some(),
        "inherited [On Deletion] must fire after the declined MID-BATTLE \
         Barrier window (15-8-3-1/15-8-3-5); the trash-play choice must \
         reach the player (15-9-2, rule 17)"
    );
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("finish inherited on-deletion play after declined mid-battle window");
    assert!(field_contains(&runner, 0, "TB-L4"));
}

/// task_69f10a66 Family 3a — the EXAM-EXACT stack shape: Ryugumon EX12-036
/// (whose `<Barrier>`/`<Evade>` are DSL `grant_keyword` declarative clauses,
/// not printed `CardData.keywords`) digivolved over Karakurumon EX12-063,
/// losing a security battle (12000 vs 12000 tie → attacker deleted). The
/// mid-battle `<Barrier>` window parks; `<Evade>` is unpayable (the attacker
/// suspended when declaring, so the suspend-self cost fails); the player
/// declines Barrier — and Karakurumon's inherited [On Deletion] trash pick
/// must park (qa/dcgo-exams/EX12/EX12-063-inherited0.yaml "SIM-SIDE GAP").
#[test]
fn ex12_063_inherited_on_deletion_fires_after_declined_barrier_in_exam_stack_shape() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .dsl_card("EX12-036")
        .expect("EX12-036 YAML loads")
        .add_card(puppet_tb("TB-L4", 4))
        .add_card(plain_digimon("SEC-WALL", CardColor::Green, 6, 12000))
        .add_card(plain_digimon("FILLER", CardColor::Green, 3, 1000))
        .deck(0, &["TB-L4"])
        .security(0, &["FILLER", "FILLER"])
        .security(1, &["SEC-WALL"])
        .memory(10)
        .start();
    let trash_card = runner.game.players[0].deck.pop().expect("seed trash card");
    runner.game.players[0].trash.push(trash_card);
    // Ryugumon on top of Karakurumon — the exam carrier.
    let carrier = runner.place_stack(0, &[CARD_ID, "EX12-036"]);
    runner.game.tick_declarative_effects();

    // Security attack: flips SEC-WALL (12000) vs Ryugumon (12000) — the tie
    // deletes the attacker → WhenWouldBeDeleted(Battle) mid-battle.
    let _ = runner.attack_player(carrier, 1, false);
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("mid-battle <Barrier> accept window parks (DSL grant)");
        assert!(
            sel.prompt.contains("Barrier"),
            "expected the Barrier accept gate, got {:?}",
            sel.prompt
        );
    }
    runner
        .decline_optional_trigger()
        .expect("decline mid-battle Barrier");

    // Karakurumon's inherited [On Deletion] must still offer its trash play.
    // (If a second replacement gate — e.g. a duplicate Barrier candidate or
    // Evade — parks instead, decline until the window family is exhausted so
    // the assertion below judges the OnDeletion trigger itself.)
    let mut declined = 0;
    while let Some(sel) = runner.game.pending_selection.as_ref() {
        let is_replacement_gate =
            sel.prompt.contains("Barrier") || sel.prompt.contains("Evade");
        if !is_replacement_gate {
            break;
        }
        declined += 1;
        assert!(declined <= 4, "replacement gates should not loop");
        runner
            .decline_optional_trigger()
            .expect("decline extra replacement gate");
    }
    assert!(
        runner.game.pending_selection.is_some(),
        "inherited [On Deletion] must fire after the declined mid-battle \
         Barrier window in the exam stack shape (15-8-3-1/15-8-3-5, rule 17)"
    );
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("finish inherited on-deletion play (exam stack shape)");
    assert!(field_contains(&runner, 0, "TB-L4"));
}
