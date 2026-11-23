//! A Digimon that survives a lost security battle keeps checking, even when
//! its top card changed before the security pipeline resumed.
//!
//! CR 4.3 13-1-5: only a Digimon that "is no longer present in the battle
//! area" stops performing security checks. CR 4.3 8-1-2-3: digivolving keeps
//! it the same Digimon (it carries over its state), and de-digivolving only
//! trashes cards from its stack. CR 4.3 13-1-2 / 13-1-8-5: the modified
//! number of checks is made, and a card that can perform another check
//! performs it. CR 4.3 11-6-2 / 6-1-4-1: the attack, and so the turn, do not
//! end until all of that processing has resolved. DCGO agrees:
//! `ISecurityCheck.StopSecurityCheck` (CardController.cs) stops only when the
//! attacking `Permanent` is gone, and the loop re-reads its `Strike`.
//!
//! The engine used to decide "is the attacker still there" after the battle
//! by looking for the top card it had BEFORE the battle, so an attacker that
//! was saved by a replacement and then changed its top card read as deleted
//! and its remaining checks were dropped (bug-scan finding F03: Chronomon:
//! Holy Mode (BT26-016) saved by its own would-leave replacement, then
//! digivolved by Yokomon (BT26-001) into Chronomon: Destroy Mode (BT26-060)).

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{Expiry, Keyword};
use digimon_engine::permanent::PermanentHandle;

/// A 2000 DP Digimon: loses every security battle against the attackers here.
const FILL: &str = "FILL";
/// A 13000 DP Security Digimon: beats a 12000 DP Chronomon: Holy Mode
/// (BT26-016), as MagnaGarurumon (BT18-042) did in the logged game.
const BIG: &str = "BIG";

fn card(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c
}

fn field_top_id(r: &DebugRunner, player: u8, index: usize) -> String {
    let perm = &r.game.players[player as usize].battle_area[index];
    perm.top_card().card_id(&r.game.card_data).to_string()
}

/// Answer every prompt with its first non-PASS action (accept / pick the
/// only candidate), recording each prompt's text.
fn accept_all(r: &mut DebugRunner) -> Vec<String> {
    let mut prompts = Vec::new();
    for _ in 0..64 {
        let Some(view) = r.pending_selection_view() else {
            return prompts;
        };
        prompts.push(format!("p{} {:?}: {}", view.selecting_player, view.kind, view.prompt));
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        r.execute_action(view.selecting_player, action)
            .unwrap_or_else(|e| panic!("action {action} rejected: {e:?}; prompts {prompts:?}"));
    }
    panic!("prompt loop did not settle: {prompts:?}");
}

/// The logged shape: Chronomon: Holy Mode (BT26-016) over Yokomon (BT26-001)
/// attacks with Chronomon: Destroy Mode (BT26-060) in hand. The first check
/// is a 13000 DP Security Digimon; the rest are 2000 DP.
fn chronomon_runner(memory: i16) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card("BT26-001")
        .expect("Yokomon (BT26-001)")
        .dsl_card("BT26-016")
        .expect("Chronomon: Holy Mode (BT26-016)")
        .dsl_card("BT26-060")
        .expect("Chronomon: Destroy Mode (BT26-060)")
        .add_card(card(FILL, 2000))
        .add_card(card(BIG, 13000))
        .hand(0, &["BT26-060"])
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        // The last card is the top of the stack.
        .security(1, &[FILL, FILL, FILL, BIG])
        .memory(memory)
        .start();
    r.set_first_player(0);
    let attacker = r.place_stack(0, &["BT26-001", "BT26-016"]);
    (r, attacker)
}

#[test]
fn chronomon_saved_then_digivolved_mid_check_keeps_checking() {
    let (mut r, attacker) = chronomon_runner(10);
    let _ = r.attack_player(attacker, 1, false);
    let prompts = accept_all(&mut r);

    assert_eq!(
        field_top_id(&r, 0, 0),
        "BT26-060",
        "Holy Mode's replacement kept it on the field and Yokomon digivolved it \
         into Destroy Mode; prompts {prompts:?}"
    );
    // 1 base check + Destroy Mode's printed <Security A. +1> = 2 checks
    // (CR 4.3 13-1-2, 13-1-8-5): the 13000 DP card, then one 2000 DP card.
    assert_eq!(
        r.security_count(1),
        2,
        "the surviving attacker makes its remaining check; prompts {prompts:?}"
    );
    assert_eq!(r.security_count(0), 2, "the replacement returned 1 own security card");
    assert_eq!(r.turn_player(), 0, "memory stayed on this side: the turn continues");
    assert!(r.game.pending_attack.is_none(), "the attack has ended");
}

/// The logged game exactly: the digivolve takes memory from 1 to -3 while the
/// Digimon is still attacking. The attack finishes its checks first and only
/// then does the turn pass (CR 4.3 11-6-2, 6-1-4-1; official Q602).
#[test]
fn chronomon_digivolve_to_negative_memory_finishes_checks_before_turn_ends() {
    let (mut r, attacker) = chronomon_runner(1);
    let _ = r.attack_player(attacker, 1, false);
    let prompts = accept_all(&mut r);

    assert_eq!(field_top_id(&r, 0, 0), "BT26-060", "prompts {prompts:?}");
    assert_eq!(
        r.security_count(1),
        2,
        "both checks are made before the turn ends; prompts {prompts:?}"
    );
    assert_eq!(
        r.turn_player(),
        1,
        "memory is on the opponent's side once the attack has ended"
    );
}

/// An attacker that lost the battle and was saved by de-digivolving instead
/// of leaving: same Digimon, new top card, strike re-read from that card.
const DEDIGI_YAML: &str = r#"
card: TEST-SAVE-DEDIGI
name: "Save De-Digivolve"
kind: digimon
level: 6
color: [red]
cost: 6
dp: 5000
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    summary: "When this Digimon would leave the battle area, by <De-Digivolve 1> on it, it doesn't leave"
    process:
      - de_digivolve: { target: source, amount: 1 }
      - cancel_replacement: {}
"#;

#[test]
fn attacker_saved_by_de_digivolving_mid_check_keeps_checking() {
    let mut under = card("UNDER", 9000);
    under.level = Some(5);
    under.keywords = vec![Keyword::SecurityAttackPlus(1)];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DEDIGI_YAML)
        .unwrap()
        .add_card(card(FILL, 2000))
        .add_card(card(BIG, 13000))
        .add_card(under)
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL, FILL, FILL, BIG])
        .memory(10)
        .start();
    r.set_first_player(0);
    let attacker = r.place_stack(0, &["UNDER", "TEST-SAVE-DEDIGI"]);
    let _ = r.attack_player(attacker, 1, false);
    let prompts = accept_all(&mut r);

    assert_eq!(
        field_top_id(&r, 0, 0),
        "UNDER",
        "the replacement de-digivolved it instead of letting it leave; prompts {prompts:?}"
    );
    // Its new top card prints <Security A. +1>: 2 checks in all.
    assert_eq!(
        r.security_count(1),
        2,
        "the de-digivolved attacker is still the attacker and keeps checking; \
         prompts {prompts:?}"
    );
}

/// The real-leave neighbour: an attacker with checks left that loses the
/// battle and has nothing to save it stops checking (CR 4.3 13-1-5).
#[test]
fn attacker_deleted_by_security_battle_stops_checking() {
    let mut atk = card("ATK", 5000);
    atk.keywords = vec![Keyword::SecurityAttackPlus(2)];
    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(card(FILL, 2000))
        .add_card(card(BIG, 13000))
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL, FILL, FILL, BIG])
        .memory(10)
        .start();
    r.set_first_player(0);
    // A second Digimon that would slide into the attacker's slot.
    let attacker = r.place_on_field(0, "ATK", Some(0));
    r.place_on_field(0, FILL, Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);
    let result = r.attack_player(attacker, 1, false);
    let prompts = accept_all(&mut r);

    assert_eq!(result, AttackResult::AttackerDeletedBySecurity, "prompts {prompts:?}");
    assert_eq!(r.game.players[0].battle_area.len(), 1, "the attacker left");
    assert_eq!(field_top_id(&r, 0, 0), FILL);
    assert_eq!(r.security_count(1), 3, "only the first check was made");
}

// ─── The attacker digivolves into a NEW Digimon mid-check ────────────────────
//
// The checks follow the attacker's stack identity
// (`Permanent::battle_identity`). A DNA digivolution builds a new stack, so
// the identity is gone. That is what the rules say: CR 4.3 8-2-1 (the
// stacked cards digivolve "into 1 new Digimon"), 8-2-2-1-3 ("If a Digimon was
// attacking before a digivolution, the attacking Digimon is considered to be
// no longer present") and 13-1-5 (a Digimon no longer present in the battle
// area makes no more security checks). DCGO agrees: the DNA branch of
// `PlayCardClass.PlayCard` (CardController.cs) removes both material
// Permanents and creates a new one, so `ISecurityCheck.StopSecurityCheck`
// sees the old `AttackingPermanent` without a top card and stops.
//
// App fusion is different: CR 4.3 8-4 has no "new Digimon" or "no longer
// present" rule, the app-fused stack keeps its bottom cards (3-1-3-1-3), and
// DCGO runs it through the ordinary digivolve onto the same Permanent. The
// attacker keeps checking.

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::make_test_dna_card;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::EffectTiming;
use digimon_engine::game_actions::AppFuseSourceZone;

fn hand_card(r: &digimon_engine::game::Game, id: &str) -> Option<CardHandle> {
    r.players[0]
        .hand
        .iter()
        .find(|c| c.card_id(&r.card_data) == id)
        .map(|c| c.handle())
}

/// "When this Digimon checks security, DNA digivolve it with `partner` into
/// RESULT from hand" (a test fixture: no pool card prints it).
struct DnaOnCheck {
    attacker: PermanentHandle,
    partner: PermanentHandle,
}
impl CardEffect for DnaOnCheck {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let (attacker, partner) = (self.attacker, self.partner);
        vec![Effect::on_play(card)
            .name("DNA digivolve the attacker as it checks security")
            .timing(EffectTiming::OnSecurityCheck)
            .process(move |ctx| {
                let Some(result) = hand_card(ctx.game, "RESULT") else {
                    return;
                };
                ctx.effect_initiated_dna_digivolve(attacker, partner, result, 0, false)
                    .expect("the DNA digivolution commits");
            })
            .build()]
    }
}

#[test]
fn attacker_dna_digivolved_mid_check_stops_checking() {
    let mut atk = card("ATK", 9000);
    atk.level = Some(6);
    atk.keywords = vec![Keyword::SecurityAttackPlus(2)];
    let mut partner = card("PARTNER", 5000);
    partner.level = Some(5);
    let mut result = make_test_dna_card("RESULT", "Result", 6, 5, 0);
    result.level = Some(7);
    result.dp = Some(14000);
    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(partner)
        .add_card(result)
        .add_card(card(FILL, 2000))
        .hand(0, &["RESULT"])
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL; 4])
        .memory(10)
        .start();
    r.set_first_player(0);
    let attacker = r.place_on_field(0, "ATK", Some(0));
    let partner = r.place_on_field(0, "PARTNER", Some(1));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);
    r.register_effect("ATK", Arc::new(DnaOnCheck { attacker, partner }));
    let _ = r.attack_player(attacker, 1, false);
    let prompts = accept_all(&mut r);

    let tops: Vec<String> = (0..r.game.players[0].battle_area.len())
        .map(|i| field_top_id(&r, 0, i))
        .collect();
    assert_eq!(tops, vec!["RESULT".to_string()], "the DNA digivolution happened; prompts {prompts:?}");
    assert_eq!(
        r.security_count(1),
        3,
        "the attacker is no longer present after the first check (CR 4.3 8-2-2-1-3, \
         13-1-5): its two remaining checks are not made; prompts {prompts:?}"
    );
    assert!(r.game.pending_attack.is_none(), "the attack has ended");
    assert!(
        !r.game.players[0].battle_area[0].is_attacking,
        "the DNA result is not attacking"
    );
}

const APP_FUSION_YAML: &str = r#"
card: TEST-APPFUSE
name: Test App Fusion Digimon
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 9000
traits: [Appmon]
alt_paths:
  - kind: app_fusion
    materials:
      - filter:
          name_in: [Kabemon, Gomimon]
    cost: 0
"#;

/// "When this Digimon checks security, app fuse it into TEST-APPFUSE from
/// hand" (a test fixture).
struct AppFuseOnCheck {
    host: PermanentHandle,
}
impl CardEffect for AppFuseOnCheck {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let host = self.host;
        vec![Effect::on_play(card)
            .name("App fuse the attacker as it checks security")
            .timing(EffectTiming::OnSecurityCheck)
            .process(move |ctx| {
                let Some(result) = hand_card(ctx.game, "TEST-APPFUSE") else {
                    return;
                };
                assert!(
                    ctx.game
                        .commit_effect_app_fuse(0, host, result, AppFuseSourceZone::Hand),
                    "the app fusion commits"
                );
            })
            .build()]
    }
}

#[test]
fn attacker_app_fused_mid_check_keeps_checking() {
    let mut kabe = make_test_card("HOST-KABE", "Kabemon");
    kabe.level = Some(4);
    kabe.dp = Some(9000);
    let mut gomi = make_test_card("HOST-GOMI", "Gomimon");
    gomi.level = Some(4);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(APP_FUSION_YAML)
        .expect("app fusion card")
        .add_card(kabe)
        .add_card(gomi)
        .add_card(card(FILL, 2000))
        .hand(0, &["TEST-APPFUSE"])
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL; 4])
        .memory(10)
        .start();
    r.set_first_player(0);
    let host = r.place_on_field(0, "HOST-KABE", Some(0));
    r.push_linked_owned(host, "HOST-GOMI", 0);
    r.game
        .modifiers
        .grant_keyword(host, Keyword::Rush, Expiry::EndOfTurn, 0);
    r.game.modifiers.grant_keyword(
        host,
        Keyword::SecurityAttackPlus(2),
        Expiry::EndOfTurn,
        0,
    );
    r.register_effect("HOST-KABE", Arc::new(AppFuseOnCheck { host }));
    let _ = r.attack_player(host, 1, false);
    let prompts = accept_all(&mut r);

    assert_eq!(
        field_top_id(&r, 0, 0),
        "TEST-APPFUSE",
        "the app fusion happened; prompts {prompts:?}"
    );
    assert_eq!(
        r.security_count(1),
        1,
        "the app-fused Digimon is the same attacker and makes all 3 checks; \
         prompts {prompts:?}"
    );
    assert!(r.game.pending_attack.is_none(), "the attack has ended");
}
