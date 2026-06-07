//! BT25-036 Craftmon — Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Traits: Design (App Name).
//!
//! # Card text (DCGO BT25_036.cs — authoritative)
//!
//! Self link-condition: link onto an [Appmon]-trait Digimon for link cost 2.
//! Alt-digivolve: from a level-2 Standard-App for cost 0.
//! App Fusion: `AddAppfuseMethodByName(Kabemon, Gomimon, Ecomon, Puzzlemon)`.
//! [Security] At the end of the battle, play this card without paying the cost.
//! [On Play] [When Digivolving] Add your top security card to the hand. Then,
//!   <Recovery +1> (Place the top card of your deck as your top security card.)
//! [When Linking] By trashing 1 [Appmon] trait card from your hand, <Draw 2>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_036.cs
//!
//! # BLOCKED — gap_kind: engine (App Fuse play-flow)
//!
//! 2026-06-07 re-adjudication: the prior blocker (G-DSL-WHEN-LINKED-TIMING — no
//! `when:` token for [When Linking]) is RESOLVED — `when: when_linked` landed
//! with DigiLink Shape-B (qa/dsl-vocab-gaps.md G-DSL-DIGILINK, 2026-06-06). The
//! link-condition, [Security] play-self, OnPlay/WhenDigivolving add-top-security
//! + <Recovery +1>, and the [When Linking] trash-Appmon→Draw2 clause are ALL now
//! expressible.
//!
//! The card is still BLOCKED on a DIFFERENT primitive: **App Fusion**
//! (`CardEffectFactory.AddAppfuseMethodByName`). App Fuse is not implemented in
//! the engine — there is no lowering in `code/digimon-engine/src/dsl_cards/` and
//! no engine-core handler (the DSL `AltPathKind::AppFusion` variant parses but
//! resolves to nothing). App Fusion is a player-choosable alternate play path
//! and cannot be silently dropped under the no-approximations policy (CLAUDE.md
//! §17), so the whole card is held BLOCKED rather than shipped partial.
//!
//! Gap tracked: docs/RUST_ENGINE_GAPS.md `App Fuse` keyword/primitive entry.

#![allow(dead_code, unused_imports)]

#[test]
#[ignore = "BLOCKED (engine): App Fuse play-flow (AddAppfuseMethodByName) not implemented — see docs/RUST_ENGINE_GAPS.md. The prior G-DSL-WHEN-LINKED-TIMING block is resolved."]
fn bt25_036_blocked_app_fusion_unavailable() {
    // Intentionally empty: BT25-036 is BLOCKED on the missing App Fuse play-flow
    // primitive. No YAML is shipped for this card until the gap closes.
}
