//! A DNA digivolution result is a NEW card — Comprehensive Rules §8-2-2-1.
//!
//! Rule text (Ver. 4.3 numbering; the base-repo PDF Ver. 3.6 pp.16-17 carries
//! the same clauses, with the [X Per Turn] reset folded into -1-5 and the
//! "played this turn" clause numbered -1-6):
//!
//! - §8-2-2-1   DNA digivolution differs from standard digivolution in that
//!              the cards that become digivolution cards are considered
//!              different Digimon; they don't carry over their previous states.
//! - §8-2-2-1-1 The card placed on top digivolves unsuspended without carrying
//!              over the display format from before the digivolution. This
//!              isn't considered unsuspending a Digimon.
//! - §8-2-2-1-2 Cards that become digivolution cards are considered new cards.
//!              If a card that would become a digivolution card has a link
//!              card, the link card is trashed immediately before placing the
//!              card as a digivolution card.
//! - §8-2-2-1-5 Any effects that a digivolution card gained will end.
//! - §8-2-2-1-6 [X Per Turn] uses on the cards that became digivolution cards
//!              are reset.
//! - §8-2-2-1-7 A card that became a digivolution card can attack in the same
//!              turn it was played.
//!
//! Contrast §8-1-2-4: an ORDINARY digivolution carries the display format
//! over (a suspended base stays suspended). DCGO builds the jogress result as
//! a fresh permanent (`Effects.CreateFieldPermanentCardEffect(..,
//! jogressEvoRoots)`), so it is unsuspended by construction.
//!
//! Every scenario here is driven through the USER-ACTION path: the DNA is
//! declared with the `DNA_DIGIVOLVE_START + hand_index` action and both
//! material picks go through `resolve_selection`; suspension comes from real
//! attacks, the OPT use from a real inherited `[When Attacking]` trigger, the
//! DP gain from a real `[Main]` activation, and the link from the engine's
//! link-into-host API. `tests/dna_new_object_state.rs` is the companion that
//! pokes the same fields directly across all three DNA entry points
//! (field pair / hand partner / trash partner).

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, DNA_DIGIVOLVE_START, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN,
    FIELD_EFFECT_START, SECURITY_TARGET,
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, make_test_dna_card, DebugRunner};
use digimon_engine::enums::{GamePhase, LinkCardSource, ModifierType};
use digimon_engine::permanent::PermanentHandle;

/// Base material A. Its `[Main]` grants itself +3000 DP for the turn — the
/// "gained effect" that §8-2-2-1-5 says must end when A becomes a
/// digivolution card.
const BASE_A: &str = r#"
card: DNA-A
name: Dna Base A
kind: digimon
level: 6
color: [red]
cost: 12
dp: 11000
effects:
  - when: main_on_field
    summary: "[Main] This Digimon gets +3000 DP for the turn"
    process:
      - add_dp_modifier:
          target: source
          value: 3000
          expiry: end_of_turn
"#;

/// A digivolution card stacked under A. Its inherited `[When Attacking]
/// [Once Per Turn]` is the per-turn use that §8-2-2-1-6 resets when the card
/// moves under the DNA result.
const LOWER: &str = r#"
card: DNA-LOWER
name: Dna Lower Source
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
effects:
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    summary: "[When Attacking] [Once Per Turn] Gain 1 memory"
    process:
      - gain_memory: 1
"#;

const RESULT_DP: i32 = 15000;

fn runner() -> DebugRunner {
    let mut b = make_test_card("DNA-B", "Dna Partner B");
    b.level = Some(6);
    b.dp = Some(11000);
    let mut result = make_test_dna_card("DNA-RESULT", "Dna Result", 6, 6, 0);
    result.level = Some(7);
    result.dp = Some(RESULT_DP);
    let mut filler = make_test_card("FILLER", "Filler");
    filler.dp = Some(1000);
    // Only the §8-1-2-4 control digivolves into this; it starts outside hand.
    let mut ordinary = make_test_card("ORDINARY", "Ordinary Lv7");
    ordinary.level = Some(7);
    ordinary.evo_costs = vec![EvoCost { card_color: 0, level: 6, memory_cost: 0 }];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(BASE_A)
        .expect("DNA-A compiles")
        .from_dsl_yaml(LOWER)
        .expect("DNA-LOWER compiles")
        .add_card(b)
        .add_card(result)
        .add_card(filler)
        .add_card(make_test_card("LINK", "Link Card"))
        .link_requirement("LINK", 0) // [Link] (Q4533; §17-1-3-2-6 / -7)
        .add_card(ordinary)
        .hand(0, &["DNA-RESULT", "LINK", "LINK"])
        // The opponent needs security so attacks on the player never end the
        // game, and both decks need cards for the DNA draw / end-of-turn draws.
        .security(1, &["FILLER"; 4])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(r.game.current_phase, GamePhase::Main);
    r
}

fn hand_index(r: &DebugRunner, card_id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in hand"))
}

fn attack_action(perm: PermanentHandle) -> usize {
    encode_attack(perm.index as u16, SECURITY_TARGET) as usize
}

fn main_action(perm: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

/// Attack the opponent's security with `attacker` and settle every trigger.
fn attack(r: &mut DebugRunner, attacker: PermanentHandle) {
    let result = r.attack_player(attacker, 1, false);
    assert!(
        !matches!(result, AttackResult::Invalid),
        "attack declaration must be accepted"
    );
    r.game.drain_effect_queue();
    while r.game.pending_selection.is_some() {
        r.auto_resolve().expect("attack follow-up prompts resolve");
    }
}

/// Declare the DNA with the user action and pick `first` then `second` as
/// materials. Returns the merged permanent's handle.
fn dna(r: &mut DebugRunner, first: PermanentHandle, second: PermanentHandle) -> PermanentHandle {
    let declaration = DNA_DIGIVOLVE_START + hand_index(r, "DNA-RESULT") as u16;
    assert_eq!(
        build_action_mask(&r.game, 0)[declaration as usize],
        1.0,
        "DNA declaration must be exposed to the agent"
    );
    r.game.decode_action(declaration, 0);
    for pick in [first, second] {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("DNA material prompt");
        assert!(
            pending.valid_action_ids.contains(&(pick.index as u16)),
            "material {pick:?} must be offered"
        );
        r.game.resolve_selection(0, pick.index as u16).expect("material pick");
    }
    assert_eq!(r.game.current_phase, GamePhase::Main);
    assert!(r.game.pending_selection.is_none());
    let merged_index = if second.index < first.index {
        first.index - 1
    } else {
        first.index
    };
    let merged = PermanentHandle { player: 0, index: merged_index };
    let perm = &r.game.players[0].battle_area[merged.index as usize];
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "DNA-RESULT");
    merged
}

fn assert_ready_to_attack(r: &mut DebugRunner, merged: PermanentHandle) {
    let perm = &r.game.players[0].battle_area[merged.index as usize];
    assert!(!perm.is_suspended, "§8-2-2-1-1: the DNA result is unsuspended");
    assert!(
        r.game.can_attack(merged, false),
        "§8-2-2-1-1 / §8-2-2-1-7: the DNA result can attack this turn"
    );
    assert_eq!(
        build_action_mask(&r.game, 0)[attack_action(merged)],
        1.0,
        "the attack must be exposed to the agent, not just to the engine API"
    );
    // Prove it by doing it: a declaration the engine accepts.
    attack(r, merged);
    assert!(r.game.players[0].battle_area[merged.index as usize].is_suspended);
}

/// (a) §8-2-2-1-1 — a suspended base A plus an unsuspended partner B: the
/// result digivolves unsuspended and can attack this turn.
#[test]
fn suspended_base_dna_result_is_unsuspended_and_can_attack() {
    let mut r = runner();
    let a = r.place_on_field(0, "DNA-A", Some(0));
    let b = r.place_on_field(0, "DNA-B", Some(0));
    attack(&mut r, a);
    assert!(r.game.players[0].battle_area[a.index as usize].is_suspended);
    assert!(!r.game.players[0].battle_area[b.index as usize].is_suspended);

    let merged = dna(&mut r, a, b);
    assert_ready_to_attack(&mut r, merged);
}

/// (b) §8-2-2-1-1 — an unsuspended base A plus a suspended partner B: the
/// result is unsuspended regardless of which material was suspended.
///
/// The merged permanent occupies the first pick's slot, so this scenario
/// passed even before the reset existed (B's orientation never reached the
/// merge); the pick-order companion below is the one that discriminates.
#[test]
fn suspended_partner_dna_result_is_unsuspended_and_can_attack() {
    let mut r = runner();
    let a = r.place_on_field(0, "DNA-A", Some(0));
    let b = r.place_on_field(0, "DNA-B", Some(0));
    attack(&mut r, b);
    assert!(!r.game.players[0].battle_area[a.index as usize].is_suspended);
    assert!(r.game.players[0].battle_area[b.index as usize].is_suspended);

    let merged = dna(&mut r, a, b);
    assert_ready_to_attack(&mut r, merged);
}

/// Picking the partner first (B then A) must give the same new-card result —
/// the carrier slot changes but §8-2-2-1-1 does not depend on pick order.
#[test]
fn suspended_materials_picked_partner_first_still_yield_unsuspended_result() {
    let mut r = runner();
    let a = r.place_on_field(0, "DNA-A", Some(0));
    let b = r.place_on_field(0, "DNA-B", Some(0));
    attack(&mut r, a);
    attack(&mut r, b);

    let merged = dna(&mut r, b, a);
    assert_ready_to_attack(&mut r, merged);
}

/// (c) §8-2-2-1-6 — an inherited `[Once Per Turn]` already used this turn
/// while the card sat under A fires again once the card sits under the DNA
/// result: the result is a different Digimon, so its per-turn uses start fresh.
#[test]
fn once_per_turn_use_on_material_stack_resets_on_dna_result() {
    let mut r = runner();
    let a = r.place_stack(0, &["DNA-LOWER", "DNA-A"]);
    let b = r.place_on_field(0, "DNA-B", Some(0));

    // First attack consumes the OPT: +1 memory.
    let before = r.game.memory;
    attack(&mut r, a);
    assert_eq!(r.game.memory, before + 1, "inherited [When Attacking] fires once");

    // Control: the SAME Digimon attacking again this turn is locked out.
    r.game.players[0].battle_area[a.index as usize].is_suspended = false;
    attack(&mut r, a);
    assert_eq!(r.game.memory, before + 1, "[Once Per Turn] locks the second use");

    // Isolate §8-2-2-1-6 from §8-2-2-1-1: hand-unsuspend A so the result's
    // attack below cannot be refused for orientation reasons; only the
    // per-turn use count is under test here.
    r.game.players[0].battle_area[a.index as usize].is_suspended = false;
    let merged = dna(&mut r, a, b);
    let lower_is_under_result = r.game.players[0].battle_area[merged.index as usize]
        .card_sources
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "DNA-LOWER");
    assert!(lower_is_under_result, "DNA-LOWER is now a digivolution card of the result");

    // New Digimon: the use count is reset, so attacking fires the OPT again.
    let before = r.game.memory;
    attack(&mut r, merged);
    assert_eq!(
        r.game.memory,
        before + 1,
        "§8-2-2-1-6: [Once Per Turn] uses reset on the DNA result"
    );
}

/// (d) §8-2-2-1-5 — a "for the turn" DP gain applied to A ends when A becomes
/// a digivolution card; the result shows only its own printed DP.
#[test]
fn for_the_turn_dp_modifier_on_base_does_not_apply_to_dna_result() {
    let mut r = runner();
    let a = r.place_on_field(0, "DNA-A", Some(0));
    let b = r.place_on_field(0, "DNA-B", Some(0));

    let main = main_action(a);
    assert_eq!(
        build_action_mask(&r.game, 0)[main as usize],
        1.0,
        "DNA-A's [Main] must be activatable"
    );
    r.game.decode_action(main, 0);
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.effective_dp(a), Some(14000), "+3000 DP for the turn applied to A");

    let merged = dna(&mut r, a, b);
    assert_eq!(
        r.game.modifiers.sum(merged, ModifierType::ChangeDp),
        0,
        "§8-2-2-1-5: effects the material gained end"
    );
    assert_eq!(
        r.effective_dp(merged),
        Some(RESULT_DP),
        "the DNA result shows its own printed DP only"
    );
}

/// (e) §8-2-2-1-7 — A was played this turn (summoning sick); the DNA result
/// is a new card that does not inherit that state and can attack.
#[test]
fn base_played_this_turn_dna_result_can_attack() {
    let mut r = runner();
    let a = r.place_on_field(0, "DNA-A", None);
    let b = r.place_on_field(0, "DNA-B", None);
    assert_eq!(r.game.players[0].battle_area[a.index as usize].turn_played, r.game.turn_count);
    assert!(!r.game.can_attack(a, false), "control: a freshly played A cannot attack");
    assert_eq!(build_action_mask(&r.game, 0)[attack_action(a)], 0.0);

    let merged = dna(&mut r, a, b);
    assert_ready_to_attack(&mut r, merged);
}

/// (f) §8-2-2-1-2 — each material's link card is trashed before the material
/// is stacked; nothing linked survives under or beside the DNA result.
#[test]
fn material_link_cards_are_trashed_before_dna_stacking() {
    let mut r = runner();
    let a = r.place_on_field(0, "DNA-A", Some(0));
    let b = r.place_on_field(0, "DNA-B", Some(0));

    let mut linked = Vec::new();
    for host in [a, b] {
        let idx = hand_index(&r, "LINK");
        let card = r.game.players[0].hand[idx].handle();
        assert!(
            r.game.link_chosen_card_into_host(host, card, LinkCardSource::Hand(0)),
            "link into {host:?}"
        );
        r.game.drain_effect_queue();
        assert_eq!(r.game.players[0].battle_area[host.index as usize].linked_cards.len(), 1);
        linked.push(card);
    }
    assert!(r.game.players[0].trash.is_empty());

    let merged = dna(&mut r, a, b);
    let perm = &r.game.players[0].battle_area[merged.index as usize];
    assert!(perm.linked_cards.is_empty(), "§8-2-2-1-2: no link card carries over");
    assert!(
        perm.card_sources.iter().all(|c| !linked.contains(&c.handle())),
        "a trashed link card must not be stacked as a digivolution card"
    );
    for card in &linked {
        assert_eq!(
            r.game.players[0].trash.iter().filter(|c| c.handle() == *card).count(),
            1,
            "§8-2-2-1-2: the link card is in the trash exactly once"
        );
    }
}

/// Control for the whole file — §8-1-2-4: an ORDINARY digivolution keeps the
/// display format, so a suspended base stays suspended. Guards against the
/// DNA reset leaking into the standard path.
#[test]
fn ordinary_digivolution_still_carries_suspension_over() {
    use digimon_engine::enums::{CostDelta, PlaySource};
    let mut r = runner();
    let a = r.place_on_field(0, "DNA-A", Some(0));
    attack(&mut r, a);
    assert!(r.game.players[0].battle_area[a.index as usize].is_suspended);

    let hand_idx = r.add_to_hand(0, "ORDINARY");
    assert!(r.game.effect_initiated_digivolve(0, hand_idx, a, CostDelta::Free, false, PlaySource::ByEffect));
    let perm = &r.game.players[0].battle_area[a.index as usize];
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "ORDINARY");
    assert!(perm.is_suspended, "§8-1-2-4: standard digivolution carries suspension over");
}
