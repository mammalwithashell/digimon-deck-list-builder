# Kazuki & Itsuki / Appmon (BT25 slice) — Model

> Scope: the 3-card BT25 slice `BT25-052`, `BT25-089`, `BT25-098`.
> Authored by `/archetype-interaction-test-author` (capstone). This run is a
> **gating report**: 2 of the 3 cards are unimplemented and one engine
> primitive (App Fuse) is missing, so no interaction test could be authored.
> See "Precondition gating" below.

## Card pool & roles

| Card | Name | Role | One-line function |
|------|------|------|-------------------|
| BT25-052 | Logimon (Lv.4, [3,0]=Black/Red, 6000DP, [Social]/[Login]) | engine / payoff | `[Main][OPT]` link a Social/Tool/Game Digimon card to self (cost −1); on getting linked with ≤1 Tamer, free-play a Kazuki & Itsuki. |
| BT25-089 | Kazuki & Itsuki (Tamer, Black, [App Driver]/[Appmon]) | engine / enabler | `[Start of Main]` ramp +1 memory if opp has a Digimon; `[Main]` suspend to link an Appmon card (cost −2); `[End of Turn][OPT]` app-fuse one of your Digimon into a hand Digimon. |
| BT25-098 | Cyber Engage (Option, Yellow, [Appmon]) | enabler | `<Use Req. [Appmon] trait>`; `[Main]` reveal top 3, add 1 Appmon to hand, trash rest, place self as Delay; `<Delay>` trash to play 1 Appmon (cost −3). [Security] place in battle area. |

Implementation status (verified):
- **BT25-052 — NOT implemented.** No YAML in `code/digimon-engine/cards/bt25/`, no behavioral test, not in `validated_cards_dsl.json`, no `raw_rust`.
- **BT25-089 — NOT implemented.** Same: nothing anywhere in the engine.
- **BT25-098 — partially present.** YAML spec exists (`cards/bt25/BT25-098.yaml`) but its per-card behavioral test `tests/cards_behavioral/bt25/bt25_098.rs` is an **empty placeholder** (0 lines) — i.e. the per-card green-tests precondition this capstone assumes is not met for it either.

## Digivolution lines

This slice has no internal digivolution line — Logimon is the only Digimon
(Lv.4) and its only printed evo cost is `{black, from Lv.3, memory 3}`. The
Lv.3 prerequisite is a **cross-set** card not in the slice. Per the task, any
cross-set evolution prerequisite would be a **synthesized DebugRunner fixture**
(`make_test_card` with explicit `evo_costs` + `can_digivolve`), and a real
cross-set card's printed effect would be pulled **only** if a combo actually
fires it (lazy). No combo here reaches that point because the slice's own cards
are unimplemented.

## Mechanics required (engine substrate audit)

| Mechanic | Card(s) | Engine support | Source |
|----------|---------|----------------|--------|
| Link (attach Digimon card to host, cost reduction) | 052, 089 | **Present.** `EffectTiming::WhenWouldLink`, `OnLink`, `OnUnlink`, `OnLinkedCardTrashed`; `ModifierType::ChangeLinkCost` / `ChangeLinkMax`. Used by AD1-005 / BT21-053/054/059/073. | `enums.rs:296-349,748-752`; DCGO `WhenLinked` (ICardEffect.cs:992) |
| "When this gets linked" trigger | 052 | **Present** (`OnLink` observer). | `enums.rs:332-338` |
| Free-play a named Tamer | 052 | Present (play-without-cost is common DSL vocab). | — |
| Memory ramp at Start of Main, conditional | 089 | Present (start-of-main triggers exist). | — |
| Suspend-cost activated ability | 089 | Present (suspend-as-cost is standard). | — |
| **App Fuse** ("1 of your Digimon may app fuse into a Digimon card in the hand") | 089 | **MISSING.** No `app_fuse` / "app fuse" anywhere in `src/`, `digimon-dsl/`, or DSL process kinds. (All `fuse` source hits are `confuse`/`refuse`.) | grep verified |
| Reveal-top-N + add-by-trait + trash rest | 098 | Present (`reveal_top_deck`/`select_reveal`/`add_to_hand_from_reveal`/`trash_from_reveal` — already in BT25-098.yaml). | `cards/bt25/BT25-098.yaml` |
| Place Option in battle area as Delay; `<Delay>` cost-reduced play | 098 | Present (`place_self_as_delay_option`, `delay` effect kind). | `cards/bt25/BT25-098.yaml` |
| `<Use Requirement>` + IgnoreColorRequirement flood-gate | 098 | Present (`flood_gate` + `IgnoreColorRequirement`). | `cards/bt25/BT25-098.yaml` |

## Named combos (candidate interactions)

### Combo A — "Logimon → free Kazuki & Itsuki" (link-into-free-Tamer)
- Cards: **BT25-052**, **BT25-089**.
- Expected mechanical outcome: with Logimon on field and ≤1 Tamer, using its
  `[Main]` link (attach a Social/Tool/Game Digimon card, cost −1) fires the
  `OnLink` trigger; because the controller has ≤1 Tamer, they may play
  Kazuki & Itsuki from hand for **0 cost** → board gains the Tamer, memory not
  spent, link's host gains the linked card.
- Rules/keyword basis: Link semantics (`general_rule.pdf §16`, keyword Link);
  `OnLink` observer (`enums.rs:332-338`, DCGO `WhenLinked`).
- Rank: high (central identity combo — it's the slice's whole point).
- **GATE: BLOCKED — both BT25-052 and BT25-089 unimplemented.**

### Combo B — "Cyber Engage digs Appmon → Kazuki & Itsuki links it" (search-then-link)
- Cards: **BT25-098**, **BT25-089**.
- Expected mechanical outcome: Cyber Engage `[Main]` reveals top 3, adds an
  Appmon (e.g. an Appmon Digimon card) to hand; later Kazuki & Itsuki's
  `[Main]` suspend-link attaches that Appmon card to a Digimon at cost −2. Or
  the `<Delay>` plays an Appmon at cost −3. Net: card advantage + a discounted
  Appmon entering play/link.
- Rules/keyword basis: trait-gated search + Link cost reduction.
- Rank: medium.
- **GATE: BLOCKED — BT25-089 unimplemented; BT25-098's own per-card test is an
  empty placeholder (precondition unmet).**

### Combo C — "Kazuki & Itsuki app-fuse end-of-turn" (app-fuse engine)
- Cards: **BT25-089** (+ any Digimon on field + a hand Digimon target).
- Expected mechanical outcome: at end of turn, one of your Digimon "app fuses"
  into a Digimon card in hand (the App Fuse keyword's swap/merge behavior).
- Rules/keyword basis: App Fuse keyword (`general_rule.pdf §16`).
- Rank: medium (recurring engine).
- **GATE: BLOCKED — BT25-089 unimplemented AND App Fuse primitive missing from
  the engine. Two independent blockers.**

## Playstyle

Combo/midrange tempo deck: Kazuki & Itsuki is the memory + recursion engine
(ramp each turn the opponent has a board, then suspend to link discounted
Appmon, then app-fuse to recycle). Logimon is the link payoff that snowballs a
free Tamer onto the board. Cyber Engage is the consistency enabler (dig + a
discounted follow-up play). Memory curve trends positive once the Tamer is down.

## Win conditions

Not self-contained in this 3-card slice — the slice supplies the engine
(ramp/link/app-fuse) and consistency; the actual finishers (large Appmon
Digimon reached via link/app-fuse, plus the suspend inherited from Logimon)
live in the rest of the BT25 Appmon package outside this slice.

## Precondition gating (Phase 4 result) — why no interaction tests were authored

Per the skill's capstone contract, interaction tests are authored only for
combos whose named cards are **all implemented** with green per-card tests.

- **Combo A (052+089): BLOCKED** — both cards unimplemented.
- **Combo B (098+089): BLOCKED** — 089 unimplemented; 098 per-card test empty.
- **Combo C (089, App Fuse): BLOCKED** — 089 unimplemented + App Fuse engine
  primitive missing.

Every candidate combo is blocked, so **0 interaction tests authored** and the
archetypes test file `tests/archetypes/kazuki-itsuki-appmon.rs` was not created
(nothing could legally pass). This is the correct capstone behavior — it
reports the gap rather than authoring tests that cannot pass or weakening them.

### Routed findings

1. **Implementation backlog** (`/batch-implement-cards-rust-dsl`, BT25):
   - BT25-052 Logimon — implement (Link `[Main]` + `OnLink` free-Tamer trigger).
   - BT25-089 Kazuki & Itsuki — implement (memory ramp, suspend-link, app-fuse).
   - BT25-098 Cyber Engage — author the missing per-card behavioral test
     (`tests/cards_behavioral/bt25/bt25_098.rs` is empty).
2. **Engine-primitive gap** → `docs/RUST_ENGINE_GAPS.md`:
   **App Fuse** keyword/primitive (`[End of Your Turn] 1 of your Digimon may
   app fuse into a Digimon card in the hand`, BT25-089). No `app_fuse` exists
   in `src/`, `digimon-dsl/`, or DSL process kinds. Needed before Combo C is
   authorable. (Link, by contrast, is already present and not a gap.)

When BT25-052 + BT25-089 land (and App Fuse for Combo C), re-run this skill to
author Combos A/B (and C) as DebugRunner interaction tests against the model
above.
