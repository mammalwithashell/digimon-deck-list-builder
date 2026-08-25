use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::replacement::ReplacementCause;

use super::support::{
    decline_all_selections, field_contains, hand_index, plain_digimon, puppet_tb, push_to_trash,
    select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-065";

#[test]
fn ex12_065_has_fortitude_and_grants_blocker_retaliation_to_puppet_or_tb() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(puppet_tb("PUPPET", 4))
        .start();
    let kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    let puppet = runner.place_on_field(0, "PUPPET", Some(0));
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(kaguyamon, Keyword::Fortitude));
    assert!(runner.game.has_keyword(puppet, Keyword::Blocker));
    assert!(runner.game.has_keyword(puppet, Keyword::Retaliation));
}

#[test]
fn ex12_065_on_play_plays_low_cost_puppet_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(puppet_tb("PUPPET-LOW", 4))
        .hand(0, &[CARD_ID])
        .memory(12)
        .start();
    push_to_trash(&mut runner, 0, "PUPPET-LOW");

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-065");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve trash play");

    assert!(field_contains(&runner, 0, "PUPPET-LOW"));
}

#[test]
fn ex12_065_on_deletion_bottom_decks_opponent_lowest_level_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(plain_digimon("LOW", CardColor::Purple, 3, 3000))
        .add_card(plain_digimon("HIGH", CardColor::Purple, 5, 7000))
        .start();
    let kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));

    runner
        .game
        .delete_permanents_batch(vec![kaguyamon], ReplacementCause::OwnEffect);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve bottom-deck");

    assert_eq!(
        runner.game.players[1]
            .deck
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "LOW"
    );
}

// ─── G-ENGINE-AURA-GRANT-NO-TRIGGER ─────────────────────────────────────────
//
// EX12-065 prints ONE sentence granting two keywords: "[All Turns] All of your
// [Puppet] or [TB] trait Digimon gain <Blocker> and <Retaliation>." `<Blocker>`
// is a persistent flag the mask reads through `Game::has_keyword`, so it always
// worked. `<Retaliation>` is a §16-12 MANDATORY TRIGGER — the DCGO oracle diff
// on `qa/dcgo-exams/EX12/EX12-065-effect3.yaml` step 16 showed DCGO deleting the
// battle winner while we left it alive, because a filtered-target aura installed
// the keyword FLAG on the recipient but no `Effect` anywhere fired it.

/// A NON-source recipient of the mass grant (a separate [Puppet]/[TB] Digimon,
/// not Kaguyamon itself) that LOSES a battle must delete the battle winner.
#[test]
fn ex12_065_mass_granted_retaliation_deletes_battle_winner_on_recipient() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(puppet_tb("PUPPET", 4))
        .add_card(plain_digimon("BIG-ATTACKER", CardColor::Purple, 6, 12000))
        .start();
    let _kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    let puppet = runner.place_on_field(0, "PUPPET", Some(0));
    let attacker = runner.place_on_field(1, "BIG-ATTACKER", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(puppet, Keyword::Retaliation),
        "sanity: the mass grant reaches the recipient's keyword lookup"
    );

    // PUPPET (5000) loses to BIG-ATTACKER (12000) → deleted with cause=Battle
    // → the granted <Retaliation> must delete the winner (§16-12, mandatory).
    // EX12-065 also mass-grants <Blocker>, so the declaration opens an
    // optional block window first — decline it and let the battle resolve.
    let _ = runner.attack_digimon(attacker, puppet, false);
    decline_all_selections(&mut runner);

    assert!(
        !field_contains(&runner, 1, "BIG-ATTACKER"),
        "granted <Retaliation> must delete the battle winner (G-ENGINE-AURA-GRANT-NO-TRIGGER)"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BIG-ATTACKER"),
        "the deleted winner lands in its owner's trash"
    );
    assert!(
        !field_contains(&runner, 0, "PUPPET"),
        "the recipient still loses the battle it lost"
    );
}

/// The GRANTOR is itself a [TB] Digimon, so the same sentence grants it
/// `<Retaliation>` too — via the filtered-target path, not the self-aura
/// marker (`ex12_065_has_fortitude_and_grants_blocker_retaliation_to_puppet_or_tb`
/// pins the keyword lookup; `mass_granted_fortitude_fires_on_its_own_grantor`
/// in `tests/replacements/granted_keywords.rs` pins the grantor-self TRIGGER
/// through a card with no competing clause).
///
/// `#[ignore]`d: EX12-065 cannot express this case cleanly, because its own
/// `[On Deletion]` bottom-deck clause fires from the same deletion and PARKS a
/// selection. Parking unwinds `delete_permanents_batch`, which restores
/// `pending_attack`/`current_deletion_cause` before the resume drains the rest
/// of the OnDeletion bundle — so by the time `<Retaliation>` runs,
/// `EffectContext::battle_opponent_of` reads `pending_attack == None` and the
/// keyword silently no-ops. That is a PRE-EXISTING battle-state-lifetime gap
/// that hits printed and granted `<Retaliation>` identically (it depends only
/// on the sibling clause parking, and on which order the controller picks in
/// the trigger-order prompt) — see `G-ONDELETION-PARK-CLEARS-BATTLE-STATE` in
/// `docs/RUST_ENGINE_GAPS.md`. Kept as the reproducer.
#[test]
#[ignore = "engine gap: G-ONDELETION-PARK-CLEARS-BATTLE-STATE — a parked sibling [On Deletion] clause unwinds the deletion batch and clears `pending_attack`, so <Retaliation> (printed OR granted) finds no battle opponent on resume; see docs/RUST_ENGINE_GAPS.md"]
fn ex12_065_mass_granted_retaliation_also_fires_on_the_grantor_itself() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(plain_digimon("HUGE-ATTACKER", CardColor::Purple, 7, 20000))
        .add_card(plain_digimon("DECOY-LOW", CardColor::Purple, 3, 1000))
        .start();
    let kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(1, "HUGE-ATTACKER", Some(0));
    runner.place_on_field(1, "DECOY-LOW", Some(0));
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(kaguyamon, Keyword::Retaliation));

    let _ = runner.attack_digimon(attacker, kaguyamon, false);
    decline_all_selections(&mut runner);

    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "HUGE-ATTACKER"),
        "the grantor's own granted <Retaliation> must delete the battle winner          (it must land in TRASH — the [On Deletion] bottom-deck clause takes          DECOY-LOW, not the winner)"
    );
}

/// Cause gate survives the grant: §16-12 fires only on BATTLE deletions, so an
/// effect deletion of the recipient must NOT take anything with it.
#[test]
fn ex12_065_mass_granted_retaliation_respects_the_battle_cause_gate() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(puppet_tb("PUPPET", 4))
        .add_card(plain_digimon("BYSTANDER", CardColor::Purple, 5, 7000))
        .start();
    let _kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    let puppet = runner.place_on_field(0, "PUPPET", Some(0));
    runner.place_on_field(1, "BYSTANDER", Some(0));
    runner.game.tick_declarative_effects();

    runner
        .game
        .delete_permanents_batch(vec![puppet], ReplacementCause::OpponentEffect);

    assert!(
        field_contains(&runner, 1, "BYSTANDER"),
        "granted <Retaliation> must not fire on a non-Battle deletion"
    );
}

/// No double-fire: a recipient that ALSO PRINTS `<Retaliation>` must delete the
/// winner exactly once. (The printed keyword already synthesizes its own
/// auto-effect through `build_effects_for_card`; the aura scan must not add a
/// second copy.)
#[test]
fn ex12_065_mass_grant_does_not_double_fire_on_a_printed_retaliation_recipient() {
    let mut printed = puppet_tb("PUPPET-RETAL", 4);
    printed.keywords = vec![Keyword::Retaliation];

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(printed)
        .add_card(plain_digimon("BIG-ATTACKER", CardColor::Purple, 6, 12000))
        .add_card(plain_digimon("SECOND", CardColor::Purple, 5, 7000))
        .start();
    let _kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    let puppet = runner.place_on_field(0, "PUPPET-RETAL", Some(0));
    let attacker = runner.place_on_field(1, "BIG-ATTACKER", Some(0));
    runner.place_on_field(1, "SECOND", Some(0));
    runner.game.tick_declarative_effects();

    let _ = runner.attack_digimon(attacker, puppet, false);
    decline_all_selections(&mut runner);

    assert!(!field_contains(&runner, 1, "BIG-ATTACKER"));
    assert!(
        field_contains(&runner, 1, "SECOND"),
        "a single <Retaliation> deletes only the battle opponent — a second \
         firing would have to find another target, and re-firing on an already \
         deleted winner must not cascade"
    );
    assert_eq!(
        runner.game.players[1]
            .trash
            .iter()
            .filter(|c| c.card_id(&runner.game.card_data) == "BIG-ATTACKER")
            .count(),
        1,
        "the winner is deleted exactly once"
    );
}

/// The three simultaneous `OnDeletion` triggers Kaguyamon raises when it is
/// deleted in battle must be TELLABLE APART, and the only field that can tell
/// them apart is `EffectChoiceEntry::keyword` — `source_card`, `timing` and
/// `is_optional` are identical across all three.
///
/// The three, and where each keyword comes from:
///   * the printed `[On Deletion]` bottom-deck clause (YAML `effects[4]`,
///     lowered to slot 6) — no keyword, it is a plain clause;
///   * `<Fortitude>` (YAML `effects[0]`, a `kind: grant_keyword` clause whose
///     BODY `effects_for_card` synthesizes and appends as slot 7) — resolved
///     through `Effect::keyword_source`, the stamp
///     `keyword_effects::keyword_to_auto_effect` puts on every body it returns;
///   * the `[All Turns]` aura's `<Retaliation>`, which Kaguyamon grants to
///     ITSELF (it is [Puppet]/[TB]) — resolved through
///     `QueuedEffect::keyword_effect`, since an aura-granted body is in no
///     card's effect list at all.
///
/// Without the `keyword_source` half, slot 7 reported `None` and the stack held
/// two indistinguishable branches, leaving the exam's `select:` step nothing to
/// name but a per-engine POSITION.
#[test]
fn ex12_065_simultaneous_on_deletion_branches_are_distinguishable_by_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(plain_digimon("HUGE-ATTACKER", CardColor::Purple, 7, 20000))
        .add_card(plain_digimon("DECOY-LOW", CardColor::Purple, 3, 1000))
        .start();
    let kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(1, "HUGE-ATTACKER", Some(0));
    runner.place_on_field(1, "DECOY-LOW", Some(0));
    runner.game.tick_declarative_effects();

    let _ = runner.attack_digimon(attacker, kaguyamon, false);
    // Walk past the pre-battle window the granted <Blocker> opens.
    advance_to_trigger_order(&mut runner);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("a TriggerOrder prompt is parked");
    let entries = pending
        .effect_choices
        .as_ref()
        .expect("a TriggerOrder prompt carries effect choices");
    assert_eq!(
        entries.len(),
        3,
        "expected three simultaneous OnDeletion triggers, got {entries:?}"
    );

    let keywords: Vec<Option<Keyword>> = entries.iter().map(|e| e.keyword).collect();
    assert!(
        keywords.contains(&Some(Keyword::Fortitude)),
        "the printed <Fortitude> branch must name itself; got {keywords:?} for {entries:?}"
    );
    assert!(
        keywords.contains(&Some(Keyword::Retaliation)),
        "the aura-granted <Retaliation> branch must name itself; got {keywords:?}"
    );
    assert!(
        keywords.contains(&None),
        "the plain printed [On Deletion] clause is not a keyword and must stay \
         unnamed; got {keywords:?}"
    );

    // The point of the field: no two branches collide.
    let mut seen = keywords.clone();
    seen.sort_by_key(|k| format!("{k:?}"));
    seen.dedup();
    assert_eq!(
        seen.len(),
        3,
        "every branch of a same-card trigger stack must be separately \
         addressable; got {keywords:?}"
    );
}

/// Resolve prompts until the simultaneous-trigger prompt is the live one.
/// Declines everything on the way (the granted `<Blocker>` window).
fn advance_to_trigger_order(runner: &mut DebugRunner) {
    use digimon_engine::action::space::PASS;
    use digimon_engine::selection::SelectionKind;

    for _ in 0..16 {
        match runner.game.pending_selection.as_ref().map(|p| p.kind.clone()) {
            Some(SelectionKind::TriggerOrder) => return,
            None => panic!("no TriggerOrder prompt was ever parked"),
            Some(_) => {
                let view = runner
                    .pending_selection_view()
                    .expect("a parked prompt has a view");
                let action = if view.is_optional || view.valid_action_ids.contains(&PASS) {
                    PASS
                } else {
                    *view
                        .valid_action_ids
                        .first()
                        .unwrap_or_else(|| panic!("prompt had no legal action: {view:?}"))
                };
                runner
                    .execute_action(view.selecting_player, action)
                    .expect("resolve pending selection");
            }
        }
    }
    panic!("gave up walking to the TriggerOrder prompt");
}

/// A `<Retaliation>` whose battle condition is not met must never REACH the
/// trigger-order prompt -- not merely resolve to nothing once chosen.
///
/// 16-12 makes `<Retaliation>` fire "when deleted in battle". On an EFFECT
/// deletion it does not trigger at all, so it is not one of the simultaneous
/// triggers 15-4-3-5-1 asks the turn player to order. Offering it anyway is
/// wrong three ways: it puts a mandatory branch in the RL action space that
/// provably does nothing (rule 17), it makes the player order a phantom, and
/// it guarantees a cross-engine divergence -- DCGO filters its stack by
/// `CanActivate` BEFORE staging candidates (MultipleSkills.cs:236), and
/// `CanActivateRetaliation` (CardEffectCommons/KeyWordEffects/Retaliation.cs:
/// 24-52) returns false when the hashtable carries no battle. DCGO offers two
/// branches here; we offered three.
///
/// The cause gate used to live inside the keyword's `process` body, which runs
/// only AFTER the branch has been offered and chosen. It is now the effect's
/// `condition`, which the trigger scan evaluates before queueing.
///
/// This is the shape `qa/dcgo-exams/EX12/EX12-065-effect1.yaml` scripts:
/// Kaguyamon deleted as Kokeshimon's cost -- an effect deletion -- where the
/// real stack is `<Fortitude>` + `[On Deletion]`.
#[test]
fn ex12_065_retaliation_is_not_offered_as_a_branch_on_an_effect_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(plain_digimon("BYSTANDER", CardColor::Purple, 5, 7000))
        .start();
    let kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "BYSTANDER", Some(0));
    runner.game.tick_declarative_effects();

    runner
        .game
        .delete_permanents_batch(vec![kaguyamon], ReplacementCause::OwnEffect);

    if let Some(pending) = runner.game.pending_selection.as_ref() {
        if let Some(choices) = pending.effect_choices.as_ref() {
            let offered: Vec<String> = choices
                .iter()
                .map(|c| format!("{:?}", c.keyword))
                .collect();
            assert!(
                !choices
                    .iter()
                    .any(|c| c.keyword == Some(Keyword::Retaliation)),
                "16-12: <Retaliation> triggers only on a BATTLE deletion, so it \
                 must not be one of the branches offered for an effect \
                 deletion; offered = {offered:?}"
            );
        }
    }
}
