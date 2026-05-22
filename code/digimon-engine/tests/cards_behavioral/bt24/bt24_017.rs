//! BT24-017 Medusamon — Digimon, Lv.6, Red, DP 11000, Cost 11.
//!
//! # Card text (cards.json)
//!
//! `<Raid>`
//! `<Progress>`
//! `<Piercing>`
//! [When Digivolving] Delete 1 of your opponent's lowest DP Digimon.
//! Then, by returning 2 cards from their trash to the bottom of the deck,
//! they play 2 [Petrification] Tokens.
//! (Digimon/White/3000 DP/[Your Turn] This Digimon can't suspend.
//!  [On Deletion] Trash your top security card.)
//! After, this Digimon gets +2000 DP for each of your opponent's Digimon
//! until their turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_017.cs
//!
//! # Engine / DSL gap status (re-verified 2026-05-21)
//!
//! G-PRED-DP-LTE — RESOLVED 2026-05-17. The "lowest DP" delete filter is
//!   expressed with the `selector: lowest_dp` field on the
//!   `select_opponent_permanent` step (not a `dp_lte` aggregate predicate).
//!   The delete prompt offers ONLY the lowest-effective-DP candidate(s).
//!   Test `bt24_017_delete_targets_only_lowest_dp_digimon` is active.
//!
//! G-ZONE-TRASH-TO-DECK — RESOLVED at the DSL level. The first-class
//!   `return_trash_list_to_deck_bottom` verb moves a bound card list out of
//!   trash to the deck bottom (lowers to
//!   `EffectContext::return_trash_cards_to_deck_bottom`). The full-sequence
//!   tests are active.
//!
//! # Patterns this test covers
//! - H9 (Raid), H3 (Piercing), Progress — declarative keyword grants
//! - D1-adjacent — dynamic DP modifier (+2000 × opp-Digimon-count) until
//!   the opponent's turn ends
//! - F2-adjacent — mandatory when-digivolving delete + chained selection
//! - Token play (Petrification) — opponent plays 2 Tokens as a paid cost

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// The production YAML for BT24-017, inlined at compile time from the
/// canonical location under `cards/bt24/`.
const YAML: &str = include_str!("../../../cards/bt24/BT24-017.yaml");

/// Compile BT24-017 from the production YAML and return the CompiledCard.
fn compiled_bt24_017() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT24-017.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT24-017.yaml compiles");
    registry
        .lookup("BT24-017")
        .expect("BT24-017 in registry")
        .clone()
}

/// Place BT24-017 on `player`'s field using YAML-loaded DSL card data,
/// returning its PermanentHandle.  The caller is responsible for having
/// set up a runner with `from_dsl_yaml(YAML)`.
fn place_bt24_on_field(runner: &mut DebugRunner, player: u8) -> PermanentHandle {
    runner.place_on_field(player, "BT24-017", Some(0))
}

/// Enqueue and drain BT24-017's WhenDigivolving trigger for the given handle.
fn trigger_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Push `card_id` onto player `p`'s trash. The card must already exist in
/// `card_data` (added via `.add_card(...)`).
fn push_to_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card in card_data");
    let idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(digimon_engine::card_source::CardSource::new(
            data_idx, p, idx,
        ));
}

/// Count Token permanents on player `p`'s battle area.
fn token_count(runner: &DebugRunner, p: u8) -> usize {
    runner
        .game
        .player(p)
        .battle_area
        .iter()
        .filter(|perm: &&digimon_engine::permanent::Permanent| {
            perm.top_card().card_kind(&runner.game.card_data) == CardKind::Token
        })
        .count()
}

/// Resolve the currently-pending selection by submitting its first legal
/// action ID. Panics if no selection is pending.
fn resolve_first(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a selection must be pending");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("selection resolves");
}

// ─────────────────────────────────────────────────────────────────────────────
// § 1  Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// BT24-017 must compile with exactly 3 declarative (keyword-grant) clauses
/// and exactly 1 triggered (when_digivolving) clause.
#[test]
fn bt24_017_structural_three_keywords_one_triggered_clause() {
    let card = compiled_bt24_017();

    let declarative: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(d),
            _ => None,
        })
        .collect();

    assert_eq!(
        declarative.len(),
        3,
        "expected exactly 3 declarative (grant_keyword) clauses"
    );
    for d in &declarative {
        assert!(
            matches!(d, CompiledDeclarativeClause::GrantKeyword { .. }),
            "all declarative clauses must be GrantKeyword"
        );
    }

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "expected exactly 1 triggered clause");
    let t = triggered[0];
    assert_eq!(
        t.when,
        vec![CompiledTiming::WhenDigivolving],
        "triggered clause must fire on WhenDigivolving"
    );
    // The [When Digivolving] clause is mandatory, not optional.
    assert!(
        !t.optional,
        "when-digivolving clause is mandatory (not optional)"
    );
    assert!(
        !t.once_per_turn,
        "when-digivolving clause has no once-per-turn restriction"
    );
    assert_eq!(
        t.scope,
        CompiledScope::FaceUp,
        "triggered clause scope must be FaceUp (not Inherited)"
    );
}

/// BT24-017 must have a Digivolve alt-path (from Lv.5 at cost 3).
#[test]
fn bt24_017_has_lv5_digivolve_alt_path() {
    let card = compiled_bt24_017();
    assert!(
        card.alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::Digivolve),
        "BT24-017 must have at least one Digivolve alt-path"
    );
}

/// BT24-017 must declare Raid as a native keyword grant.
#[test]
fn bt24_017_grants_raid_keyword() {
    let card = compiled_bt24_017();
    let has_raid = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword, ..
        }) => keyword.eq_ignore_ascii_case("Raid"),
        _ => false,
    });
    assert!(has_raid, "BT24-017 must declare a GrantKeyword(Raid) clause");
}

/// BT24-017 must declare Progress as a native keyword grant.
#[test]
fn bt24_017_grants_progress_keyword() {
    let card = compiled_bt24_017();
    let has_progress = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword, ..
        }) => keyword.eq_ignore_ascii_case("Progress"),
        _ => false,
    });
    assert!(
        has_progress,
        "BT24-017 must declare a GrantKeyword(Progress) clause"
    );
}

/// BT24-017 must declare Piercing as a native keyword grant.
#[test]
fn bt24_017_grants_piercing_keyword() {
    let card = compiled_bt24_017();
    let has_piercing = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword, ..
        }) => keyword.eq_ignore_ascii_case("Piercing"),
        _ => false,
    });
    assert!(
        has_piercing,
        "BT24-017 must declare a GrantKeyword(Piercing) clause"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2  [When Digivolving] — delete step installs OppField selection
// ─────────────────────────────────────────────────────────────────────────────

/// Positive path: after triggering BT24-017's WhenDigivolving effect with an
/// opponent Digimon on the field, a pending OppField selection installs (the
/// "delete lowest-DP Digimon" prompt).
#[test]
fn bt24_017_when_digivolving_installs_opp_field_selection() {
    let mut opp = make_test_card("OPP-DIGIMON", "OppDigimon");
    opp.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-017 YAML loads")
        .add_card(opp)
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-DIGIMON", None);

    trigger_when_digivolving(&mut runner, bt24_handle);

    let kind = runner
        .pending_kind()
        .expect("pending selection after WhenDigivolving");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "WhenDigivolving should install an OppField selection for the delete step"
    );
}

/// Negative path: if the opponent has NO Digimon, the delete step finds no
/// valid target.  The game must not panic and no selection installs.
#[test]
fn bt24_017_when_digivolving_no_opp_digimon_does_not_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-017 YAML loads")
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);

    // Should not panic even with no valid targets.
    trigger_when_digivolving(&mut runner, bt24_handle);
    assert!(
        runner.game.pending_selection.is_none(),
        "no delete selection installs when the opponent has no Digimon"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3  Lowest-DP target filter (G-PRED-DP-LTE RESOLVED — `selector: lowest_dp`)
// ─────────────────────────────────────────────────────────────────────────────

/// When multiple opponent Digimon are on the field the delete prompt must
/// offer ONLY the lowest-DP Digimon.
#[test]
fn bt24_017_delete_targets_only_lowest_dp_digimon() {
    let mut low_dp = make_test_card("OPP-LOW", "OppLow");
    low_dp.dp = Some(3000);
    let mut high_dp = make_test_card("OPP-HIGH", "OppHigh");
    high_dp.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(low_dp)
        .add_card(high_dp)
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);

    trigger_when_digivolving(&mut runner, bt24_handle);

    let view = runner
        .pending_selection_view()
        .expect("selection installed");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the lowest-DP (3000) Digimon should be a valid delete target"
    );
}

/// Negative branch for the lowest-DP filter: when two opponent Digimon TIE
/// for the lowest DP, BOTH must be offered — the choice surfaces, the engine
/// does not auto-pick (no-approximations Working Rule §17).
#[test]
fn bt24_017_delete_offers_all_tied_lowest_dp_digimon() {
    let mut tied_a = make_test_card("OPP-TIE-A", "OppTieA");
    tied_a.dp = Some(4000);
    let mut tied_b = make_test_card("OPP-TIE-B", "OppTieB");
    tied_b.dp = Some(4000);
    let mut high = make_test_card("OPP-BIG", "OppBig");
    high.dp = Some(10000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(tied_a)
        .add_card(tied_b)
        .add_card(high)
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-TIE-A", None);
    runner.place_on_field(1, "OPP-TIE-B", None);
    runner.place_on_field(1, "OPP-BIG", None);

    trigger_when_digivolving(&mut runner, bt24_handle);

    let view = runner
        .pending_selection_view()
        .expect("selection installed");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both 4000-DP Digimon tie for lowest and must both be offered; the \
         10000-DP one must be excluded"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4  Full sequence — delete → trash return → 2 tokens → DP boost
//      (G-ZONE-TRASH-TO-DECK RESOLVED — `return_trash_list_to_deck_bottom`)
// ─────────────────────────────────────────────────────────────────────────────

/// Full positive sequence: opponent Digimon deleted, 2 trash cards returned
/// to the bottom of the opponent's deck, 2 Petrification Tokens spawn, and
/// BT24-017 gains +2000 DP per opponent Digimon until the opponent's turn ends.
#[test]
fn bt24_017_full_sequence_two_trash_two_tokens_dp_boost() {
    let mut opp_digimon = make_test_card("OPP-D", "OppD");
    opp_digimon.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(opp_digimon)
        .add_card(make_test_card("TRASH-A", "TrashA"))
        .add_card(make_test_card("TRASH-B", "TrashB"))
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-D", None);

    // Populate the opponent's trash with 2 cards. After the delete step the
    // deleted OPP-D Digimon also lands in the opponent's trash, so by the
    // time the return cost evaluates the opponent has 3 trash cards.
    push_to_trash(&mut runner, 1, "TRASH-A");
    push_to_trash(&mut runner, 1, "TRASH-B");
    let opp_deck_before = runner.game.player(1).deck.len();

    trigger_when_digivolving(&mut runner, bt24_handle);

    // Step 1 — resolve the OppField delete selection.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "delete prompt must install first"
    );
    resolve_first(&mut runner);
    assert_eq!(runner.battle_area_size(1), 0, "opp Digimon deleted");
    // The deleted Digimon is now in the opponent's trash (2 pushed + 1 = 3).
    assert_eq!(
        runner.trash_size(1),
        3,
        "deleted opp Digimon joins the 2 pre-existing trash cards"
    );

    // Step 2 — the trash count-capped multi-select (driven by the ACTIVE
    // player on behalf of the opponent's trash).
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 })
        ),
        "trash multi-select (max 2) must follow the delete; got {:?}",
        runner.pending_kind()
    );
    // The active player (player 0) selects from the opponent's trash.
    assert_eq!(
        runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player,
        0,
        "the active player picks which opponent trash cards to return"
    );
    resolve_first(&mut runner); // first trash pick
    resolve_first(&mut runner); // second trash pick — auto-commits at max=2

    // After `return_trash_list_to_deck_bottom`: exactly 2 of the 3 trash
    // cards moved to the bottom of the opponent's deck; 1 remains.
    assert_eq!(
        runner.trash_size(1),
        1,
        "exactly 2 of the 3 trash cards returned; 1 remains in trash"
    );
    assert_eq!(
        runner.game.player(1).deck.len(),
        opp_deck_before + 2,
        "2 cards returned to the bottom of the opponent's deck"
    );

    // 2 Petrification Tokens on the opponent's field.
    assert_eq!(
        token_count(&runner, 1),
        2,
        "2 Petrification Tokens must spawn on the opponent's field"
    );

    // +2000 per opponent Digimon. After the delete there are 2 opponent
    // Digimon (the 2 Petrification Tokens) → +4000.
    assert_eq!(
        runner.effective_dp(bt24_handle),
        Some(11000 + 4000),
        "+2000 × 2 opponent Digimon (the 2 Tokens) = +4000 boost"
    );

    // The boost is scoped `end_of_opponents_turn`. It is player 0's turn now;
    // after player 0's turn ends and the opponent's turn ends the modifier
    // expires.
    runner.end_turn(); // -> opponent's (player 1's) turn
    runner.end_turn(); // opponent's turn ends -> back to player 0
    assert_eq!(
        runner.effective_dp(bt24_handle),
        Some(11000),
        "DP boost expires at the end of the opponent's turn"
    );
}

/// A Petrification Token must carry its printed stats: White, 3000 DP, Token
/// kind. (Sanity-check the reused token registry entry.)
#[test]
fn bt24_017_petrification_tokens_have_printed_stats() {
    use digimon_engine::enums::CardColor;

    let mut opp_digimon = make_test_card("OPP-D", "OppD");
    opp_digimon.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(opp_digimon)
        .add_card(make_test_card("TRASH-A", "TrashA"))
        .add_card(make_test_card("TRASH-B", "TrashB"))
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-D", None);
    push_to_trash(&mut runner, 1, "TRASH-A");
    push_to_trash(&mut runner, 1, "TRASH-B");

    trigger_when_digivolving(&mut runner, bt24_handle);
    resolve_first(&mut runner); // delete
    resolve_first(&mut runner); // trash pick 1
    resolve_first(&mut runner); // trash pick 2

    let tokens: Vec<&digimon_engine::permanent::Permanent> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_kind(&runner.game.card_data) == CardKind::Token)
        .collect();
    assert_eq!(tokens.len(), 2, "2 tokens spawned");
    for tok in tokens {
        let card = tok.top_card();
        assert_eq!(
            card.dp(&runner.game.card_data),
            Some(3000),
            "Petrification Token DP is 3000"
        );
        let colors = &runner.game.card_data[card.data_index].colors;
        assert!(
            colors.contains(&CardColor::White),
            "Petrification Token is White"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5  Cost gate — opponent with <2 trash cannot pay "by returning 2"
// ─────────────────────────────────────────────────────────────────────────────

/// When the opponent has FEWER than 2 trash cards the "by returning 2" cost
/// cannot be paid: the count-capped multi-select (`min: 2`) silently no-ops,
/// no tokens spawn, and BT24-017 gains NO DP boost.
///
/// Setup: the opponent starts with 0 trash cards. The delete step puts the
/// deleted Digimon into the opponent's trash (count = 1), which is still
/// below the required 2, so the cost remains unpayable. This mirrors DCGO's
/// `if (Enemy.TrashCards.Count >= 2)` gate evaluated AFTER the delete.
#[test]
fn bt24_017_less_than_two_trash_skips_token_play_and_dp_boost() {
    let mut opp_digimon = make_test_card("OPP-D", "OppD");
    opp_digimon.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(opp_digimon)
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-D", None);
    // Opponent starts with 0 trash cards.
    assert_eq!(runner.trash_size(1), 0, "opponent starts with empty trash");

    trigger_when_digivolving(&mut runner, bt24_handle);

    // Resolve the delete selection.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "delete prompt installs"
    );
    resolve_first(&mut runner);
    assert_eq!(runner.battle_area_size(1), 0, "opp Digimon deleted");
    // Only the deleted Digimon is in trash — 1 card, below the required 2.
    assert_eq!(
        runner.trash_size(1),
        1,
        "only the deleted Digimon is in the opponent's trash"
    );

    // No trash multi-select installs (min: 2 unmet — silent no-op).
    assert!(
        runner.game.pending_selection.is_none(),
        "the return-2 cost cannot be paid with only 1 trash card; no \
         selection installs"
    );

    // The single trash card is untouched.
    assert_eq!(
        runner.trash_size(1),
        1,
        "the lone trash card stays in trash — cost was not paid"
    );

    // No tokens, no DP boost.
    assert_eq!(
        token_count(&runner, 1),
        0,
        "no tokens when the opponent cannot pay the return-2 cost"
    );
    assert_eq!(
        runner.effective_dp(bt24_handle),
        Some(11000),
        "no DP boost when the return-2 cost cannot be paid"
    );
}

/// Cost gate, exact-boundary positive case: with EXACTLY 2 trash cards the
/// cost is payable and the full token + DP-boost tail fires. This is the
/// positive partner of `bt24_017_less_than_two_trash_skips_token_play_and_dp_boost`.
#[test]
fn bt24_017_exactly_two_trash_pays_cost_and_fires_tail() {
    let mut opp_digimon = make_test_card("OPP-D", "OppD");
    opp_digimon.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(opp_digimon)
        .add_card(make_test_card("TRASH-A", "TrashA"))
        .add_card(make_test_card("TRASH-B", "TrashB"))
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-D", None);
    push_to_trash(&mut runner, 1, "TRASH-A");
    push_to_trash(&mut runner, 1, "TRASH-B");

    trigger_when_digivolving(&mut runner, bt24_handle);
    resolve_first(&mut runner); // delete
    resolve_first(&mut runner); // trash pick 1
    resolve_first(&mut runner); // trash pick 2 (auto-commit)

    assert_eq!(
        token_count(&runner, 1),
        2,
        "with exactly 2 trash the cost is paid and 2 tokens spawn"
    );
    assert_eq!(
        runner.effective_dp(bt24_handle),
        Some(11000 + 4000),
        "DP boost fires once the cost is paid"
    );
}

/// When the opponent has MORE than 2 trash cards the cost is still exactly
/// 2 (`max: 2`): only 2 cards are returned, the rest stay in trash, and the
/// tail fires normally. (4 cards pushed + 1 deleted Digimon = 5 trash cards
/// when the return cost evaluates; 2 returned → 3 remain.)
#[test]
fn bt24_017_more_than_two_trash_returns_exactly_two() {
    let mut opp_digimon = make_test_card("OPP-D", "OppD");
    opp_digimon.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(opp_digimon)
        .add_card(make_test_card("TR-A", "TrA"))
        .add_card(make_test_card("TR-B", "TrB"))
        .add_card(make_test_card("TR-C", "TrC"))
        .add_card(make_test_card("TR-D", "TrD"))
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-D", None);
    for id in ["TR-A", "TR-B", "TR-C", "TR-D"] {
        push_to_trash(&mut runner, 1, id);
    }
    let opp_deck_before = runner.game.player(1).deck.len();

    trigger_when_digivolving(&mut runner, bt24_handle);
    resolve_first(&mut runner); // delete
    // 4 pushed + the deleted OPP-D = 5 trash cards.
    assert_eq!(runner.trash_size(1), 5, "4 pushed + deleted Digimon = 5");

    // The multi-select must cap at max=2 even though 5 trash cards exist.
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 })
        ),
        "the return cost caps at exactly 2 even with 5 trash cards"
    );
    resolve_first(&mut runner); // trash pick 1
    resolve_first(&mut runner); // trash pick 2 (auto-commit at max)

    assert_eq!(
        runner.trash_size(1),
        3,
        "exactly 2 of the 5 trash cards returned; 3 remain in trash"
    );
    assert_eq!(
        runner.game.player(1).deck.len(),
        opp_deck_before + 2,
        "exactly 2 cards returned to the bottom of the opponent's deck"
    );
    assert_eq!(token_count(&runner, 1), 2, "2 tokens spawn");
}

// ─────────────────────────────────────────────────────────────────────────────
// § 6  DP modifier formula — battle_area count (unit test, no digivolve chain)
// ─────────────────────────────────────────────────────────────────────────────

/// Verify the DP modifier formula: +2000 × (opponent's battle_area count).
/// Exercises the `card_count_in_zone(opponent, battle_area)` formula directly
/// without running the full digivolve sequence.
#[test]
fn bt24_017_dp_modifier_formula_scales_with_opp_digimon_count() {
    use digimon_dsl::compiled::{
        CompiledFormula, CompiledPerSelector, CompiledPlayerRef, CompiledZone,
    };
    use digimon_engine::dsl_cards::formula_eval;
    use digimon_engine::effect_context::EffectContext;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Src"))
        .add_card(make_test_card("OPP-A", "OppA"))
        .add_card(make_test_card("OPP-B", "OppB"))
        .add_card(make_test_card("OPP-C", "OppC"))
        .memory(20)
        .start();

    let target = runner.place_on_field(0, "SRC", None);
    runner.place_on_field(1, "OPP-A", None);
    runner.place_on_field(1, "OPP-B", None);
    runner.place_on_field(1, "OPP-C", None);

    // The YAML's DP-modifier formula: base=0, per=card_count_in_zone(opp, battle_area), delta=2000.
    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::CardCountInZoneScoped {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::BattleArea,
        },
        delta: 2000,
    };

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let value = formula_eval::evaluate(&formula, &ctx, target);

    assert_eq!(value, 6000, "+2000 × 3 opp Digimon must equal 6000");
}

/// Formula yields 0 when the opponent has no Digimon.
#[test]
fn bt24_017_dp_modifier_formula_zero_with_no_opp_digimon() {
    use digimon_dsl::compiled::{
        CompiledFormula, CompiledPerSelector, CompiledPlayerRef, CompiledZone,
    };
    use digimon_engine::dsl_cards::formula_eval;
    use digimon_engine::effect_context::EffectContext;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Src"))
        .memory(20)
        .start();

    let target = runner.place_on_field(0, "SRC", None);

    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::CardCountInZoneScoped {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::BattleArea,
        },
        delta: 2000,
    };

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let value = formula_eval::evaluate(&formula, &ctx, target);

    assert_eq!(value, 0, "+2000 × 0 opp Digimon must equal 0");
}
