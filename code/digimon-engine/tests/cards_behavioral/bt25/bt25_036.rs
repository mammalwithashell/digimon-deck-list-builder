//! BT25-036 Craftmon — Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Traits: Design (App Name).
//!
//! # Card text (cards.json — verbatim)
//!
//! [Security] At the end of the battle, play this card without paying the cost.
//! [On Play] [When Digivolving] Add your top security card to the hand. Then,
//! <Recovery +1> (Place the top card of your deck as your top security card.)
//! Inherited: By trashing 1 [Appmon] trait card from your hand, Draw 2
//! (the inherited fires [When Linking]).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_036.cs
//!
//! # BLOCKED — gap_kind: dsl
//!
//! Craftmon's "[When Linking] By trashing 1 [Appmon] trait card from your hand,
//! <Draw 2>" clause (DCGO `EffectTiming.WhenLinked`, `SetIsLinkedEffect(true)`)
//! cannot be expressed in the DSL: the `digimon_dsl::clause::Timing` enum has no
//! `WhenLinked` / `Linked` variant, so no `when:` string maps to
//! `CompiledTiming::Linked`. (`CompiledScope::Linked` exists but is unreachable
//! from a `when:` timing.) Per the no-approximations policy this clause cannot be
//! silently dropped, so the whole card is held BLOCKED rather than shipped with a
//! partial implementation.
//!
//! The App-fusion alt-path (`app_fusion`) and the link condition
//! (`link_requirement`) ARE expressible; the `[Security]` play-self and the
//! OnPlay add-top-security-to-hand + <Recovery +1> clauses are also expressible.
//! The single missing primitive is the [When Linking] trigger timing.
//!
//! Gap logged to qa/dsl-vocab-gaps.md (G-DSL-WHEN-LINKED-TIMING).

#![allow(dead_code, unused_imports)]

#[test]
#[ignore = "pending: G-DSL-WHEN-LINKED-TIMING from qa/dsl-vocab-gaps.md — no when: timing maps to CompiledTiming::Linked for the [When Linking] trash-Appmon→Draw2 clause"]
fn bt25_036_blocked_when_linking_timing_unavailable() {
    // Intentionally empty: BT25-036 is BLOCKED on the missing [When Linking]
    // DSL timing. No YAML is shipped for this card until the gap closes.
}
