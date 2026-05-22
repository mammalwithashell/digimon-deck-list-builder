//! EX6-011 RagnaLoardmon — Digimon, Lv.7, Red/Black, ACE.
//!
//! # Card text (cards.json)
//! Digimon, red/black, Lv.7, play cost 9, DP 15000, attribute Virus.
//! Traits: Unique / Legend-Arms. ACE (Ace Overflow <-5>).
//! Digivolve: from Black Lv.6 for 5 memory.
//! DNA Digivolve: Red Lv.6 + Black Lv.6, cost 0.
//!
//! "[Hand] [Counter] <Blast DNA Digivolve ([Durandamon] + [BryweLudramon])>
//!  (1 of your specified Digimon and 1 specified card in hand may DNA
//!  digivolve into this card.)"
//! "<Raid>", "<Reboot>"
//! "[On Play] [When Digivolving] Trash the top card of your opponent's
//!  security stack and this Digimon isn't affected by your opponent's effects
//!  until the end of their turn. Then, if DNA digivolving, <De-Digivolve 1>
//!  all of your opponent's Digimon and delete 1 of their Digimon."
//! Inherited: "Ace Overflow <-5>".
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX6/Red/EX6_011.cs
//!
//! # Patterns this test covers
//! - Alt-path metadata: standard Black Lv.6 digivolve, Red+Black Lv.6 DNA,
//!   and [Hand][Counter] Blast DNA digivolve with named materials.
//! - Counter-window Blast DNA activation (G-COUNTER-BLAST-DNA-ACTIVATION,
//!   resolved 2026-05-15 Track D / PUPPETS-G032 closed): defender attack into
//!   a field material, CounterTiming exposes the Blast DNA action, field then
//!   hand material pending selections, resulting stack ordering.
//! - E1 [On Play]/[When Digivolving] dual-timing triggered clause.
//! - D-style turn-scoped CannotBeAffected immunity grant (per source kind).
//! - DNA-origin conditional tail: for-each De-Digivolve all + mandatory
//!   single delete pending selection.
//! - G runtime keyword grants (Raid, Reboot) and BlastDigivolve marker.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, EffectSourceKind, EffectTiming, GamePhase, Keyword, ModifierType,
};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/ex6/EX6-011.yaml");

fn compiled_ex6_011() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("EX6-011 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("EX6-011 YAML compiles");
    registry
        .lookup("EX6-011")
        .expect("EX6-011 in registry")
        .clone()
}

fn colored_lv6(id: &str, color: CardColor) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(6);
    card.dp = Some(11000);
    card.colors = vec![color];
    card
}

fn leveled_digimon(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(level);
    card.dp = Some(5000 + i32::from(level) * 1000);
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-011 YAML loads")
        .add_card(make_test_card("SEC-BOTTOM", "Security Bottom"))
        .add_card(make_test_card("SEC-TOP", "Security Top"))
        .security(1, &["SEC-BOTTOM", "SEC-TOP"])
        .memory(10)
        .build()
}

fn fire_on_play(runner: &mut DebugRunner, source: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

#[test]
fn ex6_011_metadata_matches_printed_card() {
    let card = compiled_ex6_011();

    assert_eq!(card.card, "EX6-011");
    assert_eq!(card.name, "RagnaLoardmon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(7));
    assert_eq!(card.cost, Some(9));
    assert_eq!(card.dp, Some(15000));
    assert_eq!(card.color, vec![CompiledColor::Red, CompiledColor::Black]);
    assert!(card.traits.contains(&"Unique".to_string()));
    assert!(card.traits.contains(&"Legend-Arms".to_string()));
    assert_eq!(card.attribute.as_deref(), Some("Virus"));
    assert_eq!(card.ace_overflow, Some(-5));
}

#[test]
fn ex6_011_has_standard_red_lv6_and_dna_red_black_lv6_routes() {
    let card = compiled_ex6_011();

    let standard = card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && matches!(path.cost, Some(CompiledCost::Literal(5)))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(6) && from.color_is == Some(CompiledColor::Red)
            })
    });
    assert!(standard, "missing red Lv.6 cost-5 digivolve route");

    let dna = card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::DnaDigivolve
            && matches!(path.cost, Some(CompiledCost::Literal(0)))
            && path.materials.len() == 2
            && path.materials[0].filter.level_eq == Some(6)
            && path.materials[0].filter.color_is == Some(CompiledColor::Red)
            && path.materials[1].filter.level_eq == Some(6)
            && path.materials[1].filter.color_is == Some(CompiledColor::Black)
    });
    assert!(dna, "missing red Lv.6 + black Lv.6 cost-0 DNA route");

    let blast_dna = card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::BlastDnaDigivolve
            && matches!(path.cost, Some(CompiledCost::Literal(0)))
            && path.materials.len() == 2
            && path.materials[0].filter.name_is.as_deref() == Some("Durandamon")
            && path.materials[1].filter.name_is.as_deref() == Some("BryweLudramon")
    });
    assert!(
        blast_dna,
        "missing [Hand][Counter] Blast DNA route for exact Durandamon + BryweLudramon"
    );
}

#[test]
fn ex6_011_has_blast_dna_raid_and_reboot_keyword_grants() {
    let card = compiled_ex6_011();
    let keywords: Vec<_> = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        keywords.contains(&"BlastDigivolve"),
        "missing BlastDigivolve marker"
    );
    assert!(keywords.contains(&"Raid"), "missing Raid keyword grant");
    assert!(keywords.contains(&"Reboot"), "missing Reboot keyword grant");
}

#[test]
fn ex6_011_on_play_trashes_security_and_grants_opponent_effect_immunity() {
    let mut runner = runner();
    let ragna = runner.place_on_field(0, "EX6-011", Some(0));

    fire_on_play(&mut runner, ragna);

    assert_eq!(
        runner.security_count(1),
        1,
        "On Play must trash exactly one opponent security card"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "the trashed security card must move to opponent trash"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(ragna, ModifierType::CannotBeAffected),
        "RagnaLoardmon must carry CannotBeAffected after On Play"
    );
    for source_kind in [
        EffectSourceKind::Digimon,
        EffectSourceKind::Tamer,
        EffectSourceKind::Option,
        EffectSourceKind::Rule,
    ] {
        assert!(
            runner
                .game
                .permanent_is_unaffected_by_effect(ragna, 1, source_kind),
            "opponent {:?} effects must not affect RagnaLoardmon",
            source_kind
        );
        assert!(
            !runner
                .game
                .permanent_is_unaffected_by_effect(ragna, 0, source_kind),
            "own {:?} effects must still affect RagnaLoardmon",
            source_kind
        );
    }
}

#[test]
fn ex6_011_dna_route_is_action_mask_legal_for_red_and_black_lv6() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-011 YAML loads")
        .add_card(colored_lv6("RED-LV6", CardColor::Red))
        .add_card(colored_lv6("BLACK-LV6", CardColor::Black))
        .hand(0, &["EX6-011"])
        .memory(10)
        .start();

    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, "RED-LV6", Some(0));
    runner.place_on_field(0, "BLACK-LV6", Some(0));

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[DNA_DIGIVOLVE_START as usize], 1.0,
        "red Lv.6 + black Lv.6 materials must make the DNA action legal"
    );
}

#[test]
fn ex6_011_dna_origin_tail_dedigivolves_all_then_surfaces_delete_choice() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-011 YAML loads")
        .add_card(colored_lv6("RED-LV6", CardColor::Red))
        .add_card(colored_lv6("BLACK-LV6", CardColor::Black))
        .add_card(leveled_digimon("OPP-LV3", 3))
        .add_card(leveled_digimon("OPP-LV5", 5))
        .add_card(make_test_card("SEC-TOP", "Security Top"))
        .hand(0, &["EX6-011"])
        .deck(0, &["SEC-TOP"])
        .security(1, &["SEC-TOP"])
        .memory(10)
        .start();

    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, "RED-LV6", Some(0));
    runner.place_on_field(0, "BLACK-LV6", Some(0));
    let opponent = runner.place_stack(1, &["OPP-LV3", "OPP-LV5"]);

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner
        .game
        .resolve_selection(0, 0)
        .expect("first DNA material");
    runner
        .game
        .resolve_selection(0, 1)
        .expect("second DNA material");

    assert_eq!(
        runner.security_count(1),
        0,
        "When Digivolving from DNA must still trash top opponent security"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opponent.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "OPP-LV3",
        "DNA-origin tail must De-Digivolve 1 all opponent Digimon before deletion"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "DNA-origin tail must surface the mandatory delete target choice"
    );
}

#[test]
fn ex6_011_runtime_keywords_are_available_on_field() {
    let mut runner = runner();
    let ragna = runner.place_on_field(0, "EX6-011", Some(0));

    assert!(runner.game.has_keyword(ragna, Keyword::Raid));
    assert!(runner.game.has_keyword(ragna, Keyword::Reboot));
}

#[test]
fn ex6_011_counter_blast_dna_uses_durandamon_and_bryweludramon() {
    let mut durandamon = colored_lv6("EX6-DURANDAMON", CardColor::Red);
    durandamon.card_name = "Durandamon".to_string();
    durandamon.dp = Some(12000);
    let mut bryweludramon = colored_lv6("EX6-BRYWELUDRAMON", CardColor::Black);
    bryweludramon.card_name = "BryweLudramon".to_string();
    bryweludramon.dp = Some(12000);
    let mut attacker = colored_lv6("EX6-ATTACKER", CardColor::Red);
    attacker.card_name = "Attacker".to_string();
    attacker.dp = Some(17000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-011 YAML loads")
        .add_card(durandamon)
        .add_card(bryweludramon)
        .add_card(attacker)
        .add_card(make_test_card("P0-SEC", "P0 Security"))
        .hand(1, &["EX6-011", "EX6-BRYWELUDRAMON"])
        .security(0, &["P0-SEC"])
        .start();

    let attacking = runner.place_on_field(0, "EX6-ATTACKER", Some(0));
    let durandamon = runner.place_on_field(1, "EX6-DURANDAMON", Some(0));

    let result = runner.attack_digimon(attacking, durandamon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);
    assert!(
        runner
            .pending_selection()
            .expect("Counter prompt")
            .valid_action_ids
            .contains(&DNA_DIGIVOLVE_START),
        "EX6-011 should be offered as Counter Blast DNA"
    );

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose EX6-011 for Counter Blast DNA");
    assert_eq!(runner.current_phase(), GamePhase::SelectMaterial);
    assert_eq!(
        runner
            .pending_selection()
            .expect("field material prompt")
            .valid_action_ids,
        vec![0]
    );

    runner
        .execute_action(1, 0)
        .expect("choose Durandamon field material");
    assert_eq!(
        runner
            .pending_selection()
            .expect("hand material prompt")
            .valid_action_ids,
        vec![PLAY_HAND_START + 1]
    );
    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose BryweLudramon hand material");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "Blast DNA must count as DNA-origin and surface EX6-011's delete target choice"
    );
    assert_eq!(runner.game.player(1).hand.len(), 0);
    assert_eq!(runner.game.player(1).battle_area.len(), 1);
    let stack_ids: Vec<_> = runner.game.player(1).battle_area[0]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        stack_ids,
        vec!["EX6-DURANDAMON", "EX6-BRYWELUDRAMON", "EX6-011"]
    );
}
