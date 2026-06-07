# Archetype DSL Implementation: link-appmon-1 (BT25 Appmon Link slice)
Date: 2026-06-07
Total cards in slice: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl

## Context
This slice is a re-run of 6 previously-BLOCKED BT25 Appmon "Link" cards, re-adjudicated
after the **DigiLink Shape-B** engine substrate + DSL vocabulary landed (2026-06-06,
OpenSpec `implement-digilink-mechanic`; `qa/dsl-vocab-gaps.md` G-DSL-DIGILINK,
`docs/RUST_ENGINE_GAPS.md` Shape-B note). Shape-B wires the **standing-permanent
absorb** link model + `kind: link_condition`, `when: when_linked`, and
`scope: linked` ESS — which unblocks the cards whose link payoff fires when *this*
Digimon links onto a host. It did NOT wire facet #9 (link a *chosen* card from
hand/digivolution-cards) or facet #10 (host-filtered optional WhenWouldLink
cost-reduction), nor the App Fuse primitive.

## Summary
- IMPLEMENTED: 2 (BT25-007, BT25-061)
- PARTIAL: 0
- BLOCKED (engine): 4 (BT25-004, BT25-045, BT25-052, BT25-036)
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT25-007 | Gatchmon | IMPLEMENT | IMPLEMENTED | 7/7 | link_condition(Appmon,1) + alt-digivolve + OnPlay reveal-3 two-bucket add + when_linked delete opp DP<=3000 |
| BT25-061 | Offmon | IMPLEMENT | IMPLEMENTED | 7/7 | link_condition(Appmon,1) + alt-digivolve + [Start Main] optional trash-Appmon->draw+memory + when_linked CannotUnsuspend(end_of_opponents_turn) |
| BT25-004 | Tapmon | IMPLEMENT | BLOCKED (engine) | 0 | sole clause = inherited [Your Turn][OPT] WhenWouldLink host-filtered optional link-cost-reduction (facet #10) — unwired |
| BT25-045 | Onmon | IMPLEMENT | BLOCKED (engine) | 0 | link_condition + when_linked suspend ARE expressible, but mandatory [Your Turn][OPT] WhenWouldLink cost-reduction (facet #10) can't be dropped -> BLOCKED not PARTIAL |
| BT25-052 | Logimon | IMPLEMENT | BLOCKED (engine) | 0 | [Main][OPT] link a chosen Social/Tool/Game card from hand/digivolution-cards (facet #9) — wired path only absorbs a standing permanent |
| BT25-036 | Craftmon | IMPLEMENT | BLOCKED (engine) | 0 (ignored placeholder) | prior G-DSL-WHEN-LINKED-TIMING RESOLVED; now BLOCKED on App Fuse play-flow (AddAppfuseMethodByName) — engine primitive missing |

## Engine-Gap Blocked Cards
### BT25-004 Tapmon / BT25-045 Onmon — facet #10 (WhenWouldLink host-filtered cost-reduction)
- The wired Shape-B WhenWouldLink is a replacement that pays via the player-global
  `link_cost_delta_for_player` (unconditional, not host/card-trait-filtered) and
  exposes no optional cost-reduction outcome. No DSL step registers a filtered,
  optional, one-shot ChangeLinkCost on a `when_would_link` trigger.
- Tracked: `docs/RUST_ENGINE_GAPS.md` [Link] subsystem facet #10.

### BT25-052 Logimon — facet #9 (link a chosen card from hand/digivolution-cards)
- Shape-B absorbs a standing permanent (root `None`); it does not link a chosen
  card from hand / digivolution-cards (DCGO `ILinkCard.LinkCard` from Hand /
  DigivolutionCards). The `[Main][OPT]` link-of-chosen-card is the card's defining
  clause; its two `when_linked` host-self payoffs depend on a link firing first.
- Tracked: `docs/RUST_ENGINE_GAPS.md` [Link] subsystem facet #9 (cross-ref BT25-089/070).

### BT25-036 Craftmon — App Fuse primitive
- `AddAppfuseMethodByName(Kabemon, Gomimon, Ecomon, Puzzlemon)` is a player-choosable
  alternate play path. App Fuse is not implemented in the engine (no lowering in
  `code/digimon-engine/src/dsl_cards/`, no engine-core handler; the DSL
  `AltPathKind::AppFusion` variant parses but resolves to nothing). Mandatory clause
  -> BLOCKED. The prior G-DSL-WHEN-LINKED-TIMING block is now resolved.
- Tracked: `docs/RUST_ENGINE_GAPS.md` `App Fuse` keyword/primitive entry.

## Files created
- code/digimon-engine/cards/bt25/BT25-007.yaml
- code/digimon-engine/cards/bt25/BT25-061.yaml
- code/digimon-engine/tests/cards_behavioral/bt25/bt25_007.rs (7 tests)
- code/digimon-engine/tests/cards_behavioral/bt25/bt25_061.rs (7 tests)

## Verification
- `cargo run -p dsl-lint -- code/digimon-engine/cards/bt25/BT25-007.yaml` / `BT25-061.yaml`: clean.
- `cargo test --test cards_behavioral -- bt25`: 516 passed, 0 failed, 3 ignored.
- Full `cargo test --test cards_behavioral`: 4064 passed, 7 failed (ALL pre-existing,
  unrelated DP-modifier tests: bt21_072 x4, ex7_030, p_134, p_197 — see RUST_ENGINE_GAPS.md
  Shape-B note "7 unrelated pre-existing DP failures"), 65 ignored.
- `cargo test --test option_flow -- link`: 33 passed, 0 failed.
