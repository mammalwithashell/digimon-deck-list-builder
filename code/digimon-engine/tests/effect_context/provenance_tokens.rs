use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, CostDelta, Zone};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::trigger_context::EventSubject;

fn digimon(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(3000);
    card
}

#[test]
fn effect_play_provenance_token_survives_field_shift_and_zone_move() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("LEFT", 3))
        .add_card(digimon("RIGHT", 3))
        .add_card(digimon("SPAWN", 3))
        .hand(0, &["SPAWN"])
        .memory(5)
        .start();

    let left = runner.place_on_field(0, "LEFT", None);
    let _right = runner.place_on_field(0, "RIGHT", None);
    let source = runner.game.player(0).hand[0].handle();
    let (spawned, token) = {
        let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
        ctx.play_from_hand_free_with_provenance(0, 0)
            .expect("effect play succeeds")
    };
    assert_eq!(spawned.index, 2);

    runner.game.delete_permanent_with_effects(left);
    assert_eq!(
        runner.game.resolve_provenance_token(token),
        Some(EventSubject::Permanent(PermanentHandle {
            player: 0,
            index: 1,
        })),
        "token should resolve to the same spawned permanent after battle-area indices shift"
    );

    let shifted = PermanentHandle {
        player: 0,
        index: 1,
    };
    let moved = runner
        .game
        .return_to_hand(shifted)
        .expect("returns top card");
    assert_eq!(
        runner.game.resolve_provenance_token(token),
        Some(EventSubject::Card {
            card: moved,
            zone: Zone::Hand,
        }),
        "token should follow the played card after it leaves the battle area"
    );
}

#[test]
fn effect_digivolve_provenance_token_resolves_to_new_top_card() {
    let mut evo = digimon("EVO", 4);
    evo.evo_costs.push(digimon_engine::card_data::EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 0,
    });

    let mut runner = DebugRunner::builder()
        .add_card(digimon("BASE", 3))
        .add_card(evo)
        .hand(0, &["EVO"])
        .memory(5)
        .start();

    let base = runner.place_on_field(0, "BASE", None);
    let source = runner.game.player(0).hand[0].handle();
    let (evolved, token) = {
        let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
        ctx.effect_initiated_digivolve_with_provenance(0, 0, base, CostDelta::Free, true)
            .expect("effect digivolve succeeds")
    };

    assert_eq!(evolved, base);
    assert_eq!(
        runner.game.resolve_provenance_token(token),
        Some(EventSubject::Permanent(base)),
        "digivolve provenance should resolve through the new top card's stack"
    );

    let moved = runner.game.return_to_hand(base).expect("returns new top");
    assert_eq!(
        moved, source,
        "the token is keyed to the effect-digivolved top card"
    );
    assert_eq!(
        runner.game.resolve_provenance_token(token),
        Some(EventSubject::Card {
            card: CardHandle(source.0),
            zone: Zone::Hand,
        })
    );
}

#[test]
fn effect_dna_digivolve_provenance_token_resolves_to_result_top_card() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("A", 3))
        .add_card(digimon("B", 3))
        .add_card(digimon("DNA", 4))
        .hand(0, &["DNA"])
        .memory(5)
        .start();

    let a = runner.place_on_field(0, "A", None);
    let b = runner.place_on_field(0, "B", None);
    let source = runner.game.player(0).hand[0].handle();
    let (merged, token) = {
        let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
        ctx.effect_initiated_dna_digivolve_with_provenance(a, b, source, 0, true)
            .expect("effect DNA digivolve succeeds")
    };

    assert_eq!(
        runner.game.resolve_provenance_token(token),
        Some(EventSubject::Permanent(merged)),
        "DNA provenance should resolve through the result top card"
    );
}
