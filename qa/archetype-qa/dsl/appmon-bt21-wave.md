# Archetype DSL Implementation: Appmon (BT21/BT24/BT25/AD1/promo wave)
Date: 2026-06-12
Total cards in request: 32
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 26
- AUDITED-OK: 1 (BT25-098, drift fixed)
- PARTIAL: 5 (all gated on the effect-initiated App Fuse engine gap; faithfully omitted-not-stubbed)
- BLOCKED: 0
- SKIPPED (prior IMPLEMENTED verdict, untouched): 9 BT25 link cards (BT25-004/007/036/045/052/056/061/070/072)

Full-suite verification (post-merge): `cargo test --test cards_behavioral` = 4988 passed / 0 failed;
`--lib` = 226 / 0; `--test dsl` = 761 / 0.

## Per-Card Verdicts (this run's NEW work — 23 cards)
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT21-009 | Gatchmon | IMPLEMENT | IMPLEMENTED | 18 | when-linked play Haru free + link box |
| BT21-018 | DoGatchmon | IMPLEMENT | IMPLEMENTED | 19 | Rush/Raid, when-linked attack, App Fusion |
| ST22-12 | DoGatchmon | IMPLEMENT | IMPLEMENTED | 16 | WhenAttacking link, [When Linking] deck-bottom |
| BT21-023 | Globemon | IMPLEMENT | IMPLEMENTED | 22 | SecA+1, free-link, when-linked delete |
| AD1-005 | Gaiamon ACE | IMPLEMENT | IMPLEMENTED | 25 | Blast/ACE, Link+1, link-up-to-2 + delete |
| BT21-101 | Gaiamon | IMPLEMENT | IMPLEMENTED | 24 | on_any_link unsuspend→trash security |
| BT21-084 | Haru Shinkai | IMPLEMENT | PARTIAL(engine) | 12 | on_any_link draw; app-fuse omitted |
| P-217 | Haru Shinkai | IMPLEMENT | IMPLEMENTED | 19 | reveal-2-bucket + on_any_link memory |
| BT21-005 | Swipemon | IMPLEMENT | IMPLEMENTED | 8 | inherited egg draw-on-link |
| BT21-047 | Navimon | IMPLEMENT | IMPLEMENTED | 12 | reveal-2-bucket + Piercing link ESS |
| BT24-067 | Hackmon | IMPLEMENT | IMPLEMENTED | 18 | Gatchmon twin; Retaliation + +2000 link DP |
| BT24-087 | Rei Katsura | IMPLEMENT | PARTIAL(engine) | 11 | on_any_link draw+trash; app-fuse omitted |
| BT21-059 | Timemon | IMPLEMENT | IMPLEMENTED | 18 | De-Digivolve host + linked; App Fusion |
| BT21-070 | Gossipmon | IMPLEMENT | IMPLEMENTED | 18 | trash-return; +3000 link DP (review-added) |
| BT21-071 | Scopemon | IMPLEMENT | IMPLEMENTED | 16 | place-as-bottom-source; +3000 (review-added) |
| BT21-043 | Sociamon | IMPLEMENT | IMPLEMENTED | 20 | -2000 DP host + linked |
| BT25-060 | Rebootmon | IMPLEMENT | IMPLEMENTED | 18 | RE-ADJUDICATED BLOCKED→IMPLEMENTED |
| BT21-073 | Charismon | IMPLEMENT | IMPLEMENTED | 19 | forced-attack taunt grant + leave-replacement |
| BT21-097 | App Link | IMPLEMENT | IMPLEMENTED | 11 | Option + Delay link (DSL fix) |
| BT23-079 | Eri Karan | IMPLEMENT | PARTIAL(engine) | 11 | on_any_link +3000 to host; app-fuse omitted |
| P-241 | Yujin Ozora | IMPLEMENT | PARTIAL(engine) | 13 | on_any_link Vortex+3000; app-fuse omitted |
| BT25-098 | Cyber Engage | AUDIT | AUDITED-OK | 15 | drift fixed (optional add→mandatory) |
| BT25-089 | Kazuki & Itsuki | AUDIT | PARTIAL(hybrid) | 7 | color drift fixed; app-fuse + multi-src open |

## Substrate widened this run (rule 28 — widen, don't route around)
1. **DSL: `when: on_any_link`** (G-DSL-WHEN-ANY-OWN-DIGIMON-LINKED, RESOLVED) — board-wide link
   observer; lowers to `OnLink` with no forced self/host filter, gated by `active_when`
   (`event_target_owner` / `event_card_trait_has` / `your_turn`). Unblocked the "[Your Turn] when
   your Digimon get linked" Tamer/Digimon family: BT21-084, BT21-101, P-217, P-241, BT23-079,
   BT24-087.
2. **App Fusion alt-play re-adjudication** — the alt-play digivolve method (`kind: app_fusion`) is
   fully implemented + behaviorally green; the stale 2026-06-07 "resolves to nothing" block is
   cleared. BT25-060 re-adjudicated BLOCKED→IMPLEMENTED.
3. **Engine/DSL fixes**: `static_dp_aura_bonus` no longer self-applies `.linked` auras;
   `LinkCardsCount` untagged struct variants (link_cards in `kind: delay` bodies);
   `binding_ref.rs EventTarget` host-liveness fallback (+DP to the link host in on_any_link);
   `lower_triggered.rs LinkCardsToSelf` declinable-first-step.

## Recurring faithfulness lesson
The Appmon **link DP bonus** (printed `+N DP` in the link box) is **data-driven** in DCGO
(`Permanent.cs: LinkedDP += addedLinkCard.LinkDP`), so it NEVER appears in a card's `.cs` effect
file. Implementers repeatedly omitted it trusting the `.cs`; the **card image is authoritative**.
Caught and fixed in review on BT24-067, BT21-070, BT21-071. Model as
`scope: linked, kind: aura, target: {}, dp_modifier: N`.

## Residual gap (the only thing keeping 5 cards PARTIAL)
**Effect-initiated App Fuse** — "1 of your Digimon **may app fuse into** a Digimon card in the
hand/trash" — no `EffectContext::effect_initiated_app_fuse` primitive (distinct from the App
Fusion alt-play path, which works). Blocks the riders of BT21-084, BT24-087, BT23-079, P-241,
BT25-089. Tracked in `docs/RUST_ENGINE_GAPS.md` App Fuse entry. BT25-089 additionally has
G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES (multi-Digimon digivolution-source link from a Tamer).
