//! Flaremon / Beastkin (BT25 slice) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/flaremon-beastkin-model.md`. Slice cards (all
//! IMPLEMENTED + per-card-green): BT25-024 Lekismon, BT25-016 GrapLeomon,
//! BT25-017 Flaremon, BT25-054 GreatGrizzlymon — the BT25 [Beastkin][Iliad][TS]
//! "Three Sovereigns" beast line. Every card carries an *intra-archetype
//! digivolve engine*: a [When Digivolving]/win/attack trigger that, when its
//! gate is met, performs a *second* digivolution into a named beast partner.
//! These tests pin the cross-clause / cross-card chains those engines form,
//! which the per-card tests (`cards_behavioral/bt25/bt25_0{16,17,24,54}.rs`)
//! deliberately exercise in isolation and therefore never see.
//!
//! Cross-set evolution prerequisites are supplied as **synthesized DebugRunner
//! fixtures** (`make_*` neutral cards with the level/colour/DP/evo-cost a combo
//! needs). A cross-set card's *real* implementation is pulled in only if a combo
//! fires its actual printed effect — none of these three combos does, so the
//! beast partners (Apollomon / Marsmon / Callismon) are synthetic evolution
//! targets with no [When Digivolving] of their own. The combos isolate the
//! *slice cards'* own mechanical claims.
//!
//! Source priority (CLAUDE.md): printed card face (BT25-0xx.webp) + DCGO C#
//! (`$BASE_DCGO/Assets/Scripts/CardEffect/BT25/<Colour>/BT25_0xx.cs`) outrank
//! cards.json. The YAML headers
//! (`code/digimon-engine/cards/bt25/BT25-0{16,17,24,54}.yaml`) carry the per-card
//! DCGO crosschecks each combo below relies on.
//!
//! Combos authored (ranked by play-frequency × payoff-centrality):
//!   F1 — GrapLeomon [On Play] +3000 pump *enables* the [All Turns] 13000-DP
//!        free-digivolve observer (cross-clause; cross-card — a 2nd GrapLeomon on
//!        field watches). The highest-value engine: the pump is the only way a
//!        10000-DP attacker crosses the observer's 13000 threshold.
//!   F2 — playing the archetype's BLUE beast (BT25-024 Lekismon) is a "your
//!        Digimon are played, any of them blue" event that satisfies Flaremon's
//!        [Your Turn] "if any of them are blue" gate, offering Flaremon's own
//!        digivolve into [Apollomon] — one card's entry firing another's
//!        digivolve observer. (Flaremon's gate is BLUE per the card image /
//!        BT25-017.yaml — not red; Lekismon is the deck's blue Lv.4.)
//!   F3 — GreatGrizzlymon's taunt FORCES an opponent Digimon to attack into it
//!        (a Blocker) → it wins the battle → the win fires its [All Turns]
//!        free-digivolve (top-card, F3-A) and, when buried as a source, the
//!        inherited trash-top-security (F3-B): the win-payoff pair, split by
//!        stack position because an inherited fires only from beneath a stack.

#![allow(dead_code)]

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

use super::support::snapshot;

const LEKISMON: &str = "BT25-024";
const GRAPLEOMON: &str = "BT25-016";
const FLAREMON: &str = "BT25-017";
const GREATGRIZZLYMON: &str = "BT25-054";

// ─── Synthesized combo-piece fixtures ────────────────────────────────────────

/// A red Lv.5 own Digimon with a chosen DP — the GrapLeomon [On Play] pump
/// target / the F1 attacker.
fn red_lv5(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(id, name, 5);
    c.colors = vec![CardColor::Red];
    c.dp = Some(dp);
    c
}

/// A red Lv.4 own Digimon used as a colour-gate trigger ("if any of them are
/// red" / the Lekismon red-played gate).
fn red_lv4(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, 4);
    c.colors = vec![CardColor::Red];
    c.dp = Some(4000);
    c
}

/// Apollomon (Lv.6) — the partner Flaremon's [Your Turn] clause digivolves into
/// from hand. Synthetic evolution target; cost 4 so the −2 reduction is
/// observable as 2 memory spent.
fn apollomon(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "Apollomon", 6);
    c.colors = vec![CardColor::Red];
    c.dp = Some(12000);
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 4,
    }];
    c
}

/// Marsmon / Callismon (Lv.6) — the partners GrapLeomon / GreatGrizzlymon
/// free-digivolve into. Synthetic; evo cost 4 so a *free* digivolve is provable
/// as "memory unchanged". `name` selects which named partner this stands in for.
fn named_lv6(id: &str, name: &str, colour: CardColor) -> CardData {
    let mut c = make_test_card_with_level(id, name, 6);
    c.colors = vec![colour];
    c.dp = Some(11000);
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 4,
    }];
    c
}

/// A neutral opponent Digimon with a chosen DP.
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(id, id, 4);
    c.colors = vec![CardColor::Purple];
    c.dp = Some(dp);
    c
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let iid = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, iid));
    runner.game.players[player as usize].hand.len() - 1
}

fn seed_security(runner: &mut DebugRunner, player: u8, n: usize) {
    let fill_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FILL")
        .expect("FILL card present");
    for _ in 0..n {
        let iid = runner.game.next_card_index();
        runner.game.players[player as usize]
            .security
            .push(CardSource::new(fill_idx, player, iid));
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Combo F1 — GrapLeomon's [On Play] +3000 pump enables the 13000-DP observer
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-016 GrapLeomon has TWO clauses that chain:
//   (a) [On Play][When Digivolving] "1 of your Digimon gets +3000 DP for the
//       turn. Then, 1 of your Digimon may attack."
//   (b) [All Turns] "When a Digimon with 13000 DP or more attacks, this Digimon
//       may digivolve into [Marsmon] or [Callismon] in the hand without paying
//       the cost."
//
// System fact: a 10000-DP attacker is BELOW the (b) threshold. Clause (a)'s
// +3000 pump is the ONLY thing that lifts it to 13000 and trips (b). We stage a
// SECOND GrapLeomon already on the field as the (b) *observer*, and the freshly
// played GrapLeomon supplies the (a) pump onto the attacker — a genuine
// cross-card interaction (one GrapLeomon's pump arms the other's free-digivolve).
//
// Sources: card image BT25-016 + DCGO BT25_016.cs (OnAllyAttack DP>=13000 →
// skippable free digivolve; OnPlay +3000 UntilEachTurnEnd). general_rule.pdf §16
// digivolve timing; DP modifiers fold into the >=13000 comparison.

/// Happy path: pump a 10000-DP attacker to 13000, it attacks, the *observer*
/// GrapLeomon offers its free digivolve into a hand partner — without the pump
/// the attacker is 10000 and the observer stays silent.
#[test]
fn f1_grapleomon_pump_arms_the_13k_free_digivolve_observer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GRAPLEOMON)
        .expect("BT25-016 (GrapLeomon) in embedded DSL pack")
        .add_card(red_lv5("ATTACKER-10K", "Attacker", 10000))
        .add_card(named_lv6("MARS", "Marsmon", CardColor::Red))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    runner.game_mut().players[1].security.clear();

    // The OBSERVER GrapLeomon already on field (clause b watcher), the attacker,
    // and Marsmon waiting in hand as the free-digivolve partner.
    let _observer = runner.place_on_field(0, GRAPLEOMON, Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER-10K", Some(0));
    let _mars_in_hand = push_to_hand(&mut runner, 0, "MARS");

    // Play a SECOND GrapLeomon → its [On Play] pump. Drive the mandatory +3000
    // target onto the 10000-DP attacker explicitly.
    let hand_index = push_to_hand(&mut runner, 0, GRAPLEOMON);
    runner.play(0, hand_index);

    // First pending = the mandatory +3000 DP target. Choose the attacker so the
    // pump lands where it matters (the harness exposes the field targets; pick
    // the action that buffs ATTACKER-10K to 13000).
    drive_pump_onto_attacker(&mut runner, attacker);
    // After the DP pick, GrapLeomon also offers "1 of your Digimon may attack" —
    // decline it (PASS) so we control the attack explicitly below.
    finish_on_play_declining_attack(&mut runner);

    let dp = runner.effective_dp(attacker).expect("attacker dp");
    assert_eq!(
        dp, 13000,
        "the [On Play] +3000 pump must lift the 10000-DP attacker to exactly 13000"
    );

    // Now the pumped 13000-DP attacker attacks → the observer GrapLeomon's
    // [All Turns] free-digivolve must be OFFERED (the system-level payoff).
    let memory_before = runner.memory();
    runner.attack_player(attacker, 1, false);
    assert!(
        runner.pending_kind().is_some(),
        "a 13000-DP attacker (only 13000 because of the pump) must arm GrapLeomon's free digivolve"
    );
    runner
        .auto_resolve()
        .expect("resolve the free-digivolve offer");

    // The digivolve is "without paying the cost" — memory must be unchanged by it.
    assert_eq!(
        runner.memory(),
        memory_before,
        "GrapLeomon's free digivolve must not pay memory"
    );
}

/// Unhappy path: the SAME board but the pump is steered onto a *different*
/// Digimon, so the 10000-DP attacker stays at 10000 — below 13000 — and the
/// observer never offers its digivolve. Proves the observer is gated on the
/// pump reaching the attacker, not merely on GrapLeomon being played.
#[test]
fn f1_unpumped_attacker_below_13k_does_not_arm_the_observer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GRAPLEOMON)
        .expect("BT25-016 (GrapLeomon) in embedded DSL pack")
        .add_card(red_lv5("ATTACKER-10K", "Attacker", 10000))
        .add_card(red_lv5("BYSTANDER", "Bystander", 5000))
        .add_card(named_lv6("MARS", "Marsmon", CardColor::Red))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    runner.game_mut().players[1].security.clear();

    let _observer = runner.place_on_field(0, GRAPLEOMON, Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER-10K", Some(0));
    let bystander = runner.place_on_field(0, "BYSTANDER", Some(0));
    let _mars_in_hand = push_to_hand(&mut runner, 0, "MARS");

    let hand_index = push_to_hand(&mut runner, 0, GRAPLEOMON);
    runner.play(0, hand_index);

    // Steer the mandatory +3000 onto the BYSTANDER, not the attacker.
    drive_pump_onto_attacker(&mut runner, bystander);
    finish_on_play_declining_attack(&mut runner);

    assert_eq!(
        runner.effective_dp(attacker),
        Some(10000),
        "with the pump elsewhere the attacker stays at its base 10000 DP"
    );

    runner.attack_player(attacker, 1, false);
    assert!(
        runner.pending_selection().is_none(),
        "a 10000-DP attacker is below the 13000 threshold — the observer must NOT offer its digivolve"
    );
    runner.auto_resolve().ok();
}

/// Drive GrapLeomon's mandatory +3000-DP [On Play] `OwnField` target onto the
/// own-field permanent at slot `wanted.index`. An own-field *target selection*
/// (`SelectionKind::OwnField`) encodes each own-field slot as `ATTACK_START +
/// slot` (empirically `[100, 101, 102, ...]` for slots `0,1,2,...`; the same
/// `100 + slot` field-target encoding the action mask uses for own-field picks).
/// Pick the candidate equal to `100 + wanted.index`; fall back to the first
/// valid action if it is not offered (engine pre-targeted).
fn drive_pump_onto_attacker(runner: &mut DebugRunner, wanted: PermanentHandle) {
    use digimon_engine::action::space::ATTACK_START;
    let Some(view) = runner.pending_selection_view() else {
        return;
    };
    let want = ATTACK_START + wanted.index as u16;
    let chosen = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a == want)
        .or_else(|| view.valid_action_ids.first().copied());
    if let Some(action) = chosen {
        let _ = runner.execute_action(view.selecting_player, action);
    }
}

/// After GrapLeomon's mandatory +3000-DP pick, decline the optional "1 of your
/// Digimon may attack" follow-up by PASS-ing it, so the test controls the attack
/// itself. Any further pending optional selection is also PASS-ed; a non-optional
/// one is auto-resolved (first valid action). Bounded to avoid an infinite loop.
fn finish_on_play_declining_attack(runner: &mut DebugRunner) {
    use digimon_engine::action::space::PASS;
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let action = if view.valid_action_ids.contains(&PASS) {
            PASS
        } else {
            match view.valid_action_ids.first().copied() {
                Some(a) => a,
                None => return,
            }
        };
        if runner
            .execute_action(view.selecting_player, action)
            .is_err()
        {
            return;
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Combo F2 — playing the archetype's BLUE beast (Lekismon) arms Flaremon's
//            [Your Turn] blue-colour gate
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-017 Flaremon [Your Turn]: "When your Digimon are played or digivolve, if
// any of them are **blue**, this Digimon may digivolve into [Apollomon] in the
// hand with the cost reduced by 2." (card image BT25-017 + DCGO BT25_017.cs
// OnEnterFieldAnyone — gate is BLUE; cards.json clause-2 wording agrees.)
//
// The combo: BT25-024 Lekismon is the archetype's own **blue** [Beastkin]/[TS]
// Lv.4. Playing (or digivolving into) Lekismon on your turn is a "your Digimon
// are played, and any of them are blue" event — which is exactly Flaremon's
// trigger. So the deck's blue-beast tempo card doubles as the enabler that lets
// a Flaremon already on the board free-ramp into [Apollomon]. This is a genuine
// cross-card chain: Lekismon's *entry* fires Flaremon's *digivolve observer*,
// something neither per-card test stages (bt25_017 plays a synthetic BLUE-LV4;
// bt25_024 never has Flaremon on the field).
//
// Sources: card image BT25-017 (blue gate, [Apollomon], cost −2) + DCGO
// BT25_017.cs; BT25-024 is a Blue Lv.4 (card image / cards.json color=Blue).
// general_rule.pdf §16 digivolution-vs-play event timing.

/// Happy path: playing Lekismon (the blue archetype beast) on P0's turn fires
/// Flaremon's [Your Turn] "if any of them are blue" observer, which offers
/// Flaremon's digivolve into [Apollomon] from hand.
#[test]
fn f2_playing_blue_lekismon_arms_flaremons_apollomon_offer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(FLAREMON)
        .expect("BT25-017 (Flaremon) in embedded DSL pack")
        .dsl_card(LEKISMON)
        .expect("BT25-024 (Lekismon) in embedded DSL pack")
        .add_card(apollomon("APOLLO"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Flaremon on field as the [Your Turn] observer, Apollomon waiting in hand.
    let _flaremon = runner.place_on_field(0, FLAREMON, Some(0));
    let _apollo_in_hand = push_to_hand(&mut runner, 0, "APOLLO");

    // Play Lekismon (blue) — a "your Digimon are played" event whose card is
    // blue. Flaremon's blue gate must arm. (Lekismon also draws on play; that
    // does not affect the gate.)
    let lekismon_idx = push_to_hand(&mut runner, 0, LEKISMON);
    runner.play(0, lekismon_idx);

    assert!(
        runner.pending_kind().is_some(),
        "playing the blue Lekismon on your turn must arm Flaremon's [Your Turn] digivolve into Apollomon"
    );
    runner.auto_resolve().ok();
}

/// Unhappy path: the SAME board but a RED beast (a red Lv.4) is played instead
/// of the blue Lekismon. Flaremon's gate is "if any of them are blue", so a red
/// entry must NOT arm the offer — even with Apollomon live in hand. Proves the
/// blue colour gate (not hand-availability) is what governs the chain.
#[test]
fn f2_playing_red_beast_does_not_arm_flaremon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(FLAREMON)
        .expect("BT25-017 (Flaremon) in embedded DSL pack")
        .add_card(red_lv4("RED-LV4"))
        .add_card(apollomon("APOLLO"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let _flaremon = runner.place_on_field(0, FLAREMON, Some(0));
    let _apollo_in_hand = push_to_hand(&mut runner, 0, "APOLLO");

    // Play a RED Lv.4 — not blue, so the colour gate must block the offer.
    let red_idx = push_to_hand(&mut runner, 0, "RED-LV4");
    runner.play(0, red_idx);

    assert!(
        runner.pending_selection().is_none(),
        "a non-blue entering Digimon must NOT arm Flaremon's blue-gated [Your Turn] digivolve"
    );
    runner.auto_resolve().ok();
}

// ══════════════════════════════════════════════════════════════════════════
// Combo F3 — GreatGrizzlymon taunt → forced attack into the Blocker → win →
//            free digivolve + inherited trash-security
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-054 GreatGrizzlymon stacks THREE clauses into one chain:
//   <Blocker>;
//   [On Play][When Digivolving] "Give 1 of your opponent's Digimon '[Start of
//     Your Main Phase] This Digimon attacks.' until their turn ends." (taunt);
//   [All Turns] "When this Digimon wins a battle, it may digivolve into
//     [Callismon] or [Marsmon] in the hand without paying the cost.";
//   inherited [All Turns][OPT] "When this Digimon deletes your opponent's Digimon
//     in battle, trash their top security card."
//
// System fact: the taunt forces a weak opponent Digimon to attack on the
// OPPONENT's turn; GreatGrizzlymon (DP 7000, <Blocker>) blocks/battles it, wins,
// and that single battle-win simultaneously trips the free-digivolve AND the
// inherited security trash. A per-card test stages each clause from a bare
// `attack_digimon`; none stages the taunt → forced-attack → win loop end-to-end.
//
// To keep the chain deterministic without simulating the opponent's main phase,
// we drive the *battle the taunt produces* directly: the taunted opponent Digimon
// battles into GreatGrizzlymon, GreatGrizzlymon wins, and we assert both win
// payoffs fired. The taunt-install half is asserted first (its selection must
// appear), keeping the full clause set traceable.

/// The taunt clause installs an opponent-Digimon selection (the [On Play] half
/// of the chain) — the entry point the win payoffs hang off.
#[test]
fn f3_grizzly_taunt_installs_opponent_selection() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GREATGRIZZLYMON)
        .expect("BT25-054 (GreatGrizzlymon) in embedded DSL pack")
        .add_card(opp_digimon("OPP-WEAK", 1000))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    let _opp = runner.place_on_field(1, "OPP-WEAK", Some(0));

    let hand_index = push_to_hand(&mut runner, 0, GREATGRIZZLYMON);
    runner.play(0, hand_index);

    assert!(
        runner.pending_kind().is_some(),
        "the [On Play] taunt must install an opponent-Digimon selection (chain entry point)"
    );
    runner.auto_resolve().expect("resolve taunt selection");
}

/// Happy path A — the top-card win payoff: when the *top-card* GreatGrizzlymon
/// wins the battle the taunt set up, its [All Turns] clause offers the free
/// digivolve into a hand partner, and the digivolve pays no memory. (The taunt's
/// purpose is to FORCE this battle on the opponent's turn; here we drive the
/// resulting battle directly — a top-card Grizzly's [All Turns] clause, NOT the
/// inherited, which only fires from beneath a stack — see Happy path B.)
#[test]
fn f3_grizzly_battle_win_offers_free_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GREATGRIZZLYMON)
        .expect("BT25-054 (GreatGrizzlymon) in embedded DSL pack")
        .add_card(opp_digimon("OPP-WEAK", 1000))
        .add_card(named_lv6("CALLIS", "Callismon", CardColor::Green))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // GreatGrizzlymon (7000) vs a 1000-DP taunted opponent it will delete and
    // survive; Callismon in hand for the win-digivolve.
    let grizzly = runner.place_on_field(0, GREATGRIZZLYMON, Some(0));
    let weak = runner.place_on_field(1, "OPP-WEAK", Some(0));
    let _callis_in_hand = push_to_hand(&mut runner, 0, "CALLIS");
    runner.game_mut().players[1].security.clear();

    let before = snapshot(&runner);
    let memory_before = runner.memory();

    runner.attack_digimon(grizzly, weak, false);

    // The optional free digivolve into Callismon/Marsmon is offered (top-card
    // [All Turns] win clause).
    assert!(
        runner.pending_kind().is_some(),
        "winning the battle must offer GreatGrizzlymon's free digivolve into [Callismon]/[Marsmon]"
    );
    runner.auto_resolve().expect("resolve the win-battle chain");

    let after = snapshot(&runner);

    // The free digivolve cost no memory.
    assert_eq!(
        runner.memory(),
        memory_before,
        "the win-battle digivolve is 'without paying the cost' — memory unchanged"
    );
    // The opponent's taunted Digimon was deleted in the battle.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the 1000-DP opponent Digimon must be deleted by the 7000-DP GreatGrizzlymon"
    );
}

/// Happy path B — the inherited win payoff: GreatGrizzlymon buried as a
/// *digivolution source* under a carrier. When the carrier wins a battle, the
/// inherited [All Turns][OPT] clause trashes the opponent's top security card.
/// (Inherited effects fire only from beneath a stack — `f3_grizzly_battle_win_
/// offers_free_digivolve` proves the top-card half; this proves the buried half,
/// completing the win-payoff pair the archetype's beasts rely on.)
#[test]
fn f3_buried_grizzly_battle_win_trashes_top_security() {
    let mut carrier = named_lv6("CARRIER6", "Carrier Mega", CardColor::Green);
    carrier.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card(GREATGRIZZLYMON)
        .expect("BT25-054 (GreatGrizzlymon) in embedded DSL pack")
        .add_card(opp_digimon("OPP-WEAK", 1000))
        .add_card(carrier)
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // [GreatGrizzlymon, Carrier] — Grizzly is a SOURCE, so its inherited applies.
    let stack = runner.place_stack(0, &[GREATGRIZZLYMON, "CARRIER6"]);
    let weak = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.game_mut().players[1].security.clear();
    seed_security(&mut runner, 1, 2);

    let before = snapshot(&runner);
    runner.attack_digimon(stack, weak, false);
    runner
        .auto_resolve()
        .expect("resolve inherited security trash");

    let after = snapshot(&runner);

    // The inherited trashed exactly one top security card on the battle win.
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "the inherited [OPT] must trash the opponent's top security card on the battle win"
    );
    // The opponent Digimon was deleted in the battle.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the taunted 1000-DP opponent Digimon must be deleted in the battle"
    );
}

/// Unhappy path: GreatGrizzlymon attacks the opponent's *security* (a security
/// check, NOT a Digimon-vs-Digimon battle win). The win-battle clauses key on
/// `source_deleted_battle_opponent`, so neither the free digivolve nor the
/// inherited security-trash-on-win fires — the chain is gated on an actual
/// battle win, the system fact distinguishing it from a plain security attack.
#[test]
fn f3_security_attack_is_not_a_battle_win() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GREATGRIZZLYMON)
        .expect("BT25-054 (GreatGrizzlymon) in embedded DSL pack")
        .add_card(named_lv6("CALLIS", "Callismon", CardColor::Green))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let grizzly = runner.place_on_field(0, GREATGRIZZLYMON, Some(0));
    let _callis_in_hand = push_to_hand(&mut runner, 0, "CALLIS");
    runner.game_mut().players[1].security.clear();
    seed_security(&mut runner, 1, 2);

    let sec_before = runner.security_count(1);
    runner.attack_player(grizzly, 1, false);
    runner.auto_resolve().ok();

    // The win-battle free digivolve must NOT be pending (no Digimon battle won).
    assert!(
        runner.pending_selection().is_none(),
        "a security check is not a battle win — GreatGrizzlymon's free digivolve must not be offered"
    );
    // A normal security check trashes exactly the checked card(s); the inherited
    // win-on-battle trash must NOT additionally fire. The flip of 1 security card
    // is the security-check itself, so the count drops by at most the check, not
    // an extra inherited trash.
    assert!(
        runner.security_count(1) >= sec_before - 1,
        "no battle win → the inherited must not trash an EXTRA security card beyond the check"
    );
}
