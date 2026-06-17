//! ST-2 Cocytus Blue — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st-2-cocytus-blue-model.md`.
//! Per-card behavioral coverage: `tests/cards_behavioral/st2/st2_cards.rs`.
//!
//! System under test: the deck strips digivolution cards off the bottom of the
//! opponent's Digimon until a Digimon has **zero** sources ("bare" / "no
//! digivolution cards"). That single board-wide flip simultaneously enables
//! three otherwise-independent payoffs — WereGarurumon's Security Attack window
//! (ST2-08), Matt's start-of-turn memory engine (ST2-12), and Sorrow Blue's lock
//! (ST2-14). These tests assert the *cross-card* fact the per-card tests can't
//! see: stripping the LAST source turns the payoffs on, and they stay off while
//! every opponent Digimon retains a source.
//!
//! Sources cited per test: printed card text (cards.json / card_overrides.json),
//! `general_rule.pdf` §16 (keyword/inherited-effect/timing semantics), and the
//! DCGO C# reference `$BASE_DCGO/Assets/Scripts/CardEffect/ST2/Blue/ST2_XX.cs`.

#![allow(dead_code)]

use digimon_engine::action::space::encode_source_select;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{EffectTiming, ModifierType};
use digimon_engine::selection::TriggerSource;
use digimon_engine::PermanentHandle;

// ── Local fixtures (mirror the proven helpers in cards_behavioral/st2) ─────────
//
// Every body below — including neutral opponent strip targets — is a REAL vanilla
// blue ST-2 DSL card (zero effect clauses), so a stripped source never fires a
// stray inherited effect onto the combo:
//   ST2-02 Gomamon (Lv3, 3000)  ST2-04 Bearmon (Lv3, 4000)
//   ST2-05 Ikkakumon (Lv4, 5000) ST2-10 Plesiomon (Lv6, 12000)

const ST2_IDS: [&str; 16] = [
    "ST2-01", "ST2-02", "ST2-03", "ST2-04", "ST2-05", "ST2-06", "ST2-07", "ST2-08", "ST2-09",
    "ST2-10", "ST2-11", "ST2-12", "ST2-13", "ST2-14", "ST2-15", "ST2-16",
];

/// Fresh builder with the whole ST-2 DSL pack loaded (combo pieces and neutral
/// strip targets are all real cards).
fn st2_builder() -> DebugRunnerBuilder {
    let mut builder = DebugRunner::builder();
    for card_id in ST2_IDS {
        builder = builder
            .dsl_card(card_id)
            .unwrap_or_else(|err| panic!("{card_id} must load from embedded ST2 pack: {err}"));
    }
    builder
}

/// The digivolution-source ids (bottom→top) under a battle-area Digimon.
fn source_ids(runner: &DebugRunner, player: u8, index: usize) -> Vec<String> {
    runner.game.player(player).battle_area[index]
        .card_sources
        .iter()
        .map(|source| source.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Fire a triggered timing for `source` (direct enqueue + drain — the proven
/// harness pattern; `place_stack` builds a stack without firing When-Digivolving,
/// and we model When-Attacking the same way).
fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

/// Resolve the current pending selection by taking its first valid action, then
/// drain follow-ups. Used for the "choose 1 opponent Digimon" strip/target prompts.
fn choose_first_pending(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .pending_selection()
            .expect("expected a pending selection");
        (
            pending.selecting_player,
            pending
                .valid_action_ids
                .first()
                .copied()
                .expect("selection has at least one action"),
        )
    };
    runner
        .execute_action(player, action)
        .expect("pending selection resolves");
    runner.game.drain_effect_queue();
}

// ── Combo 1: Strip → WereGarurumon Security-Attack flips on ────────────────────

/// **Strip → WereGarurumon Security-Attack flip** (ST2-08 + bare opp Digimon).
///
/// ST2-08 WereGarurumon inherited: *"[Your Turn] While your opponent has a
/// Digimon with no digivolution cards, this Digimon gains <Security Attack +1>."*
/// The bonus is a board-wide read of the opponent's source state, so it must be
/// **1** the instant any opp Digimon is bare and **0** while every opp Digimon
/// still has a source. This pins the *flip across that boundary* (the per-card
/// `st2_08_…` test only reads one side).
///
/// Sources: ST2-08 card text; <Security Attack +1> keyword + inherited [Your Turn]
/// timing per `general_rule.pdf` §16; DCGO C# `…/ST2/Blue/ST2_08.cs`.
#[test]
fn strip_flips_weregarurumon_security_attack_bonus() {
    // Bare opponent → bonus on.
    let mut bare = st2_builder().start();
    let carrier = bare.place_stack(0, &["ST2-08", "ST2-10"]);
    bare.place_on_field(1, "ST2-02", Some(0)); // 0-source (bare) opp Digimon
    bare.game.tick_declarative_effects();
    assert_eq!(
        bare.game.security_attack_keyword_bonus(carrier),
        1,
        "a bare opponent Digimon must turn WereGarurumon's Security Attack +1 on"
    );

    // Every opponent Digimon sourced → bonus off.
    let mut sourced = st2_builder().start();
    let carrier = sourced.place_stack(0, &["ST2-08", "ST2-10"]);
    sourced.place_stack(1, &["ST2-02", "ST2-05"]); // 1-source opp Digimon, not bare
    sourced.game.tick_declarative_effects();
    assert_eq!(
        sourced.game.security_attack_keyword_bonus(carrier),
        0,
        "with no bare opponent Digimon the Security Attack bonus must stay off"
    );
}

// ── Combo 2: Strip → Sorrow Blue lock (gated) ──────────────────────────────────

/// **Strip → Sorrow Blue lock** (ST2-14 + 1 bare + 1 sourced opp Digimon).
///
/// ST2-14 Sorrow Blue [Main]: *"Choose 1 of your opponent's Digimon with no
/// digivolution cards. It can't attack or block until the end of your opponent's
/// next turn."* The target prompt must offer **only** the bare Digimon, and the
/// resolved lock must apply `CannotAttack` + `CannotBlock` to that one alone —
/// the sourced Digimon is neither a legal target nor affected.
///
/// Sources: ST2-14 card text; can't-attack / can't-block are turn-window
/// restriction modifiers (`general_rule.pdf` §6 attack declaration, §8 block
/// timing); DCGO C# `…/ST2/Blue/ST2_14.cs`.
#[test]
fn sorrow_blue_locks_only_the_bare_opponent_digimon() {
    let mut runner = st2_builder().hand(0, &["ST2-14"]).memory(5).start();
    let bare = runner.place_on_field(1, "ST2-02", Some(0)); // 0-source: legal target
    let sourced = runner.place_stack(1, &["ST2-02", "ST2-05"]); // 1-source: illegal

    // Baseline: the bare Digimon (placed turn_played=0 -> not summoning-sick,
    // unsuspended) IS attack-capable before the lock. This makes the post-lock
    // regression assertion below meaningful rather than passing for an
    // unrelated reason (summoning sickness / suspension).
    assert!(
        runner.game.can_attack(bare, false),
        "bare Digimon is attack-capable before Sorrow Blue's lock"
    );

    assert!(
        runner.game.activate_hand_main(0, 0),
        "Sorrow Blue [Main] should activate from hand"
    );
    {
        let pending = runner
            .pending_selection()
            .expect("Sorrow Blue installs a target prompt");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "only the bare opponent Digimon is a legal Sorrow Blue target"
        );
    }
    choose_first_pending(&mut runner);

    assert!(
        runner.game.modifiers.has(bare, ModifierType::CannotAttack),
        "the locked bare Digimon can't attack"
    );
    // Regression (eval-recording attack loop): `Game::can_attack` must HONOR the
    // permanent-scoped CannotAttack modifier so the attack leaves the legal
    // action mask. Previously the mask still offered the attack while execution
    // refused it -> a no-op a deterministic policy looped on until the step
    // limit (combat/mod.rs `can_attack*` missing the CannotAttack consult).
    assert!(
        !runner.game.can_attack(bare, false),
        "CannotAttack must make the attack illegal (mask/execution parity)"
    );
    assert!(
        runner.game.modifiers.has(bare, ModifierType::CannotBlock),
        "the locked bare Digimon can't block"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(sourced, ModifierType::CannotAttack),
        "the sourced Digimon is untouched (can still attack)"
    );
    assert!(
        !runner.game.modifiers.has(sourced, ModifierType::CannotBlock),
        "the sourced Digimon is untouched (can still block)"
    );
}

// ── Combo 3: Strip → Matt memory engine (gated) ────────────────────────────────

/// **Strip → Matt memory engine** (ST2-12 + opp Digimon, sourced then bare).
///
/// ST2-12 Matt Ishida [Start of Your Turn]: *"If your opponent has a Digimon with
/// no digivolution cards, gain 1 memory."* Same board, two firings: while the
/// opponent's only Digimon is sourced the start-of-turn gain is **0**; after the
/// last source is stripped (Digimon now bare) the gain is **+1**. That gated swing
/// is the system fact — strip output feeds Matt's recurring memory.
///
/// Sources: ST2-12 card text; conditional Start-of-Your-Turn timing
/// (`general_rule.pdf` §15 turn structure); DCGO C# `…/ST2/Blue/ST2_12.cs`.
#[test]
fn matt_memory_engine_gated_on_bare_opponent() {
    // Opponent Digimon has a source → Matt's start-of-turn gains nothing.
    // Real vanilla blue bodies: ST2-02 Gomamon (source) under ST2-10 Plesiomon.
    let mut runner = st2_builder().memory(0).start();
    let matt = runner.place_on_field(0, "ST2-12", Some(0));
    let opp = runner.place_stack(1, &["ST2-02", "ST2-10"]); // 1 source: not bare
    // `source_ids` (== `card_sources`) includes the active top card, so a
    // stack of [ST2-02, ST2-10] reads [ST2-02, ST2-10].
    assert_eq!(
        source_ids(&runner, 1, opp.index as usize),
        vec!["ST2-02", "ST2-10"]
    );

    let memory_before = runner.memory();
    fire(&mut runner, EffectTiming::StartOfYourTurn, matt);
    assert_eq!(
        runner.memory(),
        memory_before,
        "Matt gains no memory while every opponent Digimon still has a source"
    );

    // Strip the last source via ST2-06 Garurumon's [When Attacking] inherited
    // effect (no level cap), leaving the opponent Digimon bare.
    let stripper = runner.place_stack(0, &["ST2-06", "ST2-05"]);
    fire(&mut runner, EffectTiming::WhenAttacking, stripper);
    choose_first_pending(&mut runner); // choose the opponent Digimon to strip
    // "Bare" = no digivolution sources left; only the active top card remains.
    assert_eq!(
        source_ids(&runner, 1, opp.index as usize),
        vec!["ST2-10"],
        "Garurumon's strip must remove the opponent Digimon's last source (only the top remains → bare)"
    );

    // Re-fire Matt's start-of-turn — now the opponent has a bare Digimon.
    let memory_before = runner.memory();
    fire(&mut runner, EffectTiming::StartOfYourTurn, matt);
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "once the opponent has a bare Digimon, Matt's start-of-turn gains 1 memory"
    );
}

// ── Combo 4: Garurumon real strip → WereGarurumon turns on ─────────────────────

/// **Garurumon strip → WereGarurumon on** (ST2-06 strip + ST2-08 payoff + 1-source
/// opp Digimon). End-to-end: actually strip the last source with a real inherited
/// effect rather than hand-placing a 0-source body.
///
/// ST2-06 Garurumon inherited: *"[When Attacking] Trash the digivolution card at
/// the bottom of 1 of your opponent's Digimon."* With the opponent's only Digimon
/// at 1 source, WereGarurumon's bonus reads 0; firing Garurumon's strip empties
/// that source, the Digimon flips bare, and after re-ticking declaratives
/// WereGarurumon's Security Attack +1 turns on. Proves the full
/// strip→bare→payoff chain mechanically.
///
/// Sources: ST2-06 + ST2-08 card text; inherited [When Attacking] timing +
/// bottom-source trash + <Security Attack +1> per `general_rule.pdf` §16; DCGO C#
/// `…/ST2/Blue/ST2_06.cs`, `…/ST2/Blue/ST2_08.cs`.
#[test]
fn garurumon_strip_turns_weregarurumon_security_attack_on() {
    // Opponent's lone Digimon is a real vanilla blue stack: ST2-02 Gomamon
    // (source) under ST2-04 Bearmon (top).
    let mut runner = st2_builder().start();
    // WereGarurumon payoff carrier (active body on top of its inherited).
    let weregaru = runner.place_stack(0, &["ST2-08", "ST2-10"]);
    // Garurumon strip carrier — its inherited [When Attacking] does the stripping.
    let stripper = runner.place_stack(0, &["ST2-06", "ST2-05"]);
    // Opponent's lone Digimon has exactly one source.
    let opp = runner.place_stack(1, &["ST2-02", "ST2-04"]);

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.security_attack_keyword_bonus(weregaru),
        0,
        "while the opponent Digimon still has its source, WereGarurumon's bonus is off"
    );

    fire(&mut runner, EffectTiming::WhenAttacking, stripper);
    choose_first_pending(&mut runner); // choose the opponent Digimon to strip
    assert_eq!(
        source_ids(&runner, 1, opp.index as usize),
        vec!["ST2-04"],
        "Garurumon's [When Attacking] strip removes the opponent's last source (only the top remains → bare)"
    );

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.security_attack_keyword_bonus(weregaru),
        1,
        "the freshly-bared opponent Digimon turns WereGarurumon's Security Attack +1 on"
    );
}

// ── Combo 5: Zudomon strip-2 → bare → Matt ─────────────────────────────────────

/// **Zudomon strip-2 reaches bare and feeds Matt** (ST2-09 + 2-source opp Digimon
/// + ST2-12 Matt). A second strip path (a single [When Digivolving] removing two
/// sources) reaching the same shared bare-state trigger.
///
/// ST2-09 Zudomon [When Digivolving]: *"Trash 2 digivolution cards at the bottom
/// of 1 of your opponent's Digimon."* Against a Digimon with exactly two sources
/// this empties it bare in one shot; Matt's start-of-turn then reads 1 where it
/// read 0 while the Digimon held its two sources.
///
/// Sources: ST2-09 + ST2-12 card text; [When Digivolving] timing + 2× bottom-source
/// trash (`general_rule.pdf` §16); DCGO C# `…/ST2/Blue/ST2_09.cs`, `…/ST2/Blue/ST2_12.cs`.
#[test]
fn zudomon_strip_two_reaches_bare_and_feeds_matt() {
    // Opponent Digimon with two bottom sources, real vanilla blue bodies:
    // ST2-02 / ST2-04 (sources) under ST2-05 Ikkakumon (top).
    let mut runner = st2_builder().memory(0).start();
    let matt = runner.place_on_field(0, "ST2-12", Some(0));
    let zudomon = runner.place_on_field(0, "ST2-09", Some(1));
    // Opponent Digimon with two bottom sources.
    let opp = runner.place_stack(1, &["ST2-02", "ST2-04", "ST2-05"]);
    assert_eq!(
        source_ids(&runner, 1, opp.index as usize),
        vec!["ST2-02", "ST2-04", "ST2-05"]
    );

    // Pre-strip: opponent Digimon is not bare → Matt gains nothing.
    let memory_before = runner.memory();
    fire(&mut runner, EffectTiming::StartOfYourTurn, matt);
    assert_eq!(
        runner.memory(),
        memory_before,
        "Matt gains nothing while the opponent Digimon still holds its two sources"
    );

    // Zudomon strips both bottom sources at once → opponent Digimon now bare.
    fire(&mut runner, EffectTiming::WhenDigivolving, zudomon);
    choose_first_pending(&mut runner); // choose the opponent Digimon to strip
    assert_eq!(
        source_ids(&runner, 1, opp.index as usize),
        vec!["ST2-05"],
        "Zudomon's strip-2 must empty both of the opponent Digimon's sources (only the top remains → bare)"
    );

    // Post-strip: opponent has a bare Digimon → Matt's start-of-turn gains 1.
    let memory_before = runner.memory();
    fire(&mut runner, EffectTiming::StartOfYourTurn, matt);
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "after Zudomon bares the opponent Digimon, Matt's start-of-turn gains 1 memory"
    );
}

// ── Combo 6: Kaiser Nail recurs your own digivolution source ───────────────────

/// **Kaiser Nail recurs your own source** (ST2-15 + your ST2-03-under-ST2-05 stack).
///
/// ST2-15 Kaiser Nail [Main]: *"Choose 1 Digimon digivolution card placed under 1
/// of your Digimon and play it without paying the cost."* Build a real Blue stack
/// (ST2-03 Gabumon under ST2-05 Ikkakumon), play Kaiser Nail, select the under-card
/// (ST2-03), and assert it is played to the field for free: the source leaves the
/// host stack, the player's battle-area count is +1, and memory is unchanged.
///
/// Sources: ST2-15 card text ("without paying the cost"); playing a digivolution
/// card from under a Digimon (`general_rule.pdf` §16 / play rules); DCGO C#
/// `…/ST2/Blue/ST2_15.cs`.
#[test]
fn kaiser_nail_recurs_own_digivolution_source_for_free() {
    let mut runner = st2_builder().hand(0, &["ST2-15"]).memory(0).start();
    // Your own Blue stack: ST2-03 Gabumon (source) under ST2-05 Ikkakumon (active).
    let host = runner.place_stack(0, &["ST2-03", "ST2-05"]);
    let field_before = runner.game.player(0).battle_area.len();
    let memory_before = runner.memory();
    // `source_ids` includes the active top, so the host reads [ST2-03, ST2-05].
    assert_eq!(
        source_ids(&runner, 0, host.index as usize),
        vec!["ST2-03", "ST2-05"]
    );

    assert!(
        runner.game.activate_hand_main(0, 0),
        "Kaiser Nail [Main] should activate from hand"
    );
    choose_first_pending(&mut runner); // choose your host Digimon

    let source_action =
        encode_source_select(host.index as u16, 0).expect("encode under-card source action");
    {
        let pending = runner
            .pending_selection()
            .expect("Kaiser Nail installs an under-card source prompt");
        assert!(
            pending.valid_action_ids.contains(&source_action),
            "the buried ST2-03 source must be offered for free play"
        );
    }
    runner
        .execute_action(0, source_action)
        .expect("under-card source selection resolves");
    runner.game.drain_effect_queue();

    // The source left the host stack and now stands on its own — battle area +1.
    // The host keeps its active top card (ST2-05), so it reads [ST2-05].
    assert_eq!(
        source_ids(&runner, 0, host.index as usize),
        vec!["ST2-05"],
        "the recurred ST2-03 source must leave the host stack (only the ST2-05 top remains)"
    );
    assert_eq!(
        runner.game.player(0).battle_area.len(),
        field_before + 1,
        "the recurred digivolution card is played as a new Digimon on the field"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "Kaiser Nail plays the source without paying its cost — memory is unchanged"
    );
}
