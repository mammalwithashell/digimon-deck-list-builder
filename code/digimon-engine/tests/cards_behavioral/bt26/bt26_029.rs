//! BT26-029 Aegiochusmon: Holy — Lv.5 Yellow/Black, Shaman/Iliad/TS/(Rule) Angel.
//!
//! <Decode ([Aegiomon])> <Ascension>
//! [On Play] [When Digivolving] By trashing your top security card, until your
//! opponent's turn ends, their effects can't reduce the DP of 1 of your
//! Digimon, trash any of its stacked cards, or return them to hands or decks.
//! [All Turns] [Once Per Turn] When your security stack is removed from, 3 of
//! your opponent's Digimon get -5000 DP for the turn.
//! Inherited: [All Turns] [Once Per Turn] When your security stack is removed
//! from, <De-Digivolve 1> 1 of your opponent's Digimon.
//!
//! DCGO: BT26/Yellow/BT26_029.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, EffectTiming, Expiry, ModifierType};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-029";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-029")
        .add_card(filler("FILLER"))
        .add_card(digimon("MINE", "Mine", CardColor::Yellow, 4, 5, &[]))
        .add_card(digimon("ROOK", "Rookie", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("CHAMP", "Champ", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 6, 9, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&x| x != digimon_engine::action::space::PASS)
                .collect()
        })
        .unwrap_or_default()
}

/// Run an engine effect as `player` (an opponent-effect probe).
fn as_player<R>(
    r: &mut DebugRunner,
    player: u8,
    src: PermanentHandle,
    f: impl FnOnce(&mut EffectContext) -> R,
) -> R {
    let card = r.top_card(src);
    r.game.set_effect_source_player_for_test(Some(player));
    let out = {
        let mut ctx = EffectContext::new(&mut r.game, card, Some(src), player);
        f(&mut ctx)
    };
    r.game.set_effect_source_player_for_test(None);
    out
}

#[test]
fn bt26_029_identity_and_keywords() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(format!("{:?}", card.traits).contains("Angel"));
    assert!(format!("{:?}", card.alt_paths).contains("Aegiomon"));
    assert!(format!("{:?}", card.effects).contains("Ascension"));
}

#[test]
fn bt26_029_on_play_is_optional_and_needs_security() {
    let mut r = setup();
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, ae);
    let v = r.pending_selection_view().expect("may-activate prompt");
    assert!(v.is_optional);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3, "declined ⇒ security untouched");
    assert!(!r.modifiers().has(ae, ModifierType::ImmuneFromDPMinus));
}

#[test]
fn bt26_029_on_play_trashes_security_and_protects_one_digimon() {
    let mut r = setup();
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    let mine = r.place_stack(0, &["ROOK", "MINE"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnPlay, ae);
    let accept = non_pass(&r)[0];
    r.execute_action(0, accept).expect("activate");
    // Resolve every prompt; the protection pick offers both of my Digimon.
    let mut saw_protect_pick = false;
    while let Some(v) = r.pending_selection_view() {
        let picks = non_pass(&r);
        if v.prompt.contains("can't reduce its DP") {
            assert_eq!(picks.len(), 2, "both of my Digimon are candidates");
            saw_protect_pick = true;
            r.execute_action(0, picks[1]).unwrap();
        } else {
            r.execute_action(v.selecting_player, picks[0]).unwrap();
        }
    }
    assert!(saw_protect_pick);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 2, "top security trashed as the cost");
    let protected: Vec<PermanentHandle> = [ae, mine]
        .into_iter()
        .filter(|h| r.modifiers().has(*h, ModifierType::ImmuneFromDPMinus))
        .collect();
    assert_eq!(protected.len(), 1, "exactly 1 of my Digimon protected");
    let mine = protected[0];
    let ae = if mine == ae {
        r.place_on_field(0, "MINE", Some(0))
    } else {
        ae
    };
    // Give the protected Digimon a stacked card to probe stack trashing.
    if sources(&r, mine) == 0 {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "ROOK")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[0].battle_area[mine.index as usize]
            .card_sources
            .insert(
                0,
                digimon_engine::card_source::CardSource::new(idx, 0, next),
            );
    }
    assert!(r
        .modifiers()
        .has(mine, ModifierType::ImmuneFromStackTrashing));
    assert!(r
        .modifiers()
        .has(mine, ModifierType::CannotBeReturnedToHand));
    assert!(r
        .modifiers()
        .has(mine, ModifierType::CannotBeReturnedToDeck));
    assert!(r.modifiers().has(mine, ModifierType::CannotBeDeDigivolved));

    // Opponent's effects: DP reduction ignored, stack trash blocked, bounce blocked.
    let base = r.effective_dp(mine).unwrap();
    as_player(&mut r, 1, opp, |ctx| {
        ctx.add_dp_modifier(mine, -3000, Expiry::EndOfTurn)
    });
    assert_eq!(r.effective_dp(mine).unwrap(), base, "opp DP-minus ignored");
    assert!(!as_player(&mut r, 1, opp, |ctx| ctx.trash_top_source(mine)));
    assert_eq!(sources(&r, mine), 1, "stacked card not trashed by opp");
    // Own effects are unaffected.
    as_player(&mut r, 0, ae, |ctx| {
        ctx.add_dp_modifier(mine, -1000, Expiry::EndOfTurn)
    });
    assert_eq!(
        r.effective_dp(mine).unwrap(),
        base - 1000,
        "own DP-minus applies"
    );
    assert!(as_player(&mut r, 0, ae, |ctx| ctx.trash_top_source(mine)));
    assert_eq!(
        sources(&r, mine),
        0,
        "own effect may trash its stacked card"
    );
    // Opponent's effects can't return it to the hand or deck.
    assert!(as_player(&mut r, 1, opp, |ctx| ctx.return_to_hand(mine)).is_none());
    assert!(!as_player(&mut r, 1, opp, |ctx| ctx.return_to_deck(
        mine,
        digimon_engine::enums::StackPosition::Bottom
    )));
    assert_eq!(r.game.players[0].battle_area.len(), 2, "still on the field");
}

#[test]
fn bt26_029_security_removed_debuffs_three_opponent_digimon_once_per_turn() {
    let mut r = setup();
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    let c = r.place_on_field(1, "OPP", Some(0));
    let d = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnOwnSecurityRemoved, ae);
    // Exactly 3 picks, mandatory.
    for _ in 0..3 {
        let v = r.pending_selection_view().expect("pick pending");
        assert!(!v.is_optional);
        pick_first(&mut r, 0);
    }
    let _ = r.auto_resolve();
    let n = [a, b, c, d]
        .iter()
        .filter(|h| r.effective_dp(**h).unwrap() == 1000)
        .count();
    assert_eq!(n, 3, "3 opponent Digimon at 6000-5000");
    // Once per turn.
    fire(&mut r, EffectTiming::OnOwnSecurityRemoved, ae);
    assert!(r.game.pending_selection.is_none(), "once per turn");
}

#[test]
fn bt26_029_inherited_de_digivolves_on_security_removed() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "MINE"]);
    let opp = r.place_stack(1, &["ROOK", "CHAMP"]);
    fire(&mut r, EffectTiming::OnOwnSecurityRemoved, host);
    while r.game.pending_selection.is_some() {
        pick_first(&mut r, 0);
    }
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["ROOK".to_string()], "De-Digivolve 1");
    let _ = opp;
}
