//! BT3-109 Back for Revenge! — Option, Purple, Cost 2.
//!
//! # Card text (card image / cards.json)
//!   [Main] 1 of your Digimon gains "[On Deletion] Play this card without paying
//!     its memory cost. Any [On Play] effects on Digimon played with this effect
//!     don't 'activate'" for the turn.
//!   Security: — (none).
//!
//! # What this exercises
//! - `grant_triggered_effect` granting an [On Deletion] body to a chosen own Digimon.
//! - The granted body replays the just-deleted carrier from trash via the
//!   `event_card` binding (= `DeletedObjectSnapshot.top_card`) + `play_from_trash_free`
//!   (G-DSL-DELETED-SELF-TRASH-BINDING closed by accepting a card-handle binding).
//! - Judge-quiz Q21 activation-site rule: replaying the carrier from trash leaves
//!   the trash, suppressing the carrier's remaining [On Deletion] draw bundle.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

fn pad(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

/// P0 holds Back for Revenge! in hand; Eyesmon: Scatter Mode (BT7-069, own
/// [On Deletion] <Draw 3>) is the deletion subject. Deep filler deck so a draw
/// would visibly shrink it.
fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT3-109")
        .expect("BT3-109 YAML must be present in the embedded DSL pack")
        .dsl_card("BT7-069")
        .expect("BT7-069 Eyesmon: Scatter Mode loads")
        .add_card(pad("PAD"))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 10])
        .hand(0, &["BT3-109"])
        .memory(5)
        .start()
}

#[test]
fn bt3_109_yaml_compiles_and_is_in_pack() {
    let r = runner();
    assert!(
        r.compiled_card("BT3-109").is_some(),
        "BT3-109 must be present in the embedded DSL pack"
    );
}

/// Core scenario: grant Eyesmon the [On Deletion] self-replay, delete it, and
/// confirm the granted body replays it from trash. The carrier leaving the trash
/// suppresses its own [On Deletion] <Draw 3>.
#[test]
fn bt3_109_grants_on_deletion_self_replay_from_trash() {
    let mut r = runner();
    let eyesmon: PermanentHandle = r.place_on_field(0, "BT7-069", Some(0));
    assert_eq!(r.battle_area_size(0), 1, "Eyesmon staged");

    // [Main] Back for Revenge! → grant Eyesmon the [On Deletion] replay.
    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "BT3-109 [Main] must fire");
    let _ = r.auto_resolve();
    assert_eq!(
        r.battle_area_size(0),
        1,
        "granting does not move Eyesmon off the field"
    );

    let deck_before = r.deck_size(0);

    // Delete Eyesmon → its [On Deletion] bundle fires: own <Draw 3> + the granted
    // self-replay. Resolve to completion.
    r.game
        .delete_permanent_with_cause(eyesmon, ReplacementCause::OwnEffect);
    let _ = r.auto_resolve();

    // The granted replay returns Eyesmon to the field from trash.
    assert_eq!(
        r.battle_area_size(0),
        1,
        "Eyesmon is replayed from trash by the granted [On Deletion]"
    );
    // Carrier left the trash before its own <Draw 3> could resolve ⇒ 0 draws.
    assert_eq!(
        deck_before - r.deck_size(0),
        0,
        "replaying the carrier from trash suppresses its own [On Deletion] <Draw 3>"
    );
}
