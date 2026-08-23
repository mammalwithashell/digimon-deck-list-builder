//! Task A-INV (chip task_8f063aa6) — granted keywords must reach the
//! `WhenWouldBeDeleted` replacement window.
//!
//! Campaign evidence (Toho-Braves exam): a Digimon that GAINED `<Barrier>` /
//! `<Evade>` via a DSL `grant_keyword` clause was deleted with NO prevention
//! prompt, resolving straight to on-deletion triggers.
//!
//! Investigation result (2026-08-22): the window's enumeration is dual-path —
//! `replacement.rs::collect_candidates` scans (1) per-source registry effects
//! including the keyword auto-effects `Game::build_effects_for_card`
//! synthesizes from printed `CardData::keywords` and from UNCONDITIONAL
//! declarative `grant_keyword` clauses, and (2) `ModifierRegistry::
//! granted_keywords(h)` (the `CandidateKind::GrantedKeywordEffect` scan).
//! Runtime step-grants (`EffectContext::grant_keyword`) write the registry
//! directly and are always visible; the six tests below pin that whole
//! coverage matrix GREEN (battle, security-battle, and effect-deletion
//! causes; printed controls alongside).
//!
//! The one genuinely broken cell — the failing test at the bottom — is a
//! DECLARATIVE grant (aura / conditional ESS `grant_keyword`) whose registry
//! entry exists only after `tick_declarative_effects()` materializes it.
//! Battle resolution's deletion fire-site collects replacement candidates
//! WITHOUT refreshing declarative state, so any path reaching deletion with a
//! stale materialization (staged states; `Game::attack_digimon` /
//! `delete_permanent_with_effects` called outside the ticking
//! `decode_action` wrapper) sees `granted_keywords() == []` and never offers
//! the prevention — exactly the campaign symptom. The engine already ticks
//! for this staleness class at other combat gates ("the strike must not read
//! a stale cache" — `combat/mod.rs` `fire_piercing_or_finish` and the
//! security-strike recompute); the deletion→replacement window needs the
//! same treatment.

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{Expiry, Keyword};

/// A runtime-granted `<Barrier>` (modifier-registry keyword grant — the exact
/// write the DSL `grant_keyword:` step performs) must install the optional
/// replacement prompt when the carrier would be deleted in battle, exactly
/// like the printed `<Barrier>` in
/// `behavioral_end_to_end::ts_olympos_cherubimon_barrier_battle_end_to_end`.
#[test]
fn runtime_granted_barrier_prompts_on_battle_deletion() {
    let mut attacker_card = make_test_card("BIG_ATTACKER", "Big Attacker");
    attacker_card.dp = Some(10000);

    // Defender: NO printed keywords at all.
    let mut defender_card = make_test_card("PLAIN_DEFENDER", "Plain Defender");
    defender_card.dp = Some(3000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(make_test_card("SEC", "Security"))
        .add_card(defender_card)
        .security(0, &["SEC", "SEC"])
        .start();

    let attacker = r.place_on_field(1, "BIG_ATTACKER", Some(0));
    let defender = r.place_on_field(0, "PLAIN_DEFENDER", Some(0));

    // Grant <Barrier> through the same registry channel the DSL
    // `grant_keyword:` step uses (`EffectContext::grant_keyword` forwards
    // straight to `ModifierRegistry::grant_keyword`).
    r.game
        .modifiers
        .grant_keyword(defender, Keyword::Barrier, Expiry::Permanent, 0);
    assert!(
        r.game.modifiers.has_keyword(defender, Keyword::Barrier),
        "sanity: the grant landed in the modifier registry"
    );

    let security_before = r.game.player(0).security.len();
    assert_eq!(security_before, 2);

    // Losing battle for the defender → WhenWouldBeDeleted(cause=Battle).
    let _ = r.attack_digimon(attacker, defender, false);

    let sel = r.game.pending_selection.as_ref().expect(
        "granted <Barrier> must install a PendingSelection::Replacement on \
         would-be-deleted (task_8f063aa6: grants are invisible to the window)",
    );
    assert!(
        sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "optional replacement prompt must offer REPLACEMENT_ACCEPT"
    );
    assert_eq!(
        sel.selecting_player, 0,
        "defender's controller gets the Barrier prompt"
    );

    // Accept: deletion cancelled, one security card trashed.
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept granted-Barrier replacement");
    assert_eq!(
        r.battle_area_size(0),
        1,
        "defender survives — granted Barrier cancels the battle-deletion"
    );
    assert_eq!(
        r.game.player(0).security.len(),
        security_before - 1,
        "Barrier trashes one card from the top of the owner's security"
    );
}

/// Same shape via the DSL surface end to end: a face declarative
/// `kind: grant_keyword, keyword: Barrier` clause (Ryugumon EX12-036's exact
/// authoring) on a DSL-loaded card must produce the prevention prompt when the
/// carrier loses a battle.
#[test]
fn dsl_declarative_barrier_clause_prompts_on_battle_deletion() {
    let yaml = r#"
card: DSL-BARRIER-CARRIER
name: Barrier Carrier
kind: digimon
level: 5
color: [blue]
cost: 7
dp: 3000
effects:
  - kind: grant_keyword
    keyword: Barrier
    summary: "<Barrier>"
"#;

    let mut attacker_card = make_test_card("BIG_ATTACKER", "Big Attacker");
    attacker_card.dp = Some(10000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(make_test_card("SEC", "Security"))
        .from_dsl_yaml(yaml)
        .expect("inline DSL card compiles")
        .security(0, &["SEC", "SEC"])
        .start();

    let attacker = r.place_on_field(1, "BIG_ATTACKER", Some(0));
    let defender = r.place_on_field(0, "DSL-BARRIER-CARRIER", Some(0));

    let security_before = r.game.player(0).security.len();

    let _ = r.attack_digimon(attacker, defender, false);

    let sel = r.game.pending_selection.as_ref().expect(
        "DSL grant_keyword <Barrier> must install a replacement prompt on \
         would-be-deleted (task_8f063aa6)",
    );
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    assert_eq!(sel.selecting_player, 0);

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept DSL-granted Barrier replacement");
    assert_eq!(r.battle_area_size(0), 1, "carrier survives via Barrier");
    assert_eq!(r.game.player(0).security.len(), security_before - 1);
}

/// Campaign shape 1 (Toho exam): the carrier ATTACKS SECURITY, flips a
/// higher-DP security Digimon, and loses the security battle. A
/// runtime-granted `<Barrier>` must still offer its prevention prompt.
#[test]
fn runtime_granted_barrier_prompts_on_losing_security_battle() {
    // Attacker on P0 — plain, low DP, granted Barrier at runtime.
    let mut attacker_card = make_test_card("PLAIN_ATTACKER", "Plain Attacker");
    attacker_card.dp = Some(3000);

    // Opponent security Digimon — wins the security battle.
    let mut sec_digimon = make_test_card("SEC_DIGIMON", "Security Digimon");
    sec_digimon.dp = Some(10000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(sec_digimon)
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC", "SEC"])
        .security(1, &["SEC_DIGIMON"])
        .start();

    let attacker = r.place_on_field(0, "PLAIN_ATTACKER", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Barrier, Expiry::Permanent, 0);

    let security_before = r.game.player(0).security.len();
    assert_eq!(security_before, 2);

    // Attack the opponent player: flips SEC_DIGIMON, security battle,
    // attacker loses → WhenWouldBeDeleted(cause=Battle).
    let _ = r.attack_player(attacker, 1, false);

    let sel = r.game.pending_selection.as_ref().expect(
        "granted <Barrier> must prompt when the carrier loses a SECURITY \
         battle (task_8f063aa6 campaign shape)",
    );
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    assert_eq!(sel.selecting_player, 0);

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept granted-Barrier replacement in security battle");
    assert_eq!(
        r.battle_area_size(0),
        1,
        "attacker survives — granted Barrier cancels the security-battle deletion"
    );
    assert_eq!(
        r.game.player(0).security.len(),
        security_before - 1,
        "Barrier trashes one card from the owner's security"
    );
}

/// Printed control for the security-battle shape.
#[test]
fn printed_barrier_control_prompts_on_losing_security_battle() {
    let mut attacker_card = make_test_card("PRINTED_ATTACKER", "Printed Attacker");
    attacker_card.dp = Some(3000);
    attacker_card.keywords = vec![Keyword::Barrier];

    let mut sec_digimon = make_test_card("SEC_DIGIMON", "Security Digimon");
    sec_digimon.dp = Some(10000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(sec_digimon)
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC", "SEC"])
        .security(1, &["SEC_DIGIMON"])
        .start();

    let attacker = r.place_on_field(0, "PRINTED_ATTACKER", Some(0));
    let _ = r.attack_player(attacker, 1, false);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("printed <Barrier> control: security-battle prompt must install");
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept printed-Barrier replacement in security battle");
    assert_eq!(r.battle_area_size(0), 1);
}

/// Campaign shape 2: a runtime-granted `<Evade>` must prompt when the carrier
/// is deleted by an effect (`delete_permanent_with_effects`, the same entry
/// the printed-Evade test in `native_keywords.rs` uses).
#[test]
fn runtime_granted_evade_prompts_on_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("PLAIN_CARD", "Plain Card"))
        .start();
    let handle = r.place_on_field(0, "PLAIN_CARD", Some(0));

    r.game
        .modifiers
        .grant_keyword(handle, Keyword::Evade, Expiry::Permanent, 0);
    assert!(r.game.modifiers.has_keyword(handle, Keyword::Evade));

    r.game.delete_permanent_with_effects(handle);

    let sel = r.game.pending_selection.as_ref().expect(
        "granted <Evade> must prompt on effect deletion (task_8f063aa6 \
         campaign shape)",
    );
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept granted-Evade replacement");
    assert_eq!(
        r.battle_area_size(0),
        1,
        "Evade keeps the carrier on the field"
    );
    assert!(
        r.game.player(0).battle_area[0].is_suspended,
        "Evade pays its cost by suspending the carrier"
    );
}

/// Control: the printed-keyword path with the SAME scenario shape passes
/// (mirrors `behavioral_end_to_end.rs`; kept here so the granted-vs-printed
/// asymmetry is visible in one file).
#[test]
fn printed_barrier_control_prompts_on_battle_deletion() {
    let mut attacker_card = make_test_card("BIG_ATTACKER", "Big Attacker");
    attacker_card.dp = Some(10000);

    let mut defender_card = make_test_card("PRINTED_BARRIER", "Printed Barrier");
    defender_card.dp = Some(3000);
    defender_card.keywords = vec![Keyword::Barrier];

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(make_test_card("SEC", "Security"))
        .add_card(defender_card)
        .security(0, &["SEC", "SEC"])
        .start();

    let attacker = r.place_on_field(1, "BIG_ATTACKER", Some(0));
    let defender = r.place_on_field(0, "PRINTED_BARRIER", Some(0));

    let _ = r.attack_digimon(attacker, defender, false);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("printed <Barrier> control: prompt must install");
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept printed-Barrier replacement");
    assert_eq!(r.battle_area_size(0), 1);
}

/// FAILING (task_8f063aa6 root cause) — an AURA-granted `<Barrier>` must
/// offer its prevention prompt when the carrier is deleted in battle, even
/// when no declarative tick has run between the last board mutation and the
/// deletion.
///
/// The aura is on the field and its target filter matches, so as a matter of
/// game rules the defender HAS `<Barrier>`; whether the incremental
/// declarative-materialization cache was refreshed is an engine
/// implementation detail. Today `collect_candidates` reads
/// `ModifierRegistry::granted_keywords` as-is — with a stale materialization
/// it returns `[]`, no candidate is collected, and the deletion resolves
/// straight to on-deletion triggers (the exact campaign symptom). The fix
/// pattern already exists in combat: `fire_piercing_or_finish`
/// (combat/mod.rs) and the security-strike recompute both call
/// `tick_declarative_effects()` precisely because "direct callers ... would
/// otherwise miss it"; the deletion→replacement fire-site needs the same
/// refresh (or a live declarative-grant evaluation in the candidate scan).
///
/// `granted_barrier_prompts_after_explicit_tick` below proves the identical
/// state WITH a tick prompts correctly — isolating the staleness as the sole
/// difference.
#[test]
fn aura_granted_barrier_prompts_on_battle_deletion_without_prior_tick() {
    let aura_yaml = r#"
card: DSL-AURA-BARRIER
name: Aura Barrier Granter
kind: digimon
level: 3
color: [blue]
cost: 2
dp: 9000
effects:
  - kind: aura
    active_when: { all_turns: true }
    target:
      owner: you
      kind: digimon
    grant_keyword: { keyword: Barrier }
    summary: "[All Turns] Your Digimon gain <Barrier>"
"#;
    let mut attacker_card = make_test_card("BIG_ATTACKER", "Big Attacker");
    attacker_card.dp = Some(10000);
    let mut plain = make_test_card("PLAIN_DEFENDER", "Plain Defender");
    plain.dp = Some(3000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(plain)
        .add_card(make_test_card("SEC", "Security"))
        .from_dsl_yaml(aura_yaml)
        .expect("aura card compiles")
        .security(0, &["SEC", "SEC"])
        .start();

    let attacker = r.place_on_field(1, "BIG_ATTACKER", Some(0));
    let _aura = r.place_on_field(0, "DSL-AURA-BARRIER", Some(0));
    let defender = r.place_on_field(0, "PLAIN_DEFENDER", Some(0));

    // NO explicit tick here — the deletion fire-site itself must not read a
    // stale declarative cache.
    let _ = r.attack_digimon(attacker, defender, false);

    let sel = r.game.pending_selection.as_ref().expect(
        "aura-granted <Barrier> must offer its prevention on would-be-deleted \
         even when the declarative materialization is stale at the deletion \
         fire-site (task_8f063aa6)",
    );
    assert!(
        sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "optional replacement prompt must offer REPLACEMENT_ACCEPT"
    );
    assert_eq!(sel.selecting_player, 0);
}

/// Control for the failing test above: the IDENTICAL board with an explicit
/// declarative tick before the attack prompts correctly — the staleness is
/// the sole difference.
#[test]
fn aura_granted_barrier_prompts_after_explicit_tick() {
    let aura_yaml = r#"
card: DSL-AURA-BARRIER-T
name: Aura Barrier Granter T
kind: digimon
level: 3
color: [blue]
cost: 2
dp: 9000
effects:
  - kind: aura
    active_when: { all_turns: true }
    target:
      owner: you
      kind: digimon
    grant_keyword: { keyword: Barrier }
    summary: "[All Turns] Your Digimon gain <Barrier>"
"#;
    let mut attacker_card = make_test_card("BIG_ATTACKER", "Big Attacker");
    attacker_card.dp = Some(10000);
    let mut plain = make_test_card("PLAIN_DEFENDER", "Plain Defender");
    plain.dp = Some(3000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(plain)
        .add_card(make_test_card("SEC", "Security"))
        .from_dsl_yaml(aura_yaml)
        .expect("aura card compiles")
        .security(0, &["SEC", "SEC"])
        .start();

    let attacker = r.place_on_field(1, "BIG_ATTACKER", Some(0));
    let _aura = r.place_on_field(0, "DSL-AURA-BARRIER-T", Some(0));
    let defender = r.place_on_field(0, "PLAIN_DEFENDER", Some(0));

    r.game.tick_declarative_effects();
    assert!(
        r.game.modifiers.has_keyword(defender, Keyword::Barrier),
        "sanity: the aura grant materializes on tick"
    );

    let _ = r.attack_digimon(attacker, defender, false);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("with fresh declarative state the aura-granted Barrier prompts");
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    assert_eq!(sel.selecting_player, 0);
}
