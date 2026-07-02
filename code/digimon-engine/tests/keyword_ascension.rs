//! `Keyword::Ascension` auto-install behavioral tests.
//!
//! Printed: "＜Ascension＞ (When this Digimon is deleted, you may place this card
//! as the top security card.)" (BT25-034 Angemon, BT25-040 MagnaAngemon).
//!
//! DCGO behavioral source: `CardEffectCommons/KeyWordEffects/Ascension.cs`
//! (`AscensionProcess`) — `CanTriggerOnDeletion` trigger; on fire, if the owner
//! `CanAddSecurity`, prompt a Yes/No boolean selection ("Will you place this card
//! in security?"); on Yes, `AddSecurityCard(card, /*toTop=*/ true)`. The carrier
//! has already moved to trash when the OnDeletion handler drains (CLAUDE.md rule
//! 25), so the card is rescued from trash to the top of the security stack.
//!
//! "You may" optionality is exposed as a real two-branch EffectChoice (Yes / No)
//! — no auto-resolution (no-approximations, CLAUDE.md §17).

use digimon_engine::card_data::{parse_printed_keywords, CardData};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

/// The printed `＜Ascension＞` token (BT25-034 Angemon) parses to the keyword.
#[test]
fn ascension_parses_from_printed_text() {
    let kws = parse_printed_keywords(
        "＜Ascension＞ (When this Digimon is deleted, you may place this card \
         as the top security card.)",
        "",
        "",
    );
    assert!(
        kws.contains(&Keyword::Ascension),
        "＜Ascension＞ must parse to Keyword::Ascension, got {kws:?}"
    );
}

fn ascension_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Yellow],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        // Printed-only Ascension: the auto-install MUST be the sole behavior.
        keywords: vec![Keyword::Ascension],
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// On deletion, the owner is offered a Yes/No; choosing Yes moves the carrier
/// from trash to the TOP of the owner's security stack.
#[test]
fn ascension_yes_places_self_on_top_of_security() {
    let mut r = DebugRunner::builder()
        .add_card(ascension_card("ASCENSION"))
        .start();

    let carrier = r.place_on_field(0, "ASCENSION", None);
    let sec_before = r.game.players[0].security.len();

    r.game.delete_permanent_with_effects(carrier);

    // The optional Yes/No must be parked as a real selection (not auto-resolved).
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Ascension must park a Yes/No effect choice on deletion");
    assert_eq!(pending.selecting_player, 0);
    assert_eq!(pending.valid_action_ids.len(), 2, "two branches: Yes / No");

    // Choose Yes (first branch).
    let yes = pending.valid_action_ids[0];
    r.game.resolve_selection(0, yes).expect("resolve Yes");

    assert_eq!(
        r.game.players[0].security.len(),
        sec_before + 1,
        "Ascension adds the carrier to security"
    );
    let top = r.game.players[0]
        .security
        .last()
        .expect("security has a top card");
    assert_eq!(
        top.card_id(&r.game.card_data),
        "ASCENSION",
        "the carrier is the TOP security card"
    );
    // It is no longer in the trash (moved, not copied).
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ASCENSION"),
        "carrier moved out of trash into security"
    );
}

#[test]
fn ascension_choice_prompt_clones_faithfully() {
    let mut r = DebugRunner::builder()
        .add_card(ascension_card("ASCENSION"))
        .start();

    let carrier = r.place_on_field(0, "ASCENSION", None);
    r.game.delete_permanent_with_effects(carrier);

    let yes = {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Ascension must park a Yes/No effect choice on deletion");
        assert!(
            r.game.pending_selection_resume.is_some(),
            "Ascension choice must be resume-driven before cloning"
        );
        pending.valid_action_ids[0]
    };

    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, yes)
        .expect("cloned Ascension choice resolves");
    assert!(
        r.game.pending_selection.is_some(),
        "resolving the clone must leave the original Ascension prompt intact"
    );
    r.game
        .resolve_selection(0, yes)
        .expect("original Ascension choice resolves");

    assert_eq!(
        clone.players[0].security.len(),
        r.game.players[0].security.len(),
        "clone and original should replay the same Ascension result"
    );
}

/// Choosing No leaves the carrier in the trash; security is unchanged.
#[test]
fn ascension_no_leaves_carrier_in_trash() {
    let mut r = DebugRunner::builder()
        .add_card(ascension_card("ASCENSION"))
        .start();

    let carrier = r.place_on_field(0, "ASCENSION", None);
    let sec_before = r.game.players[0].security.len();

    r.game.delete_permanent_with_effects(carrier);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Ascension parks a choice");
    // Choose No (second branch).
    let no = pending.valid_action_ids[1];
    r.game.resolve_selection(0, no).expect("resolve No");

    assert_eq!(
        r.game.players[0].security.len(),
        sec_before,
        "declining Ascension does not add to security"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ASCENSION"),
        "carrier stays in trash when declined"
    );
}
