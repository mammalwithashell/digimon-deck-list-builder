use super::support::{
    fire_when_digivolving, plain_digimon, select_first_non_pass, vb_digimon, vb_text_digimon,
    DebugRunner,
};
use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::{PASS, PLAY_HAND_END, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX12-018";

#[test]
fn ex12_018_is_dual_with_digimon_and_option_face_effects() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-018 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-018");
    assert_eq!(card.kind, CompiledCardKind::Dual);
    let dual = card.dual.as_ref().expect("dual metadata");

    for keyword in ["Progress", "Piercing", "SecurityAttackPlus"] {
        assert!(
            dual.digimon.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Declarative(d) if format!("{d:?}").contains(keyword)
            )),
            "Digimon face must grant {keyword}"
        );
    }
    assert!(
        dual.digimon.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
                    && t.process.iter().any(|step| matches!(step, CompiledStep::SelectZoneCards { .. }))
        )),
        "Digimon face must pick its up-to-2 source cards in ONE pick over hand AND trash (\"from your hand or trash\")"
    );
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "Option face must carry Arts Digivolve"
    );
    assert_eq!(dual.option.security_text, "");
    assert_eq!(
        dual.option.effects.len(),
        1,
        "Option face should only have [Main]"
    );
}

fn hand_id(runner: &DebugRunner, card: &str) -> u16 {
    let i = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card)
        .unwrap_or_else(|| panic!("{card} not in hand"));
    PLAY_HAND_START + i as u16
}

fn trash_id(runner: &DebugRunner, card: &str) -> u16 {
    let i = runner.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card)
        .unwrap_or_else(|| panic!("{card} not in trash"));
    TRASH_EFFECT_START + i as u16
}

/// card_sources run bottom -> top card.
fn stack(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Siriusmon on BASE, VB-H in hand, VB-T in trash, a 9000 DP opponent
/// Digimon; [When Digivolving] fired.
fn sirius_with_hand_and_trash_fuel() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-018 YAML loads")
        .add_card(vb_digimon("BASE", CardColor::Red, 5, 8000))
        .add_card(vb_digimon("VB-H", CardColor::Yellow, 4, 5000))
        .add_card(vb_text_digimon("VB-T", CardColor::Red, 3, 3000))
        .add_card(plain_digimon("OPP", CardColor::Purple, 5, 9000))
        .hand(0, &["VB-H"])
        .memory(10)
        .start();
    runner.inject_trash(0, "VB-T");
    let sirius = runner.place_stack(0, &["BASE", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP", Some(0));
    fire_when_digivolving(&mut runner, sirius);
    (runner, sirius, opp)
}

/// "You may place up to 2 ... from your hand or trash": ONE pick offers the
/// hand card and the trash card together, and may take one of each. DCGO
/// (EX12_018.cs:84-160) reaches the same pair through its "Where from?" loop.
/// Top placement: DCGO reverses the picks (:186-192) and
/// `Permanent.AddDigivolutionCardsTop` inserts each at index 1 under the top
/// card (Permanent.cs:1145), so the FIRST pick ends up the topmost
/// digivolution card. CR 4.3 3-1-3-3 / 3-1-3-4: the cards are placed at the
/// same time and the effect's player chooses their order (here: pick order).
#[test]
fn ex12_018_places_one_hand_and_one_trash_card_in_one_resolution_on_top() {
    let (mut runner, sirius, opp) = sirius_with_hand_and_trash_fuel();

    let pick = runner.pending_selection_view().expect("source-card pick");
    assert!(
        matches!(pick.kind, SelectionKind::CountCappedMultiSelect { max: 2, picked: 0, .. }),
        "{:?}",
        pick.kind
    );
    let (h, t) = (hand_id(&runner, "VB-H"), trash_id(&runner, "VB-T"));
    assert!(pick.valid_action_ids.contains(&h), "hand card offered: {:?}", pick.valid_action_ids);
    assert!(pick.valid_action_ids.contains(&t), "trash card offered in the SAME pick: {:?}", pick.valid_action_ids);
    assert!(pick.valid_action_ids.contains(&PASS), "\"you may ... up to 2\": declining is legal");

    runner.execute_action(0, h).expect("pick the hand card first");
    let t = trash_id(&runner, "VB-T");
    runner.execute_action(0, t).expect("then the trash card");

    let pos = runner.pending_selection_view().expect("top / bottom choice");
    assert_eq!(pos.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("top of digivolution sources");

    assert_eq!(
        stack(&runner, sirius),
        vec!["BASE", "VB-T", "VB-H", CARD_ID],
        "both placed on top, the first pick (hand VB-H) topmost"
    );
    assert!(runner.game.players[0].hand.is_empty());
    assert!(runner.game.players[0].trash.iter().all(|c| c.card_id(&runner.game.card_data) != "VB-T"));

    let dp_prompt = runner.pending_selection_view().expect("opponent DP target prompt");
    assert_eq!(dp_prompt.kind, SelectionKind::OppField);
    select_first_non_pass(&mut runner);
    assert_eq!(
        runner.effective_dp(opp),
        Some(3000),
        "3 digivolution cards (BASE, VB-T, VB-H): -6000 DP"
    );
}

/// Bottom placement keeps the pick order top-down: DCGO
/// `AddDigivolutionCardsBottom` appends each in list order (Permanent.cs:
/// 1247), so the LAST pick is the bottommost digivolution card.
#[test]
fn ex12_018_trash_then_hand_at_the_bottom_keeps_pick_order() {
    let (mut runner, sirius, _opp) = sirius_with_hand_and_trash_fuel();
    let t = trash_id(&runner, "VB-T");
    runner.execute_action(0, t).expect("trash card first");
    let h = hand_id(&runner, "VB-H");
    runner.execute_action(0, h).expect("then the hand card");
    runner.execute_branch(1).expect("bottom of digivolution sources");
    assert_eq!(
        stack(&runner, sirius),
        vec!["VB-H", "VB-T", "BASE", CARD_ID],
        "first pick (VB-T) above the second (VB-H), which is bottommost"
    );
}

/// One card only, then end the pick: "up to 2".
#[test]
fn ex12_018_one_card_then_end_the_pick() {
    let (mut runner, sirius, opp) = sirius_with_hand_and_trash_fuel();
    let t = trash_id(&runner, "VB-T");
    runner.execute_action(0, t).expect("trash card");
    let still = runner.pending_selection_view().expect("the pick stays open for a 2nd card");
    assert!(still.valid_action_ids.contains(&PASS));
    runner.execute_action(0, PASS).expect("end the pick at 1 card");
    runner.execute_branch(0).expect("top");
    assert_eq!(stack(&runner, sirius), vec!["BASE", "VB-T", CARD_ID]);
    select_first_non_pass(&mut runner);
    assert_eq!(runner.effective_dp(opp), Some(5000), "2 digivolution cards: -4000 DP");
}

/// Declining places nothing, so "If this effect placed" is false: no
/// top/bottom choice and no -DP.
#[test]
fn ex12_018_declining_places_nothing_and_gives_no_minus_dp() {
    let (mut runner, sirius, opp) = sirius_with_hand_and_trash_fuel();
    runner.execute_action(0, PASS).expect("place none");
    assert!(runner.pending_selection_view().is_none(), "{:?}", runner.pending_selection_view());
    assert_eq!(stack(&runner, sirius), vec!["BASE", CARD_ID]);
    assert_eq!(runner.effective_dp(opp), Some(9000));
}

/// Only a hand candidate: the pick offers it (no trash ids), and the single
/// placed source makes 2 digivolution cards.
#[test]
fn ex12_018_when_digivolving_places_source_then_debuffs_per_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-018 YAML loads")
        .add_card(vb_digimon("BASE", CardColor::Red, 5, 8000))
        .add_card(vb_text_digimon("VB-FUEL", CardColor::Yellow, 4, 5000))
        .add_card(plain_digimon("OPP", CardColor::Purple, 5, 9000))
        .hand(0, &["VB-FUEL"])
        .memory(10)
        .start();

    let sirius = runner.place_stack(0, &["BASE", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_when_digivolving(&mut runner, sirius);
    let pick = runner.pending_selection_view().expect("source-card pick");
    assert!(
        pick.valid_action_ids
            .iter()
            .all(|id| *id == PASS || (PLAY_HAND_START..PLAY_HAND_END).contains(id)),
        "no trash candidate: {:?}",
        pick.valid_action_ids
    );
    select_first_non_pass(&mut runner);
    runner.execute_branch(0).expect("place as top source");

    let dp_prompt = runner
        .pending_selection_view()
        .expect("opponent DP target prompt");
    assert_eq!(dp_prompt.kind, SelectionKind::OppField);
    select_first_non_pass(&mut runner);

    assert_eq!(
        runner.effective_dp(opp),
        Some(5000),
        "BASE plus the placed source means 2 sources, so the target gets -4000 DP"
    );
}

/// CR 4.3 13-1-7 / 4-6-1: a DUAL card checked from security is a Digimon card, so it is a Security Digimon and
/// battles the checking Digimon (13-1-8-3), then goes to the trash (13-1-8-4). Found by playing a sim game back in
/// DCGO, which battles it: our engine revealed Siriusmon and skipped the battle, so the 2000 DP attacker survived.
#[test]
fn ex12_018_checked_from_security_battles_as_a_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-018 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Blue, 3, 2000))
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        !super::support::field_contains(&runner, 0, "ATTACKER"),
        "2000 DP attacker loses the battle against the 12000 DP Security Digimon"
    );
    assert!(super::support::trash_contains(&runner, 0, "ATTACKER"));
    assert!(super::support::trash_contains(&runner, 1, CARD_ID), "the checked card is trashed (13-1-8-4)");
}

/// Option face (Q6749 names it "Planet Punch"): "[Main] Delete 1 of your
/// opponent's highest DP Digimon. Then, 1 of your Digimon may attack." plus
/// <Arts Digivolve>. The attack is declared inside the effect and the Option
/// FINISHES at that declaration: its own <Arts Digivolve> offer comes before
/// the attack's [When Attacking] effects, the counter timing and any security
/// check (DCGO: AttackProcess.cs:242 returns at the declaration,
/// CardController.cs:2026-2129 resolves the Arts or trashes the card before
/// TurnStateMachine.cs:1277-1286 runs the attack; official Q4962 / Q4346).
/// G-ENGINE-OPTION-BODY-ATTACK-NOT-HELD, resolved by reversal (owner default
/// "Generally let's look to match dcgo", applied here by the orchestrator).
#[test]
fn ex12_018_option_face_finishes_at_the_attack_declaration() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::selection::OptionPlayResult;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-018 YAML loads")
        .add_card(vb_digimon("BASE", CardColor::Red, 5, 8000))
        .add_card(plain_digimon("OPP", CardColor::Purple, 5, 9000))
        .add_card(make_test_card("SEC", "Sec"))
        .hand(0, &[CARD_ID])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(1, "OPP", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let attack_id = encode_attack(base.index as u16, SECURITY_TARGET);
    // Answer the deletion, "Attack", and the attacker pick until the attack
    // target prompt is up.
    for _ in 0..6 {
        let view = runner
            .pending_selection_view()
            .expect("the [Main] parks until its attack prompt");
        if view.kind == SelectionKind::Target {
            break;
        }
        let action = match view
            .effect_choices
            .as_ref()
            .filter(|_| view.kind == SelectionKind::EffectChoice)
        {
            Some(choices) => choices
                .iter()
                .find(|c| c.label == "Attack")
                .map(|c| c.action_id)
                .expect("an Attack choice"),
            None => view
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .expect("a legal pick"),
        };
        runner
            .execute_action(view.selecting_player, action)
            .expect("answer the [Main]");
    }
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Target));
    assert_eq!(runner.game.players[1].battle_area.len(), 0, "OPP deleted first");
    runner
        .execute_action(0, attack_id)
        .expect("BASE attacks the player");

    let arts = runner
        .pending_selection_view()
        .expect("the Arts offer follows the declaration");
    assert_eq!(arts.kind, SelectionKind::OwnField, "the Arts offer");
    assert!(arts.is_optional);
    assert!(
        arts.valid_action_ids
            .contains(&encode_attack(0, base.index as u16)),
        "the red Lv.5 attacker is an Arts host: {:?}",
        arts.valid_action_ids
    );
    assert!(runner.game.pending_attack.is_some(), "the attack is open");
    assert_eq!(runner.security_count(1), 3, "no check before the Option finished");
    runner.execute_action(0, PASS).expect("decline the Arts Digivolve");
    let sirius_in_trash = runner.game.players[0]
        .trash
        .iter()
        .filter(|c| c.card_id(&runner.game.card_data) == CARD_ID)
        .count();
    assert_eq!(sirius_in_trash, 1, "trashed at the declaration");
    assert!(runner.game.pending_option.is_none());

    for _ in 0..10 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        runner
            .execute_action(view.selecting_player, action)
            .expect("drive the attack");
    }
    assert!(runner.game.pending_attack.is_none(), "the attack ended");
    assert_eq!(runner.security_count(1), 2, "one security check");
}
