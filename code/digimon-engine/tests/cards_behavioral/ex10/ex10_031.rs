//! EX10-031 DarkKnightmon — Digimon, Lv.5, Black/Purple, DP 7000, Cost 7.
//! Traits: [Dark Knight, Bagra Army, Twilight].
//!
//! # Card text (card image — verified)
//!
//! [On Play][When Digivolving] Until your opponent's turn ends, their
//! <De-Digivolve> effects don't affect 1 of your Digimon, and it gets
//! +3000 DP.
//!
//! [All Turns][Once Per Turn] When this Digimon would leave the battle area,
//! you may play 1 play cost 4 or lower card from its digivolution cards
//! without paying the cost. — ***OMITTED (PARTIAL)***: triggered (non-
//! replacement) observer in the would-leave window with stack access has no
//! DSL vocabulary (G-DSL-WOULD-LEAVE-TRIGGERED-OBSERVER, qa/dsl-vocab-gaps.md).
//!
//! DigiXros -1: [SkullKnightmon] × [DeadlyAxemon]
//!
//! Inherited: [Opponent's Turn][Once Per Turn] When one of your opponent's
//! Digimon attacks, you may change the attack target to this Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_031.cs

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::selection::TriggerSource;

fn make_ally(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Ally"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

#[test]
fn ex10_031_compiles_with_three_alt_paths() {
    let r = DebugRunner::builder()
        .dsl_card("EX10-031")
        .expect("EX10-031 YAML parses and compiles")
        .build();
    let card = r.compiled_card("EX10-031").expect("present in pack");
    assert_eq!(
        card.alt_paths.len(),
        3,
        "2 digivolve paths + 1 digixros path"
    );
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::DigiXros)),
        "the [DigiXros -1: SkullKnightmon × DeadlyAxemon] path must compile"
    );
}

#[test]
fn ex10_031_has_inherited_opponent_attack_redirect_clause() {
    let r = DebugRunner::builder()
        .dsl_card("EX10-031")
        .expect("EX10-031 loads")
        .build();
    let card = r.compiled_card("EX10-031").expect("present in pack");
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnOpponentAttack)
                    && t.once_per_turn
                    && t.optional
        )),
        "the inherited [Opp Turn][OPT] attack-redirect clause must compile"
    );
}

/// [On Play]: the selected Digimon gets +3000 DP and the opponent-scoped
/// De-Digivolve protection until the opponent's turn ends.
#[test]
fn ex10_031_on_play_shields_one_digimon_from_de_digivolve_and_grants_3000_dp() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-031")
        .expect("EX10-031 loads")
        .add_card(make_ally("DK-ALLY"))
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let ally = r.place_on_field(0, "DK-ALLY", Some(0));
    let dk = r.place_on_field(0, "EX10-031", Some(0));
    let ally_dp_before = r.dp_of(ally).expect("ally DP readable");

    r.fire_on_play(0, dk.index as usize);

    // Mandatory pick of 1 of your Digimon — target the ally.
    let view = r
        .pending_selection_view()
        .expect("the [On Play] shield target select must surface");
    let want = digimon_engine::action::space::encode_attack(ally.player as u16, ally.index as u16);
    assert!(
        view.valid_action_ids.contains(&want),
        "the ally must be a valid shield target"
    );
    r.execute_action(0, want).expect("pick the ally");
    let _ = r.auto_resolve();

    assert!(
        r.game
            .modifiers
            .has(ally, ModifierType::CannotBeDeDigivolved),
        "the picked Digimon must have the (opponent-scoped) De-Digivolve protection"
    );
    assert_eq!(
        r.dp_of(ally),
        Some(ally_dp_before + 3000),
        "the picked Digimon must get +3000 DP"
    );
}

/// [When Digivolving] fires the same clause.
#[test]
fn ex10_031_when_digivolving_fires_the_shield_clause() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-031")
        .expect("EX10-031 loads")
        .add_card(make_ally("DK-WD-ALLY"))
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let _ally = r.place_on_field(0, "DK-WD-ALLY", Some(0));
    let dk = r.place_on_field(0, "EX10-031", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(dk));
    r.game.drain_effect_queue();

    assert!(
        r.pending_selection().is_some(),
        "[When Digivolving] must surface the shield target select"
    );
}
