//! BT16-077 Dinobeemon — Lv.5 Purple/Red, DP 8000, Cost 8.
//! The inherited <Partition> copy under Imperialdramon: Dragon Mode is the
//! judge-quiz Q30 board element; the interruptive Partition machinery is
//! pinned by `judge_quiz/c_declare_then_pay.rs` and
//! `keyword_phase_d/partition.rs`.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

#[test]
fn bt16_077_compiles_with_dna_path_and_dual_partition() {
    let r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 YAML parses and compiles")
        .build();
    let card = r.compiled_card("BT16-077").expect("present in pack");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::DnaDigivolve)),
        "the DNA Digivolution (purple Lv.4 + red Lv.4 / 0) path compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving) && t.optional
        )),
        "[When Digivolving] (DNA-gated trash-play) + Rush clause compiles"
    );
    // The inherited Partition copy is a distinct inherited-scope clause.
    let inherited_partitions = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(
                    digimon_dsl::compiled::CompiledDeclarativeClause::Partition { scope, .. }
                ) if *scope == CompiledScope::Inherited
            )
        })
        .count();
    assert_eq!(inherited_partitions, 1, "one inherited <Partition> clause");
}

#[test]
fn bt16_077_grants_raid_and_partition_on_field() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .build();
    let h = r.place_on_field(0, "BT16-077", None);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Raid), "<Raid> granted");
    assert!(
        r.game.has_keyword(h, Keyword::Partition),
        "<Partition> granted"
    );
}

/// [When Digivolving] on an ORDINARY (non-DNA) digivolve: only the trash-play
/// half is gated by "If DNA digivolving"; the "Then, 1 of your Digimon may
/// gain <Rush> ... and attack a player" half still resolves.
///
/// Citations: official Bandai Q&A for BT16-077 ("Even if this Digimon didn't
/// DNA digivolve, this card's [When Digivolving] effect can give 1 of your
/// Digimon <Rush> and that Digimon can attack" — data/card_bundles/BT16-077.md);
/// DCGO `BT16_077.cs:182` gates only the trash-play on `IsJogress`, while the
/// Rush pick at `:231` is unconditional. Exam BT16-077#effect#1 step 15.
#[test]
fn bt16_077_non_dna_digivolve_still_offers_rush_attack_but_not_trash_play() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::{SelectionKind, TriggerSource};

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .memory(3)
        .start();
    // A legal trash-play target (Lv.5 [Free] Digimon) so an ungated trash
    // pick WOULD surface if the DNA gate were missing.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BT16-077")
        .expect("BT16-077 card data");
    let next = r.game.next_card_index();
    r.game.players[0].trash.push(CardSource::new(data_idx, 0, next));

    let h = r.place_on_field(0, "BT16-077", Some(0));
    // Non-DNA WhenDigivolving (no DNA context).
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();

    assert!(
        r.pending_selection().is_some() && r.pending_is_optional(),
        "the optional [When Digivolving] prompt must open on a non-DNA digivolve \
         (the Rush/attack half is not DNA-gated)"
    );
    r.accept_optional_trigger().expect("accept [When Digivolving]");
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OwnField),
        "non-DNA: the trash-play is skipped; the next pick is the Rush target"
    );
}
