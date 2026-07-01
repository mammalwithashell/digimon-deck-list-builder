//! App Fusion mechanic (Appmon "App Fusion" alt-play) — Gap 4.
//!
//! # Mechanic (BT25-036 Craftmon / BT25-060 Rebootmon)
//!
//! "App Fusion [A] & [B] & [C] & [D]: Cost 0. If 2 such cards are linked
//! together, stack the link card on top and digivolve."
//!
//! App Fusion is an ALTERNATE PLAY of the App-Fusion card from hand. It is
//! legal onto a host Digimon when that host has **2 distinct named cards
//! linked together**: the host's TOP card matches one named condition AND
//! one of the host's LINKED cards matches a *different* named condition
//! (DCGO `AddAppfusionMethod.cs` `GetAppFusion.digimonCondition`, the
//! `i != j` loop). On resolution the App-Fusion card is placed on TOP of
//! the host (a digivolve), the host's existing linked cards move UNDER the
//! new top as digivolution sources (consumed), the App-Fusion cost (0) is
//! paid, and the normal digivolve triggers fire (`[When Digivolving]`,
//! `OnDigivolve`). It ignores printed evo-cost / level / colour
//! requirements.
//!
//! Engine integration: App Fusion is folded into the digivolve route
//! funnel (`normal_digivolve_route_for_card` →
//! `app_fusion_digivolve_route_for_card`), so it surfaces through the
//! existing `encode_digivolve(hand, field)` action + mask (offered, never
//! auto-triggered) and the `commit_digivolve_from_hand_no_replace` commit
//! path. The commit path checks the route's `app_fusion` flag to also run
//! the linked-card consumption step.
//!
//! # DCGO C# reference
//! - DCGO/Assets/Scripts/Script/CardEffectFactory/AddAppfusionMethod.cs
//! - DCGO/Assets/Scripts/Script/CardController.cs (IsAppFusion resolution)
//! - DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_036.cs

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;

/// A minimal App-Fusion card authored in YAML. The four App-Fusion names
/// are listed as a single `name_in` material (the engine flattens them into
/// distinct named conditions). A `[When Digivolving] gain_memory: 2` clause
/// lets the resolution test prove the digivolve triggers fire.
const APP_FUSION_CARD: &str = r#"
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
          name_in: [Kabemon, Gomimon, Ecomon, Puzzlemon]
    cost: 0
effects:
  - when: [when_digivolving]
    summary: "[When Digivolving] gain 2 memory (probe that triggers fire)"
    process:
      - gain_memory: 2
"#;

/// Build a named Digimon test card (no effects, just a name for the
/// App-Fusion condition match).
fn named_digimon(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.colors = vec![CardColor::Yellow];
    card.level = Some(4);
    card
}

fn builder() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(APP_FUSION_CARD)
        .expect("App Fusion test card compiles")
        .add_card(named_digimon("TEST-KABEMON", "Kabemon"))
        .add_card(named_digimon("TEST-GOMIMON", "Gomimon"))
        .add_card(named_digimon("TEST-ECOMON", "Ecomon"))
        .hand(0, &["TEST-APPFUSE"])
        // Start below the +10 gauge cap so the [When Digivolving] gain_memory
        // is observable (App Fusion cost is 0, so memory only moves on the
        // trigger).
        .memory(3)
        .start()
}

/// 1. A host Digimon whose TOP card is a named card A and whose LINKED card
///    is a different named card B makes the App-Fusion card's app-fusion
///    play legal (appears in the action mask).
#[test]
fn gap4_app_fusion_eligible_when_named_cards_linked() {
    let mut runner = builder();
    // Host: top = Kabemon, linked = Gomimon → 2 distinct named cards linked.
    let host = runner.place_on_field(0, "TEST-KABEMON", Some(0));
    runner.push_linked_owned(host, "TEST-GOMIMON", 0);

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "App Fusion must be offered as a legal digivolve action onto a host \
         with 2 distinct named cards linked together"
    );
}

/// 2. Only one named card present (top matches, but NO linked named card) →
///    not eligible. The "2 such cards linked together" requirement fails.
#[test]
fn gap4_app_fusion_not_eligible_without_two_named() {
    let mut runner = builder();
    // Host: top = Kabemon, NO linked named card.
    let host = runner.place_on_field(0, "TEST-KABEMON", Some(0));

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "App Fusion must NOT be legal when only one named card is present \
         (no linked second named card)"
    );

    // Same host, but the linked card is the SAME name as the top (Kabemon &
    // Kabemon) — DCGO requires two DISTINCT named conditions (i != j), so a
    // duplicate name does not satisfy it.
    runner.push_linked_owned(host, "TEST-KABEMON", 0);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "App Fusion requires two DISTINCT named conditions; a duplicate name \
         (Kabemon top + Kabemon linked) must not qualify"
    );
}

/// 3. Performing the app fusion places the App-Fusion card on top of the
///    host, the host's linked cards become digivolution sources (consumed
///    from linked_cards), the cost (0) is paid, and `[When Digivolving]`
///    fires.
#[test]
fn gap4_app_fusion_stacks_and_consumes_links() {
    let mut runner = builder();
    let host = runner.place_on_field(0, "TEST-KABEMON", Some(0));
    let linked_handle = runner.push_linked_owned(host, "TEST-GOMIMON", 0);

    let memory_before = runner.memory();
    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[action as usize], 1.0, "App Fusion action is legal");

    runner.game.decode_action(action, 0);

    let perm = &runner.game.players[0].battle_area[host.index as usize];
    // App-Fusion card is now on top of the host (a digivolve).
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "TEST-APPFUSE",
        "App-Fusion card must be stacked on top of the host"
    );
    // The Kabemon host top is now a digivolution source under the new top.
    assert!(
        perm.card_sources
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TEST-KABEMON"),
        "the host's former top (Kabemon) remains as a digivolution source"
    );
    // The linked Gomimon was consumed INTO the digivolution sources.
    assert!(
        perm.card_sources
            .iter()
            .any(|c| c.handle() == linked_handle),
        "the host's linked card (Gomimon) must be moved under the new top as \
         a digivolution source"
    );
    // It is no longer a linked card.
    assert!(
        perm.linked_cards.is_empty(),
        "the consumed linked card must be removed from linked_cards"
    );

    // Cost 0 paid, then [When Digivolving] gain_memory: 2 fired → net +2.
    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "App Fusion cost is 0; the [When Digivolving] effect (gain 2) fires"
    );
}

/// 4. Full DSL flow: the App-Fusion card authored with
///    `alt_paths: [{ kind: app_fusion, materials: [...], cost: 0 }]`
///    compiles, registers the app-fusion path, and drives the complete
///    eligible → stack → consume → trigger flow.
#[test]
fn gap4_dsl_app_fusion() {
    // Confirm the DSL `kind: app_fusion` compiles to an AppFusion alt-path.
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(APP_FUSION_CARD).expect("App Fusion YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("App Fusion YAML compiles");
    let compiled = registry
        .lookup("TEST-APPFUSE")
        .expect("TEST-APPFUSE in registry");
    assert!(
        compiled.alt_paths.iter().any(|p| matches!(
            p.kind,
            digimon_dsl::compiled::CompiledAltPathKind::AppFusion
        )),
        "the DSL `kind: app_fusion` alt-path must compile to an AppFusion path"
    );

    // End-to-end via the DSL-authored card with a DIFFERENT name pairing
    // (Ecomon top + Kabemon linked) to exercise distinct conditions drawn
    // from the four-name `name_in` list.
    let mut runner = builder();
    let host = runner.place_on_field(0, "TEST-ECOMON", Some(0));
    runner.push_linked_owned(host, "TEST-KABEMON", 0);

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "App Fusion legal for Ecomon top + Kabemon linked (distinct names \
         from the four-name list)"
    );

    runner.game.decode_action(action, 0);
    let perm = &runner.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "TEST-APPFUSE"
    );
    assert!(
        perm.card_sources
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TEST-KABEMON"),
        "linked Kabemon consumed into sources"
    );
    assert!(perm.linked_cards.is_empty());
}
