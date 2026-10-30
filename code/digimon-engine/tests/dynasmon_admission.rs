//! Focused runner for the same card regressions registered in cards_behavioral.
//! Keeps local TDD iterations independent of the full behavioral binary's size.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "support/owned_card_choice.rs"]
mod physical_menu;
#[path = "cards_behavioral/bt18/bt18_030.rs"]
mod bt18_030;
#[path = "cards_behavioral/bt18/bt18_036.rs"]
mod bt18_036;
#[path = "cards_behavioral/bt18/bt18_098.rs"]
mod bt18_098;
#[path = "cards_behavioral/bt19/bt19_036.rs"]
mod bt19_036;
#[path = "cards_behavioral/bt19/bt19_042.rs"]
mod bt19_042;
#[path = "cards_behavioral/ex6/ex6_063.rs"]
mod ex6_063;
#[path = "cards_behavioral/ex13/ex13_025.rs"]
mod ex13_025;
#[path = "cards_behavioral/dynasmon_exact_fixture.rs"]
mod exact_fixture;
#[path = "dsl/event_card_kind.rs"]
mod event_card_kind;
#[path = "dsl/trash_bottom_security.rs"]
mod trash_bottom_security;

#[path = "cards_behavioral/bt3/bt3_093.rs"]
mod bt3_093;

#[path = "cards_behavioral/bt20/bt20_048.rs"]
mod bt20_048;

#[path = "cards_behavioral/ex12/ex12_009.rs"]
mod ex12_009;

#[path = "cards_behavioral/ex13/ex13_048.rs"]
mod ex13_048;

#[path = "cards_behavioral/p/p_118.rs"]
mod p_118;

#[path = "dsl/original_dp_presence.rs"]
mod original_dp_presence;

#[path = "cards_behavioral/bt18/bt18_039.rs"]
mod bt18_039;

#[path = "cards_behavioral/bt19/bt19_029.rs"]
mod bt19_029;

#[path = "cards_behavioral/bt23/bt23_035.rs"]
mod bt23_035;

#[path = "cards_behavioral/bt6/bt6_089.rs"]
mod bt6_089;

#[path = "dsl/deck_trash_event.rs"]
mod deck_trash_event;

#[path = "cards_behavioral/bt9/bt9_084.rs"]
mod bt9_084;

#[path = "cards_behavioral/ex7/ex7_026.rs"]
mod ex7_026;

#[path = "cards_behavioral/lm/lm_045.rs"]
mod lm_045;

#[path = "cards_behavioral/ex10/ex10_041.rs"]
mod ex10_041;

#[path = "dsl/permanent_security_receipt.rs"]
mod permanent_security_receipt;

#[path = "cards_behavioral/lm/lm_020.rs"]
mod lm_020;

#[path = "dsl/public_security_reveal.rs"]
mod public_security_reveal;

#[path = "effect_identity_immunity.rs"]
mod effect_identity_immunity;

#[path = "own_carrier_immunity_regressions.rs"]
mod own_carrier_immunity_regressions;

#[path = "own_granted_immunity.rs"]
mod own_granted_immunity;

#[path = "native_selection_origin.rs"]
mod native_selection_origin;

#[path = "floating_effect_origin.rs"]
mod floating_effect_origin;

#[path = "dsl/quantumon_primitives.rs"]
mod quantumon_primitives;
