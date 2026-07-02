//! BT20-083 Omekamon - Digimon, Lv.4, White.
//!
//! Supported slice:
//! - <Blocker>.
//! - [On Play] If you have 1 or fewer security cards, may digivolve this
//!   Digimon into [Omnimon (X Antibody)] in hand ignoring requirements/free.
//! - [On Deletion] may place itself under [King Drasil_7D6] in breeding.
//! - Inherited [Breeding][Opponent's Turn] when your security is removed,
//!   suspend the breeding carrier to play an [Omekamon] source for free.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledDpConstraint, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_breeding_select, encode_breeding_source_select, BREEDING_TARGET, PASS,
};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::trigger_context::EventCause;
use std::sync::{Arc, Mutex};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-083")
        .expect("BT20-083 YAML loads")
        .memory(10)
        .start()
}

fn add_breeding_source(r: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("add_breeding_source: unknown card_id {card_id}"));
    let next_idx = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, next_idx);
    let handle = card.handle();
    let permanent = r.game.players[player as usize]
        .breeding_area
        .as_mut()
        .expect("breeding permanent exists");
    let top = permanent.card_sources.pop().expect("breeding top exists");
    permanent.card_sources.push(card);
    permanent.card_sources.push(top);
    handle
}

struct Bt20_083BreedingFanoutWitness {
    seen: Arc<
        Mutex<
            Vec<(
                Option<u8>,
                Option<u8>,
                Option<EventCause>,
                Option<PermanentHandle>,
            )>,
        >,
    >,
}

impl CardEffect for Bt20_083BreedingFanoutWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::inherited(card)
            .name("BT20-083 breeding security removed witness")
            .timing(EffectTiming::OnOwnSecurityRemoved)
            .process(move |ctx| {
                seen.lock().unwrap().push((
                    ctx.event_affected_player(),
                    ctx.event_source_player(),
                    ctx.event_cause(),
                    ctx.source_permanent,
                ));
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn bt20_083_has_printed_metadata() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-083")
        .expect("BT20-083 compiled card present");

    assert_eq!(card.name, "Omekamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Puppet"));
    assert!(card.traits.iter().any(|name| name == "X Antibody"));
    assert!(card.traits.iter().any(|name| name == "LIBERATOR"));
    assert_eq!(card.attribute.as_deref(), Some("Data"));
}

#[test]
fn bt20_083_has_blocker_grant_and_low_security_on_play_digivolve() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-083")
        .expect("BT20-083 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            ..
        }) if keyword == "Blocker"
    )));

    let on_play = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("low-security On Play digivolve clause exists");

    assert!(on_play.optional, "printed digivolve says 'may'");
    assert_eq!(
        on_play
            .condition
            .as_ref()
            .and_then(|predicate| predicate.security_count_lte.clone()),
        Some(CompiledDpConstraint::Literal(1)),
        "On Play clause is gated to 1 or fewer own security"
    );
    assert!(
        on_play.process.iter().any(|step| matches!(
            step,
            CompiledStep::SelectHand { filter, .. }
                if filter.name_is.as_deref() == Some("Omnimon (X Antibody)")
        )),
        "On Play must select [Omnimon (X Antibody)] from hand"
    );
    assert!(
        on_play.process.iter().any(|step| matches!(
            step,
            CompiledStep::EffectInitiatedDigivolve {
                cost: CompiledCostDelta::Literal(0),
                ignore_requirements: true,
                ..
            }
        )),
        "selected hand card must digivolve this Omekamon for free, ignoring requirements"
    );
}

#[test]
fn bt20_083_has_on_deletion_and_inherited_security_removed_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-083")
        .expect("BT20-083 compiled card present");

    let on_deletion = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnDeletion) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("On Deletion clause exists");
    assert!(
        on_deletion.process.iter().any(|step| matches!(
            step,
            CompiledStep::SelectOwnBreedingPermanent {
                optional: true,
                filter,
                ..
            } if filter.name_is.as_deref() == Some("King Drasil_7D6")
        )),
        "On Deletion must expose an optional King Drasil breeding selection"
    );

    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if matches!(triggered.scope, CompiledScope::Inherited)
                    && triggered
                        .when
                        .contains(&CompiledTiming::OnOwnSecurityRemoved) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("inherited own-security-removed clause exists");
    assert!(inherited.optional, "printed inherited effect is optional");
    assert!(
        inherited
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::Suspend { .. })),
        "inherited flow must suspend the breeding carrier as the cost"
    );
    assert!(
        inherited.process.iter().any(|step| matches!(
            step,
            CompiledStep::SelectMaterials { filter, .. }
                if filter.name_is.as_deref() == Some("Omekamon")
        )),
        "inherited flow must select an Omekamon source"
    );
    assert!(
        inherited.process.iter().any(|step| matches!(
            step,
            CompiledStep::PlayFromMaterials {
                cost_delta: Some(CompiledCostDelta::Free),
                ..
            }
        )),
        "selected Omekamon source must be played for free"
    );
}

#[test]
fn bt20_083_on_deletion_can_decline_place_self_under_king_drasil() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-083")
        .expect("BT20-083 YAML loads")
        .add_card(make_test_card("KING", "King Drasil_7D6"))
        .hand(0, &["BT20-083"])
        .memory(10)
        .start();
    runner.place_in_breeding(0, "KING");
    runner.play(0, 0).expect("Omekamon plays");

    let omekamon_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_name(&runner.game.card_data) == "Omekamon")
        .expect("Omekamon on field");
    let omekamon_handle = runner.game.player(0).battle_area[omekamon_idx]
        .top_card()
        .handle();

    runner.game.delete_permanent_with_effects(PermanentHandle {
        player: 0,
        index: omekamon_idx as u8,
    });

    let pending = runner
        .pending_selection_view()
        .expect("optional King Drasil breeding selection opens");
    assert_eq!(pending.kind, SelectionKind::BreedingPermanent);
    assert!(pending.is_optional);
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        1.0,
        "PASS must be legal through the action mask for optional On Deletion placement"
    );

    runner
        .execute_action(0, PASS)
        .expect("decline optional On Deletion placement");

    let breeding = runner.game.player(0).breeding_area.as_ref().unwrap();
    assert_eq!(breeding.stack_size(), 1);
    assert!(
        !breeding
            .card_sources
            .iter()
            .any(|source| source.handle() == omekamon_handle),
        "declining leaves Omekamon out of the breeding stack"
    );
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|card| card.handle() == omekamon_handle),
        "declined On Deletion lets normal deletion move Omekamon to trash"
    );
}

#[test]
fn bt20_083_on_deletion_places_self_under_king_drasil_only() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-083")
        .expect("BT20-083 YAML loads")
        .add_card(make_test_card("KING", "King Drasil_7D6"))
        .hand(0, &["BT20-083"])
        .memory(10)
        .start();
    runner.place_in_breeding(0, "KING");
    runner.play(0, 0).expect("Omekamon plays");

    let omekamon_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_name(&runner.game.card_data) == "Omekamon")
        .expect("Omekamon on field");
    let omekamon_handle = runner.game.player(0).battle_area[omekamon_idx]
        .top_card()
        .handle();

    runner.game.delete_permanent_with_effects(PermanentHandle {
        player: 0,
        index: omekamon_idx as u8,
    });
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::BreedingPermanent)
    );
    runner
        .execute_action(0, encode_breeding_select(0).unwrap())
        .expect("choose King Drasil");

    let breeding = runner.game.player(0).breeding_area.as_ref().unwrap();
    assert_eq!(breeding.stack_size(), 2);
    assert_eq!(
        breeding.card_sources[0].handle(),
        omekamon_handle,
        "Omekamon is tucked as the bottom source under King Drasil"
    );
    assert!(
        !runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|card| card.handle() == omekamon_handle),
        "Omekamon moved to breeding instead of normal deletion trash"
    );
}

#[test]
fn bt20_083_inherited_breeding_security_removed_fans_out_once_with_payload() {
    let mut attacker = make_test_card("BT20-083-ATTACKER", "BT20-083 attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(5);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-083")
        .expect("BT20-083 YAML loads")
        .add_card(attacker)
        .add_card(make_test_card("BT20-083-BREED-TOP", "King Drasil carrier"))
        .add_card(make_test_card("BT20-083-SEC", "Security card"))
        .add_card(make_test_card("BT20-083-FILL", "Filler"))
        .security(1, &["BT20-083-SEC", "BT20-083-SEC"])
        .deck(0, &["BT20-083-FILL"; 10])
        .deck(1, &["BT20-083-FILL"; 10])
        .memory(5)
        .start();

    let seen = Arc::new(Mutex::new(Vec::new()));
    runner.register_effect(
        "BT20-083",
        Arc::new(Bt20_083BreedingFanoutWitness {
            seen: Arc::clone(&seen),
        }),
    );

    let attacker = runner.place_on_field(0, "BT20-083-ATTACKER", Some(0));
    runner.place_in_breeding(1, "BT20-083-BREED-TOP");
    add_breeding_source(&mut runner, 1, "BT20-083");

    let _ = runner.attack_player(attacker, 1, true);

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        1,
        "BT20-083 as an inherited breeding source should observe the opponent-security removal exactly once"
    );
    assert_eq!(seen[0].0, Some(1), "defender security was removed");
    assert_eq!(seen[0].1, Some(0), "attacker caused the security removal");
    assert_eq!(seen[0].2, Some(EventCause::SecurityRemoval));
    assert_eq!(
        seen[0].3,
        Some(PermanentHandle {
            player: 1,
            index: BREEDING_TARGET as u8,
        }),
        "BT20-083 inherited source should retain the breeding carrier sentinel"
    );
}

#[test]
fn bt20_083_inherited_breeding_security_removed_suspends_carrier_and_plays_omekamon_source() {
    let mut attacker = make_test_card("BT20-083-ATTACKER", "BT20-083 attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(5);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-083")
        .expect("BT20-083 YAML loads")
        .add_card(attacker)
        .add_card(make_test_card("BT20-083-BREED-TOP", "King Drasil_7D6"))
        .add_card(make_test_card("BT20-083-SEC", "Security card"))
        .add_card(make_test_card("BT20-083-FILL", "Filler"))
        .security(1, &["BT20-083-SEC", "BT20-083-SEC"])
        .deck(0, &["BT20-083-FILL"; 10])
        .deck(1, &["BT20-083-FILL"; 10])
        .memory(5)
        .start();

    let attacker = runner.place_on_field(0, "BT20-083-ATTACKER", Some(0));
    runner.place_in_breeding(1, "BT20-083-BREED-TOP");
    let source_handle = add_breeding_source(&mut runner, 1, "BT20-083");

    let _ = runner.attack_player(attacker, 1, true);
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::Replacement | SelectionKind::TriggerOrder)
        ),
        "optional inherited trigger should expose an accept/decline prompt"
    );
    assert!(runner.pending_is_optional());
    runner
        .accept_optional_trigger()
        .expect("accept optional inherited Omekamon trigger");

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect {
                max: 1,
                picked: 0,
                ..
            })
        ),
        "select_materials should expose a one-pick source selection"
    );
    runner
        .execute_action(1, encode_breeding_source_select(1, 0).unwrap())
        .expect("choose Omekamon breeding source");

    let breeding = runner.game.player(1).breeding_area.as_ref().unwrap();
    assert!(
        breeding.is_suspended,
        "the breeding carrier is suspended as the printed cost"
    );
    assert!(
        !breeding
            .card_sources
            .iter()
            .any(|source| source.handle() == source_handle),
        "played Omekamon source leaves the breeding stack"
    );
    assert!(
        runner.game.player(1).battle_area.iter().any(|perm| {
            perm.top_card().handle() == source_handle
                && perm.top_card().card_name(&runner.game.card_data) == "Omekamon"
        }),
        "selected Omekamon source is played for free"
    );
}
