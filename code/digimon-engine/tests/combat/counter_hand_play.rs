//! Phase 9 Task 3 — Counter window broadening.
//!
//! The Counter window now offers a union-zone selection over three
//! candidate shapes (spec §5):
//!   - Blast Digivolve candidates from the defender's hand (existing).
//!   - Hand Counter Options — Option cards in the defender's hand with
//!     `.counter()` effects, played through Phase 8's
//!     `play_option_from_hand` pipeline with a CounterEffect overlay.
//!   - Field Counter abilities — defender's battle-area Digimon with
//!     `.counter()` + `CounterEffect` timing effects, fired directly
//!     (no play-from-hand, no cost).
//!
//! See `docs/superpowers/specs/2026-04-21-combat-interrupt-completion-design.md`
//! §5 for the authoritative spec.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_attack, encode_digivolve, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

// ─── Fixture helpers ──────────────────────────────────────────────────

fn dgmn(id: &str, level: u8, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(dp),
        play_cost: 0,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// A blast-digivolve-capable card with an evo_cost matching the given base
/// level/color. Mirrors the helper in counter_interrupt.rs.
fn blast_card(id: &str, level: u8, base_level: u8, base_color: u8) -> CardData {
    let mut c = dgmn(id, level, 5000);
    c.evo_costs.push(EvoCost {
        card_color: base_color,
        level: base_level,
        memory_cost: 0,
    });
    c
}

fn option_card(id: &str, cost: u16, color: CardColor) -> CardData {
    let mut cd = make_test_card(id, id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = cost;
    cd.colors = vec![color];
    cd
}

// ─── Inline effect scaffolds ─────────────────────────────────────────

/// An Option card whose Counter body gains 3 memory. Uses `.counter()` +
/// `.timing(CounterEffect)` to register as a Counter-Option candidate;
/// also carries an `.option_main()` effect so Phase 8's pipeline disposes
/// it normally after the Counter body fires.
struct CounterOptionGainMemory {
    counter_witness: Arc<Mutex<u32>>,
    main_witness: Arc<Mutex<u32>>,
}
impl CardEffect for CounterOptionGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let counter_w = self.counter_witness.clone();
        let main_w = self.main_witness.clone();
        vec![
            // Counter body — fires during the counter window.
            Effect::on_play(card)
                .name("Counter body — gain 3 memory")
                .counter()
                .timing(EffectTiming::CounterEffect)
                .process(move |ctx| {
                    *counter_w.lock().unwrap() += 1;
                    ctx.gain_memory(3);
                })
                .build(),
            // OptionMain — no-op, just a sentinel witness.
            Effect::on_play(card)
                .name("OptionMain — witness")
                .option_main()
                .process(move |_ctx| {
                    *main_w.lock().unwrap() += 1;
                })
                .build(),
        ]
    }
}

/// Field-activated Counter ability: a Digimon on the defender's battle area
/// with `.counter()` + `.timing(CounterEffect)`. When the defender picks it
/// during the Counter window, the body fires directly — no card play, no
/// cost. Body gains 2 memory for witness.
struct FieldCounterAbility {
    witness: Arc<Mutex<u32>>,
}
impl CardEffect for FieldCounterAbility {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let w = self.witness.clone();
        vec![Effect::on_play(card)
            .name("Field counter ability")
            .counter()
            .timing(EffectTiming::CounterEffect)
            .process(move |ctx| {
                *w.lock().unwrap() += 1;
                ctx.gain_memory(2);
            })
            .build()]
    }
}

/// An Option Counter card whose body triggers another attack against the
/// opponent — used to exercise the chain-depth guard (Test 6).
struct CounterOptionRecursiveAttack {
    attacker: PermanentHandle,
    target: PermanentHandle,
    counter_witness: Arc<Mutex<u32>>,
}
impl CardEffect for CounterOptionRecursiveAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let atk = self.attacker;
        let tgt = self.target;
        let w = self.counter_witness.clone();
        vec![
            Effect::on_play(card)
                .name("Counter body — launches nested attack")
                .counter()
                .timing(EffectTiming::CounterEffect)
                .process(move |ctx| {
                    *w.lock().unwrap() += 1;
                    // Launch a nested attack. The chain-depth guard should
                    // suppress offering a further Counter window.
                    let _ = ctx.game.attack_digimon(atk, tgt, false);
                })
                .build(),
            Effect::on_play(card)
                .name("OptionMain — no-op")
                .option_main()
                .process(|_ctx| {})
                .build(),
        ]
    }
}

/// Shared witness for Test 5 — fires on the CounterEffect body of TWO
/// separate hand cards. Each card has its own instance of this struct
/// pointing at the same Arc so we can assert fire-count.
struct CounterOptionSharedWitness {
    witness: Arc<Mutex<u32>>,
}
impl CardEffect for CounterOptionSharedWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let w = self.witness.clone();
        vec![
            Effect::on_play(card)
                .name("Counter body — shared witness")
                .counter()
                .timing(EffectTiming::CounterEffect)
                .process(move |_ctx| {
                    *w.lock().unwrap() += 1;
                })
                .build(),
            Effect::on_play(card)
                .name("OptionMain — no-op")
                .option_main()
                .process(|_ctx| {})
                .build(),
        ]
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[test]
fn counter_window_emits_hand_option_candidates() {
    // Defender has a Counter Option in hand. Attacker declares attack on
    // defender's Digimon. The Counter selection should list the hand-Option
    // candidate as a valid action.
    let counter_w = Arc::new(Mutex::new(0u32));
    let main_w = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-CTR", 0, CardColor::Red))
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("BASE", 3, 3000))
        .hand(1, &["OPT-CTR"])
        .memory(0)
        .start();
    r.register_effect(
        "OPT-CTR",
        Arc::new(CounterOptionGainMemory {
            counter_witness: counter_w.clone(),
            main_witness: main_w.clone(),
        }),
    );
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _base = r.place_on_field(1, "BASE", Some(0));

    let result = r.attack_digimon(atk, r.perm_handle(1, 0), false);
    assert_eq!(result, AttackResult::InProgress);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Counter window installs a selection");
    assert_eq!(sel.selecting_player, 1);
    assert!(sel.is_optional);
    // Hand Option candidate encoded as PLAY_HAND_START + hand_index (0).
    assert!(
        sel.valid_action_ids.contains(&(PLAY_HAND_START + 0)),
        "expected PLAY_HAND_START+0 in {:?}",
        sel.valid_action_ids
    );
}

#[test]
fn counter_hand_option_resolves_through_play_option_pipeline() {
    // Resolve the hand Counter Option: its CounterEffect body must fire
    // BEFORE OptionMain; the card ends up trashed; cost paid (zero here).
    let counter_w = Arc::new(Mutex::new(0u32));
    let main_w = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-CTR", 0, CardColor::Red))
        .add_card(dgmn("ATK", 4, 3000))
        .add_card(dgmn("BASE", 3, 5000))
        .hand(1, &["OPT-CTR"])
        .memory(0)
        .start();
    r.register_effect(
        "OPT-CTR",
        Arc::new(CounterOptionGainMemory {
            counter_witness: counter_w.clone(),
            main_witness: main_w.clone(),
        }),
    );
    let atk = r.place_on_field(0, "ATK", Some(0));
    let base = r.place_on_field(1, "BASE", Some(0));

    let hand_before = r.hand_size(1);
    let trash_before = r.trash_size(1);
    let memory_before = r.memory();

    r.attack_digimon(atk, base, false);
    r.game
        .resolve_selection(1, PLAY_HAND_START + 0)
        .expect("hand Counter Option resolves");

    assert_eq!(*counter_w.lock().unwrap(), 1, "CounterEffect fired once");
    assert_eq!(*main_w.lock().unwrap(), 1, "OptionMain fired once");
    assert_eq!(r.hand_size(1), hand_before - 1, "card left hand");
    assert_eq!(r.trash_size(1), trash_before + 1, "card in trash");
    // Counter body gained +3 memory (cost was zero).
    assert_eq!(r.memory(), memory_before + 3);
}

#[test]
fn counter_field_ability_fires_without_play_cost() {
    // A defender's battle-area Digimon carries a `.counter()` +
    // CounterEffect ability. Picking it fires the body directly — no card
    // play, no cost, no change in hand/trash.
    let witness = Arc::new(Mutex::new(0u32));

    // FIELD-CTR must survive the battle (DP > ATK) so the "no card consumed
    // to trash" assertion holds — otherwise the permanent gets deleted by
    // the subsequent battle resolution, which is orthogonal to the field-
    // ability firing we want to witness.
    let mut r = DebugRunner::builder()
        .add_card(dgmn("FIELD-CTR", 4, 9000))
        .add_card(dgmn("ATK", 4, 5000))
        .hand(1, &[])
        .memory(0)
        .start();
    r.register_effect(
        "FIELD-CTR",
        Arc::new(FieldCounterAbility {
            witness: witness.clone(),
        }),
    );
    let atk = r.place_on_field(0, "ATK", Some(0));
    let fld = r.place_on_field(1, "FIELD-CTR", Some(0));

    let hand_before = r.hand_size(1);
    let trash_before = r.trash_size(1);
    let memory_before = r.memory();

    let result = r.attack_digimon(atk, fld, false);
    assert_eq!(result, AttackResult::InProgress);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Counter window installs a selection");
    // Field counter ability: encoded via `encode_attack(0, field_index)`.
    let expected = encode_attack(0, fld.index as u16);
    assert!(
        sel.valid_action_ids.contains(&expected),
        "field counter candidate {} missing from {:?}",
        expected,
        sel.valid_action_ids
    );

    r.game
        .resolve_selection(1, expected)
        .expect("field counter ability resolves");

    assert_eq!(*witness.lock().unwrap(), 1);
    assert_eq!(r.hand_size(1), hand_before, "no card consumed from hand");
    assert_eq!(r.trash_size(1), trash_before, "no card consumed to trash");
    // Field counter body gained +2 memory.
    assert_eq!(r.memory(), memory_before + 2);
}

#[test]
fn counter_blast_and_hand_option_coexist_in_selection() {
    // Defender has both a blast-digivolve source AND a Counter Option in
    // hand. The Counter window selection must include both candidate
    // action IDs.
    let counter_w = Arc::new(Mutex::new(0u32));
    let main_w = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(blast_card("TEST-013", 4, 3, 0 /* Red */))
        .add_card(option_card("OPT-CTR", 0, CardColor::Red))
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("BASE", 3, 3000))
        .hand(1, &["TEST-013", "OPT-CTR"])
        .memory(0)
        .start();
    r.register_effect(
        "OPT-CTR",
        Arc::new(CounterOptionGainMemory {
            counter_witness: counter_w.clone(),
            main_witness: main_w.clone(),
        }),
    );
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _base = r.place_on_field(1, "BASE", Some(0));

    r.attack_digimon(atk, r.perm_handle(1, 0), false);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Counter window installs a selection");
    // Blast candidate at hand_idx 0 × field_idx 0.
    assert!(
        sel.valid_action_ids.contains(&encode_digivolve(0, 0)),
        "blast candidate missing from {:?}",
        sel.valid_action_ids
    );
    // Hand Option candidate at hand_idx 1.
    assert!(
        sel.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "hand-option candidate missing from {:?}",
        sel.valid_action_ids
    );
}

#[test]
fn counter_effect_timing_fires_only_on_selected_card() {
    // Defender has TWO hand cards each with a `.counter()` CounterEffect
    // effect, attributed to the same shared witness. Selecting only ONE
    // must fire the witness exactly once — not twice.
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-A", 0, CardColor::Red))
        .add_card(option_card("OPT-B", 0, CardColor::Red))
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("BASE", 3, 3000))
        .hand(1, &["OPT-A", "OPT-B"])
        .memory(0)
        .start();
    r.register_effect(
        "OPT-A",
        Arc::new(CounterOptionSharedWitness {
            witness: witness.clone(),
        }),
    );
    r.register_effect(
        "OPT-B",
        Arc::new(CounterOptionSharedWitness {
            witness: witness.clone(),
        }),
    );
    let atk = r.place_on_field(0, "ATK", Some(0));
    let base = r.place_on_field(1, "BASE", Some(0));

    r.attack_digimon(atk, base, false);
    r.game
        .resolve_selection(1, PLAY_HAND_START + 0)
        .expect("select OPT-A only");

    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "CounterEffect must fire only on the selected card"
    );
}

#[test]
fn counter_chain_depth_guard_blocks_nested_counter() {
    // A Counter Option's body launches a nested attack. The second
    // attack's Counter window must be suppressed by the depth guard
    // (MAX_COUNTER_DEPTH = 1). We confirm:
    //   - the outer counter body fires once,
    //   - after the nested attack resolves, no second Counter prompt
    //     is installed.
    let counter_w = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("OPT-NEST", 0, CardColor::Red))
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("BASE", 3, 3000))
        // Secondary attacker/target pair for the nested attack launched
        // by the counter body. The secondary target carries a blast
        // candidate in hand[1] that WOULD otherwise trigger a second
        // Counter window — the depth guard must suppress it.
        .add_card(dgmn("ATK2", 4, 5000))
        .add_card(dgmn("DEF2", 3, 3000))
        .add_card(blast_card("TEST-013", 4, 3, 0))
        .hand(1, &["OPT-NEST", "TEST-013"])
        .memory(0)
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let base = r.place_on_field(1, "BASE", Some(0));
    let atk2 = r.place_on_field(0, "ATK2", Some(0));
    let def2 = r.place_on_field(1, "DEF2", Some(0));

    // OPT-NEST body launches attack atk2 → def2 during its Counter
    // resolution. Register it after we know the handles.
    r.register_effect(
        "OPT-NEST",
        Arc::new(CounterOptionRecursiveAttack {
            attacker: atk2,
            target: def2,
            counter_witness: counter_w.clone(),
        }),
    );

    // Primary attack atk → base. Resolves the Counter window with the
    // hand Option.
    r.attack_digimon(atk, base, false);
    r.game
        .resolve_selection(1, PLAY_HAND_START + 0)
        .expect("outer Counter Option resolves");

    assert_eq!(*counter_w.lock().unwrap(), 1, "outer counter fired once");
    // After the outer Counter's body (which ran a nested attack) finishes,
    // neither the outer attack nor the nested attack should leave a
    // Counter selection parked — the depth guard must have short-circuited.
    assert!(
        r.game.pending_attack.is_none(),
        "both attacks terminated, no residual pending_attack"
    );
}
