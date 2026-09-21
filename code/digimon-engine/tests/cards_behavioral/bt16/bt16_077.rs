//! BT16-077 Dinobeemon — Lv.5 Purple/Red, DP 8000, Cost 8.
//! The inherited <Partition> copy under Imperialdramon: Dragon Mode is the
//! judge-quiz Q30 board element; the interruptive Partition machinery is
//! pinned by `judge_quiz/c_declare_then_pay.rs` and
//! `keyword_phase_d/partition.rs`.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

#[test]
fn bt16_077_compiles_with_dna_path_and_dual_partition() {
    let r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 YAML parses and compiles")
        .build();
    let card = r.compiled_card("BT16-077").expect("present in pack");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::DnaDigivolve)),
        "the DNA Digivolution (purple Lv.4 + red Lv.4 / 0) path compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving) && t.optional
        )),
        "[When Digivolving] (DNA-gated trash-play) + Rush clause compiles"
    );
    // The inherited Partition copy is a distinct inherited-scope clause.
    let inherited_partitions = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(
                    digimon_dsl::compiled::CompiledDeclarativeClause::Partition { scope, .. }
                ) if *scope == CompiledScope::Inherited
            )
        })
        .count();
    assert_eq!(inherited_partitions, 1, "one inherited <Partition> clause");
}

#[test]
fn bt16_077_grants_raid_and_partition_on_field() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .build();
    let h = r.place_on_field(0, "BT16-077", None);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Raid), "<Raid> granted");
    assert!(
        r.game.has_keyword(h, Keyword::Partition),
        "<Partition> granted"
    );
}

/// [When Digivolving] on an ORDINARY (non-DNA) digivolve: only the trash-play
/// half is gated by "If DNA digivolving"; the "Then, 1 of your Digimon may
/// gain <Rush> ... and attack a player" half still resolves.
///
/// Citations: official Bandai Q&A for BT16-077 ("Even if this Digimon didn't
/// DNA digivolve, this card's [When Digivolving] effect can give 1 of your
/// Digimon <Rush> and that Digimon can attack" — data/card_bundles/BT16-077.md);
/// DCGO `BT16_077.cs:182` gates only the trash-play on `IsJogress`, while the
/// Rush pick at `:231` is unconditional. Exam BT16-077#effect#1 step 15.
#[test]
fn bt16_077_non_dna_digivolve_still_offers_rush_attack_but_not_trash_play() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::{SelectionKind, TriggerSource};

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .memory(3)
        .start();
    // A legal trash-play target (Lv.5 [Free] Digimon) so an ungated trash
    // pick WOULD surface if the DNA gate were missing.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BT16-077")
        .expect("BT16-077 card data");
    let next = r.game.next_card_index();
    r.game.players[0].trash.push(CardSource::new(data_idx, 0, next));

    let h = r.place_on_field(0, "BT16-077", Some(0));
    // Non-DNA WhenDigivolving (no DNA context).
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();

    assert!(
        r.pending_selection().is_some() && r.pending_is_optional(),
        "the optional [When Digivolving] prompt must open on a non-DNA digivolve \
         (the Rush/attack half is not DNA-gated)"
    );
    r.accept_optional_trigger().expect("accept [When Digivolving]");
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OwnField),
        "non-DNA: the trash-play is skipped; the next pick is the Rush target"
    );
}

/// "Then, 1 of your Digimon may gain <Rush> for the turn and attack a player."
/// The attack is part of the effect's own processing -- an IMMEDIATE attack on
/// the opponent player, not a deferred end-of-turn "may attack" grant.
///
/// Citations: DCGO `BT16_077.cs:251-262` -- once a Digimon is chosen, the
/// effect runs `SelectAttackEffect` inline with `defenderCondition: _ => false`
/// (player only) and `SetCanNotSelectNotAttack()` (not declinable); the printed
/// "may" is the Digimon pick (`canNoSelect: true`, :236). general_rule.pdf
/// §11-2-7-1 (attack targets) / §11-2-8-1 (declaration suspends the attacker).
/// Exam BT16-077#effect#3 step 17-18 (DCGO: suspended Dinobeemon, P1 security
/// 5 -> 4 before P1's turn; ours had neither).
#[test]
fn bt16_077_rush_pick_attacks_the_player_immediately() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::{SelectionKind, TriggerSource};

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .add_card(digimon_engine::debug_runner::make_test_card("SEC-2K", "Sec"))
        .security(1, &["SEC-2K", "SEC-2K", "SEC-2K"])
        .memory(3)
        .start();
    let h = r.place_on_field(0, "BT16-077", Some(0));
    let sec_before = r.security_count(1);

    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    r.accept_optional_trigger().expect("accept [When Digivolving]");
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField), "Rush pick");

    // Pick Dinobeemon itself, then answer any (player-only) attack prompt.
    for _ in 0..4 {
        let Some(view) = r.pending_selection_view() else { break };
        let id = *view
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("a non-pass choice");
        r.game
            .resolve_selection(view.selecting_player, id)
            .expect("selection resolves");
        r.game.drain_effect_queue();
    }

    assert!(
        r.game.player(0).battle_area[h.index as usize].is_suspended,
        "the chosen Digimon attacked NOW (suspended by the declaration)"
    );
    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "the effect attack hit the opponent player (1 security check)"
    );
}

// ── <Partition (purple Lv.4 & red Lv.4)> — per-slot source enforcement ───────
//
// G-ENGINE-PARTITION-SLOT-ENFORCEMENT-DEFERRED. Before the fix the keyword
// body parked ONE slot-blind `CountCappedMultiSelect { min: 1, max: 2 }` over
// every digivolution card, so the controller could play a card no slot admits
// (the Lv.5 Dinobeemon itself, under an inherited copy) or play only ONE of
// the two specified cards.
//
// general_rule.pdf §16-28-5 — "the 'specified cards' refers to the cards that
// meet the conditions shown in parentheses in the <Partition> icon text";
// §16-28-6 — "1 of each of the specified cards is played ... A player can't
// choose to only play one or some of the specified cards"; §16-28-1 — the
// trigger itself needs "1 of each of the specified cards in its digivolution
// cards". DCGO builds one filtered candidate list per `PartitionCondition`
// (`CardEffectFactory/KeyWordEffects/Partition.cs:66-119`), refuses to
// activate when any list is empty (`:145-159`), and prompts for a slot only
// when that slot has more than one candidate
// (`CardEffectCommons/KeyWordEffects/Partition.cs:89,117`).

fn partition_source(id: &str, color: digimon_engine::enums::CardColor, level: u8) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: digimon_engine::enums::CardKind::Digimon,
        level: Some(level),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Only the SPECIFIED cards are offered, the pick set is exactly one per
/// printed parenthetical, and a card no slot admits is never playable.
#[test]
fn bt16_077_partition_plays_one_of_each_specified_card_and_ignores_the_rest() {
    use digimon_engine::action::space::{encode_source_select, PASS, REPLACEMENT_ACCEPT};
    use digimon_engine::enums::CardColor;
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .add_card(partition_source("PURPLE-LV4", CardColor::Purple, 4))
        .add_card(partition_source("RED-LV4", CardColor::Red, 4))
        // Matches NEITHER parenthetical (green, and the wrong level).
        .add_card(partition_source("GREEN-LV3", CardColor::Green, 3))
        .start();

    // sources bottom→top: 0 = PURPLE-LV4, 1 = GREEN-LV3, 2 = RED-LV4.
    let carrier = r.place_stack(0, &["PURPLE-LV4", "GREEN-LV3", "RED-LV4", "BT16-077"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept <Partition>");

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("the <Partition> source pick must surface");
    assert!(
        !pending.is_optional && !pending.valid_action_ids.contains(&PASS),
        "§16-28-6: once <Partition> is accepted the picks are mandatory — a \
         player can't choose to play only one or some of the specified cards"
    );
    assert_eq!(
        pending.valid_action_ids,
        vec![
            encode_source_select(0, 0).unwrap(),
            encode_source_select(0, 2).unwrap()
        ],
        "§16-28-5: only the purple Lv.4 and the red Lv.4 are 'specified \
         cards'; the green Lv.3 source is never offered"
    );

    r.game
        .resolve_selection(0, encode_source_select(0, 0).unwrap())
        .expect("pick the purple Lv.4");
    assert!(
        r.game.pending_selection.is_none(),
        "the red Lv.4 is the only card that can complete the slot set, so it \
         is taken without a prompt (DCGO Partition.cs:89,117)"
    );

    let mut on_field = field_ids(&r, 0);
    on_field.sort();
    assert_eq!(
        on_field,
        vec!["PURPLE-LV4".to_string(), "RED-LV4".to_string()],
        "§16-28-6: exactly 1 of EACH specified card is played; the \
         non-specified source is not"
    );
    let trash = trash_ids(&r, 0);
    assert!(
        trash.contains(&"BT16-077".to_string()) && trash.contains(&"GREEN-LV3".to_string()),
        "the carrier and the non-specified source still leave: <Partition> is \
         interruptive, not preventive (§16-28-2). trash = {trash:?}"
    );
}

/// A pick that would strand a slot is masked out: with two purple Lv.4s and
/// two red Lv.4s, taking a purple first removes the OTHER purple from the
/// second pick — one card per parenthetical, never two of the same slot.
#[test]
fn bt16_077_partition_masks_picks_that_cannot_complete_the_slot_set() {
    use digimon_engine::action::space::{encode_source_select, REPLACEMENT_ACCEPT};
    use digimon_engine::enums::CardColor;
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .add_card(partition_source("PURPLE-A", CardColor::Purple, 4))
        .add_card(partition_source("PURPLE-B", CardColor::Purple, 4))
        .add_card(partition_source("RED-A", CardColor::Red, 4))
        .add_card(partition_source("RED-B", CardColor::Red, 4))
        .start();

    // sources bottom→top: 0 = PURPLE-A, 1 = PURPLE-B, 2 = RED-A, 3 = RED-B.
    let carrier = r.place_stack(0, &["PURPLE-A", "PURPLE-B", "RED-A", "RED-B", "BT16-077"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept <Partition>");

    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .expect("first pick")
            .valid_action_ids,
        vec![
            encode_source_select(0, 0).unwrap(),
            encode_source_select(0, 1).unwrap(),
            encode_source_select(0, 2).unwrap(),
            encode_source_select(0, 3).unwrap()
        ],
        "every specified card can start a complete slot assignment"
    );
    r.game
        .resolve_selection(0, encode_source_select(0, 0).unwrap())
        .expect("pick PURPLE-A first");

    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .expect("a second pick is still owed")
            .valid_action_ids,
        vec![
            encode_source_select(0, 2).unwrap(),
            encode_source_select(0, 3).unwrap()
        ],
        "§16-28-6 is ONE of EACH: after a purple Lv.4 is taken the other          purple can no longer fill the red parenthetical and is masked out"
    );

    r.game
        .resolve_selection(0, encode_source_select(0, 3).unwrap())
        .expect("pick RED-B");

    let mut on_field = field_ids(&r, 0);
    on_field.sort();
    assert_eq!(
        on_field,
        vec!["PURPLE-A".to_string(), "RED-B".to_string()],
        "exactly the two picked cards are played"
    );
    let trash = trash_ids(&r, 0);
    assert!(
        trash.contains(&"PURPLE-B".to_string()) && trash.contains(&"RED-A".to_string()),
        "the unchosen sources leave with the carrier. trash = {trash:?}"
    );
}

/// The gap's negative probe, on the INHERITED copy: Dinobeemon itself sits in
/// the stack under a higher-level top card. It is a Lv.5 that answers neither
/// parenthetical, so it must never be playable by its own <Partition> — the
/// slot-blind select used to offer (and play) it.
#[test]
fn bt16_077_inherited_partition_never_plays_a_source_no_slot_admits() {
    use digimon_engine::action::space::{encode_source_select, REPLACEMENT_ACCEPT};
    use digimon_engine::enums::CardColor;
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .add_card(partition_source("PURPLE-LV4", CardColor::Purple, 4))
        .add_card(partition_source("RED-LV4", CardColor::Red, 4))
        .add_card(partition_source("HOST-LV6", CardColor::Purple, 6))
        .start();

    // sources bottom→top: 0 = PURPLE-LV4, 1 = RED-LV4, 2 = BT16-077 (Lv.5).
    let carrier = r.place_stack(0, &["PURPLE-LV4", "RED-LV4", "BT16-077", "HOST-LV6"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the inherited <Partition>");

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("the inherited <Partition> source pick must surface");
    assert!(
        !pending
            .valid_action_ids
            .contains(&encode_source_select(0, 2).unwrap()),
        "the Lv.5 Dinobeemon source matches no parenthetical and must never \
         be offered: it is not one of the 'specified cards' (§16-28-5)"
    );

    r.game
        .resolve_selection(0, pending.valid_action_ids[0])
        .expect("pick a specified card");

    let mut on_field = field_ids(&r, 0);
    on_field.sort();
    assert_eq!(
        on_field,
        vec!["PURPLE-LV4".to_string(), "RED-LV4".to_string()],
        "only the two specified cards reach the battle area"
    );
    assert!(
        trash_ids(&r, 0).contains(&"BT16-077".to_string()),
        "Dinobeemon leaves with the carrier"
    );
}
