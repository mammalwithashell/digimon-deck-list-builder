//! Behavioral tests for `ctx.play_token` and token-aware
//! `delete_permanent` (remove-from-game instead of trash).
//!
//! Uses TEST-023, a synthetic test card whose OnPlay is
//! `ctx.play_token(player, "petrification")`, so we exercise the full
//! play -> registry -> EffectContext -> mutations path rather than
//! calling `play_token` from a test in isolation.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

/// Happy path: playing TEST-023 materializes a Petrification Token on
/// P0's field. The token reads as `CardKind::Token`, carries the
/// Petrification stats, and takes up a field slot.
#[test]
fn test_023_play_token_spawns_petrification() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .hand(0, &["TEST-023"])
        .memory(5) // pre-fund cost (TEST-023 uses default cost=3)
        .start();

    assert_eq!(r.battle_area_size(0), 0);
    r.play(0, 0);

    // After the play, the TEST-023 permanent itself is on the field AND
    // a Petrification Token sits next to it.
    assert_eq!(r.battle_area_size(0), 2, "test card + token");

    // Find the token by kind.
    let token_perm = r
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .expect("token missing from battle_area");
    assert_eq!(
        token_perm.top_card().card_name(&r.game.card_data),
        "Petrification Token"
    );
    assert_eq!(token_perm.base_dp(&r.game.card_data), Some(3000));
}

/// Removal-from-game contract: deleting a token empties the battle_area
/// slot WITHOUT growing P0's trash. Contrast with a normal Digimon, which
/// would land in trash.
#[test]
fn token_delete_removes_from_game_not_trash() {
    for (card_id, card_name) in [
        ("TEST-023", "PlayPetrificationToken"),
        ("TEST-029", "PlayFamiliarToken"),
    ] {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card(card_id, card_name))
            .hand(0, &[card_id])
            .memory(5)
            .start();
        r.play(0, 0);
        assert_eq!(r.trash_size(0), 0, "{card_name}");

        let token_field_idx = r
            .game
            .player(0)
            .battle_area
            .iter()
            .position(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
            .expect("token missing");
        r.game.players[0].delete_permanent(token_field_idx);

        // Battle area loses the token; trash does NOT gain it.
        assert_eq!(r.battle_area_size(0), 1, "only the test card remains");
        assert_eq!(r.trash_size(0), 0, "{card_name} removed from game");
    }
}

/// Petrification Token OnDeletion: when the token is deleted via the
/// full effect-firing path, the top card of the token-owner's security
/// stack goes to trash.
#[test]
fn petrification_on_deletion_trashes_top_security() {
    use digimon_engine::permanent::PermanentHandle;

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .add_card(make_test_card("SEC-A", "SecA"))
        .add_card(make_test_card("SEC-B", "SecB"))
        .add_card(make_test_card("SEC-C", "SecC"))
        .hand(0, &["TEST-023"])
        .security(0, &["SEC-A", "SEC-B", "SEC-C"])
        .memory(5)
        .start();
    r.play(0, 0);
    let sec_before = r.security_count(0);
    let trash_before = r.trash_size(0);

    // Locate the token on P0's field.
    let token_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .expect("token missing");

    // Use the full deletion path so OnDeletion observers fire.
    let handle = PermanentHandle {
        player: 0,
        index: token_idx as u8,
    };
    r.game.delete_permanent_with_effects(handle);

    assert_eq!(
        r.security_count(0),
        sec_before - 1,
        "Petrification OnDeletion trashed top of security"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the trashed security card landed in trash (token itself removed from game)"
    );
}

/// Atho/René/Por (Royal Knights Jesmon token) materializes through the
/// standard `play_token` pipeline with printed stats and keywords.
#[test]
fn test_030_play_token_spawns_atho_rene_por_with_printed_keywords() {
    use digimon_engine::enums::{CardColor, Keyword};

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-030", "PlayAthoReneAndPorToken"))
        .hand(0, &["TEST-030"])
        .memory(5)
        .start();

    assert_eq!(r.battle_area_size(0), 0);
    r.play(0, 0);

    assert_eq!(
        r.battle_area_size(0),
        2,
        "test card + Atho, René & Por token"
    );

    let token_perm = r
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .expect("token missing from battle_area");
    let card_data = &r.game.card_data;
    assert_eq!(
        token_perm.top_card().card_name(card_data),
        "Atho, René & Por"
    );
    assert_eq!(token_perm.base_dp(card_data), Some(6000));

    // Printed keywords must be visible to the unified `has_keyword` query.
    let token_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: r
            .game
            .player(0)
            .battle_area
            .iter()
            .position(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
            .unwrap() as u8,
    };
    assert!(r.game.has_keyword(token_handle, Keyword::Reboot));
    assert!(r.game.has_keyword(token_handle, Keyword::Blocker));
    let red_black = (1u8 << CardColor::Red as u8) | (1u8 << CardColor::Black as u8);
    assert!(r.game.has_keyword(token_handle, Keyword::Decoy(red_black)));
}

/// Hinukamuy (Royal Knights — BT23-057 Gankoomon token) materializes
/// through the standard `play_token` pipeline with its printed stats and
/// keywords: Digimon/White/6000 DP/<Alliance> <Reboot> <Blocker>.
#[test]
fn test_031_play_token_spawns_hinukamuy_with_printed_keywords() {
    use digimon_engine::enums::Keyword;

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-031", "PlayHinukamuyToken"))
        .hand(0, &["TEST-031"])
        .memory(5)
        .start();

    assert_eq!(r.battle_area_size(0), 0);
    r.play(0, 0);

    assert_eq!(r.battle_area_size(0), 2, "test card + Hinukamuy token");

    let token_perm = r
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .expect("token missing from battle_area");
    let card_data = &r.game.card_data;
    assert_eq!(
        token_perm.top_card().card_name(card_data),
        "Hinukamuy Token"
    );
    assert_eq!(token_perm.base_dp(card_data), Some(6000));

    // Printed keywords must be visible to the unified `has_keyword` query.
    let token_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: r
            .game
            .player(0)
            .battle_area
            .iter()
            .position(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
            .unwrap() as u8,
    };
    assert!(r.game.has_keyword(token_handle, Keyword::Alliance));
    assert!(r.game.has_keyword(token_handle, Keyword::Reboot));
    assert!(r.game.has_keyword(token_handle, Keyword::Blocker));
}

#[test]
fn ex12_tokens_materialize_with_printed_stats_and_keywords() {
    use digimon_engine::enums::{CardColor, Keyword};

    let mut r = DebugRunner::builder().start();

    let kotenken = r.place_on_field(0, "TOKEN_KOTENKEN", Some(0));
    let paishu = r.place_on_field(0, "TOKEN_PAISHU", Some(0));

    let kotenken_perm = &r.game.player(0).battle_area[kotenken.index as usize];
    assert_eq!(
        kotenken_perm.top_card().card_name(&r.game.card_data),
        "Kotenken Token"
    );
    assert_eq!(
        kotenken_perm.top_card().card_kind(&r.game.card_data),
        CardKind::Token
    );
    assert_eq!(kotenken_perm.base_dp(&r.game.card_data), Some(9000));
    assert_eq!(
        kotenken_perm.top_card().digimon_colors(&r.game.card_data),
        vec![CardColor::Black]
    );
    assert!(r.game.has_keyword(kotenken, Keyword::Blocker));

    let paishu_perm = &r.game.player(0).battle_area[paishu.index as usize];
    assert_eq!(
        paishu_perm.top_card().card_name(&r.game.card_data),
        "Paishu Token"
    );
    assert_eq!(
        paishu_perm.top_card().card_kind(&r.game.card_data),
        CardKind::Token
    );
    assert_eq!(paishu_perm.base_dp(&r.game.card_data), Some(6000));
    assert_eq!(
        paishu_perm.top_card().digimon_colors(&r.game.card_data),
        vec![CardColor::Yellow]
    );
    assert!(r.game.has_keyword(paishu, Keyword::Blocker));
    assert!(r.game.has_keyword(paishu, Keyword::Guard));
}

#[test]
fn paishu_token_guard_protects_another_digimon() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ALLY", "Ally"))
        .start();

    r.place_on_field(0, "TOKEN_PAISHU", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));

    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_some(),
        "Paishu Token's carried Guard keyword should offer the replacement"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Paishu Guard replacement");

    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ALLY",
        "Paishu should leave and the protected Digimon should remain"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .all(|card| card.card_id(&r.game.card_data) != "TOKEN_PAISHU"),
        "tokens are removed from game rather than trashed when paid as Guard cost"
    );
    assert_eq!(
        r.game.effective_dp(PermanentHandle {
            player: 0,
            index: 0
        }),
        Some(2000)
    );
}

#[test]
fn familiar_on_deletion_prompts_opponent_target_and_applies_minus_3000() {
    use digimon_engine::action::build_action_mask;
    use digimon_engine::action::space::{encode_attack, PASS};
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::selection::SelectionKind;

    let mut opp_low = make_test_card("OPP-LOW", "OppLow");
    opp_low.dp = Some(4000);
    let mut opp_high = make_test_card("OPP-HIGH", "OppHigh");
    opp_high.dp = Some(7000);

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-029", "PlayFamiliarToken"))
        .add_card(opp_low)
        .add_card(opp_high)
        .hand(0, &["TEST-029"])
        .memory(5)
        .start();

    r.place_on_field(1, "OPP-LOW", Some(0));
    r.place_on_field(1, "OPP-HIGH", Some(0));
    r.play(0, 0).expect("TEST-029 plays");

    let token_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_name(&r.game.card_data) == "Familiar Token")
        .expect("Familiar Token missing");

    r.game.delete_permanent_with_effects(PermanentHandle {
        player: 0,
        index: token_idx as u8,
    });

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Familiar OnDeletion must ask for an opponent Digimon");
    assert_eq!(pending.kind, SelectionKind::OppField);
    assert_eq!(pending.selecting_player, 0);
    assert_eq!(pending.valid_action_ids.len(), 2);
    assert!(!pending.valid_action_ids.contains(&PASS));

    let target_action = encode_attack(0, 1);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[target_action as usize], 1.0);

    r.game.decode_action(target_action, 0);

    let target = PermanentHandle {
        player: 1,
        index: 1,
    };
    assert_eq!(
        r.game.effective_dp(target),
        Some(4000),
        "OPP-HIGH should be 7000 DP with a -3000 DP until-turn-end modifier"
    );
}

#[test]
fn familiar_on_deletion_target_prompt_clones_faithfully() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::selection::SelectionKind;

    let mut opp = make_test_card("OPP", "Opponent");
    opp.dp = Some(7000);

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-029", "PlayFamiliarToken"))
        .add_card(opp)
        .hand(0, &["TEST-029"])
        .memory(5)
        .start();

    r.place_on_field(1, "OPP", Some(0));
    r.play(0, 0).expect("TEST-029 plays");

    let token_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_name(&r.game.card_data) == "Familiar Token")
        .expect("Familiar Token missing");

    r.game.delete_permanent_with_effects(PermanentHandle {
        player: 0,
        index: token_idx as u8,
    });

    let action = encode_attack(0, 0);
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Familiar OnDeletion must ask for an opponent Digimon");
    assert_eq!(pending.kind, SelectionKind::OppField);
    assert!(pending.valid_action_ids.contains(&action));
    assert!(
        r.game.pending_selection_resume.is_some(),
        "Familiar token target prompt must be resume-driven before cloning"
    );

    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, action)
        .expect("clone resolves Familiar target prompt");
    assert_eq!(
        clone.effective_dp(PermanentHandle {
            player: 1,
            index: 0
        }),
        Some(4000),
        "clone should apply Familiar's -3000 DP"
    );

    assert!(
        r.game.pending_selection.is_some(),
        "resolving the clone must leave the original at the Familiar prompt"
    );
    assert_eq!(
        r.game.effective_dp(PermanentHandle {
            player: 1,
            index: 0
        }),
        Some(7000),
        "resolving the clone must not mutate the original"
    );

    r.game
        .resolve_selection(0, action)
        .expect("original resolves Familiar target prompt");
    assert_eq!(
        r.game.effective_dp(PermanentHandle {
            player: 1,
            index: 0
        }),
        clone.effective_dp(PermanentHandle {
            player: 1,
            index: 0
        }),
        "original and clone replay the Familiar target prompt identically"
    );
}

/// Rotate the active player to `target`, flushing the EndOfTurnAction step.
fn petrification_advance_to_turn_player(r: &mut DebugRunner, target: u8) {
    for _ in 0..8 {
        if r.turn_player() == target {
            break;
        }
        let _ = r.auto_resolve();
        r.pass_turn();
        let _ = r.auto_resolve();
        if r.game.current_phase == digimon_engine::enums::GamePhase::EndOfTurnAction {
            r.game.pass_end_of_turn_action();
            let _ = r.auto_resolve();
        }
    }
    assert_eq!(r.turn_player(), target, "failed to rotate turn player");
    assert_eq!(
        r.current_phase(),
        digimon_engine::enums::GamePhase::Main,
        "rotation must land in the Main phase"
    );
}

/// Petrification Token (TOKEN_PETRIFICATION), printed on Medusamon (BT21-029),
/// Medusamon (EX11-012) and Medusamon (BT24-017):
/// "(Digimon/White/3000 DP/[Your Turn] This Digimon can't suspend.
///   [On Deletion] Trash your top security card.)"
///
/// "[Your Turn]" is the token CONTROLLER's turn (the opponent of the Medusamon
/// that made them play it). DCGO `BT21_029_token.cs` region "Cant Suspend":
/// `CantSuspendStaticEffect` gated on `IsExistOnBattleArea && IsOwnerTurn`.
/// CR 4.3 11-2-5: "An attack declaration can't be made using a Digimon that
/// can't suspend."  The gate is a continuous effect, so it must hold on EVERY
/// one of the controller's turns and lift on every opponent turn.
#[test]
fn petrification_token_cannot_suspend_or_attack_on_each_of_its_controllers_turns() {
    use digimon_engine::action::mask::build_action_mask;
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::enums::ModifierType;
    use digimon_engine::permanent::PermanentHandle;

    let filler = ["FILL"; 8];
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["TEST-023"])
        .deck(0, &filler)
        .deck(1, &filler)
        .security(0, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    r.play(0, 0);
    let token_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .expect("token missing");
    let host_idx = 1 - token_idx;
    let token = PermanentHandle {
        player: 0,
        index: token_idx as u8,
    };
    let host = PermanentHandle {
        player: 0,
        index: host_idx as u8,
    };
    let token_attack = encode_attack(token_idx as u16, SECURITY_TARGET) as usize;
    let host_attack = encode_attack(host_idx as u16, SECURITY_TARGET) as usize;
    let cant_suspend =
        |r: &DebugRunner| r.game.modifiers.has(token, ModifierType::CannotSuspend);
    let token_suspended =
        |r: &DebugRunner| r.game.player(0).battle_area[token_idx].is_suspended;

    // Controller's turn (the turn it was played): the prohibition is live, so
    // an effect-driven suspension silently no-ops (CR 4.3 15-1-3).
    r.game.tick_declarative_effects();
    assert!(cant_suspend(&r), "[Your Turn] can't suspend on the play turn");
    r.game.suspend(token);
    assert!(!token_suspended(&r), "a can't-suspend token is not suspended");

    for round in 0..2 {
        // Opponent's turn: "[Your Turn]" is off, the token can be suspended.
        petrification_advance_to_turn_player(&mut r, 1);
        assert!(
            !cant_suspend(&r),
            "round {round}: the gate lifts on the opponent's turn"
        );
        r.game.suspend(token);
        assert!(
            token_suspended(&r),
            "round {round}: the token can be suspended on the opponent's turn"
        );

        // Controller's next turn: unsuspended by the unsuspend phase, no
        // summoning sickness -- and still unable to attack (11-2-5).
        petrification_advance_to_turn_player(&mut r, 0);
        assert!(!token_suspended(&r), "round {round}: unsuspend phase");
        assert!(
            cant_suspend(&r),
            "round {round}: the gate is back on the controller's turn"
        );
        assert!(
            !r.game.can_attack(token, false),
            "round {round}: 11-2-5 -- a Digimon that can't suspend can't attack"
        );
        let mask = build_action_mask(&r.game, 0);
        assert!(
            mask[token_attack] < 0.5,
            "round {round}: the mask must not offer the token as an attacker"
        );
        // Control: the ordinary Digimon beside it CAN attack, so the token's
        // refusal is the printed clause and not turn state.
        assert!(r.game.can_attack(host, false), "round {round}: control");
        assert!(mask[host_attack] > 0.5, "round {round}: control in the mask");
        r.game.suspend(token);
        assert!(
            !token_suspended(&r),
            "round {round}: effects can't suspend it on its controller's turn"
        );
    }
}
