//! BT12-057 Quartzmon — Digimon, Lv.7, Green, DP 15000, Cost 16.
//! Traits: Unidentified. Attribute: Unknown.
//!
//! # Card text (card image + judge-quiz PDF p.5 + cards.json — agree)
//! `[When Digivolving] Suspend all other Digimon and all Tamers. Then, for`
//! `  every 2 suspended Digimon and/or Tamers, gain 1 memory.`
//! `[All Turns] All other Digimon and Tamers don't unsuspend.`
//! `[When Attacking] Suspend 1 of your opponent's Digimon or Tamers. Then,`
//! `  trash the top card of your opponent's security stack for every 5`
//! `  suspended Digimon and Tamers.`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Green/BT12_057.cs
//!   - [When Digivolving]: foreach all players' battle-area permanents ≠ self
//!     that are Digimon/Tamer → Tap; then AddMemory(count(ALL suspended
//!     Digimon+Tamers) / 2).
//!   - [All Turns]: CantUnsuspendStaticEffect over battle-area Digimon/Tamer
//!     ≠ self (both players).
//!   - [When Attacking]: select 1 opp Digimon/Tamer → Tap; then
//!     IDestroySecurity(opponent, count(ALL suspended)/5, fromTop) — the
//!     security trash runs even when the pick had no candidates.
//!
//! Judge-quiz consumer: Q3 (Quartzmon is the digivolve target Puppetmon's
//! restriction would forbid — outside the breeding area).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT12-057")
}

fn digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.dp = None;
    c.level = None;
    c
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt12_057_metadata_is_green_lv7_15000() {
    let card = compiled();
    assert_eq!(card.card, "BT12-057");
    assert_eq!(card.name, "Quartzmon");
    assert_eq!(card.level, Some(7));
    assert_eq!(card.dp, Some(15000));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Green));
    // Two digivolve alt-paths: the printed Lv.6 Green/6 route + the
    // "Lv.5 w/＜Save＞ in text: Cost 9" special route.
    assert_eq!(card.alt_paths.len(), 2);
}

// ─── SECTION 2 — [When Digivolving] suspend all others + memory ÷2 ──────────

#[test]
fn bt12_057_when_digivolving_suspends_all_others_and_gains_memory() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT12-057")
        .expect("BT12-057 in pack")
        .add_card(digimon("OWN-D"))
        .add_card(tamer("OWN-T"))
        .add_card(digimon("OPP-D"))
        .add_card(tamer("OPP-T"))
        .memory(0)
        .start();
    let quartz = r.place_on_field(0, "BT12-057", Some(0));
    let own_d = r.place_on_field(0, "OWN-D", Some(0));
    let own_t = r.place_on_field(0, "OWN-T", Some(0));
    let opp_d = r.place_on_field(1, "OPP-D", Some(0));
    let opp_t = r.place_on_field(1, "OPP-T", Some(0));
    let memory_before = r.memory();

    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(quartz),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    for h in [own_d, own_t, opp_d, opp_t] {
        assert!(
            r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended,
            "every OTHER Digimon/Tamer (both players) is suspended"
        );
    }
    assert!(
        !r.game.players[0].battle_area[quartz.index as usize].is_suspended,
        "Quartzmon itself is NOT suspended"
    );
    // 4 suspended Digimon/Tamers → floor(4 / 2) = 2 memory.
    assert_eq!(
        r.memory(),
        memory_before + 2,
        "for every 2 suspended Digimon and/or Tamers, gain 1 memory"
    );
}

// ─── SECTION 3 — [All Turns] all other Digimon and Tamers don't unsuspend ──

#[test]
fn bt12_057_aura_blocks_other_unsuspends_at_turn_start() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT12-057")
        .expect("BT12-057 in pack")
        .add_card(digimon("OWN-D"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(5)
        .start();
    r.skip_mulligan();
    let quartz = r.place_on_field(0, "BT12-057", Some(0));
    let own_d = r.place_on_field(0, "OWN-D", Some(0));
    r.game.tick_declarative_effects();

    assert!(
        r.game.modifiers.has(own_d, ModifierType::CannotUnsuspend),
        "the aura installs CannotUnsuspend on the OTHER Digimon"
    );
    assert!(
        !r.game.modifiers.has(quartz, ModifierType::CannotUnsuspend),
        "Quartzmon itself is exempt ('all OTHER')"
    );

    // Behavioral: suspend both; rotate to P0's next turn; the other Digimon
    // must STAY suspended while Quartzmon unsuspends.
    r.game.players[0].battle_area[quartz.index as usize].is_suspended = true;
    r.game.players[0].battle_area[own_d.index as usize].is_suspended = true;
    r.end_turn(); // -> P1's turn
    let _ = r.auto_resolve();
    r.end_turn(); // -> back to P0's turn (unsuspend phase runs)
    let _ = r.auto_resolve();

    assert!(
        r.game.players[0].battle_area[own_d.index as usize].is_suspended,
        "the other Digimon does NOT unsuspend at turn start"
    );
    assert!(
        !r.game.players[0].battle_area[quartz.index as usize].is_suspended,
        "Quartzmon unsuspends normally"
    );
}

// ─── SECTION 4 — [When Attacking] suspend 1 opp + trash security ÷5 ─────────

#[test]
fn bt12_057_when_attacking_suspends_one_and_trashes_security_per_five() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT12-057")
        .expect("BT12-057 in pack")
        .add_card(digimon("OWN-1"))
        .add_card(digimon("OWN-2"))
        .add_card(digimon("OPP-1"))
        .add_card(tamer("OPP-T"))
        .add_card(make_test_card("SEC", "Sec"))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    let quartz = r.place_on_field(0, "BT12-057", Some(0));
    // Stage 3 already-suspended own Digimon + the opponent pair.
    let own1 = r.place_on_field(0, "OWN-1", Some(0));
    let own2 = r.place_on_field(0, "OWN-2", Some(0));
    let opp1 = r.place_on_field(1, "OPP-1", Some(0));
    let oppt = r.place_on_field(1, "OPP-T", Some(0));
    for h in [own1, own2, opp1] {
        r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
    }
    // Quartzmon suspended too (it is attacking) → suspended count before the
    // pick: own1, own2, opp1, quartzmon = 4.
    r.game.players[0].battle_area[quartz.index as usize].is_suspended = true;
    let sec_before = r.security_count(1);

    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(quartz),
    );
    r.game.drain_effect_queue();

    // The mandatory pick targets the opponent's UNSUSPENDED Tamer or the
    // suspended Digimon — pick the Tamer (suspend it: 4 → 5 suspended).
    let view = r
        .pending_selection_view()
        .expect("suspend-1-opponent pick pending");
    let want = digimon_engine::action::space::encode_attack(0, oppt.index as u16);
    assert!(
        view.valid_action_ids.contains(&want) || view.valid_action_ids.len() == 2,
        "both opponent permanents (Digimon + Tamer) are legal picks; got {:?}",
        view.valid_action_ids
    );
    // Pick the Tamer if encodable; otherwise the highest action id (the
    // second field slot).
    let pick = if view.valid_action_ids.contains(&want) {
        want
    } else {
        *view.valid_action_ids.iter().max().unwrap()
    };
    r.execute_action(view.selecting_player, pick)
        .expect("suspend the opponent Tamer");
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1].battle_area[oppt.index as usize].is_suspended,
        "the picked opponent Tamer is suspended"
    );
    // 5 suspended Digimon+Tamers → floor(5/5) = 1 security trashed.
    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "one security card trashed for every 5 suspended Digimon and Tamers"
    );
}
