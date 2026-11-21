//! BT11-042 Angewomon — inherited [Opponent's Turn] Blocker aura.
//!
//! Printed (official Bandai DB bundle `data/card_bundles/BT11-042.md`):
//! "[Opponent's Turn] While you have a purple Digimon in play, all of your
//! Digimon with [Angel], [Archangel], or [Fallen Angel] in their traits gain
//! <Blocker>." "... in their traits" is a SUBSTRING reading (DCGO
//! `CardSource.HasAngelTraits` -> `ContainsTraits`), so [Three Great Angels]
//! and [Mini Angel] Digimon qualify too. Synthetic boards only.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

const ID: &str = "BT11-042";

fn digimon(id: &str, traits: &[&str], color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT11-042 YAML compiles")
        .add_card(digimon("CARRIER", &["Plain"], CardColor::Purple))
        .add_card(digimon("TGA", &["Three Great Angels"], CardColor::White))
        .add_card(digimon("MINI-ANGEL", &["Mini Angel"], CardColor::White))
        .add_card(digimon("ARCH", &["Archangel"], CardColor::White))
        .add_card(digimon("FALLEN", &["Fallen Angel"], CardColor::Purple))
        .add_card(digimon("PLAIN", &["Machine"], CardColor::White))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
}

#[test]
fn bt11_042_inherited_blocker_matches_angel_as_substring_of_any_trait() {
    for (id, expect) in [
        ("TGA", true),
        ("MINI-ANGEL", true),
        ("ARCH", true),
        ("FALLEN", true),
        ("PLAIN", false),
    ] {
        let mut runner = base().start();
        // Angewomon is a digivolution source of the purple carrier, so the
        // inherited effect is live and "you have a purple Digimon in play".
        runner.place_stack(0, &[ID, "CARRIER"]);
        let target = runner.place_on_field(0, id, Some(0));
        runner.game.turn_player_idx = 1; // opponent's turn
        runner.game.tick_declarative_effects();
        assert_eq!(
            runner.game.has_keyword(target, Keyword::Blocker),
            expect,
            "{id}: Angewomon's inherited Blocker aura on the opponent's turn"
        );
    }
}

#[test]
fn bt11_042_inherited_blocker_is_inactive_on_your_own_turn() {
    let mut runner = base().start();
    runner.place_stack(0, &[ID, "CARRIER"]);
    let target = runner.place_on_field(0, "TGA", Some(0));
    runner.game.turn_player_idx = 0;
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(target, Keyword::Blocker),
        "[Opponent's Turn] aura must not apply on your own turn"
    );
}
