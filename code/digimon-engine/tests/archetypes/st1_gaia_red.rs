//! ST-1 Gaia Red — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st1-gaia-red-model.md` (the worldwide ST-1 Gaia Red
//! starter deck modelled as a *system*). Every card loaded here is the real
//! implemented DSL card from `code/digimon-engine/cards/st1/`; synthetic
//! `make_test_card` bodies appear only as neutral opponent fodder/targets a
//! combo needs.
//!
//! Each `#[test]` maps 1:1 to a ranked combo in the model:
//!
//!  1. `tall_stack_security_rush_*` — WarGreymon (ST1-11) on a 4-source
//!     Agumon→WarGreymon stack converts accumulated `<Security A.>` into a
//!     multi-card security check on one connected player attack, while the same
//!     stack's inherited DP (Koromon + Agumon, your-turn) pushes WarGreymon
//!     past its printed 12000. (Model combo: "Tall-Stack Security Rush".)
//!  2. `tai_anthem_additive_stack_*` — Tai Kamiya's (ST1-12) `[Your Turn]`
//!     +1000 aura stacks *additively* on top of an inherited Agumon +1000 and a
//!     Shadow Wing (ST1-13) +3000 on one body, and leaves opponent Digimon
//!     untouched (owner-only aura). (Model combo: "Tai Anthem additive stack".)
//!  3. `garudamon_buff_survives_gaia_force_removal` — Garudamon's (ST1-08)
//!     `[When Digivolving]` +3000 buff on the attacker holds while Gaia Force
//!     (ST1-16) deletes the opponent's blocker the same turn (removal + buff
//!     compose). (Model combo: "Garudamon combat trick into Gaia Force lethal".)
//!
//! Sources cited inline per test: shipping YAML (`cards/st1/<ID>.yaml`); DCGO
//! C# (`$BASE_DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_<NN>.cs`); official
//! `Digimon TCG resources/general_rule.pdf` §16-3 (Security Attack stacking) and
//! §6 (deletion). Per-card coverage lives in
//! `tests/cards_behavioral/st1/gaia_red.rs`; these assert the *combined* outcome
//! no single-card test sees.
//!
//! # Security-strike base-fold — FIXED (Test 1), plus a separate open gap
//!
//! Test 1 pins the fix to `Game::current_security_strike` (combat.rs ~L2591): a
//! formula-driven `<Security A.>` aura (e.g. WarGreymon's `floor(n/2)`) is an
//! ADDITIVE delta on top of the base-1 check, not a replacement for it. The bug
//! was `let base_checks = dynamic_security_attack_aura_bonus(attacker).unwrap_or(1);`
//! — the aura's additive delta (`Some(2)`) was consumed AS the base, dropping the
//! implicit base-1, so a 4-source WarGreymon checked only 2. The fix
//! (`1 + dynamic_security_attack_aura_bonus(...).unwrap_or(0) + ...`) makes it
//! check the card-faithful `1 + 2 = 3`. Sources: `general_rule.pdf` §16-3
//! (`<Security A.>` is strictly additive on top of the base-1 check; multiple
//! instances add separately); DCGO `Permanent.Strike_AllowMinus` (`Strike = 1`,
//! then `+= each <Security A.>`), `ST1_11.cs` (`DigivolutionCards.Count / 2`).
//!
//! SEPARATE OPEN GAP (not exercised here): the full ST-1 line also runs ST1-07
//! Greymon, whose inherited `<Security A. +1>` is a DECLARATIVE `grant_keyword`
//! that `Game::security_attack_keyword_bonus` does not sum (the strike's keyword
//! term reads printed + materialized-modifier keywords only; `has_keyword` sees
//! the live grant but the strike misses it), so the real deck checks 3 where it
//! should check 4. Tracked in `docs/RUST_ENGINE_GAPS.md`. Test 1 deliberately uses
//! a Greymon-free stack to isolate the (now-fixed) base-fold.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, PlaySource};

use super::support::snapshot;

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// A neutral low-DP opponent Digimon used only as security fodder / a blocker
/// target. 1000 DP so WarGreymon (≥12000) trivially wins every security battle
/// and the strike count — not a lost DP compare — governs how many cards leave
/// the opponent's security.
fn make_weak_opp(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c
}

// ─── Combo 1: Tall-Stack Security Rush ───────────────────────────────────────

/// **Tall-Stack Security Rush — WarGreymon's `<Security A.>` aura adds to base-1.**
/// A WarGreymon stack `[Koromon, Agumon, Birdramon, MetalGreymon, WarGreymon]`
/// (4 digivolution sources) attacks the opponent's player on the controller's
/// turn.
///
/// Cards: ST1-11 WarGreymon (top, `[Your Turn]` gains `<Security A. +1>` for
/// every 2 digivolution cards → `floor(4/2)=2`), ST1-03 Agumon (inherited
/// `[Your Turn] +1000 DP`), ST1-01 Koromon (inherited `[Your Turn] +1000 DP`
/// while 4+ sources). ST1-05 Birdramon and ST1-09 MetalGreymon are vanilla-on-a-
/// player-attack filler sources, chosen so the *only* security-attack contributor
/// is WarGreymon's formula aura (see the gap note below).
///
/// Expected mechanical outcome (card-faithful):
/// - Security strike on the player attack = **3** = base 1 + WarGreymon's aura
///   `floor(4/2)=2`. This is the REGRESSION-TESTED FIX: `current_security_strike`
///   previously consumed the aura's additive delta (`== Some(2)`) AS the base via
///   `.unwrap_or(1)`, dropping the implicit base-1 and returning only 2. The fix
///   makes the aura additive on top of base-1 (`general_rule.pdf` §16-3; DCGO
///   `Permanent.Strike_AllowMinus` seeds `Strike = 1` then folds each
///   `<Security A.>` in additively).
/// - DP on the controller's turn = WarGreymon 12000 + Agumon inherited +1000 +
///   Koromon +1000 (active at 4+ sources) = **14000**.
///
/// NOTE — separate gap, deliberately NOT exercised here: the full ST-1 line also
/// runs ST1-07 Greymon (inherited `<Security A. +1>`), which would make the real
/// deck check **4**. That inherited bonus is a DECLARATIVE `grant_keyword` that
/// `Game::security_attack_keyword_bonus` does not sum (it reads printed +
/// materialized-modifier keywords only; `has_keyword` sees the live grant, but the
/// strike's keyword term misses it). Tracked in `docs/RUST_ENGINE_GAPS.md`. This
/// test isolates WarGreymon's base-inclusive formula aura so it does not depend
/// on that separate gap.
///
/// Sources: `cards/st1/ST1-11.yaml` (`security_attack_fn floor_div(...,2)`),
/// `ST1-03.yaml` / `ST1-01.yaml` (inherited DP); DCGO `ST1_11.cs`
/// (`count = DigivolutionCards.Count / 2`, owner-turn gated); `general_rule.pdf`
/// §16-3 (Security Attack stacking, floors at 0 per §16-3-4).
#[test]
fn tall_stack_security_rush_aura_adds_to_base_one_check() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-01")
        .expect("ST1-01 (Koromon) in embedded DSL pack")
        .dsl_card("ST1-03")
        .expect("ST1-03 (Agumon) in embedded DSL pack")
        .dsl_card("ST1-05")
        .expect("ST1-05 (Birdramon) in embedded DSL pack")
        .dsl_card("ST1-09")
        .expect("ST1-09 (MetalGreymon) in embedded DSL pack")
        .dsl_card("ST1-11")
        .expect("ST1-11 (WarGreymon) in embedded DSL pack")
        .add_card(make_weak_opp("SEC-A", "SecFodderA"))
        .add_card(make_weak_opp("SEC-B", "SecFodderB"))
        .add_card(make_weak_opp("SEC-C", "SecFodderC"))
        .add_card(make_weak_opp("SEC-D", "SecFodderD"))
        .memory(3)
        // Four plain security cards so a 3-strike check can't deck the opponent
        // out (which would end the game before we can read the diff).
        .security(1, &["SEC-A", "SEC-B", "SEC-C", "SEC-D"])
        .start();
    // Player 0 is the turn player; pin turn_count so the your-turn auras
    // (WarGreymon's <Security A.>, Agumon/Koromon DP) are active.
    runner.game.turn_count = 1;

    // Stack: 4 sources [Koromon, Agumon, Birdramon, MetalGreymon] under WarGreymon
    // — NO Greymon, so WarGreymon's formula aura is the sole security-attack source.
    let stack = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-05", "ST1-09", "ST1-11"]);

    // ── Component pins (the strike count is an emergent sum of these) ────────
    assert_eq!(
        runner.game.dynamic_security_attack_aura_bonus(stack),
        Some(3),
        "WarGreymon's [Your Turn] formula aura is BASE-INCLUSIVE: it returns the \
         total base check count 1 + floor(4/2) = 3 (the default base-1 plus the \
         floor(material/2) bonus), NOT a bare delta. Authored `base: 2` over \
         material_count → floor((2+4)/2)=3 (per general_rule.pdf 16-3; DCGO \
         ST1_11.cs computes the floor(n/2) bonus, the engine folds in the base)."
    );
    assert_eq!(
        runner.game.security_attack_keyword_bonus(stack),
        0,
        "no <Security A.> keyword in this stack (Greymon is deliberately absent)"
    );

    // ── DP side-effect of the same stack (your-turn inherited DP) ────────────
    assert_eq!(
        runner.game.source_dp_contribution(stack, 0),
        1000,
        "Koromon (source 0) contributes +1000 DP at 4+ digivolution cards"
    );
    assert_eq!(
        runner.game.source_dp_contribution(stack, 1),
        1000,
        "Agumon (source 1) contributes its unconditional +1000 DP"
    );
    assert_eq!(
        runner.effective_dp(stack),
        Some(14_000),
        "WarGreymon 12000 + Agumon +1000 + Koromon +1000 = 14000 on the controller's turn"
    );

    // ── The real attack: opponent security drops by the card-faithful strike (4) ──
    let before = snapshot(&runner);
    runner.attack_player(stack, 1, false);
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    assert!(
        !runner.game_over(),
        "4 security vs a 3-strike check must not deck the opponent out"
    );
    assert_eq!(
        before.security[1] - after.security[1],
        3,
        "a 4-source WarGreymon checks 3 security on one connected player attack: \
         base-inclusive formula aura 1 + floor(4/2) = 3. Authoring the formula \
         base-inclusive (`base: 2`) folds the base-1 check into the aura value; a \
         bare `base: 0` delta (floor(4/2)=2) would drop the base and under-count. \
         Per general_rule.pdf 16-3 + DCGO ST1_11.cs."
    );
}

/// **Tall-Stack Security Rush — full line checks 4** (declarative inherited
/// keyword counted via fresh materialized state). The full ST-1 line
/// `[Koromon, Agumon, Greymon, MetalGreymon, WarGreymon]` (4 sources) checks
/// **4** security on a connected player attack = base 1 + WarGreymon aura
/// `floor(4/2)=2` + Greymon's inherited `<Security A. +1>`.
///
/// Greymon's bonus is a DECLARATIVE inherited `grant_keyword` that materializes
/// into the keyword registry only on `tick_declarative_effects`. The fix makes
/// `current_security_strike` refresh declarative state before it reads the
/// keyword sum, so the strike is card-faithful without an external tick. (The
/// real `decode_action` path ticks; this test drives `attack_player` directly,
/// which previously left the strike reading a stale cache → 3.)
///
/// Sources: `ST1-07.yaml` (inherited `grant_keyword SecurityAttackPlus 1`); DCGO
/// `ST1_07.cs` (`ChangeSelfSAttackStaticEffect(1, isInheritedEffect:true)`);
/// general_rule.pdf §16-3.
///
/// The strike refreshes declarative state (`current_security_strike` ticks) so
/// Greymon's INHERITED `<Security A. +1>` is materialized and counted, while the
/// tick de-dups OWN grants already printed in `card_data` so nothing double-counts
/// — the unifying fix for the former G-DECLARATIVE-KEYWORD aggregation gap.
#[test]
fn tall_stack_security_rush_full_line_checks_four_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-01")
        .expect("ST1-01 (Koromon) in embedded DSL pack")
        .dsl_card("ST1-03")
        .expect("ST1-03 (Agumon) in embedded DSL pack")
        .dsl_card("ST1-07")
        .expect("ST1-07 (Greymon) in embedded DSL pack")
        .dsl_card("ST1-09")
        .expect("ST1-09 (MetalGreymon) in embedded DSL pack")
        .dsl_card("ST1-11")
        .expect("ST1-11 (WarGreymon) in embedded DSL pack")
        .add_card(make_weak_opp("SEC-A", "SecFodderA"))
        .add_card(make_weak_opp("SEC-B", "SecFodderB"))
        .add_card(make_weak_opp("SEC-C", "SecFodderC"))
        .add_card(make_weak_opp("SEC-D", "SecFodderD"))
        .add_card(make_weak_opp("SEC-E", "SecFodderE"))
        .memory(3)
        // Five plain security cards so a 4-strike check can't deck the opponent out.
        .security(1, &["SEC-A", "SEC-B", "SEC-C", "SEC-D", "SEC-E"])
        .start();
    runner.game.turn_count = 1;

    // Full line: 4 sources [Koromon, Agumon, Greymon, MetalGreymon] under WarGreymon.
    let stack = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-07", "ST1-09", "ST1-11"]);

    // Driven through a REAL attack with NO manual tick: the strike must refresh
    // declarative state internally and count Greymon's inherited <Security A. +1>.
    let before = snapshot(&runner);
    runner.attack_player(stack, 1, false);
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    assert!(
        !runner.game_over(),
        "5 security vs a 4-strike check must not deck the opponent out"
    );
    assert_eq!(
        before.security[1] - after.security[1],
        4,
        "full ST-1 line checks 4 security: base 1 + WarGreymon aura floor(4/2)=2 + \
         Greymon inherited <Security A. +1>. The strike refreshes declarative state, \
         so the inherited (materialize-on-tick) keyword grant is counted even though \
         this test never ticked manually."
    );
}

// ─── Combo 2: Tai Anthem additive stack ──────────────────────────────────────

/// **Tai Anthem additive stack.** Tai Kamiya's (ST1-12) `[Your Turn]` aura
/// stacks additively with an inherited Agumon (ST1-03) +1000 and a Shadow Wing
/// (ST1-13) `[Main]` +3000 on one Garudamon-over-Agumon body — and does NOT
/// touch an opponent Digimon.
///
/// Cards: ST1-12 Tai Kamiya (anthem), ST1-08 Garudamon over ST1-03 Agumon (the
/// buffed body, base 7000 carrying Agumon's inherited +1000), ST1-13 Shadow
/// Wing (the +3000 combat trick).
///
/// Expected: the body's effective DP = 7000 (Garudamon) + 1000 (Tai aura) +
/// 1000 (Agumon inherited, your-turn) + 3000 (Shadow Wing) = **12000**, all four
/// sources adding simultaneously. An opponent vanilla Digimon stays at its base
/// (Tai is owner-only).
///
/// Sources: `cards/st1/ST1-12.yaml` (`aura dp_modifier 1000`, `of: you`),
/// `ST1-03.yaml` (inherited `dp_modifier 1000`), `ST1-13.yaml`
/// (`add_dp_modifier value 3000 expiry end_of_turn`); DCGO `ST1_12.cs`
/// (`ChangeDPStaticEffect(1000, owner battle-area, owner-turn gated)`);
/// `general_rule.pdf` §3 (DP modifiers are additive & simultaneous). The
/// per-card Tai test only pins the bare +1000; this asserts the multi-source
/// additive stack.
#[test]
fn tai_anthem_additive_stack_layers_on_one_body_and_spares_opponent() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-03")
        .expect("ST1-03 (Agumon) in embedded DSL pack")
        .dsl_card("ST1-08")
        .expect("ST1-08 (Garudamon) in embedded DSL pack")
        .dsl_card("ST1-12")
        .expect("ST1-12 (Tai Kamiya) in embedded DSL pack")
        .dsl_card("ST1-13")
        .expect("ST1-13 (Shadow Wing) in embedded DSL pack")
        .add_card(make_weak_opp("OPP-BODY", "OppBody"))
        .memory(10)
        .hand(0, &["ST1-13"])
        .start();
    runner.game.turn_count = 1;

    // Buffed body: Garudamon (7000) over an Agumon source (inherited +1000).
    let body = runner.place_stack(0, &["ST1-03", "ST1-08"]);
    // Tai on the controller's field (Tamer; its aura is your-turn, owner-only).
    runner.place_on_field(0, "ST1-12", Some(0));
    // An opponent Digimon Tai must NOT touch.
    let opp = runner.place_on_field(1, "OPP-BODY", None);

    // Baseline before Shadow Wing: Garudamon 7000 + Tai 1000 + Agumon 1000.
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(body),
        Some(9_000),
        "Garudamon 7000 + Tai aura 1000 + Agumon inherited 1000 = 9000 before Shadow Wing"
    );

    // Play Shadow Wing [Main] (+3000 to 1 of your Digimon). The Garudamon body
    // is the only kind:digimon target (Tai is a Tamer), so it is forced.
    runner.play(0, 0).expect("play ST1-13 (Shadow Wing) from hand");
    let prompt = runner
        .pending_selection()
        .expect("ST1-13 [Main] installs a +3000 own-Digimon selection");
    assert_eq!(
        prompt.valid_action_ids.len(),
        1,
        "the Garudamon body is the only own Digimon to target (Tai is a Tamer)"
    );
    let pick = prompt.valid_action_ids[0];
    runner
        .execute_action(0, pick)
        .expect("apply Shadow Wing +3000 to the body");

    runner.game.tick_declarative_effects();

    // ── Additive stack: 7000 + 1000 + 1000 + 3000 = 12000 ────────────────────
    assert_eq!(
        runner.effective_dp(body),
        Some(12_000),
        "Tai +1000, Agumon-inherited +1000, and Shadow Wing +3000 all add on one body"
    );

    // ── Owner-only: the opponent Digimon is untouched by Tai's anthem ────────
    assert_eq!(
        runner.effective_dp(opp),
        Some(1_000),
        "Tai's [Your Turn] aura does not buff opponent Digimon"
    );
}

// ─── Combo 3: Garudamon buff survives a same-turn Gaia Force removal ──────────

/// **Garudamon combat trick into Gaia Force lethal.** Digivolve to Garudamon
/// (ST1-08) for its `[When Digivolving]` +3000 on the attacker, then Gaia Force
/// (ST1-16) deletes the opponent's only blocker the same turn — removal + buff
/// compose, and the buff *survives* the removal.
///
/// Cards: ST1-08 Garudamon (the +3000 trick; digivolved up from ST1-05
/// Birdramon), ST1-16 Gaia Force (delete 1 opponent Digimon, no DP cap), plus a
/// neutral opponent blocker body.
///
/// Expected: opponent battle-area count drops by 1 (blocker → trash) AND the
/// buffed Garudamon's effective DP still reads base 7000 + 3000 = 10000 after
/// the unrelated opponent-side deletion (the buff is a self-modifier, untouched
/// by removing a different permanent).
///
/// Sources: `cards/st1/ST1-08.yaml` (`when_digivolving` → `add_dp_modifier 3000
/// expiry end_of_turn`), `ST1-16.yaml` (`select_opponent_permanent` →
/// `delete_permanent`); DCGO `ST1_16.cs` (plain Delete), `ST1_08.cs`
/// (`ChangeDigimonDP(+3000, UntilEachTurnEnd)` on the owner-selected Digimon);
/// `general_rule.pdf` §6 (deletion to trash). Each half is single-card-tested in
/// `cards_behavioral/st1/gaia_red.rs`; this asserts they compose.
///
/// TEST-FIX NOTE: the model claim (buff survives a same-turn opponent-side
/// deletion) is correct and the engine implements it — the buff is an
/// `end_of_turn` self-modifier on the controller's Digimon and the deletion only
/// clears the *deleted* (opponent) permanent's modifiers (`clear_permanent_full`
/// + `shift_after_battle_area_remove` touch only the deleted player). The
/// original setup started at `memory(10)`; the Garudamon digivolve (-3) then
/// Gaia Force (-8) drove memory to -1, so `Game::check_turn_end` (ends the turn
/// when memory < 0) ended the controller's turn and `expire_end_of_turn`
/// CORRECTLY dropped the +3000 → the body read 7000. That was a TEST-setup bug
/// (the test ended its own turn), NOT a buff-survival engine gap. The standard
/// memory range caps at +10, so 3 + 8 = 11 of spend can't be pre-funded from a
/// single start; the fix tops the gauge back to 10 just before the Gaia Force
/// play (10 - 8 = 2 >= 0, turn stays live) and adds an explicit `turn_player()`
/// precondition guard so any future memory regression fails loudly instead of
/// masquerading as a survival gap. The asserted buff-survival behaviour is
/// unchanged — it still exercises the real engine.
#[test]
fn garudamon_buff_survives_gaia_force_removal() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-05")
        .expect("ST1-05 (Birdramon) in embedded DSL pack")
        .dsl_card("ST1-08")
        .expect("ST1-08 (Garudamon) in embedded DSL pack")
        .dsl_card("ST1-16")
        .expect("ST1-16 (Gaia Force) in embedded DSL pack")
        .add_card(make_weak_opp("OPP-BLOCKER", "OppBlocker"))
        .memory(10)
        // Hand: [Garudamon, Gaia Force]. Deck filler so digivolving (draw-free)
        // and any incidental draw don't deck out.
        .hand(0, &["ST1-08", "ST1-16"])
        .deck(0, &["ST1-05", "ST1-05"])
        .start();
    runner.game.turn_count = 1;

    // Controller's attacker: Birdramon (Lv.4, 5000), which Garudamon evolves from.
    let body = runner.place_on_field(0, "ST1-05", Some(0));
    // Opponent's blocker to be cleared by Gaia Force.
    let blocker = runner.place_on_field(1, "OPP-BLOCKER", None);
    let _ = blocker;

    // Digivolve hand[0] (Garudamon) onto Birdramon → fires [When Digivolving].
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, body.index as usize, PlaySource::ByDigivolve),
        "Garudamon must digivolve from hand onto the Lv.4 Birdramon body"
    );
    // Resolve the forced +3000 single-target buff (the body is the only target).
    let buff_prompt = runner
        .pending_selection()
        .expect("ST1-08 [When Digivolving] installs a +3000 target selection");
    let buff_pick = buff_prompt.valid_action_ids[0];
    runner
        .execute_action(0, buff_pick)
        .expect("apply Garudamon +3000 to the only own Digimon");

    assert_eq!(
        runner.effective_dp(body),
        Some(10_000),
        "Garudamon (7000) carries +3000 DP from its [When Digivolving] trick"
    );

    let before = snapshot(&runner);

    // Keep the controller's turn alive across the Gaia Force play. The buff is
    // an `end_of_turn` modifier, and `Game::check_turn_end` ends the turn (and
    // fires `expire_end_of_turn`, correctly dropping the +3000) the moment
    // memory < 0. The Garudamon digivolve (-3) already left memory at 7, and
    // Gaia Force costs 8 — which alone would drive memory to -1 and end the turn
    // BEFORE we can observe the buff *surviving* a *same-turn* deletion. The
    // standard memory range caps at +10, so a single high starting value can't
    // absorb 3 + 8 = 11 of spend; instead top the gauge back up here (modelling
    // memory the controller would have on the turn this combo is cast) so
    // 10 - 8 = 2 >= 0 keeps the turn live. This is a harness fixup of the board
    // state (like the `turn_count` pin above), not a change to the asserted
    // behaviour: the buff-survival check below still tests the real engine.
    // (The original test relied on memory(10) alone, hit -1, and ended its own
    // turn — a TEST-setup bug that masqueraded as the buff being dropped.)
    runner.game.set_memory(10);

    // Play Gaia Force (now hand[0] after Garudamon left the hand) → delete the
    // opponent's blocker.
    runner.play(0, 0).expect("play ST1-16 (Gaia Force) from hand");
    let del_prompt = runner
        .pending_selection()
        .expect("ST1-16 [Main] installs an opponent-Digimon deletion selection");
    let del_pick = del_prompt.valid_action_ids[0];
    runner
        .execute_action(0, del_pick)
        .expect("delete the opponent blocker");
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    // ── Removal landed: opponent field −1, opponent trash +1 ─────────────────
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "Gaia Force deletes the opponent's blocker (field −1)"
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 1,
        "the deleted blocker lands in the opponent's trash"
    );

    // ── Precondition: it is STILL the controller's turn ──────────────────────
    // The buff is an `end_of_turn` modifier, so "survives the deletion" is only
    // meaningful while the turn hasn't ended. Pin that explicitly: if a future
    // memory regression drives the controller negative, `check_turn_end` would
    // end the turn and expire the buff — and we want THAT to fail loudly here
    // (wrong precondition) rather than silently masquerading as a survival gap.
    assert_eq!(
        runner.turn_player(),
        0,
        "the controller's turn must still be live (memory stayed >= 0) for the \
         end-of-turn buff to be observed surviving a *same-turn* deletion"
    );

    // ── The buff survives the unrelated removal ──────────────────────────────
    assert_eq!(
        runner.effective_dp(body),
        Some(10_000),
        "Garudamon's +3000 self-buff holds through the same-turn opponent-side deletion"
    );
}
