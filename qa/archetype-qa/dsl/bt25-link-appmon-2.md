# Archetype DSL Implementation: BT25 "link-appmon-2" slice
Date: 2026-06-07
Total cards in pool: 4
Processed this run: 4
Pipeline: batch-implement-cards-rust-dsl

Cards (low→high stage): BT25-070 Logamon (Lv.4), BT25-056 Bootmon (Lv.5),
BT25-072 Shutmon (Lv.5), BT25-060 Rebootmon (Lv.6) — the Offmon/Hackmon →
Logamon → Bootmon/Shutmon → Rebootmon Appmon "App Fusion" line.

## Summary
- IMPLEMENTED: 0
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 4
- SKIPPED (prior verdict): 0

All four were prior-BLOCKED and re-adjudicated this run (rerun, not SKIP — only
`IMPLEMENTED`/`AUDITED-OK` short-circuit). Conclusion unchanged: all remain
**BLOCKED (hybrid)** on the `[Link]` keyword subsystem facet #9.

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-070 | Logamon | IMPLEMENT | BLOCKED (hybrid) | — | 0 | [Main][OPT] link Social/Tool/Game card from trash/digi-cards −1; [Your Turn][OPT] WhenLinked delete opp cost≤4; inherited [When Linking] can't-unsuspend |
| BT25-056 | Bootmon | IMPLEMENT | BLOCKED (hybrid) | — | 0 | Barrier (ok) + link from hand/digi-cards −2; [All Turns] WhenLinked suspend opp; inherited return suspended opp to bottom of deck |
| BT25-072 | Shutmon | IMPLEMENT | BLOCKED (hybrid) | — | 0 | Jamming (ok) + link from trash/digi-cards −2; [All Turns][OPT] WhenLinked deny-digivolve; inherited 2× can't-unsuspend |
| BT25-060 | Rebootmon | IMPLEMENT | BLOCKED (hybrid) | — | 0 | SecA+1/Reboot/Link+1 (ok) + free-link Appmon from hand/digi-cards as cost → unsuspend; [All Turns][OPT] WhenLinked-or-OnUnsuspend self-buff Piercing/Blocker/effect-immunity |

## Controlling gap (why BLOCKED, not PARTIAL)

Every one of these cards' defining clause links a **chosen card** selected from
`{hand | trash | this Digimon's digivolution-cards}` onto the **host Digimon**
(itself). DCGO models this as `new ILinkCard(true, cardSource, host).LinkCard()`
/ `thisPermanent.AddLinkCard(cardSource)` with `SelectCardEffect.Root` =
`Hand` / `Trash` / `DigivolutionCards`:
- `BT25_070.cs:181` (trash + digivolution-cards)
- `BT25_056.cs:196` (hand + digivolution-cards)
- `BT25_072.cs:201` (trash + digivolution-cards)
- `BT25_060.cs:160` (hand + digivolution-cards, free, as activation cost)

The Rust engine has **no primitive** for this, and the DSL has **no verb** that
lowers to one:
- `link_to_own_digimon` (only DSL link step) links the **carrier Option** being
  played (`attach_linked_card` reads `pending_option`), not an arbitrary card
  from hand/trash/digisources.
- The 2026-06-06 Shape-B substrate (`begin_digimon_link` / `commit_digimon_link`
  / `absorb_standing_digimon_as_link`) **absorbs a standing permanent** (DCGO
  root `None`) — explicitly noted in `docs/RUST_ENGINE_GAPS.md` as covering the
  dominant BT21+ shape, with "from-hand/trash Digimon-link initiation and the
  rarer source origins … not yet wired" listed as **Residual (BLOCKED)**.

This is facet **#9** of the `docs/RUST_ENGINE_GAPS.md` `[Link]` keyword subsystem
entry, which already names BT25-070 (and BT25-052/089) as BLOCKED.

Since the link clause is each card's central mechanic AND gates the
`WhenLinked` / `[When Linking]` payloads, no faithful PARTIAL exists — shipping
only the declarative keywords (Barrier / Jamming / Security A.+1 / Reboot /
Link +1) and the App-Fusion/alt-digivolve requirement would silently drop the
whole effect kit (no-approximations). Ship no YAML.

### Stale sub-reason corrected this run
The prior orphan-d notes cited a secondary "`when: when_linked` DSL token
missing" blocker. That gap (**G-DSL-WHEN-LINKED-TIMING**) has since **landed**
(`Timing::WhenLinked` → `CompiledTiming::WhenLinked`, `clause.rs:146` /
`compile.rs:258`). It is no longer a blocker; facet #9 alone keeps these BLOCKED.

## Hybrid-Gap Blocked Cards (engine + DSL)
### BT25-070 / BT25-056 / BT25-072 / BT25-060 — link-a-chosen-card-onto-host
- **Missing engine API:** a primitive to attach a chosen `CardSource` from
  `{hand | trash | digivolution-cards}` onto a host Digimon as a link card,
  with a cost-reduction delta and optional free-as-cost semantics —
  e.g. `ctx.link_card_from_source_to_own_digimon(source_zone, card_filter, host, cost_delta|free)`.
  (Distinct from `attach_linked_card`, which links only `pending_option`.)
- **Missing DSL verb:** `link_card_from_{hand|trash|sources}_to_own_digimon`
  (or a unified `link_card_to_own_digimon: { from: [hand,trash,digivolution_cards], card_filter, host_filter, free|cost_delta }`)
  that lowers to the above. Suggested shape mirrors the orphan-d /
  BT25-069/101 documented `link_card_from_trash_to_own_digimon` gap, extended to
  span hand + the carrier's digivolution stack.
- **Cross-refs:** `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem facets #6/#9/#10/#11;
  `qa/dsl-vocab-gaps.md` orphan-d + BT25-069/101 + G-DSL-DIGILINK (LANDED).
