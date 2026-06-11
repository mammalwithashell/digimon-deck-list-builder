//! EX10-056 Bagramon — Digimon, Lv.6, Purple, DP 13000, Cost 13.
//! Traits: [Demon Lord, Bagra Army].
//!
//! # Card text (card image — verified)
//!
//! [On Play][When Digivolving] You may place 1 of your opponent's Digimon as
//! any of their other Digimon's bottom digivolution card or under any of
//! their Tamers. — ***OMITTED (PARTIAL)***: placing a battle-area PERMANENT
//! as a digivolution source has no DSL verb
//! (G-DSL-PLACE-PERMANENT-AS-SOURCE, qa/dsl-vocab-gaps.md).
//!
//! [All Turns][Once Per Turn] When any of your opponent's Digimon or Tamers
//! digivolve or effects place cards under them, by trashing any 2 of this
//! Digimon's digivolution cards, trash your opponent's top security card.
//! — The "digivolve" half is authored; the "effects place cards under them"
//! half is ***OMITTED (PARTIAL)***
//! (G-DSL-ON-CARD-PLACED-UNDER-TRIGGER, qa/dsl-vocab-gaps.md).
//!
//! DigiXros -2: 2 Digimon cards w/ [Bagra Army] trait
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Purple/EX10_056.cs

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

fn make_source(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Src"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c
}

fn make_opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Opp"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

#[test]
fn ex10_056_compiles_with_digixros_path_and_observer_clause() {
    let r = DebugRunner::builder()
        .dsl_card("EX10-056")
        .expect("EX10-056 YAML parses and compiles")
        .build();
    let card = r.compiled_card("EX10-056").expect("present in pack");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::DigiXros)),
        "the [DigiXros -2: 2 × Bagra Army Digimon] path must compile"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::OnDigivolve)
                    && t.once_per_turn
                    && t.optional
        )),
        "the [All Turns][Once Per Turn] opponent-digivolve observer must compile"
    );
}

/// [All Turns][OPT] (digivolve half): when an opponent Digimon digivolves,
/// trashing 2 of Bagramon's digivolution cards trashes the opponent's top
/// security card.
#[test]
fn ex10_056_opponent_digivolve_trash_2_sources_trashes_top_security() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-056")
        .expect("EX10-056 loads")
        .add_card(make_source("BAGRA-SRC1"))
        .add_card(make_source("BAGRA-SRC2"))
        .add_card(make_opp_digimon("BAGRA-OPP"))
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let bagramon = r.place_on_field(0, "EX10-056", Some(0));
    r.push_source(bagramon, "BAGRA-SRC1");
    r.push_source(bagramon, "BAGRA-SRC2");
    let opp = r.place_on_field(1, "BAGRA-OPP", Some(0));

    // Give the opponent 2 security cards.
    let sec_filler = make_test_card("BAGRA-SEC", "Sec Filler");
    r.game.card_data.push(sec_filler);
    let fi = r.game.card_data.len() - 1;
    for _ in 0..2 {
        let cs = CardSource::new(fi, 1, r.game.next_card_index());
        r.game.players[1].security.push(cs);
    }
    let sec_before = r.security_count(1);
    let sources_before = r.game.players[0].battle_area[bagramon.index as usize]
        .card_sources
        .len();

    // The opponent's Digimon digivolves.
    let card = r.game.players[1].battle_area[opp.index as usize]
        .top_card()
        .handle();
    r.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 1,
            permanent: opp,
            card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    r.game.drain_effect_queue();

    // Optional clause: pay by trashing 2 of Bagramon's digivolution cards.
    assert!(
        r.pending_selection().is_some(),
        "the observer must surface its source-trash cost picks"
    );
    r.auto_resolve().expect("the 2 source picks resolve");

    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "the opponent's top security card must be trashed"
    );
    assert_eq!(
        r.game.players[0].battle_area[bagramon.index as usize]
            .card_sources
            .len(),
        sources_before - 2,
        "exactly 2 of Bagramon's digivolution cards must be trashed as the cost"
    );
}

/// Negative: the observer is gated on the OPPONENT digivolving — one of your
/// own Digimon digivolving does not fire it.
#[test]
fn ex10_056_own_digivolve_does_not_fire_the_observer() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-056")
        .expect("EX10-056 loads")
        .add_card(make_source("BAGRA-N1"))
        .add_card(make_source("BAGRA-N2"))
        .add_card(make_opp_digimon("BAGRA-OWN"))
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let bagramon = r.place_on_field(0, "EX10-056", Some(0));
    r.push_source(bagramon, "BAGRA-N1");
    r.push_source(bagramon, "BAGRA-N2");
    let own = r.place_on_field(0, "BAGRA-OWN", Some(0));

    let card = r.game.players[0].battle_area[own.index as usize]
        .top_card()
        .handle();
    r.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 0,
            permanent: own,
            card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    r.game.drain_effect_queue();

    assert!(
        r.pending_selection().is_none(),
        "your own Digimon digivolving must NOT fire the opponent-gated observer"
    );
}
