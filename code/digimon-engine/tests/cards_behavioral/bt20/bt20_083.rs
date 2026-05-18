//! BT20-083 Omekamon - Digimon, Lv.4, White.
//!
//! Supported slice:
//! - <Blocker>.
//! - [On Play] If you have 1 or fewer security cards, may digivolve this
//!   Digimon into [Omnimon (X Antibody)] in hand ignoring requirements/free.
//!
//! Gap-routed:
//! - [On Deletion] place this card under [King Drasil_7D6] in breeding — the
//!   RK-G001 filter shipped in Phase 2 Track J PR 1, but the printed "you may"
//!   optionality needs an `optional: bool` field on
//!   `select_own_breeding_permanent` (today hardcoded `is_optional: false`),
//!   so authoring would silently make the trigger mandatory.
//! - Inherited [Breeding][Opponent's Turn] security-removed trigger needs the
//!   printed body (suspend the breeding carrier as a cost and play [Omekamon]
//!   from that breeding stack's materials without paying the cost). The
//!   fan-out timing already fires per `bt20_083_inherited_breeding_security_removed_fans_out_once_with_payload`.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledDpConstraint, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::BREEDING_TARGET;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
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
            .timing(EffectTiming::OnOpponentSecurityRemoved)
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
#[ignore = "pending: G-OPTIONAL-BREEDING-SELECTION — RK-G001 filter shipped (Phase 2 Track J PR 1), but `select_own_breeding_permanent` is hardcoded `is_optional: false`, so authoring the printed 'you may' clause would silently turn the trigger mandatory"]
fn bt20_083_on_deletion_places_self_under_king_drasil_only() {
    panic!("requires optional select_own_breeding_permanent before the printed 'you may' clause can be authored faithfully");
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
    runner.place_in_breeding(0, "BT20-083-BREED-TOP");
    add_breeding_source(&mut runner, 0, "BT20-083");

    let before = runner.memory();
    let _ = runner.attack_player(attacker, 1, true);

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        1,
        "BT20-083 as an inherited breeding source should observe the opponent-security removal exactly once"
    );
    assert_eq!(
        runner.memory(),
        before + 1,
        "BT20-083 witness gained memory only if the inherited source fired"
    );
    assert_eq!(seen[0].0, Some(1), "defender security was removed");
    assert_eq!(seen[0].1, Some(0), "attacker caused the security removal");
    assert_eq!(seen[0].2, Some(EventCause::SecurityRemoval));
    assert_eq!(
        seen[0].3,
        Some(PermanentHandle {
            player: 0,
            index: BREEDING_TARGET as u8,
        }),
        "BT20-083 inherited source should retain the breeding carrier sentinel"
    );
}

#[test]
#[ignore = "pending: G-BREEDING-TRIGGER-DISPATCH plus source-play from materials"]
fn bt20_083_inherited_breeding_security_removed_suspends_carrier_and_plays_omekamon_source() {
    panic!("requires breeding inherited security-removed observer and source-play support");
}
