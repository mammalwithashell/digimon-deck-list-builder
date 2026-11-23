//! BT22-009 Effecmon
//!
//! Implemented:
//! - [On Play] [When Digivolving] Delete 1 opponent Digimon with 4000 DP or less.
//! - [Security] At end of battle, play this card free.
//! - Link box (card face BT22-009.webp; official bundle "### Link Condition" /
//!   "### Link DP" / "### Link Effect"): <Link> [Appmon] trait: Cost 2,
//!   +3000 DP, and "[When Linking] Delete 1 of your opponent's Digimon with
//!   4000 DP or less." (DCGO BT22_009.cs: AddSelfLinkConditionStaticEffect
//!   HasAppmonTraits linkCost 2; WhenLinked SetIsLinkedEffect, not optional,
//!   canNoSelect false).

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn bt22_009_has_stnd_trait_digivolve_path() {
    let runner = runner();
    let card = runner.compiled_card("BT22-009").expect("compiled");
    // Second digivolve circle on the card face ("Stnd. 2"): digivolve onto a
    // Digimon with the [Stnd.] form-trait for cost 2 (DCGO EqualsTraits("Stnd.")).
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(2))
                && path
                    .from
                    .as_ref()
                    .is_some_and(|f| f.trait_has.as_deref() == Some("Stnd."))
        }),
        "BT22-009 must have a [Stnd.]-trait digivolve path at cost 2"
    );
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load from embedded DSL pack")
        .memory(5)
        .start()
}

#[test]
fn bt22_009_has_on_play_and_when_digivolving_delete_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT22-009")
        .expect("BT22-009 must be compiled");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT22-009 must have a shared OnPlay/WhenDigivolving clause");

    assert!(
        !clause.optional,
        "delete clause is mandatory when a target exists"
    );
}

#[test]
fn bt22_009_on_play_deletes_4000_dp_or_lower_target() {
    let mut low = make_test_card("BT22-009-LOW", "LowTarget");
    low.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load")
        .add_card(low)
        .hand(0, &["BT22-009"])
        .memory(5)
        .start();

    runner.place_on_field(1, "BT22-009-LOW", None);
    runner.play(0, 0);
    runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area.is_empty(),
        "BT22-009 should delete the eligible low-DP opponent Digimon"
    );
}

/// Structural: BT22-009 must carry an [Security] (`on_security`) clause that is
/// mandatory (the card plays itself automatically — no PASS choice).
#[test]
fn bt22_009_has_security_play_self_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT22-009")
        .expect("BT22-009 must be compiled");
    let sec = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when == vec![CompiledTiming::OnSecurity] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT22-009 must have an OnSecurity play-self clause");

    assert!(
        !sec.optional,
        "[Security] play-self must NOT be optional (the card plays automatically)"
    );
}

/// Integration: place BT22-009 in P1's security, attack with P0's Digimon.
/// At the end of the battle the [Security] clause plays Effecmon onto P1's
/// battle area for free (play_from_security), so security drops to 0 and P1's
/// battle area gains the card.
#[test]
fn bt22_009_security_plays_self_at_end_of_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .memory(10)
        .start();

    // Place attacker on P0's field (turn_played=0 to skip summoning sickness).
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    // Put Effecmon at the top of P1's security stack.
    {
        let eff_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT22-009")
            .expect("BT22-009 in card_data");
        let next = runner.game.next_card_index();
        let eff_src = CardSource::new(eff_idx, 1, next);
        runner.game.players[1].security.push(eff_src);
    }

    assert_eq!(
        runner.security_count(1),
        1,
        "pre: P1 has exactly 1 security (Effecmon)"
    );

    // P0 attacks P1's security.
    runner.attack_player(attacker, 1, false);
    // Resolve any pending selections from the security effect (the On Play delete
    // clause resolves immediately when there is no eligible target).
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        0,
        "security card (Effecmon) must be removed by the attack"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Effecmon must be played onto P1's battle area for free by its [Security] effect at end of battle"
    );
}

// ─── Link box: <Link> [Appmon] trait: Cost 2, +3000 DP, [When Linking] delete ─

fn link_bit(perm: digimon_engine::permanent::PermanentHandle) -> u16 {
    use digimon_engine::action::space::{
        EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START,
    };
    FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK
}

fn link_digimon(id: &str, dp: i32, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Effecmon (BT22-009) on the field, an `host_traits` host beside it, and
/// opponent Digimon of 4000 and 5000 DP.
fn link_runner(
    host_traits: &[&str],
) -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    let mut r = DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load from embedded DSL pack")
        .add_card(link_digimon("HOST", 5000, host_traits))
        .add_card(link_digimon("OPP-4000", 4000, &["Beast"]))
        .add_card(link_digimon("OPP-5000", 5000, &["Beast"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "HOST", Some(0));
    let effecmon = r.place_on_field(0, "BT22-009", Some(0));
    r.place_on_field(1, "OPP-4000", Some(0));
    r.place_on_field(1, "OPP-5000", Some(0));
    (r, host, effecmon)
}

fn link_onto_host(r: &mut DebugRunner, effecmon: digimon_engine::permanent::PermanentHandle) {
    r.game.decode_action(link_bit(effecmon), 0);
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("the link host pick")
        .valid_action_ids[0];
    r.game.resolve_selection(0, sel).expect("link onto the host");
}

fn host_index(r: &DebugRunner) -> usize {
    r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "HOST")
        .expect("host on the field")
}

fn opp_has(r: &DebugRunner, id: &str) -> bool {
    r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

/// The link box's link condition: [Appmon] trait host, cost 2.
#[test]
fn bt22_009_has_link_condition_appmon_cost_2() {
    use digimon_dsl::compiled::CompiledDeclarativeClause;
    let runner = runner();
    let card = runner.compiled_card("BT22-009").expect("compiled");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. })
                if *cost == 2
        )),
        "BT22-009 prints <Link> [Appmon] trait: Cost 2"
    );
}

/// Linking Effecmon onto an [Appmon] host pays 2, gives the host +3000 DP,
/// and its [When Linking] deletes 1 opponent Digimon with 4000 DP or less --
/// a mandatory pick that offers only the 4000 DP one.
#[test]
fn bt22_009_linking_onto_an_appmon_host_gives_3000_dp_and_deletes_a_4000_dp_digimon() {
    use digimon_engine::action::space::PASS;
    let (mut r, host, effecmon) = link_runner(&["Appmon"]);
    let dp0 = r.effective_dp(host).expect("dp");
    let mem0 = r.memory();
    link_onto_host(&mut r, effecmon);
    let view = r
        .pending_selection_view()
        .expect("[When Linking] delete pick");
    assert!(!view.is_optional, "the delete is printed without a may");
    assert!(!view.valid_action_ids.contains(&PASS));
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the 4000 DP Digimon is offered: {:?}",
        view.valid_action_ids
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("delete");
    let _ = r.auto_resolve();
    assert!(!opp_has(&r, "OPP-4000"), "the 4000 DP Digimon was deleted");
    assert!(opp_has(&r, "OPP-5000"), "the 5000 DP Digimon stays");
    let hi = host_index(&r);
    assert_eq!(
        r.game.players[0].battle_area[hi].linked_cards.len(),
        1,
        "Effecmon is linked to the host"
    );
    r.game.tick_declarative_effects();
    let host_now = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: hi as u8,
    };
    assert_eq!(r.effective_dp(host_now).expect("dp"), dp0 + 3000, "Link DP +3000");
    assert_eq!(mem0 - r.memory(), 2, "the link costs 2");
}

/// A host without the [Appmon] trait cannot take Effecmon's link.
#[test]
fn bt22_009_cannot_link_onto_a_host_without_the_appmon_trait() {
    let (mut r, _host, effecmon) = link_runner(&["Beast"]);
    r.game.decode_action(link_bit(effecmon), 0);
    let _ = r.auto_resolve();
    assert!(
        r.game.players[0].battle_area.iter().all(|p| p.linked_cards.is_empty()),
        "no [Appmon] host: nothing links"
    );
    assert!(opp_has(&r, "OPP-4000"), "no [When Linking] delete");
}
