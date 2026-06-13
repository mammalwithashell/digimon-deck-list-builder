# Appmon (BT25 slice) — Model

> Scope: the BT25 "appmon" slice passed to `/archetype-interaction-test-author`
> = **BT25-052 Logimon**, **BT25-089 Kazuki & Itsuki**, **BT25-098 Cyber Engage**.
> This is a capstone interaction-test pass: cards are assumed already
> implemented (or BLOCKED) by `/batch-implement-cards-rust-dsl`. Source priority
> per CLAUDE.md: `general_rule.pdf` (canonical) + DCGO C# outrank card-text JSON.

## Card pool & roles
| Card | Role | One-line function | Impl status (validated_cards_dsl.json, 2026-06-06) |
|------|------|-------------------|-----|
| BT25-098 Cyber Engage (Yellow Option, cost 3, [Appmon]) | enabler / engine | `[Main]` reveal top 3 → add 1 [Appmon] to hand, trash rest, place self as `<Delay>`; `<Delay>` trash self → play 1 [Appmon] for cost −3; `[Security]` place self; Use Req. ([Appmon] trait) → IgnoreColorRequirement | **IMPLEMENTED** (`AUDITED-MISSING-TESTS`; YAML `cards/bt25/BT25-098.yaml`, per-card tests `cards_behavioral/bt25/bt25_098.rs`) |
| BT25-089 Kazuki & Itsuki (Green Tamer, cost 4, App Driver/Appmon) | engine / payoff-enabler | `[Start of Main]` +1 memory if opp has a Digimon; `[Main]` suspend self → link 1 [Appmon] Digimon card from hand/digivolution-cards to a Digimon (cost −2); `[End of Turn][OPT]` 1 of your Digimon may **App Fuse** into a Digimon card in hand | **BLOCKED** (hybrid: [Link] facet-9 link-of-chosen-card primitive + App Fuse keyword both missing) |
| BT25-052 Logimon (Green/Red Lv.4, cost 5, Sup./Social/Login) | engine / link host | `[Main][OPT]` link 1 [Social]/[Tool]/[Game] Digimon card from hand/this Digimon's digivolution-cards to self (cost −1); `[Your Turn][OPT]` **When this Digimon gets linked**, if you have ≤1 Tamers, you may play 1 [Kazuki & Itsuki] from hand free | **BLOCKED** (hybrid: [Link] facet-9 link-of-chosen-card primitive + facet-11 WhenLinked host self-effect) |

## Digivolution lines
Not exercised — the slice contains one Lv.4 Digimon (Logimon, BLOCKED), one
Tamer, and one Option. No egg→rookie→champion chain is testable within the slice;
cross-set evolution prerequisites would have to be synthesized, but no surviving
combo needs them (see below).

## Named combos
### Combo A — Logimon WhenLinked → play Kazuki & Itsuki free
- Cards: **BT25-052** (Logimon), **BT25-089** (Kazuki & Itsuki).
- Expected mechanical outcome: linking a [Social]/[Tool]/[Game] card to Logimon
  triggers its `[Your Turn]` WhenLinked; with ≤1 Tamers you play Kazuki & Itsuki
  from hand without paying cost (Tamer enters battle area, no memory spent).
- Rules/keyword basis: [Link] subsystem (facet 9 link-of-chosen-card; facet 11
  WhenLinked host self-effect) — `general_rule.pdf` §16 [Link]; DCGO `BT25_052.cs`.
- Rank: high (the slice's signature engine), **but BLOCKED**.

### Combo B — Kazuki & Itsuki suspend-link (−2) + End-of-turn App Fuse
- Cards: **BT25-089** (Kazuki & Itsuki) (+ any [Appmon] Digimon as link/fuse fodder).
- Expected mechanical outcome: suspend the Tamer to attach a chosen [Appmon]
  Digimon card from hand/digivolution-cards as a link for −2; at end of turn,
  App Fuse one of your Digimon into a hand Digimon card.
- Rules/keyword basis: [Link] facet 9 + **App Fuse** net-new keyword —
  `general_rule.pdf` §16; DCGO App Fuse keyword effect + `BT25_089.cs`.
- Rank: high, **but BLOCKED**.

### (Non-combo) Cyber Engage reveal → `<Delay>` replay
- Card: **BT25-098** only. This is a single-card sequence (reveal-3/add-Appmon,
  then later trash-self to replay an [Appmon] for −3), already fully covered by
  the per-card behavioral suite `cards_behavioral/bt25/bt25_098.rs` (reveal/add/
  trash/place, `<Delay>` activation with/without play, optional decline, Use Req.
  color-ignore gate). Authoring it under `tests/archetypes/` would duplicate a
  per-card test, not assert a *cross-card* interaction — so it is intentionally
  NOT authored here (one test ⇄ one multi-card combo; capstone scope).

## Playstyle
Combo/midrange Appmon value engine: search + cost-cheat [Appmon] cards into play
(Cyber Engage), then chain links and App Fuse swaps off Kazuki & Itsuki / Logimon
for board development and tempo. The two engine pieces (link-of-chosen-card,
App Fuse) are not yet expressible in the Rust engine.

## Win conditions
Not reachable within the implemented slice — the deck closes via the linked/
App-Fused Appmon board, which depends on the two BLOCKED primitives.

## Ranked interactions to test
1. Combo A (Logimon WhenLinked → free Kazuki & Itsuki) — **BLOCKED** on
   BT25-052 + BT25-089 (engine [Link] facet 9/11). Not authored; logged.
2. Combo B (Kazuki suspend-link −2 + App Fuse) — **BLOCKED** on BT25-089
   (engine [Link] facet 9 + App Fuse). Not authored; logged.

## Outcome of this run
- **Interaction tests authored: 0.** Every cross-card combo in the slice names a
  BLOCKED card (BT25-052 and/or BT25-089).
- **Gaps** are already logged in `docs/RUST_ENGINE_GAPS.md` (2026-06-06,
  attributed to this skill): the [Link] **facet-9 link-of-chosen-card** primitive
  (+ facet-11 WhenLinked host self-effect) and the **App Fuse** keyword. No new
  gap entries were needed; no engine code was edited.
- Run recorded in `qa/qa-reports/archetype_interactions.json` under `appmon`.
- DCGO source was **not** pulled for BT25-052/BT25-089 (lazy/no-eager-closure):
  no authored combo test fires their printed effects, and they are unimplemented.
