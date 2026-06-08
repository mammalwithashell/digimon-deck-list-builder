//! ST-1 Starter Deck Gaia Red — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st-1-gaia-red-model.md`. Red mono-color aggro that
//! races the opponent's security stack: it stacks ＜Security A.＞ for extra
//! checks, pumps DP with a Tamer aura + one-shot option buffs, and clears
//! blockers with Red removal options. Per-card behavioral coverage lives in
//! `tests/cards_behavioral/st1/gaia_red.rs`; this file asserts the cross-card
//! SYSTEM combos (a buff that only stacks across two cards, a removal window
//! that ignores your own buffs) that no per-card test can see.

#![allow(dead_code)]

use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::support::{dsl_builder, snapshot};

// ─── Shared fixtures ─────────────────────────────────────────────────────────

/// Fire a `[When Digivolving]` timing for `handle` (direct enqueue + drain —
/// `place_stack` builds the stack without firing the trigger, so we fire it
/// explicitly to model the digivolve, mirroring `rocks.rs`).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 1 — Security-Attack stacking (Greymon source + WarGreymon top)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST1-11 WarGreymon (active top), ST1-07 Greymon (digivolution source).
///
/// Expected mechanical outcome: a tall WarGreymon stack that contains a Greymon
/// source simultaneously carries TWO independent ＜Security A.＞ contributions —
/// WarGreymon's own dynamic, BASE-INCLUSIVE "1 + floor(material/2)" formula aura
/// AND the Greymon source's inherited flat ＜Security A. +1＞. The card-faithful
/// way to read the combined window is the REAL security-strike count: a 4-source
/// WarGreymon (incl. Greymon) checks **4** security on one connected player
/// attack (base-inclusive aura 3 + Greymon inherited 1). This combined lever is
/// the deck's win condition and is invisible to either card's per-card test.
///
/// Per main's (correct) contract:
/// - `dynamic_security_attack_aura_bonus(stack)` is BASE-INCLUSIVE — it returns
///   the total base check count `1 + floor(n/2)` → `Some(3)` at 4 sources, NOT a
///   bare floor(4/2)=2 delta.
/// - `security_attack_keyword_bonus(stack)` does NOT count the inherited
///   declarative ＜Security A.＞ grant from a source (returns 0). The actual
///   strike counts it via a tick-fresh `current_security_strike`, not this helper.
///
/// Sources:
/// - Printed text: ST1-11 "[Your Turn] for every 2 digivolution cards this
///   Digimon has, it gains ＜Security A. +1＞"; ST1-07 inherited ＜Security A. +1＞.
/// - Rules: `general_rule.pdf` §16-3 (Security Attack keyword — multiple grants
///   stack additively; inherited effects resolve from below the top card).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_11.cs`, `ST1_07.cs`.
#[test]
fn wargreymon_over_greymon_source_stacks_both_security_attack_bonuses() {
    // Real plain security fodder (ST1-02 Biyomon: vanilla 3000-DP Rookie, no
    // effect) so a 4-strike check can't deck the opponent out before we read the
    // diff. WarGreymon (12000) wins every security battle without dying.
    let mut runner = dsl_builder(&["ST1-01", "ST1-03", "ST1-07", "ST1-08", "ST1-11", "ST1-02"])
        .memory(3)
        .security(1, &["ST1-02", "ST1-02", "ST1-02", "ST1-02", "ST1-02"])
        .start();
    // Player 0 is the turn player; pin turn_count so WarGreymon's [Your Turn]
    // ＜Security A.＞ aura is active.
    runner.game.turn_count = 1;

    // Stack [Koromon, Agumon, Greymon, Garudamon, WarGreymon]: 4 digivolution
    // sources under the WarGreymon top, one of which is the Greymon source.
    let stack = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-07", "ST1-08", "ST1-11"]);

    // The Greymon source's flat inherited ＜Security A. +1＞ is present on the top.
    assert!(
        runner
            .game
            .has_keyword(stack, Keyword::SecurityAttackPlus(1)),
        "Greymon (ST1-07) source must grant the WarGreymon top a flat ＜Security A. +1＞"
    );
    // `security_attack_keyword_bonus` deliberately does NOT count the inherited
    // declarative ＜Security A.＞ grant from a source — the real strike counts it
    // via a tick-fresh `current_security_strike`, not this helper. (main's
    // contract; the combined faithful outcome is asserted by the real attack.)
    assert_eq!(
        runner.game.security_attack_keyword_bonus(stack),
        0,
        "security_attack_keyword_bonus does not count a source's inherited declarative \
         ＜Security A.＞ grant; the SECURITY STRIKE counts it via current_security_strike"
    );

    // WarGreymon's own dynamic aura is BASE-INCLUSIVE: it returns the total base
    // check count 1 + floor(4/2) = 3 (default base-1 plus floor(material/2)),
    // NOT a bare delta.
    assert_eq!(
        runner.game.dynamic_security_attack_aura_bonus(stack),
        Some(3),
        "WarGreymon's [Your Turn] formula aura is BASE-INCLUSIVE: 1 + floor(4/2) = 3 \
         (per general_rule.pdf 16-3; DCGO ST1_11.cs computes the floor(n/2) bonus, the \
         engine folds in the base-1)"
    );

    // ── The combined faithful outcome: a REAL attack checks 4 security ──
    // base-inclusive aura 3 + Greymon inherited ＜Security A. +1＞ = 4. The strike
    // refreshes declarative state internally and counts the inherited grant even
    // though this test never ticked manually.
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
        "the full ST-1 line (4 sources incl. Greymon) checks 4 security on one connected \
         player attack: base-inclusive WarGreymon aura 3 + Greymon inherited ＜Security A. +1＞. \
         Per general_rule.pdf 16-3 + DCGO ST1_11.cs / ST1_07.cs"
    );
}

/// Unhappy path: a SHORTER WarGreymon stack (fewer than 4 digivolution cards)
/// still carries the Greymon source's flat ＜Security A. +1＞, but WarGreymon's
/// dynamic per-2-sources aura drops to +1 (2 sources ÷ 2). The dynamic
/// component is gated on stack height — exactly the system fact per-card TDD
/// can't express.
#[test]
fn wargreymon_fewer_sources_lowers_dynamic_security_attack_bonus() {
    let mut runner = dsl_builder(&["ST1-03", "ST1-07", "ST1-11"]).start();

    // Stack [Agumon, Greymon, WarGreymon]: only 2 digivolution sources.
    let stack = runner.place_stack(0, &["ST1-03", "ST1-07", "ST1-11"]);

    // The Greymon source keyword grant is unchanged — it doesn't depend on count.
    assert!(
        runner
            .game
            .has_keyword(stack, Keyword::SecurityAttackPlus(1)),
        "the Greymon flat ＜Security A. +1＞ is present regardless of stack height"
    );

    // The base-inclusive aura is now only 2 (1 + floor(2/2)) — strictly lower
    // than the 4-source value of 3 in the happy path.
    assert_eq!(
        runner.game.dynamic_security_attack_aura_bonus(stack),
        Some(2),
        "WarGreymon's BASE-INCLUSIVE ＜Security A.＞ aura is 1 + floor(2/2) = 2 at 2 \
         digivolution cards — strictly lower than the 4-source value of 3"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 2 — Tai + Shadow Wing lethal DP push
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST1-12 Tai Kamiya (+1000 aura to all your Digimon on your turn),
/// ST1-13 Shadow Wing ([Main] +3000 to one of your Digimon), on one own
/// attacker (ST1-05 Birdramon, base 5000 DP).
///
/// Expected mechanical outcome: the attacker's `effective_dp` =
/// base 5000 + 1000 (Tai aura) + 3000 (Shadow Wing one-shot) = 9000. The Tamer
/// aura and the option's one-shot DP modifier stack additively — a tamer + an
/// option + a Digimon working together, which no single card's test sees.
///
/// Sources:
/// - Printed text: ST1-12 "[Your Turn] all of your Digimon get +1000 DP";
///   ST1-13 "[Main] 1 of your Digimon gets +3000 DP for the turn".
/// - Rules: `general_rule.pdf` §11 (DP — auras and modifiers sum).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_12.cs`, `ST1_13.cs`.
#[test]
fn tai_plus_shadow_wing_stacks_dp_on_own_attacker() {
    let mut runner = dsl_builder(&["ST1-05", "ST1-12", "ST1-13"])
        .hand(0, &["ST1-13"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ST1-05", Some(0));
    // Birdramon at base 5000 before any buff.
    assert_eq!(
        runner.effective_dp(attacker),
        Some(5_000),
        "Birdramon base DP is 5000"
    );

    // Tai Kamiya on field → +1000 to all own Digimon on your turn.
    runner.place_on_field(0, "ST1-12", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(attacker),
        Some(6_000),
        "Tai's aura lifts Birdramon to 6000"
    );

    // Play Shadow Wing [Main] and target Birdramon for +3000 for the turn.
    runner.play(0, 0).expect("play ST1-13 Shadow Wing from hand");
    runner.game.drain_effect_queue();
    runner
        .execute_action(0, encode_attack(0, attacker.index as u16))
        .expect("target Birdramon with Shadow Wing +3000");
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(attacker),
        Some(9_000),
        "Birdramon = base 5000 + Tai 1000 + Shadow Wing 3000 = 9000"
    );
}

/// Unhappy path: NEITHER Tai nor Shadow Wing buffs an opponent Digimon. Tai's
/// aura is own-only and Shadow Wing's [Main] targets "1 of YOUR Digimon", so an
/// opponent body stays at its base DP even with both online on your side.
#[test]
fn tai_and_shadow_wing_do_not_buff_opponent_digimon() {
    // Opponent body is a real vanilla blue Digimon (ST2-05 Ikkakumon, base 5000)
    // — a neutral target with no effects that could perturb the buffs.
    let mut runner = dsl_builder(&["ST1-05", "ST1-12", "ST1-13", "ST2-05"])
        .hand(0, &["ST1-13"])
        .memory(10)
        .start();

    let own = runner.place_on_field(0, "ST1-05", Some(0));
    let opp = runner.place_on_field(1, "ST2-05", Some(0));

    runner.place_on_field(0, "ST1-12", Some(0));
    runner.game.tick_declarative_effects();

    // Play Shadow Wing — its only legal targets are your own Digimon, so the
    // opponent body can never be picked. Target your own Birdramon.
    runner.play(0, 0).expect("play ST1-13 Shadow Wing from hand");
    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("Shadow Wing installs an own-Digimon target prompt");
    // Shadow Wing's selector is `select_own_permanent {kind: digimon}` — own
    // field only. The only own Digimon is the lone Birdramon (Tai is a Tamer
    // and excluded), so the prompt offers exactly one id. An own-field selector
    // cannot encode an opponent target at all, so asserting own-only positively
    // (exactly one legal id) is the faithful check.
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "Shadow Wing offers exactly one own Digimon (Birdramon); the opponent body is never selectable, and Tai the Tamer is excluded"
    );
    let own_target = view.valid_action_ids[0];
    let _ = opp; // opponent body is intentionally never a Shadow Wing target
    runner
        .execute_action(0, own_target)
        .expect("target own Birdramon");
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(opp),
        Some(5_000),
        "opponent body stays at base 5000 — neither Tai nor Shadow Wing touches it"
    );
    assert_eq!(
        runner.effective_dp(own),
        Some(9_000),
        "your Birdramon still receives both buffs (5000 + 1000 + 3000)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 3 — Garudamon When-Digivolving + Tai
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST1-08 Garudamon ([When Digivolving] +3000 to one of your Digimon),
/// ST1-12 Tai Kamiya (+1000 aura), buffing the SAME Digimon.
///
/// Expected mechanical outcome: the Garudamon When-Digivolving +3000 one-shot
/// and Tai's +1000 aura stack additively on the chosen target. Garudamon's
/// own stack (base 7000) becomes 7000 + 1000 (Tai) + 3000 (WD) = 11000 — a
/// burst node that the digivolve effect and the Tamer aura produce together.
///
/// Sources:
/// - Printed text: ST1-08 "[When Digivolving] 1 of your Digimon gets +3000 DP
///   for the turn"; ST1-12 "[Your Turn] all of your Digimon get +1000 DP".
/// - Rules: `general_rule.pdf` §11 (DP sum) + §16 (When Digivolving timing).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_08.cs`, `ST1_12.cs`.
#[test]
fn garudamon_when_digivolving_buff_stacks_additively_with_tai_aura() {
    let mut runner = dsl_builder(&["ST1-05", "ST1-08", "ST1-12"]).memory(10).start();

    // Tai on field for the +1000 aura.
    runner.place_on_field(0, "ST1-12", Some(0));
    // Garudamon stack [Birdramon, Garudamon] — base 7000 top.
    let garudamon = runner.place_stack(0, &["ST1-05", "ST1-08"]);
    runner.game.tick_declarative_effects();

    // Before the digivolve trigger: only Tai's aura applies → 7000 + 1000.
    assert_eq!(
        runner.effective_dp(garudamon),
        Some(8_000),
        "Garudamon base 7000 + Tai 1000 before the When-Digivolving buff"
    );

    // Fire Garudamon's [When Digivolving] and direct its +3000 onto itself.
    fire_when_digivolving(&mut runner, garudamon);
    runner
        .execute_action(0, encode_attack(0, garudamon.index as u16))
        .expect("target Garudamon itself with its +3000 When-Digivolving buff");
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(garudamon),
        Some(11_000),
        "Garudamon = base 7000 + Tai 1000 + When-Digivolving 3000 = 11000"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 4 — Starlight Explosion security wall
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST1-14 Starlight Explosion ([Main]).
///
/// Expected mechanical outcome: playing the [Main] installs the player-level
/// `ChangeOwnSecurityDigimonDp = 7000` modifier — every Security Digimon the
/// attacker meets is +7000 DP, the deck's lone stall pivot. The modifier is
/// absent before the play and exactly 7000 after, demonstrating the defensive
/// window the option opens for the whole player (not a single permanent).
///
/// Sources:
/// - Printed text: ST1-14 "[Main] all of your Security Digimon get +7000 DP
///   until the end of your opponent's next turn".
/// - Rules: `general_rule.pdf` §11 (DP — security-Digimon DP window).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_14.cs`.
#[test]
fn starlight_explosion_installs_own_security_digimon_dp_window() {
    let mut runner = dsl_builder(&["ST1-14"]).hand(0, &["ST1-14"]).memory(10).start();

    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(0, ModifierType::ChangeOwnSecurityDigimonDp),
        0,
        "no own-security DP window before Starlight Explosion is played"
    );

    runner.play(0, 0).expect("play ST1-14 Starlight Explosion from hand");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(0, ModifierType::ChangeOwnSecurityDigimonDp),
        7000,
        "Starlight Explosion installs a +7000 DP window for your Security Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 5 — Giga Destroyer DP-window removal (Tai does not widen it)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST1-15 Giga Destroyer ([Main] delete up to 2 opp Digimon ≤4000 DP),
/// opponent bodies are real vanilla red Digimon at 3000 / 4000 / 5000 DP
/// (ST1-02 Biyomon, ST1-04 Dracomon, ST1-05 Birdramon), ST1-12 Tai on YOUR field.
///
/// Expected mechanical outcome: Giga Destroyer's filter reads the OPPONENT's DP,
/// so it can only delete the 3000- and 4000-DP bodies; the 5000-DP body is
/// outside the window and survives. Tai's +1000 aura buffs YOUR Digimon, not
/// the opponent's — so it never widens (or narrows) this window. The system
/// fact: the removal window is independent of your own DP buffs.
///
/// Sources:
/// - Printed text: ST1-15 "[Main] delete up to 2 of your opponent's Digimon
///   with 4000 DP or less"; ST1-12 own-only +1000 aura.
/// - Rules: `general_rule.pdf` §11 (DP comparison) + deletion semantics.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_15.cs`, `ST1_12.cs`.
///
// CANDIDATE ENGINE BUG (CONFIRMED): `select_count_capped_multi { max:2 } +
// per_selected delete` resolves the selected targets by STALE field-slot
// index. Selection is correct (the second pick still applies `dp_lte 4000`
// and offers only the remaining ≤4000 body — confirmed below), but deletion
// applies the recorded slot indices sequentially WITHOUT accounting for the
// array shift from the first deletion: picking opp-slot0 (Biyomon 3000) +
// opp-slot1 (Dracomon 4000) trashes Biyomon + Birdramon (slot 1 after the first
// delete shifted the array down). Per card text "delete up to 2 ... with 4000 DP
// or less", the two SELECTED ≤4000 bodies must die and the 5000 body must survive.
#[test]
fn giga_destroyer_deletes_only_le_4000_opponents_and_tai_does_not_widen_window() {
    // Opponent bodies are real vanilla red Digimon: ST1-02 Biyomon (3000),
    // ST1-04 Dracomon (4000), ST1-05 Birdramon (5000). The first two are ≤4000
    // (deletable); the 5000 body is outside the window.
    let mut runner = dsl_builder(&["ST1-12", "ST1-15", "ST1-02", "ST1-04", "ST1-05"])
        .hand(0, &["ST1-15"])
        .memory(10)
        .start();

    // Tai on YOUR field — buffs only your Digimon, irrelevant to the opp window.
    runner.place_on_field(0, "ST1-12", Some(0));
    let opp_3k = runner.place_on_field(1, "ST1-02", Some(0));
    let opp_4k = runner.place_on_field(1, "ST1-04", Some(0));
    let opp_5k = runner.place_on_field(1, "ST1-05", Some(0));
    runner.game.tick_declarative_effects();

    // Opponent DPs are untouched by Tai (own-only aura).
    assert_eq!(runner.effective_dp(opp_3k), Some(3_000));
    assert_eq!(runner.effective_dp(opp_4k), Some(4_000));
    assert_eq!(runner.effective_dp(opp_5k), Some(5_000));

    let before = snapshot(&runner);

    runner.play(0, 0).expect("play ST1-15 Giga Destroyer from hand");
    let prompt = runner
        .pending_selection()
        .expect("Giga Destroyer installs an up-to-2 selection");
    // Only the two ≤4000 bodies are eligible — the 5000 body is not offered.
    assert_eq!(
        prompt.valid_action_ids.len(),
        2,
        "only the 3000- and 4000-DP opponent Digimon are legal targets; the 5000-DP body is outside the window"
    );
    assert!(
        !prompt
            .valid_action_ids
            .contains(&encode_attack(0, opp_5k.index as u16)),
        "the 5000-DP body must NOT be an eligible Giga Destroyer target"
    );

    // Select both eligible targets EXPLICITLY by their encoded ids (the two
    // ≤4000 bodies), so the survivor can't be a blind-pick artifact.
    // During a selection prompt, field targets (own OR opponent) are encoded
    // identically as encode_attack(0, slot) = ATTACK_START + slot; which
    // player's field they belong to is carried by the OppField/OwnField
    // selection kind, not the action-id range.
    let pick_3k = encode_attack(0, opp_3k.index as u16);
    let pick_4k = encode_attack(0, opp_4k.index as u16);
    let pick_5k = encode_attack(0, opp_5k.index as u16);
    assert!(
        prompt.valid_action_ids.contains(&pick_3k) && prompt.valid_action_ids.contains(&pick_4k),
        "both ≤4000 bodies are first-pick eligible"
    );
    runner.execute_action(0, pick_3k).expect("pick the 3000-DP target");

    // CONFIRMATION (candidate engine bug #9): after the first ≤4000 pick, the
    // SECOND prompt must still apply the `dp_lte 4000` filter — it must offer
    // exactly the one remaining ≤4000 body (the 4000-DP Dracomon) and must NOT
    // offer the 5000-DP body.
    let second_prompt = runner
        .pending_selection()
        .expect("second Giga Destroyer pick pending");
    assert!(
        !second_prompt.valid_action_ids.contains(&pick_5k),
        "the 5000-DP body must NOT be a SECOND-pick Giga Destroyer target (dp_lte filter must persist across picks)"
    );
    assert!(
        second_prompt.valid_action_ids.contains(&pick_4k),
        "the remaining 4000-DP body must be the SECOND-pick eligible target"
    );
    runner.execute_action(0, pick_4k).expect("pick the 4000-DP target");
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // Exactly the two ≤4000 bodies are deleted; the 5000-DP body survives.
    assert_eq!(
        after.field[1],
        before.field[1] - 2,
        "the two ≤4000 opponent Digimon are deleted"
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 2,
        "both deleted opponent Digimon land in the opponent's trash"
    );
    let survivors: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        survivors,
        vec!["Birdramon".to_string()],
        "the 5000-DP body (Birdramon) is the sole survivor; survivors={survivors:?}"
    );
}
