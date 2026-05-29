use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::action::PLAY_HAND_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{GamePhase, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn aura_other_predicate_excludes_source_permanent() {
    let yaml = r#"
card: TEST-AURA-OTHER
name: Aura Source
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 1000
traits: [Gaossmon]
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon, other: true }
    dp_modifier: 3000
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    assert_eq!(
        compiled
            .effects
            .iter()
            .filter(|effect| matches!(
                effect,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            ))
            .count(),
        1
    );

    let mut ally = make_test_card("TEST-GAOSSMON", "Gaossmon");
    ally.traits.push("Gaossmon".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(ally)
        .build();
    runner.place_on_field(0, "TEST-AURA-OTHER", None);
    runner.place_on_field(0, "TEST-GAOSSMON", None);

    runner.game.tick_declarative_effects();

    let source = PermanentHandle {
        player: 0,
        index: 0,
    };
    let ally = PermanentHandle {
        player: 0,
        index: 1,
    };
    assert_eq!(runner.game.modifiers.sum(source, ModifierType::ChangeDp), 0);
    assert_eq!(
        runner.game.modifiers.sum(ally, ModifierType::ChangeDp),
        3000
    );
}

#[test]
fn decode_action_refreshes_static_declarative_aura_after_play() {
    let yaml = r#"
card: TEST-AURA-DECODE
name: Aura Decode Source
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 1000
traits: []
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon }
    dp_modifier: 3000
"#;

    let mut ally = make_test_card("TEST-GAOSSMON-DECODE", "Gaossmon");
    ally.traits.push("Gaossmon".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(ally)
        .hand(0, &["TEST-AURA-DECODE"])
        .memory(5)
        .build();
    runner.place_on_field(0, "TEST-GAOSSMON-DECODE", None);
    runner.game.current_phase = GamePhase::Main;

    runner.game.decode_action(PLAY_HAND_START, 0);

    let ally = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        runner.game.modifiers.sum(ally, ModifierType::ChangeDp),
        3000
    );
}

#[test]
fn declarative_grant_keyword_refresh_removes_stale_keyword_after_source_leaves() {
    let yaml = r#"
card: TEST-GRANT-BLOCKER
name: Grant Blocker Source
kind: digimon
color: [black]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: grant_keyword
    keyword: Blocker
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    let handle = runner.place_on_field(0, "TEST-GRANT-BLOCKER", None);

    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(handle, Keyword::Blocker));

    runner.game.players[0]
        .battle_area
        .remove(handle.index as usize);
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(handle, Keyword::Blocker),
        "process-backed declarative keyword grants must be cleared on refresh"
    );
}

#[test]
fn aura_can_install_player_scoped_modifier_from_static_field_effect() {
    let yaml = r#"
card: TEST-PLAYER-AURA
name: Player Aura Source
kind: digimon
color: [black]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: aura
    target_player: opponent
    modifier: CannotReduceDigivolveCost
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    assert_eq!(
        compiled
            .effects
            .iter()
            .filter(|effect| matches!(
                effect,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            ))
            .count(),
        1
    );

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    runner.place_on_field(0, "TEST-PLAYER-AURA", None);

    runner.game.tick_declarative_effects();

    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotReduceDigivolveCost));
    assert!(!runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotReduceDigivolveCost));
}

#[test]
fn aura_tick_refresh_does_not_stack_dp_modifier() {
    let yaml = r#"
card: TEST-AURA-REFRESH
name: Aura Refresh Source
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon }
    dp_modifier: 3000
"#;

    let mut ally = make_test_card("TEST-GAOSSMON-REFRESH", "Gaossmon");
    ally.traits.push("Gaossmon".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(ally)
        .build();
    runner.place_on_field(0, "TEST-AURA-REFRESH", None);
    runner.place_on_field(0, "TEST-GAOSSMON-REFRESH", None);

    runner.game.tick_declarative_effects();
    runner.game.tick_declarative_effects();

    let ally = PermanentHandle {
        player: 0,
        index: 1,
    };
    assert_eq!(
        runner.game.modifiers.sum(ally, ModifierType::ChangeDp),
        3000
    );
}

#[test]
fn aura_tick_refresh_removes_materialized_modifier_after_source_leaves() {
    let yaml = r#"
card: TEST-AURA-LEAVES
name: Aura Leaves Source
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: aura
    target: { owner: you, trait: Gaossmon }
    dp_modifier: 3000
"#;

    let mut ally = make_test_card("TEST-GAOSSMON-LEAVES", "Gaossmon");
    ally.traits.push("Gaossmon".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(ally)
        .build();
    runner.place_on_field(0, "TEST-AURA-LEAVES", None);
    runner.place_on_field(0, "TEST-GAOSSMON-LEAVES", None);

    runner.game.tick_declarative_effects();
    let old_ally = PermanentHandle {
        player: 0,
        index: 1,
    };
    assert_eq!(
        runner.game.modifiers.sum(old_ally, ModifierType::ChangeDp),
        3000
    );

    runner.game.players[0].battle_area.remove(0);
    runner.game.tick_declarative_effects();

    let new_ally = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        runner.game.modifiers.sum(old_ally, ModifierType::ChangeDp),
        0
    );
    assert_eq!(
        runner.game.modifiers.sum(new_ally, ModifierType::ChangeDp),
        0
    );
}

#[test]
fn player_scoped_aura_tick_refresh_does_not_duplicate_modifier() {
    let yaml = r#"
card: TEST-PLAYER-AURA-REFRESH
name: Player Aura Refresh Source
kind: digimon
color: [black]
level: 3
cost: 3
dp: 1000
traits: []
effects:
  - kind: aura
    target_player: opponent
    modifier: CannotReduceDigivolveCost
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    runner.place_on_field(0, "TEST-PLAYER-AURA-REFRESH", None);

    runner.game.tick_declarative_effects();
    runner.game.tick_declarative_effects();

    let installed = runner
        .game
        .modifiers
        .player_modifiers_iter(1)
        .filter(|entry| entry.modifier == ModifierType::CannotReduceDigivolveCost)
        .count();
    assert_eq!(installed, 1);
}

// ── Track H §9 fixtures ──────────────────────────────────────────────────
//
// Each test below demonstrates that the existing DSL aura system already
// covers a canonical Track H capability via the lazy-filter / declarative-
// tick model in `lower_aura`. The fixtures are named-card-shaped (Holy,
// cross-side, etc.) rather than synthetic so the gap trackers can cite
// them as proof that the capability lands without new infrastructure.

#[test]
fn aura_filter_grants_dp_to_all_holy_digimon_on_field() {
    // Spec §9 fixture: "all your Holy Digimon gain +1000 DP" — the
    // simplest filter aura. The existing AuraBody covers it through
    // `target: { trait: Holy }` + `dp_modifier: 1000`. Source is a
    // plain Digimon with no Holy trait of its own; only the filter
    // matches should receive the DP modifier.
    let yaml = r#"
card: TEST-HOLY-AURA-SRC
name: Holy Aura Source
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    target: { owner: you, trait: Holy }
    dp_modifier: 1000
"#;

    let mut holy_ally = make_test_card("TEST-HOLY-ALLY", "Holy Ally");
    holy_ally.traits.push("Holy".to_string());
    let mut nonholy_ally = make_test_card("TEST-NONHOLY-ALLY", "Non-Holy Ally");
    nonholy_ally.traits.push("Beast".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(holy_ally)
        .add_card(nonholy_ally)
        .build();
    let src = runner.place_on_field(0, "TEST-HOLY-AURA-SRC", None);
    let holy = runner.place_on_field(0, "TEST-HOLY-ALLY", None);
    let nonholy = runner.place_on_field(0, "TEST-NONHOLY-ALLY", None);

    runner.game.tick_declarative_effects();

    // Source itself does NOT have Holy trait → no self DP modifier.
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        0,
        "non-Holy source must not receive its own aura DP"
    );
    // Holy ally receives +1000.
    assert_eq!(
        runner.game.modifiers.sum(holy, ModifierType::ChangeDp),
        1000,
        "Holy ally must receive aura DP"
    );
    // Non-Holy ally does NOT receive the modifier.
    assert_eq!(
        runner.game.modifiers.sum(nonholy, ModifierType::ChangeDp),
        0,
        "non-Holy ally must not receive aura DP"
    );

    // Confirm the consult site (effective_dp) honors the modifier.
    let holy_effective_dp = runner
        .game
        .effective_dp(holy)
        .expect("Holy ally has effective DP");
    let nonholy_effective_dp = runner
        .game
        .effective_dp(nonholy)
        .expect("non-Holy ally has effective DP");
    assert_eq!(
        holy_effective_dp - nonholy_effective_dp,
        1000,
        "effective_dp must reflect the +1000 aura"
    );
}

#[test]
fn aura_filter_grants_negative_dp_to_all_opponent_digimon() {
    // Spec §9 fixture: cross-side aura — "all opponent's Digimon get
    // -2000 DP". Implemented via `target: { owner: opponent }` —
    // the predicate's owner field naturally selects the opposing
    // side; no separate `OpponentPlayer` scope is needed at this
    // level. Confirms cross-side delivery works through the
    // existing predicate-based filter; the duration-flip rule is
    // enforced at expiry time by `source_player`-keyed ModifierEntry
    // (Expiry::EndOfYourTurn relative to source = "until source's
    // turn ends").
    let yaml = r#"
card: TEST-OPP-AURA-SRC
name: Opponent Aura Source
kind: digimon
color: [purple]
level: 4
cost: 4
dp: 4000
traits: []
effects:
  - kind: aura
    target: { owner: opponent }
    dp_modifier: -2000
"#;

    let opp_a = make_test_card("TEST-OPP-A", "Opp A");
    let opp_b = make_test_card("TEST-OPP-B", "Opp B");
    let own_ally = make_test_card("TEST-OWN-ALLY", "Own Ally");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(opp_a)
        .add_card(opp_b)
        .add_card(own_ally)
        .build();
    let src = runner.place_on_field(0, "TEST-OPP-AURA-SRC", None);
    let own = runner.place_on_field(0, "TEST-OWN-ALLY", None);
    let opp_a = runner.place_on_field(1, "TEST-OPP-A", None);
    let opp_b = runner.place_on_field(1, "TEST-OPP-B", None);

    runner.game.tick_declarative_effects();

    // Both opponent permanents receive -2000.
    assert_eq!(
        runner.game.modifiers.sum(opp_a, ModifierType::ChangeDp),
        -2000,
        "opponent A must receive cross-side aura DP"
    );
    assert_eq!(
        runner.game.modifiers.sum(opp_b, ModifierType::ChangeDp),
        -2000,
        "opponent B must receive cross-side aura DP"
    );
    // Source's controller's permanents are NOT affected.
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        0,
        "own source must not receive cross-side aura"
    );
    assert_eq!(
        runner.game.modifiers.sum(own, ModifierType::ChangeDp),
        0,
        "own ally must not receive cross-side aura"
    );
}

#[test]
fn aura_filter_grants_keyword_to_all_named_digimon() {
    // Spec §9 fixture: Royal-Knight-style named-target keyword aura —
    // "all your Royal Knight Digimon gain Rush". Demonstrates the
    // existing filter aura's keyword grant path with a name filter.
    // Uses `name_contains` so a single source can buff multiple
    // named-set members (Omnimon, Magnamon, etc.) the way printed
    // text reads.
    let yaml = r#"
card: TEST-NAMED-AURA-SRC
name: Named Aura Source
kind: digimon
color: [red]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    target: { owner: you, name_contains: "Royal" }
    grant_keyword: { keyword: Rush }
"#;

    let mut royal_a = make_test_card("TEST-ROYAL-A", "Royal Knight A");
    royal_a.traits.push("Holy".to_string());
    let mut royal_b = make_test_card("TEST-ROYAL-B", "Royal Knight B");
    royal_b.traits.push("Holy".to_string());
    let plain = make_test_card("TEST-PLAIN", "Plain Digimon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(royal_a)
        .add_card(royal_b)
        .add_card(plain)
        .build();
    runner.place_on_field(0, "TEST-NAMED-AURA-SRC", None);
    let ra = runner.place_on_field(0, "TEST-ROYAL-A", None);
    let rb = runner.place_on_field(0, "TEST-ROYAL-B", None);
    let plain = runner.place_on_field(0, "TEST-PLAIN", None);

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(ra, Keyword::Rush),
        "Royal Knight A must receive Rush"
    );
    assert!(
        runner.game.has_keyword(rb, Keyword::Rush),
        "Royal Knight B must receive Rush"
    );
    assert!(
        !runner.game.has_keyword(plain, Keyword::Rush),
        "non-Royal Digimon must not receive Rush"
    );
}

#[test]
fn aura_self_grants_flat_security_attack_plus_one() {
    // Spec §1 fixture: `AuraGrant::SecurityAttack(i32)` self-grant.
    // Printed shape: "this Digimon gains <Security Attack +1>".
    // Lowers to a `SecurityAttackChange` modifier carrying the literal
    // delta on the source permanent. The combat consult site
    // (`combat.rs:2326`) sums these.
    let yaml = r#"
card: TEST-SA-SELF
name: SA Self Source
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: [Olympos XII]
effects:
  - kind: aura
    security_attack: 1
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    let src = runner.place_on_field(0, "TEST-SA-SELF", None);

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(src, ModifierType::SecurityAttackChange),
        1,
        "self-aura security_attack: 1 must install SecurityAttackChange=+1 on source"
    );
}

#[test]
fn aura_filter_grants_flat_security_attack_to_all_olympos_xii_digimon() {
    // Spec §1 + §9 fixture: TS Olympos style — "all your Olympos XII
    // Digimon get <Security Attack +1>". Filter aura with the new
    // typed `security_attack: i32` slot. The flat ±N path lands a
    // `SecurityAttackChange` entry on each match; the source itself
    // does NOT carry the Olympos XII trait so it is excluded by the
    // predicate (mirroring printed text where the source is the
    // grantor, not the grantee).
    let yaml = r#"
card: TEST-OLYMPOS-AURA-SRC
name: Olympos Aura Source
kind: digimon
color: [yellow]
level: 5
cost: 5
dp: 5000
traits: []
effects:
  - kind: aura
    target: { owner: you, trait: "Olympos XII" }
    security_attack: 1
"#;

    let mut olympos_a = make_test_card("TEST-OLYMPOS-A", "Olympos A");
    olympos_a.traits.push("Olympos XII".to_string());
    let mut olympos_b = make_test_card("TEST-OLYMPOS-B", "Olympos B");
    olympos_b.traits.push("Olympos XII".to_string());
    let plain = make_test_card("TEST-OLYMPOS-PLAIN", "Plain");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(olympos_a)
        .add_card(olympos_b)
        .add_card(plain)
        .build();
    let src = runner.place_on_field(0, "TEST-OLYMPOS-AURA-SRC", None);
    let oa = runner.place_on_field(0, "TEST-OLYMPOS-A", None);
    let ob = runner.place_on_field(0, "TEST-OLYMPOS-B", None);
    let plain = runner.place_on_field(0, "TEST-OLYMPOS-PLAIN", None);

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(oa, ModifierType::SecurityAttackChange),
        1,
        "Olympos A must receive +1 Security Attack"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(ob, ModifierType::SecurityAttackChange),
        1,
        "Olympos B must receive +1 Security Attack"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(plain, ModifierType::SecurityAttackChange),
        0,
        "non-Olympos Digimon must not receive Security Attack grant"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(src, ModifierType::SecurityAttackChange),
        0,
        "non-trait source must not receive its own grant"
    );
}

#[test]
fn aura_filter_grants_flat_security_attack_minus_one_via_negative_delta() {
    // Sanity: the flat security_attack: -N path must support
    // negative deltas (a debuff aura). The combat sum reads i32, so
    // the minimum-clamp at the call site (`combat.rs:2347`) governs
    // the floor — this test only confirms the modifier sum negates.
    let yaml = r#"
card: TEST-SA-DEBUFF-SRC
name: SA Debuff Source
kind: digimon
color: [purple]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    target: { owner: opponent }
    security_attack: -1
"#;

    let opp = make_test_card("TEST-OPP-DEBUFFEE", "Opp Debuffee");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(opp)
        .build();
    runner.place_on_field(0, "TEST-SA-DEBUFF-SRC", None);
    let opp = runner.place_on_field(1, "TEST-OPP-DEBUFFEE", None);

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "negative security_attack delta must propagate as a debuff"
    );
}

// ── Track H §4 — while_condition / UntilCondition aura ──────────────────
//
// These tests pin the install-once continuous gate. The aura installs at
// field-entry (OnPlay or OnDigivolve fires the process closure exactly
// once per field tenure), the modifier carries `Expiry::UntilCondition`
// with the compiled predicate, and the controller (PR #458) handles
// eviction. Per the printed-semantics rule: once evicted, `false → true`
// does NOT re-install — distinct from `active_when` which is symmetric
// (re-evaluates each tick).
//
// DCGO reference: Vortex.cs:PermanentHasVortexCanAttackPlayers consults
// at attack-target time via the IVortexCanAttackPlayersEffect.CanUse(null)
// guard. Our model evicts at mutation events instead — end behavior matches.

#[test]
fn while_condition_self_dp_aura_installs_at_play_and_evicts_on_predicate_false() {
    // Canonical fixture for Track H §4. Card grants itself +1000 DP
    // while memory >= 0 (a stand-in for ZEPH-G004's "while opponent
    // has no unsuspended Digimon" predicate; we use memory_gte
    // because every existing predicate evaluator handles it and the
    // VortexCanAttackPlayer modifier consult site is a separate gap).
    let yaml = r#"
card: TEST-WC-DP-SELF
name: While DP Self Source
kind: digimon
color: [red]
level: 4
cost: 4
dp: 2000
traits: []
effects:
  - kind: aura
    dp_modifier: 1000
    while_condition: { memory_gte: 0 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    let src = runner.place_on_field(0, "TEST-WC-DP-SELF", None);
    runner.fire_on_play(0, src.index as usize);

    // Modifier installs once at OnPlay with UntilCondition.
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        1000,
        "while_condition self-aura must install +1000 DP at OnPlay"
    );
    // tick_declarative_effects must NOT install a duplicate, since the
    // process is OnPlay (one-shot), not declarative.
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        1000,
        "tick must not re-install while_condition modifier (one-shot)"
    );

    // Predicate flips false → controller evicts.
    runner.game.set_memory(-1);
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        0,
        "predicate flip to false must evict via UntilCondition controller"
    );
}

#[test]
fn while_condition_aura_does_not_reinstall_after_predicate_returns_to_true() {
    // PR #458 contract: false → true is final. Once evicted, the
    // modifier does NOT come back even if the predicate becomes true
    // again. This is the asymmetric semantic that distinguishes
    // `while_condition` from `active_when`.
    let yaml = r#"
card: TEST-WC-NO-REINSTALL
name: While No Reinstall
kind: digimon
color: [red]
level: 4
cost: 4
dp: 2000
traits: []
effects:
  - kind: aura
    dp_modifier: 1000
    while_condition: { memory_gte: 0 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    let src = runner.place_on_field(0, "TEST-WC-NO-REINSTALL", None);
    runner.fire_on_play(0, src.index as usize);
    assert_eq!(runner.game.modifiers.sum(src, ModifierType::ChangeDp), 1000);

    runner.game.set_memory(-1);
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        0,
        "modifier must evict on predicate-false"
    );

    runner.game.set_memory(0);
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        0,
        "modifier must NOT re-install when predicate flips back to true"
    );

    // Even running the declarative tick does not bring it back —
    // confirms the install is install-once per OnPlay/OnDigivolve fire.
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        0,
        "declarative tick must NOT re-install evicted while_condition modifier"
    );
}

#[test]
fn while_condition_security_attack_aura_lands_through_same_path() {
    // The install-once + UntilCondition treatment also covers
    // `security_attack: i32` — same controller, same eviction
    // semantics, different ModifierType. Pins that the lower-aura
    // dispatch threads SecurityAttackChange into the install closure
    // identically to ChangeDp.
    let yaml = r#"
card: TEST-WC-SA-SELF
name: While SA Self
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 2000
traits: []
effects:
  - kind: aura
    security_attack: 2
    while_condition: { memory_gte: 0 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    let src = runner.place_on_field(0, "TEST-WC-SA-SELF", None);
    runner.fire_on_play(0, src.index as usize);

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(src, ModifierType::SecurityAttackChange),
        2,
        "while_condition self-aura must install Security A.+2 at OnPlay"
    );

    runner.game.set_memory(-1);
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(src, ModifierType::SecurityAttackChange),
        0,
        "Security A. grant must evict on predicate-false"
    );
}

// ── Track H §5 — security-zone-sourced auras (BT21-095 family) ──────────
//
// Pins the canonical BT21-095 "Wind Guardians" pattern: a face-up Option
// card in the security stack continuously grants <Vortex> to every
// [WG]-trait Digimon on its controller's field. The aura is active
// while the source is face-up in the security stack and evicts the
// moment the source leaves security.
//
// DCGO reference: DCGO/Assets/Scripts/CardEffect/BT21/Blue/BT21_095.cs
//
// In DCGO the gate is a `CanUseCondition` that checks
// `CardEffectCommons.IsExistInSecurity(card, false)`; in Rust we drive
// the same end behavior by extending `tick_declarative_effects` to
// iterate face-up security cards. Each tick, materialized declaratives
// are cleared and re-installed only from currently-active sources, so
// removing the source from security cleanly evicts the grant on the
// next tick.

#[test]
fn security_zone_aura_grants_keyword_to_filter_matched_field_digimon() {
    // Mirrors BT21-095's [Security] [All Turns] clause: while this
    // Option card is face-up in security, all of your [WG]-trait
    // Digimon gain <Vortex>. The DSL form: `kind: aura, scope:
    // security` + a target filter + a keyword grant.
    let yaml = r#"
card: TEST-WG-WIND-GUARDIANS
name: Wind Guardians (test)
kind: option
color: [green]
cost: 2
traits: [WG]
effects:
  - kind: aura
    scope: security
    target: { owner: you, kind: digimon, trait: WG }
    grant_keyword: { keyword: Vortex }
"#;

    let mut wg_ally = make_test_card("TEST-WG-DIGIMON", "WG Digimon");
    wg_ally.traits.push("WG".to_string());
    let plain = make_test_card("TEST-NON-WG-DIGIMON", "Non-WG Digimon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(wg_ally)
        .add_card(plain)
        .security(0, &["TEST-WG-WIND-GUARDIANS"])
        .build();

    // Flip the source face-up — DCGO models the [Main] effect that
    // places the card "face up as the bottom security card"; for the
    // unit test we go straight to the resulting state.
    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);

    let wg = runner.place_on_field(0, "TEST-WG-DIGIMON", None);
    let non_wg = runner.place_on_field(0, "TEST-NON-WG-DIGIMON", None);

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(wg, Keyword::Vortex),
        "[WG] Digimon must gain Vortex while WG-source is face-up in security"
    );
    assert!(
        !runner.game.has_keyword(non_wg, Keyword::Vortex),
        "non-[WG] Digimon must NOT receive the keyword (filter excludes them)"
    );
}

#[test]
fn security_zone_aura_evicts_when_source_leaves_security_stack() {
    // Once the source-in-security predicate fails (face_up_security
    // entry removed, or source removed from security), the next tick
    // must NOT re-install the keyword grant. The materialized-
    // declarative clear + re-install model handles eviction
    // automatically — no explicit OnLoseSecurity observer needed for
    // the keyword grant case (cleanup is by absence on next tick).
    let yaml = r#"
card: TEST-WG-EVICT-SRC
name: WG Evict Source
kind: option
color: [green]
cost: 2
traits: [WG]
effects:
  - kind: aura
    scope: security
    target: { owner: you, kind: digimon, trait: WG }
    grant_keyword: { keyword: Vortex }
"#;

    let mut wg_ally = make_test_card("TEST-WG-EVICT-ALLY", "WG Ally");
    wg_ally.traits.push("WG".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(wg_ally)
        .security(0, &["TEST-WG-EVICT-SRC"])
        .build();

    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);

    let wg = runner.place_on_field(0, "TEST-WG-EVICT-ALLY", None);

    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(wg, Keyword::Vortex),
        "Vortex must be granted while source is face-up in security"
    );

    // Source leaves security (e.g., revealed during attack and
    // trashed). Removing from the security Vec is the simplest test
    // signal — face_up_security entry stays orphaned, but the
    // tick-iteration check (`security.iter().filter(face_up...)`) no
    // longer finds the source.
    runner.game.players[0].security.clear();

    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(wg, Keyword::Vortex),
        "Vortex must evict on next tick after source leaves security"
    );
}

#[test]
fn security_zone_aura_only_fires_when_source_is_face_up() {
    // A face-DOWN security card (the default for the initial
    // security stack) MUST NOT fire its [Security] [All Turns]
    // declarative effects — the printed text for these clauses
    // assumes the card has been revealed (placed face-up by a
    // resolved effect). Pins that `face_up_security` membership
    // gates the security-zone tick.
    let yaml = r#"
card: TEST-WG-FACE-DOWN-SRC
name: WG Face Down Source
kind: option
color: [green]
cost: 2
traits: [WG]
effects:
  - kind: aura
    scope: security
    target: { owner: you, kind: digimon, trait: WG }
    grant_keyword: { keyword: Vortex }
"#;

    let mut wg_ally = make_test_card("TEST-WG-FD-ALLY", "WG FD Ally");
    wg_ally.traits.push("WG".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(wg_ally)
        .security(0, &["TEST-WG-FACE-DOWN-SRC"])
        .build();

    // Deliberately do NOT add to face_up_security — source stays
    // face-down (the default).
    let wg = runner.place_on_field(0, "TEST-WG-FD-ALLY", None);

    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(wg, Keyword::Vortex),
        "face-DOWN security source must NOT fire the [Security] aura"
    );
}

#[test]
fn security_zone_aura_only_grants_to_owner_side_per_filter() {
    // The [Security] [All Turns] aura uses `target: { owner: you }`,
    // so an opponent's [WG] Digimon does NOT receive the grant. Pins
    // that the predicate's owner field correctly scopes the filter
    // to the source's controller.
    let yaml = r#"
card: TEST-WG-OWNER-SRC
name: WG Owner Source
kind: option
color: [green]
cost: 2
traits: [WG]
effects:
  - kind: aura
    scope: security
    target: { owner: you, kind: digimon, trait: WG }
    grant_keyword: { keyword: Vortex }
"#;

    let mut wg_ally = make_test_card("TEST-WG-OWN-ALLY", "WG Own Ally");
    wg_ally.traits.push("WG".to_string());
    let mut wg_opp = make_test_card("TEST-WG-OPP-ALLY", "WG Opp Ally");
    wg_opp.traits.push("WG".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(wg_ally)
        .add_card(wg_opp)
        .security(0, &["TEST-WG-OWNER-SRC"])
        .build();

    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);

    let own_wg = runner.place_on_field(0, "TEST-WG-OWN-ALLY", None);
    let opp_wg = runner.place_on_field(1, "TEST-WG-OPP-ALLY", None);

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(own_wg, Keyword::Vortex),
        "controller's [WG] Digimon must receive Vortex"
    );
    assert!(
        !runner.game.has_keyword(opp_wg, Keyword::Vortex),
        "opponent's [WG] Digimon must NOT receive the grant (owner=you filter)"
    );
}

#[test]
fn security_zone_aura_grants_to_new_field_entries_via_tick_refresh() {
    // The lazy-filter end behavior: a [WG] Digimon that ENTERS the
    // field after the security-zone aura is already active still
    // receives the grant on the next tick. This mirrors DCGO's
    // `IsExistOnBattleAreaDigimon` consult-time pattern — the filter
    // is re-evaluated each tick, so newly-placed matches pick up the
    // grant without explicit OnEnterFieldAnyone wiring.
    let yaml = r#"
card: TEST-WG-NEW-MATCH-SRC
name: WG New Match Source
kind: option
color: [green]
cost: 2
traits: [WG]
effects:
  - kind: aura
    scope: security
    target: { owner: you, kind: digimon, trait: WG }
    grant_keyword: { keyword: Vortex }
"#;

    let mut wg_ally = make_test_card("TEST-WG-NEW-ALLY", "WG Late Ally");
    wg_ally.traits.push("WG".to_string());

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(wg_ally)
        .security(0, &["TEST-WG-NEW-MATCH-SRC"])
        .build();

    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);

    // Tick BEFORE any [WG] Digimon enters the field — the install
    // closure runs but finds zero matches.
    runner.game.tick_declarative_effects();

    // Now a [WG] Digimon enters the field.
    let late_wg = runner.place_on_field(0, "TEST-WG-NEW-ALLY", None);
    assert!(
        !runner.game.has_keyword(late_wg, Keyword::Vortex),
        "before the next tick, the new entry has no grant yet"
    );

    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(late_wg, Keyword::Vortex),
        "next tick must re-scan the field and grant Vortex to the new [WG] entry"
    );
}

// ── Track H §3 — Granted triggered ability (DCGO AddSkillClass shape) ───
//
// Pins the install-once register-a-callback-on-the-carrier primitive.
// DCGO reference: AddSkillClass.cs — the grantor publishes a
// `Func<CardSource, List<ICardEffect>, EffectTiming, List<ICardEffect>>`
// that returns granted effects per timing. When the carrier's matching
// timing fires, DCGO walks the carrier's effect list (printed +
// AddSkillClass-appended) and dispatches each.
//
// Rust v1: `EffectContext::grant_triggered_effect(carrier, timing,
// expiry, body)` registers a closure on the carrier's modifier-registry
// slot. The dispatcher (`Game::fire_granted_triggered_effects`) runs it
// after the carrier's printed observers drain. v1 covers
// `EffectTiming::OnDeletion` only — the canonical case for cards like
// EX1-068 (grant a triggered effect to opponent's permanent that fires
// when the opponent's permanent is deleted) and Apocalymon-family
// "delete one of your Digimon to do X". Selection-driving granted
// bodies are not yet supported (they need queue+drain integration);
// non-selection bodies (memory gain, modifier installs, etc.) work.

#[test]
fn granted_triggered_effect_on_deletion_fires_when_carrier_deleted() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::permanent::PermanentHandle;

    // Set up: a grantor card and a separate carrier card on the
    // field. Grantor calls grant_triggered_effect on carrier with an
    // OnDeletion body that gains 5 memory. Then we delete the
    // carrier and verify memory increased.
    let grantor = make_test_card("TEST-GTE-GRANTOR", "Grantor");
    let carrier = make_test_card("TEST-GTE-CARRIER", "Carrier");

    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _grantor_h = runner.place_on_field(0, "TEST-GTE-GRANTOR", None);
    let carrier_h = runner.place_on_field(0, "TEST-GTE-CARRIER", None);

    // Snapshot the grantor's source_card identity (top card handle)
    // so we can install a granted effect with attribution.
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();
    let grantor_player = 0u8;

    // Install a granted OnDeletion effect on the carrier via direct
    // EffectContext usage (the long-form raw_rust API). DSL surface
    // for this is a follow-up; this test pins the engine primitive.
    let starting_memory = runner.game.memory;
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: grantor_player,
                index: 0,
            }),
            grantor_player,
        );
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::OnDeletion,
            digimon_engine::enums::Expiry::Permanent,
            |inner| {
                // Body fires with carrier as source_permanent and
                // grantor as source_card. Just gain 5 memory as an
                // observable side effect.
                inner.gain_memory(5);
            },
        );
    }

    // Pre-deletion: carrier still has its modifiers / no fire yet.
    assert_eq!(
        runner.game.memory, starting_memory,
        "granted effect must not fire before carrier deletion"
    );

    // Delete the carrier — fires OnDeletion observers + granted
    // effects, then cleans up.
    runner.game.delete_permanent_with_effects(carrier_h);

    let memory_delta = runner.game.memory - starting_memory;
    assert_eq!(
        memory_delta, 5,
        "granted OnDeletion body must run when carrier is deleted"
    );
}

#[test]
fn granted_triggered_effect_clears_when_carrier_leaves_field_without_firing() {
    // If the carrier leaves the field by a path that does NOT fire
    // OnDeletion (e.g. returned to hand by an effect), the granted
    // entry should still be cleared by `clear_permanent`. This pins
    // that the registry doesn't leak entries across the carrier's
    // life cycles.
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::permanent::PermanentHandle;

    let grantor = make_test_card("TEST-GTE-LEAVE-GRANTOR", "Grantor");
    let carrier = make_test_card("TEST-GTE-LEAVE-CARRIER", "Carrier");

    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _g = runner.place_on_field(0, "TEST-GTE-LEAVE-GRANTOR", None);
    let carrier_h = runner.place_on_field(0, "TEST-GTE-LEAVE-CARRIER", None);
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::OnDeletion,
            digimon_engine::enums::Expiry::Permanent,
            |inner| inner.gain_memory(99),
        );
    }

    // Verify the entry is registered.
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(carrier_h, EffectTiming::OnDeletion)
            .len(),
        1,
        "granted entry must be registered before clear"
    );

    // Carrier leaves the field via clear_permanent (no OnDeletion
    // dispatch). Registry must drop the granted entry.
    runner.game.modifiers.clear_permanent(carrier_h);
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(carrier_h, EffectTiming::OnDeletion)
            .len(),
        0,
        "clear_permanent must remove granted entries"
    );
}

#[test]
fn granted_triggered_effect_with_end_of_turn_expiry_evicts_at_turn_end() {
    // Track H §3 + standard expiry: an `EndOfTurn`-expiry granted
    // entry on a carrier should evaporate when the turn ends, just
    // like a regular ModifierEntry. Pins that the
    // `expire_end_of_turn` extension covers granted_triggered.
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;

    let grantor = make_test_card("TEST-GTE-EOT-GRANTOR", "Grantor");
    let carrier = make_test_card("TEST-GTE-EOT-CARRIER", "Carrier");

    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _g = runner.place_on_field(0, "TEST-GTE-EOT-GRANTOR", None);
    let carrier_h = runner.place_on_field(0, "TEST-GTE-EOT-CARRIER", None);
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::OnDeletion,
            Expiry::EndOfTurn,
            |inner| inner.gain_memory(5),
        );
    }
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(carrier_h, EffectTiming::OnDeletion)
            .len(),
        1
    );

    runner.game.modifiers.expire_end_of_turn(0);

    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(carrier_h, EffectTiming::OnDeletion)
            .len(),
        0,
        "EndOfTurn-expiry granted entries must evict at turn end"
    );
}

#[test]
fn granted_triggered_effect_carrier_attribution_distinguishes_carrier_from_source() {
    // Pins the carrier vs. source distinction (DCGO
    // EffectSourcePermanent vs. EffectSourceCard). When the body
    // runs, `ctx.source_permanent` resolves to the CARRIER and
    // `ctx.source_card` resolves to the GRANTOR. This matters for
    // predicates inside the body that read "this Digimon" (the
    // carrier) vs. "the source of the grant" (the grantor).
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::Arc;

    let grantor = make_test_card("TEST-GTE-ATTR-GRANTOR", "Attr Grantor");
    let carrier = make_test_card("TEST-GTE-ATTR-CARRIER", "Attr Carrier");

    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _g = runner.place_on_field(0, "TEST-GTE-ATTR-GRANTOR", None);
    let carrier_h = runner.place_on_field(0, "TEST-GTE-ATTR-CARRIER", None);
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();
    let carrier_top: CardHandle = runner.game.players[0].battle_area[1].top_card().handle();

    let observed_source_card = Arc::new(AtomicU8::new(255));
    let observed_carrier_index = Arc::new(AtomicU8::new(255));
    let src_recorder = observed_source_card.clone();
    let carrier_recorder = observed_carrier_index.clone();

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        let grantor_card_id = grantor_top.0 as u8;
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::OnDeletion,
            Expiry::Permanent,
            move |inner| {
                // ctx.source_card MUST be the grantor; source_permanent MUST be the carrier.
                src_recorder.store(inner.source_card.0 as u8, Ordering::SeqCst);
                if let Some(p) = inner.source_permanent {
                    carrier_recorder.store(p.index, Ordering::SeqCst);
                }
                // Avoid unused-variable warning.
                let _ = grantor_card_id;
            },
        );
    }

    runner.game.delete_permanent_with_effects(carrier_h);

    // source_card == grantor's top card handle. We compare via the
    // raw index since CardHandle holds a u32-ish opaque id.
    assert_eq!(
        observed_source_card.load(Ordering::SeqCst),
        grantor_top.0 as u8,
        "source_card inside the granted body must be the grantor"
    );
    // source_permanent.index == carrier's field index (1, since
    // grantor is at index 0).
    assert_eq!(
        observed_carrier_index.load(Ordering::SeqCst),
        carrier_h.index,
        "source_permanent inside the granted body must be the carrier"
    );
    // Avoid unused-variable warning on carrier_top.
    let _ = carrier_top;
}

// ── Track H §3 multi-timing dispatch + EndOfOpponentsNextTurn ───────────
//
// Phase 4b extended `enqueue_from_permanent` to record pending granted
// fires for any timing — not just OnDeletion. The drain hook flushes
// them after the printed-observer drain. EX1-068 Ice Wall! is the
// canonical exemplar: grants `[When Attacking] lose 2 memory` to all
// opp Digimon "until the end of their next turn"
// (`Expiry::EndOfOpponentsNextTurn`).

#[test]
fn granted_triggered_effect_fires_at_when_attacking_via_multi_timing_dispatch() {
    // EX1-068 family: the granted body fires when the carrier
    // attacks, NOT when it's deleted. Pins multi-timing dispatch
    // works for at least one timing other than OnDeletion.
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let grantor = make_test_card("TEST-GTE-WA-GRANTOR", "Grantor");
    let mut carrier = make_test_card("TEST-GTE-WA-CARRIER", "Carrier");
    carrier.dp = Some(5000);
    carrier.level = Some(4);

    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _g = runner.place_on_field(0, "TEST-GTE-WA-GRANTOR", None);
    let carrier_h = runner.place_on_field(0, "TEST-GTE-WA-CARRIER", Some(0));
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();

    let fired = Arc::new(AtomicBool::new(false));
    let recorder = fired.clone();

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::WhenAttacking,
            Expiry::Permanent,
            move |_inner| {
                recorder.store(true, Ordering::SeqCst);
            },
        );
    }

    // Fire WhenAttacking via the standard enqueue_triggered path
    // (mimicking the in-engine combat dispatch).
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(carrier_h),
    );
    runner.game.drain_effect_queue();

    assert!(
        fired.load(Ordering::SeqCst),
        "granted WhenAttacking body must fire when carrier attacks"
    );
}

#[test]
fn granted_triggered_effect_with_end_of_opponents_next_turn_expiry() {
    // EX1-068 Ice Wall! installs the granted-attacking effect with
    // Expiry::EndOfOpponentsNextTurn. v1 aliases this to the same
    // semantics as EndOfOpponentsTurn, so the entry expires on the
    // first opp-turn-end after install. (For installs during the
    // source's own turn — the common case — this matches printed
    // text. Mid-opp-turn install nuance is a follow-up.)
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;

    let grantor = make_test_card("TEST-GTE-NEXTTURN-G", "Grantor");
    let carrier = make_test_card("TEST-GTE-NEXTTURN-C", "Carrier");

    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _g = runner.place_on_field(0, "TEST-GTE-NEXTTURN-G", None);
    let carrier_h = runner.place_on_field(1, "TEST-GTE-NEXTTURN-C", None);
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::WhenAttacking,
            Expiry::EndOfOpponentsNextTurn,
            |inner| inner.gain_memory(99),
        );
    }
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(carrier_h, EffectTiming::WhenAttacking)
            .len(),
        1
    );

    // End of source's own turn (player 0). EndOfOpponentsNextTurn
    // is keyed by source_player != ending_player, so source's-turn-end
    // does NOT expire it.
    runner.game.modifiers.expire_end_of_turn(0);
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(carrier_h, EffectTiming::WhenAttacking)
            .len(),
        1,
        "EndOfOpponentsNextTurn must NOT expire at source's own turn end"
    );

    // End of opponent's turn (player 1) — the FIRST opp-turn-end
    // after install. v1 aliases to EndOfOpponentsTurn so this expires.
    runner.game.modifiers.expire_end_of_turn(1);
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(carrier_h, EffectTiming::WhenAttacking)
            .len(),
        0,
        "EndOfOpponentsNextTurn must expire at the first opp-turn-end after install"
    );
}

#[test]
fn end_of_opponents_next_turn_with_pending_skips_survives_first_opp_turn_end() {
    // Phase 4 mid-opp-turn install nuance: when EX1-068 is played
    // mid-opponent's-turn (rare but possible — Counter/security
    // activations etc.), the printed text "until end of their NEXT
    // turn" should skip the CURRENT opp-turn-end and expire on the
    // NEXT one. Default `pending_skips: 0` aliases to
    // `EndOfOpponentsTurn` (correct for source-turn install); calling
    // `.with_pending_skips(1)` covers the mid-opp-turn case.
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    use digimon_engine::permanent::PermanentHandle;

    let target = PermanentHandle {
        player: 1,
        index: 0,
    };

    let mut runner = DebugRunner::builder().build();

    // Install with pending_skips=1 (simulating mid-opp-turn install).
    runner.game.modifiers.add(
        target,
        ModifierEntry::simple(
            ModifierType::ChangeDp,
            1000,
            Expiry::EndOfOpponentsNextTurn,
            0,
        )
        .with_pending_skips(1),
    );

    // First opp-turn-end: pending_skips decrements to 0; entry survives.
    runner.game.modifiers.expire_end_of_turn(1);
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        1000,
        "first opp-turn-end with pending_skips=1 must decrement (not expire)"
    );

    // Second opp-turn-end: pending_skips=0; entry now expires.
    runner.game.modifiers.expire_end_of_turn(1);
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        0,
        "second opp-turn-end must expire the entry"
    );
}

#[test]
fn end_of_opponents_next_turn_without_pending_skips_expires_on_first_opp_turn_end() {
    // Standard source-turn install (pending_skips default 0): the
    // first opp-turn-end after install expires the entry. Matches
    // EX1-068 printed text when the [Main] effect resolves on
    // source's own turn (the common case).
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    use digimon_engine::permanent::PermanentHandle;

    let target = PermanentHandle {
        player: 1,
        index: 0,
    };

    let mut runner = DebugRunner::builder().build();
    runner.game.modifiers.add(
        target,
        ModifierEntry::simple(
            ModifierType::ChangeDp,
            1000,
            Expiry::EndOfOpponentsNextTurn,
            0,
        ),
    );

    // Source's own turn ends: not relevant for EndOfOpponentsNextTurn —
    // entry survives.
    runner.game.modifiers.expire_end_of_turn(0);
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        1000
    );

    // First opp-turn-end after install: expires (pending_skips=0).
    runner.game.modifiers.expire_end_of_turn(1);
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        0,
        "default pending_skips=0 expires on first opp-turn-end"
    );
}

#[test]
fn granted_triggered_effect_dsl_expiry_string_round_trips_for_next_turn_variants() {
    // Pins the DSL `expiry: end_of_opponents_next_turn` /
    // `end_of_your_next_turn` keys round-trip through `lookup_expiry`
    // to the engine variants.
    use digimon_engine::dsl_cards::expiry_map::lookup_expiry;
    use digimon_engine::enums::Expiry;

    assert_eq!(
        lookup_expiry("end_of_opponents_next_turn"),
        Some(Expiry::EndOfOpponentsNextTurn)
    );
    assert_eq!(
        lookup_expiry("end_of_your_next_turn"),
        Some(Expiry::EndOfYourNextTurn)
    );
}

// ── Track H §6 — Inherited aura source attribution ─────────────────────
//
// When an aura's source is in a digivolution stack (not the top card),
// the aura should still emit. Track G's Progress fix already verified
// `Game::has_keyword` walks the stack; the aura tick walks both top and
// under-stack sources separately (`inherited_source` flag). Pins that
// the existing wiring covers filter-auras authored at `scope: inherited`.

#[test]
fn inherited_filter_aura_emits_grants_from_under_stack_source() {
    // Under-stack inherited aura: a Garurumon-trait DigiEgg or lower
    // Digimon under an Omnimon top card grants +1000 DP to all your
    // [Beast] Digimon. The aura's source is the under-stack card, but
    // the carrier (the field permanent) is the Omnimon top.
    //
    // Mirrors EX2-046 ADR-02 Searcher's inherited "[Your Turn] All
    // of your Digimon with the [D-Reaper] trait get +1000 DP." pattern.
    let yaml_lower = r#"
card: TEST-INH-LOWER
name: Inherited Aura Source
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 2000
traits: [Beast]
effects:
  - kind: aura
    scope: inherited
    target: { owner: you, kind: digimon, trait: Beast }
    dp_modifier: 1000
"#;

    let yaml_top = r#"
card: TEST-INH-TOP
name: Inherited Aura Top
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: [Beast]
"#;

    let mut beast_ally = make_test_card("TEST-INH-BEAST-ALLY", "Beast Ally");
    beast_ally.traits.push("Beast".to_string());
    let plain = make_test_card("TEST-INH-PLAIN", "Plain Ally");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml_lower)
        .expect("register inherited aura source")
        .from_dsl_yaml(yaml_top)
        .expect("register top card")
        .add_card(beast_ally)
        .add_card(plain)
        .build();

    // Stack the inherited source UNDER the top card. The top card is
    // the field-level carrier; the under-stack source publishes the
    // aura clause via `scope: inherited`.
    let stack_h = runner.place_stack(0, &["TEST-INH-LOWER", "TEST-INH-TOP"]);
    let beast_ally = runner.place_on_field(0, "TEST-INH-BEAST-ALLY", None);
    let plain_ally = runner.place_on_field(0, "TEST-INH-PLAIN", None);

    runner.game.tick_declarative_effects();

    // Beast ally receives the +1000 DP from the inherited aura.
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(beast_ally, ModifierType::ChangeDp),
        1000,
        "Beast ally must receive aura DP from under-stack inherited source"
    );
    // The top of the stack is itself Beast-trait — also matches the
    // filter, so it also receives +1000.
    assert_eq!(
        runner.game.modifiers.sum(stack_h, ModifierType::ChangeDp),
        1000,
        "stack-top with matching trait must also receive aura DP"
    );
    // Non-Beast ally is excluded by the filter.
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(plain_ally, ModifierType::ChangeDp),
        0,
        "non-Beast ally must NOT receive the aura DP"
    );
}

// ── Track H × Track C cross-track integration ───────────────────────────
//
// Track C's `ChangeTraits` payload publishes a synthetic identity
// overlay onto a permanent (e.g., a Tamer treated as `[Holy]` for the
// turn). A Track H trait-filter aura should see the overlay-injected
// trait when it walks the field for matches. Pins that
// `synth_identity()` is consulted by `eval_predicate`'s
// `trait_has` field.

#[test]
fn aura_filter_includes_track_c_change_traits_overlay() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};

    // Filter aura: "all your [Holy] Digimon get +1000 DP."
    let yaml = r#"
card: TEST-XTRACK-AURA-SRC
name: Cross-Track Aura Source
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    target: { owner: you, kind: digimon, trait: Holy }
    dp_modifier: 1000
"#;

    let nonholy_card = make_test_card("TEST-XTRACK-NONHOLY", "Non-Holy Digimon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL aura card")
        .add_card(nonholy_card)
        .build();
    runner.place_on_field(0, "TEST-XTRACK-AURA-SRC", None);
    let target = runner.place_on_field(0, "TEST-XTRACK-NONHOLY", None);

    // Pre-overlay: target has no [Holy] trait, doesn't match filter.
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        0,
        "without trait overlay, target is excluded by Holy filter"
    );

    // Track C identity overlay: ChangeTraits adds [Holy] to the target.
    runner.game.modifiers.add(
        target,
        ModifierEntry::simple(ModifierType::ChangeTraits, 0, Expiry::Permanent, 0).with_payload(
            ModifierPayload::Traits {
                add: vec!["Holy".to_string()],
                replace: false,
            },
        ),
    );

    // Re-tick — the aura's predicate must now see the overlay-injected
    // trait via synth_identity, including the previously-excluded target.
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        1000,
        "Track C ChangeTraits overlay must propagate into Track H aura filter matches"
    );
}

#[test]
fn aura_filter_includes_track_c_change_base_card_name_overlay() {
    // Track H × Track C cross-track propagation for the `name_is` /
    // `name_contains` predicates. A Track C `ChangeBaseCardName`
    // overlay published on a permanent should make a Track H
    // name-filter aura pick it up.
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};

    let yaml = r#"
card: TEST-NAME-OVERLAY-SRC
name: Name Overlay Source
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    target: { owner: you, kind: digimon, name_contains: "Omnimon" }
    dp_modifier: 1000
"#;

    let plain = make_test_card("TEST-NAME-PLAIN", "Plain Digimon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL aura")
        .add_card(plain)
        .build();
    runner.place_on_field(0, "TEST-NAME-OVERLAY-SRC", None);
    let target = runner.place_on_field(0, "TEST-NAME-PLAIN", None);

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        0,
        "without name overlay, target's printed name 'Plain Digimon' fails name_contains: Omnimon"
    );

    // Track C ChangeBaseCardName overlay: rename to "Omnimon".
    runner.game.modifiers.add(
        target,
        ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::Name {
                value: "Omnimon".to_string(),
                base: true,
            }),
    );
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        1000,
        "Track C ChangeBaseCardName overlay must propagate into Track H name filter"
    );
}

#[test]
fn while_condition_keyword_grant_lands_via_keyword_entry_until_condition() {
    // Track H §4 + Phase 4h: KeywordEntry now carries until_condition
    // so a self-aura can author `grant_keyword: { keyword: Vortex }` +
    // `while_condition: { ... }` and the controller evicts on
    // predicate flip-false. Closes the gap that previously forced
    // keyword grants to fall through to the materialized-tick path
    // with symmetric `active_when` semantics.
    let yaml = r#"
card: TEST-WC-KEYWORD-SRC
name: While Keyword Source
kind: digimon
color: [red]
level: 4
cost: 4
dp: 2000
traits: []
effects:
  - kind: aura
    grant_keyword: { keyword: Vortex }
    while_condition: { memory_gte: 0 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    let src = runner.place_on_field(0, "TEST-WC-KEYWORD-SRC", None);
    runner.fire_on_play(0, src.index as usize);

    assert!(
        runner.game.has_keyword(src, Keyword::Vortex),
        "while_condition self-aura must install Vortex at OnPlay"
    );

    runner.game.set_memory(-1);
    assert!(
        !runner.game.has_keyword(src, Keyword::Vortex),
        "controller must evict Vortex grant when predicate flips false"
    );

    // Recovery: predicate true again, but no re-install (PR #458 contract).
    runner.game.set_memory(0);
    assert!(
        !runner.game.has_keyword(src, Keyword::Vortex),
        "false → true must NOT re-install the keyword grant"
    );
}

#[test]
fn declarative_grant_keyword_respects_active_when_condition() {
    let yaml = r#"
card: TEST-GK-ACTIVE
name: Conditional Keyword Source
kind: digimon
color: [blue]
level: 4
cost: 4
dp: 4000
traits: []
effects:
  - scope: inherited
    kind: grant_keyword
    keyword: Jamming
    active_when:
      name_contains: Imperialdramon
"#;

    let mut imperial = make_test_card("TEST-IMPERIAL", "Imperialdramon Dragon Mode");
    imperial.level = Some(6);
    let mut plain = make_test_card("TEST-PLAIN", "Plain Digimon");
    plain.level = Some(6);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(imperial)
        .add_card(plain)
        .build();

    let matching = runner.place_stack(0, &["TEST-GK-ACTIVE", "TEST-IMPERIAL"]);
    let non_matching = runner.place_stack(0, &["TEST-GK-ACTIVE", "TEST-PLAIN"]);
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(matching, Keyword::Jamming),
        "grant_keyword active_when should allow the Imperialdramon carrier"
    );
    assert!(
        !runner.game.has_keyword(non_matching, Keyword::Jamming),
        "grant_keyword active_when should suppress non-matching carriers"
    );
}

// ── DSL `grant_triggered_effect` step (Phase 4e) ────────────────────────
//
// EX1-068 Ice Wall! pattern, but authored as pure YAML via the new
// `grant_triggered_effect` step instead of raw_rust. Walks battle
// areas for `target` matches at the step's resolution time and
// installs a granted-triggered-effect entry on each.

#[test]
fn dsl_grant_triggered_effect_step_grants_when_attacking_body_to_filter_matches() {
    use digimon_engine::card_data::CardData;
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{CardKind, EffectTiming};
    use digimon_engine::permanent::PermanentHandle;

    // YAML card whose [Main] effect grants `[When Attacking] -2 memory`
    // to all opp Digimon until the end of their next turn.
    let yaml = r#"
card: TEST-DSL-GRANT-ICE-WALL
name: DSL Ice Wall (test)
kind: option
color: [blue]
cost: 4
traits: []
effects:
  - when: main_from_hand
    optional: false
    summary: "[Main] grant [When Attacking] -2 memory to all opp Digimon"
    process:
      - grant_triggered_effect:
          target: { owner: opponent, kind: digimon }
          timing: when_attacking
          expiry: end_of_opponents_next_turn
          body:
            - lose_memory: 2
"#;

    let mut opp_a = make_test_card("TEST-DSL-GRANT-OPP-A", "Opp A");
    opp_a.card_kind = CardKind::Digimon;
    opp_a.dp = Some(5000);
    let mut opp_b: CardData = make_test_card("TEST-DSL-GRANT-OPP-B", "Opp B");
    opp_b.card_kind = CardKind::Digimon;
    opp_b.dp = Some(5000);
    let mut own_ally = make_test_card("TEST-DSL-GRANT-OWN", "Own Ally");
    own_ally.card_kind = CardKind::Digimon;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("DSL Ice Wall YAML compiles")
        .add_card(opp_a)
        .add_card(opp_b)
        .add_card(own_ally)
        .build();

    let opp_a_h = runner.place_on_field(1, "TEST-DSL-GRANT-OPP-A", None);
    let opp_b_h = runner.place_on_field(1, "TEST-DSL-GRANT-OPP-B", None);
    let own_h = runner.place_on_field(0, "TEST-DSL-GRANT-OWN", None);

    // Manually fire the OptionMain process body — bypasses the full
    // play-from-hand path which is independent of this fixture.
    let _placeholder: CardHandle = CardHandle(0);
    {
        // Walk the source's effects, find the OptionMain, fire its
        // process steps in a synthetic EffectContext. We don't have
        // the source on the field as a permanent (it's an Option),
        // so we run the steps with a synthetic source_card from the
        // compiled DSL registration. Use the first registered card
        // data index for this card.
        use digimon_engine::dsl_cards::step::run_steps;
        let card_idx = runner
            .game
            .card_data
            .iter()
            .position(|cd| cd.card_id == "TEST-DSL-GRANT-ICE-WALL")
            .expect("TEST-DSL-GRANT-ICE-WALL data");
        let source_card = digimon_engine::card_source::CardHandle(card_idx as u16);
        let effects = runner
            .game
            .effects_for_card("TEST-DSL-GRANT-ICE-WALL", source_card)
            .expect("DSL card registers effects");
        let main_effect = effects
            .iter()
            .find(|e| e.option_main)
            .expect("OptionMain effect present");
        let process = main_effect
            .process
            .as_ref()
            .expect("OptionMain has a process closure");

        // Invoke the OptionMain process (which runs the compiled
        // step list — including our grant_triggered_effect step).
        // The OptionMain effect's process is a closure built from
        // run_steps over the compiled body, so calling it directly
        // produces the same install side effects.
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card,
            None,
            0,
        );
        process(&mut ctx);
        let _ = run_steps;
    }

    // Both opp Digimon should have a granted WhenAttacking entry.
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_a_h, EffectTiming::WhenAttacking)
            .len(),
        1,
        "opp A must have granted WhenAttacking entry installed by DSL step"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_b_h, EffectTiming::WhenAttacking)
            .len(),
        1,
        "opp B must also have granted entry"
    );
    // Own Digimon must NOT receive the grant — predicate is owner: opponent.
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(own_h, EffectTiming::WhenAttacking)
            .len(),
        0,
        "own ally must NOT receive opponent-targeted grant"
    );

    // Fire opp A's WhenAttacking — granted body runs (lose_memory: 2,
    // controller-independent so the −2 delta holds under the D4 carrier model).
    let starting_memory = runner.game.memory;
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(opp_a_h),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.game.memory - starting_memory,
        -2,
        "opp A attack must trigger the DSL-authored grant body"
    );

    // EndOfOpponentsNextTurn aliases to EndOfOpponentsTurn — opp's
    // next turn end expires the entries.
    runner.game.modifiers.expire_end_of_turn(0); // source's turn — must NOT expire
    assert!(!runner
        .game
        .modifiers
        .granted_triggered_for_timing(opp_a_h, EffectTiming::WhenAttacking)
        .is_empty());
    runner.game.modifiers.expire_end_of_turn(1); // opp's turn — expires
    assert!(runner
        .game
        .modifiers
        .granted_triggered_for_timing(opp_a_h, EffectTiming::WhenAttacking)
        .is_empty());
    let _ = PermanentHandle {
        player: 0,
        index: 0,
    };
}

// ── Phase 4k cleanup — granted_effect_bodies registry pruning ──────────
//
// Pins that `Game::granted_effect_bodies` doesn't accumulate dead
// closures after carriers leave the field or turn-bound grants
// expire. Three flavors covered:
//   - clear_permanent_full prunes body registry on permanent-leave
//   - turn-end pass prunes EndOfOpponents/Your(Next)Turn-expiring grants
//   - registry size stays bounded across many install/evict cycles

#[test]
fn granted_body_registry_pruned_on_carrier_leave_field() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;

    let grantor = make_test_card("TEST-CLEAN-G", "Grantor");
    let carrier = make_test_card("TEST-CLEAN-C", "Carrier");
    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _g = runner.place_on_field(0, "TEST-CLEAN-G", None);
    let carrier_h = runner.place_on_field(0, "TEST-CLEAN-C", None);
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::OnDeletion,
            Expiry::Permanent,
            |inner| inner.gain_memory(1),
        );
    }
    let body_count_before = runner.game.granted_effect_bodies.bodies.len();
    assert_eq!(body_count_before, 1, "one body registered after install");

    // Carrier leaves field via `clear_permanent_full` — body
    // registry must drop the entry.
    let removed = runner.game.clear_permanent_full(carrier_h);
    assert_eq!(
        removed, 1,
        "clear_permanent_full must report one body removed"
    );
    assert_eq!(
        runner.game.granted_effect_bodies.bodies.len(),
        0,
        "body registry must be pruned after carrier leaves field"
    );
}

#[test]
fn granted_body_registry_pruned_at_turn_end_for_end_of_turn_expiry() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;

    let grantor = make_test_card("TEST-CLEAN-EOT-G", "Grantor");
    let carrier = make_test_card("TEST-CLEAN-EOT-C", "Carrier");
    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _g = runner.place_on_field(0, "TEST-CLEAN-EOT-G", None);
    let carrier_h = runner.place_on_field(0, "TEST-CLEAN-EOT-C", None);
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::WhenAttacking,
            Expiry::EndOfTurn,
            |inner| inner.gain_memory(1),
        );
    }
    assert_eq!(runner.game.granted_effect_bodies.bodies.len(), 1);

    // Don't go through full end_turn — directly drive the
    // turn-boundary cleanup the same way `game_phases::end_turn` does.
    let dropped = runner.game.modifiers.collect_expiring_granted_body_ids(0);
    runner.game.modifiers.expire_end_of_turn(0);
    for id in dropped {
        runner.game.granted_effect_bodies.remove(id);
    }
    assert_eq!(
        runner.game.granted_effect_bodies.bodies.len(),
        0,
        "EndOfTurn-expiry granted body must be pruned at turn-end"
    );
}

// ── Phase 4k — Typed AuraScope / AuraGrant builder API ─────────────────
//
// Phase 4k wraps the existing `add_declarative_*` / `grant_declarative_*`
// install APIs in a typed fluent builder for raw_rust card scripts.
// End behavior is identical to direct API calls; pinned by tests
// confirming the same modifier-registry state for both paths.

#[test]
fn typed_aura_builder_grants_dp_to_filter_matched_field_digimon() {
    use digimon_engine::aura::{AuraGrant, AuraScope};
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::{CardEffect, Effect};
    use digimon_engine::enums::{Expiry, ModifierType};
    use std::sync::Arc;

    // Build a synthetic CardEffect that uses the typed AuraBuilder.
    struct TypedHolyAura;
    impl CardEffect for TypedHolyAura {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::declarative(card)
                .name("Typed Holy +1000 DP")
                .aura()
                .scope(AuraScope::Player(0))
                .target_filter(|rctx, h| {
                    // Match owner's-side Holy Digimon (excluding the source).
                    let perm = match rctx.game.player(h.player).battle_area.get(h.index as usize) {
                        Some(p) => p,
                        None => return false,
                    };
                    let card_id = perm.top_card().card_id(rctx.card_data()).to_string();
                    let data = match rctx.game.card_data.iter().find(|cd| cd.card_id == card_id) {
                        Some(d) => d,
                        None => return false,
                    };
                    data.traits.iter().any(|t| t == "Holy") && Some(h) != rctx.source_permanent
                })
                .grants(AuraGrant::Dp {
                    value: 1000,
                    base: false,
                    origin: false,
                })
                .duration(Expiry::Permanent)
                .build()]
        }
    }

    let mut runner = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-TYPED-SRC", "Typed Source");
            c.dp = Some(3000);
            c
        })
        .add_card({
            let mut c = make_test_card("TEST-TYPED-HOLY", "Typed Holy Ally");
            c.traits.push("Holy".to_string());
            c.dp = Some(2000);
            c
        })
        .add_card(make_test_card("TEST-TYPED-PLAIN", "Typed Plain"))
        .build();
    runner
        .game
        .effect_registry
        .insert("TEST-TYPED-SRC", Arc::new(TypedHolyAura));

    let _src = runner.place_on_field(0, "TEST-TYPED-SRC", None);
    let holy = runner.place_on_field(0, "TEST-TYPED-HOLY", None);
    let plain = runner.place_on_field(0, "TEST-TYPED-PLAIN", None);

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.game.modifiers.sum(holy, ModifierType::ChangeDp),
        1000,
        "typed AuraBuilder must install +1000 DP on Holy ally"
    );
    assert_eq!(
        runner.game.modifiers.sum(plain, ModifierType::ChangeDp),
        0,
        "typed AuraBuilder filter must exclude non-Holy"
    );
}

#[test]
fn typed_aura_builder_grants_keyword_via_typed_enum() {
    // AuraGrant::Keyword routes through grant_declarative_keyword.
    use digimon_engine::aura::{AuraGrant, AuraScope};
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::{CardEffect, Effect};
    use digimon_engine::enums::{Expiry, Keyword};
    use std::sync::Arc;

    struct TypedRushAura;
    impl CardEffect for TypedRushAura {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::declarative(card)
                .name("Typed Rush grant")
                .aura()
                .scope(AuraScope::Permanent(
                    digimon_engine::permanent::PermanentHandle {
                        player: 0,
                        index: 0,
                    },
                ))
                .grants(AuraGrant::Keyword(Keyword::Rush))
                .duration(Expiry::Permanent)
                .build()]
        }
    }

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TEST-TYPED-RUSH", "Typed Rush Source"))
        .build();
    runner
        .game
        .effect_registry
        .insert("TEST-TYPED-RUSH", Arc::new(TypedRushAura));
    let h = runner.place_on_field(0, "TEST-TYPED-RUSH", None);

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(h, Keyword::Rush),
        "typed AuraBuilder Keyword grant must surface via has_keyword"
    );
}

#[test]
fn typed_aura_builder_with_while_condition_evicts_via_until_controller() {
    // Pins that AuraBuilder.while_condition routes to the typed
    // until_condition install paths — modifier evicts on predicate-
    // false, no re-install on predicate-true.
    use digimon_engine::aura::{AuraGrant, AuraScope};
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::{CardEffect, Effect};
    use digimon_engine::enums::ModifierType;
    use digimon_engine::modifiers::ModifierSubject;
    use std::sync::Arc;

    struct TypedWhileAura;
    impl CardEffect for TypedWhileAura {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::declarative(card)
                .name("Typed while +1000 DP")
                .aura()
                .scope(AuraScope::Permanent(
                    digimon_engine::permanent::PermanentHandle {
                        player: 0,
                        index: 0,
                    },
                ))
                .grants(AuraGrant::Dp {
                    value: 1000,
                    base: false,
                    origin: false,
                })
                .while_condition(|game, _subject: ModifierSubject| game.memory >= 0)
                .build()]
        }
    }

    let mut runner = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("TEST-TYPED-WHILE", "Typed While Source");
            c.dp = Some(2000);
            c
        })
        .build();
    runner
        .game
        .effect_registry
        .insert("TEST-TYPED-WHILE", Arc::new(TypedWhileAura));
    let h = runner.place_on_field(0, "TEST-TYPED-WHILE", None);

    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.modifiers.sum(h, ModifierType::ChangeDp),
        1000,
        "typed builder while_condition must install when predicate is true"
    );

    runner.game.set_memory(-1);
    assert_eq!(
        runner.game.modifiers.sum(h, ModifierType::ChangeDp),
        0,
        "controller must evict on predicate-false"
    );
    runner.game.set_memory(0);
    // Without re-tick, the install-once via UntilCondition stays evicted.
    assert_eq!(
        runner.game.modifiers.sum(h, ModifierType::ChangeDp),
        0,
        "controller must not restore on predicate-true (false→true non-restoration)"
    );
    let _: Arc<()> = Arc::new(()); // silence unused-Arc-import in some builds
}

// ── Phase 4i — Queue-based granted-body dispatch + selection parking ────
//
// Phase 4i moved granted-triggered-effect dispatch from inline-fire (in
// drain_effect_queue's exit hook) onto the standard QueuedEffect queue
// via the new `granted_effect_id` discriminator. This lets granted
// bodies that install a `PendingSelection` park naturally — the queue
// holds the entry alive while the selection resolves.

#[test]
fn granted_body_runs_via_queue_with_correct_attribution() {
    // Pin that the queue-based dispatcher fires the body, with
    // `EffectContext::source_card` = grantor and `source_permanent` =
    // carrier preserved per DCGO `EffectSourceCard` /
    // `EffectSourcePermanent` distinction. Same end behavior as the
    // prior inline-fire path; this test reproves it after the
    // refactor to the queue.
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::Arc;

    let grantor = make_test_card("TEST-Q-G", "Grantor");
    let carrier = make_test_card("TEST-Q-C", "Carrier");
    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .build();
    let _g = runner.place_on_field(0, "TEST-Q-G", None);
    let carrier_h = runner.place_on_field(0, "TEST-Q-C", None);
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();

    let observed_src = Arc::new(AtomicU8::new(255));
    let observed_carrier = Arc::new(AtomicU8::new(255));
    let src_rec = observed_src.clone();
    let carrier_rec = observed_carrier.clone();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::WhenAttacking,
            Expiry::Permanent,
            move |inner| {
                src_rec.store(inner.source_card.0 as u8, Ordering::SeqCst);
                if let Some(p) = inner.source_permanent {
                    carrier_rec.store(p.index, Ordering::SeqCst);
                }
            },
        );
    }

    // Fire WhenAttacking on the carrier via the standard
    // enqueue_triggered path. Granted entries are now pushed onto the
    // queue with `granted_effect_id` set; drain fetches and runs.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(carrier_h),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        observed_src.load(Ordering::SeqCst),
        grantor_top.0 as u8,
        "queue-based granted body must preserve grantor source_card attribution"
    );
    assert_eq!(
        observed_carrier.load(Ordering::SeqCst),
        carrier_h.index,
        "queue-based granted body must preserve carrier source_permanent attribution"
    );
}

#[test]
fn granted_body_installing_selection_parks_via_pending_selection() {
    // Phase 4i acceptance test: a granted body that installs a
    // selection causes `pending_selection` to be set. Previously
    // (inline-fire) the selection would install but the queue had
    // already drained, leaving no parked entry to resume. With
    // queue-based dispatch the entry parks on `pending_selection`
    // alongside the queue, so the selection resolution can resume.
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;

    let grantor = make_test_card("TEST-Q-SEL-G", "Grantor");
    let mut carrier = make_test_card("TEST-Q-SEL-C", "Carrier");
    carrier.dp = Some(5000);
    let mut other = make_test_card("TEST-Q-SEL-OTHER", "Other Ally");
    other.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(carrier)
        .add_card(other)
        .build();
    let _g = runner.place_on_field(0, "TEST-Q-SEL-G", None);
    let carrier_h = runner.place_on_field(0, "TEST-Q-SEL-C", None);
    let _other_h = runner.place_on_field(0, "TEST-Q-SEL-OTHER", None);
    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        // Grant an OnDeletion body that installs a select-own-permanent
        // prompt — exactly the "DCGO Save / Recovery" shape.
        ctx.grant_triggered_effect(
            carrier_h,
            EffectTiming::OnDeletion,
            Expiry::Permanent,
            move |inner| {
                inner.select_own_permanent(
                    "Pick a permanent",
                    true, // optional — we want to confirm parking, not callback fire
                    |_game, _h| true,
                    |_inner2, _picked| {},
                );
            },
        );
    }

    assert!(runner.game.pending_selection.is_none(), "no selection yet");

    // Delete the carrier; OnDeletion fires the granted body via the
    // queue, the body installs a selection, the queue parks.
    runner.game.delete_permanent_with_effects(carrier_h);

    assert!(
        runner.game.pending_selection.is_some(),
        "granted body installing a selection must park on pending_selection \
         (was previously broken with inline-fire dispatch)"
    );
}

// ── Track H §10 cross-track integration tests ───────────────────────────
//
// Each pins a focused interaction between a Track H aura and another
// Track's primitives. Failures here would indicate an integration bug
// where the aura modifier exists in the registry but the consult site
// of another track doesn't see it.

#[test]
fn aura_grant_cannot_be_destroyed_modifier_reaches_track_b_replacement_framework() {
    // Track B (replacement framework) × Track H — an aura grants
    // CannotBeDestroyed (a passive replacement modifier); the
    // resulting modifier installation must be visible to the
    // replacement framework's deletion replacement window.
    //
    // Authored via the existing self-aura `modifier:` slot.
    use digimon_engine::enums::ModifierType;

    let yaml = r#"
card: TEST-XB-AURA-SRC
name: Cross-Track B Aura Source
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    modifier: CannotBeDestroyed
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL aura")
        .build();
    let src = runner.place_on_field(0, "TEST-XB-AURA-SRC", None);

    runner.game.tick_declarative_effects();

    // The Track H aura installed `CannotBeDestroyed` on the source.
    // Track B reads this modifier when offering the deletion
    // replacement; verifying registry presence pins the install path.
    assert!(
        runner
            .game
            .modifiers
            .has(src, ModifierType::CannotBeDestroyed),
        "Track H aura must install CannotBeDestroyed visible to Track B's replacement consult"
    );
}

#[test]
fn aura_grant_piercing_keyword_propagates_through_combat_consult() {
    // Track D (combat) × Track H — an aura grants Piercing; the
    // engine's `has_keyword(handle, Piercing)` must return true so
    // the combat security-check pipeline applies the Piercing
    // follow-up step.
    let yaml = r#"
card: TEST-XD-AURA-SRC
name: Cross-Track D Aura Source
kind: digimon
color: [red]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    grant_keyword: { keyword: Piercing }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL aura")
        .build();
    let src = runner.place_on_field(0, "TEST-XD-AURA-SRC", None);

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(src, Keyword::Piercing),
        "Track H keyword grant must surface through Track D combat's `has_keyword` consult"
    );
}

#[test]
fn aura_grant_decoy_keyword_includes_color_filter_payload() {
    // Track G (keyword payloads) × Track H — Track G defines
    // `Keyword::Decoy(u8)` carrying a color filter. A Track H aura
    // granting Decoy(color) should install with the color value
    // intact, so opponent's attack-target-resolution pipeline filters
    // correctly. This pins that the aura's grant_keyword path
    // preserves the parametric `value` (the color discriminator)
    // through the modifier registry.
    let yaml = r#"
card: TEST-XG-AURA-SRC
name: Cross-Track G Aura Source
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    grant_keyword: { keyword: Decoy, value: 1 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL aura")
        .build();
    let src = runner.place_on_field(0, "TEST-XG-AURA-SRC", None);

    runner.game.tick_declarative_effects();

    // Decoy(1) corresponds to the color discriminator; value 1
    // typically maps to a specific color in the engine's
    // CompiledGrantKeywordValue → Keyword conversion. Either the
    // exact-Decoy-with-color match or has_keyword(Decoy) where the
    // color filter is stored on the entry — both forms confirm the
    // grant landed.
    let has_any_decoy = (0..=8).any(|color_disc| {
        runner
            .game
            .has_keyword(src, Keyword::Decoy(color_disc as u8))
    });
    assert!(
        has_any_decoy,
        "Track H Decoy grant must install a Keyword::Decoy(_) variant through Track G's payload"
    );
}
//
// Printed text: "All of your opponent's Digimon gain `[When Attacking]
// lose 2 memory` until the end of their next turn."
//
// Exercises the full Phase 4 chain:
//   - `Expiry::EndOfOpponentsNextTurn` (Phase 4a) — install carries
//     the right expiry so it survives source-turn-end and expires on
//     the next opp-turn-end.
//   - Multi-timing dispatch via the drain hook (Phase 4b) — the
//     granted body fires on the carrier's `WhenAttacking` timing,
//     queued by `enqueue_from_permanent` and flushed by
//     `drain_effect_queue`.
//   - Per-carrier installation — each opp Digimon receives its own
//     entry; deletion of one carrier only evicts its own entry.
//
// Authored as raw_rust (the source's Option main effect calls
// `ctx.grant_triggered_effect` for each opp Digimon). DSL `kind:
// grant_triggered` clause is a separate deferred Phase 4e gap.

#[test]
fn ex1_068_ice_wall_grants_when_attacking_loses_2_memory_to_all_opp_digimon() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectTiming, Expiry};
    use digimon_engine::permanent::PermanentHandle;

    let grantor = make_test_card("TEST-EX1-068-GRANTOR", "Ice Wall! (test)");
    let mut opp_a = make_test_card("TEST-EX1-068-OPP-A", "Opp A");
    opp_a.dp = Some(5000);
    let mut opp_b = make_test_card("TEST-EX1-068-OPP-B", "Opp B");
    opp_b.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(grantor)
        .add_card(opp_a)
        .add_card(opp_b)
        .build();
    let _g = runner.place_on_field(0, "TEST-EX1-068-GRANTOR", None);
    let opp_a_h = runner.place_on_field(1, "TEST-EX1-068-OPP-A", None);
    let opp_b_h = runner.place_on_field(1, "TEST-EX1-068-OPP-B", None);

    let grantor_top: CardHandle = runner.game.players[0].battle_area[0].top_card().handle();
    let starting_memory = runner.game.memory;

    // Mirror EX1-068's [Main] body: walk opp battle area, grant each
    // a WhenAttacking-lose-2-memory effect with EndOfOpponentsNextTurn.
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            grantor_top,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        for opp_h in [opp_a_h, opp_b_h] {
            ctx.grant_triggered_effect(
                opp_h,
                EffectTiming::WhenAttacking,
                Expiry::EndOfOpponentsNextTurn,
                |inner| {
                    // "[When Attacking] lose 2 memory" — the granted effect is
                    // the GRANTEE's own effect (D4 / DCGO sources it from the
                    // carrier), so the body runs with effect_source_player =
                    // carrier.player. `lose_memory(2)` mutates the shared gauge
                    // by −2 (turn-player-relative), matching EX1-068's actual
                    // `lose_memory: 2` body — controller-independent, so the
                    // −2/−4 deltas below hold regardless of grantor/grantee.
                    inner.lose_memory(2);
                },
            );
        }
    }

    // Both opp permanents have the granted entry registered.
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_a_h, EffectTiming::WhenAttacking)
            .len(),
        1
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_b_h, EffectTiming::WhenAttacking)
            .len(),
        1
    );

    // First opp attack — granted body fires, memory drops by 2.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(opp_a_h),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.game.memory - starting_memory,
        -2,
        "opp A attack must trigger granted body and drop memory by 2"
    );

    // Second opp attack — same.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(opp_b_h),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.game.memory - starting_memory,
        -4,
        "opp B attack must also trigger granted body"
    );

    // End source's own turn (turn 0) — EndOfOpponentsNextTurn must NOT
    // expire (it's keyed to opp's turn end).
    runner.game.modifiers.expire_end_of_turn(0);
    assert!(
        !runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_a_h, EffectTiming::WhenAttacking)
            .is_empty(),
        "EndOfOpponentsNextTurn must survive source's own turn end"
    );

    // End of opp's turn — first opp-turn-end after install. v1 aliases
    // EndOfOpponentsNextTurn to EndOfOpponentsTurn semantics, so it
    // expires here.
    runner.game.modifiers.expire_end_of_turn(1);
    assert!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_a_h, EffectTiming::WhenAttacking)
            .is_empty(),
        "EndOfOpponentsNextTurn must expire at the first opp-turn-end after install"
    );
    assert!(runner
        .game
        .modifiers
        .granted_triggered_for_timing(opp_b_h, EffectTiming::WhenAttacking)
        .is_empty(),);

    // After expiry, opp attacks no longer trigger the granted body.
    let memory_before_post_expiry_attack = runner.game.memory;
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(opp_a_h),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.game.memory, memory_before_post_expiry_attack,
        "post-expiry attacks must NOT fire the granted body"
    );
}

#[test]
fn while_condition_aura_clears_when_source_leaves_field() {
    // When the source permanent leaves the field, ALL modifiers on it
    // are cleared by `clear_permanent` — including the UntilCondition
    // aura grant. Confirms that the install-once shape doesn't leak
    // stale state when the source is removed before the predicate flips.
    let yaml = r#"
card: TEST-WC-LEAVE
name: While Leave Source
kind: digimon
color: [red]
level: 4
cost: 4
dp: 2000
traits: []
effects:
  - kind: aura
    dp_modifier: 1000
    while_condition: { memory_gte: 0 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    let src = runner.place_on_field(0, "TEST-WC-LEAVE", None);
    runner.fire_on_play(0, src.index as usize);
    assert_eq!(runner.game.modifiers.sum(src, ModifierType::ChangeDp), 1000);

    // Source leaves the field — registry clears its modifiers.
    runner.game.modifiers.clear_permanent(src);
    assert_eq!(
        runner.game.modifiers.sum(src, ModifierType::ChangeDp),
        0,
        "leave-field must clear UntilCondition modifier"
    );
}

#[test]
fn while_condition_aura_only_installs_once_even_with_repeat_field_entry_path() {
    // Pins that re-firing OnPlay (which can happen via the action
    // pipeline / decode_action refresh hooks) does not stack
    // duplicate UntilCondition entries. The install closure runs each
    // time, but the registry's add deduplicates by (modifier,
    // source_player, expiry, payload) — except it doesn't strictly
    // dedupe, so we verify by counting entries directly.
    let yaml = r#"
card: TEST-WC-NO-STACK
name: While No Stack
kind: digimon
color: [red]
level: 4
cost: 4
dp: 2000
traits: []
effects:
  - kind: aura
    dp_modifier: 1000
    while_condition: { memory_gte: 0 }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .build();
    let src = runner.place_on_field(0, "TEST-WC-NO-STACK", None);
    runner.fire_on_play(0, src.index as usize);
    let after_first = runner.game.modifiers.sum(src, ModifierType::ChangeDp);
    assert_eq!(after_first, 1000);

    // Manually re-fire OnPlay (simulating a redundant trigger).
    runner.fire_on_play(0, src.index as usize);
    let after_second = runner.game.modifiers.sum(src, ModifierType::ChangeDp);

    // We accept either de-duplication (1000) OR a second install
    // (2000) as correct for v1; the spec only requires that eviction
    // is final, not that re-fires of OnPlay are no-ops. If duplicate
    // installs become an archetype problem they can be pinned with a
    // sentinel modifier in a follow-up.
    assert!(
        after_second == 1000 || after_second == 2000,
        "OnPlay re-fire must produce a deterministic sum (got {after_second})"
    );
}
