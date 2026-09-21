# EX12 meta refresh and next campaign target — 2026-09-12

**Status:** prioritisation aid, not a verdict. Columns are measurements; the pick is editorial.
**Supersedes for target selection:** `2026-08-22-unimplemented-winning-decks.md` (whose corpus-wide
counts are still valid; this doc scopes to the *current* EX12 field only).

## What was refreshed

`data/deck_library.json` was rebuilt from DCG Nexus (the only source that tags each event's legal
format): 69 EX12 events since 2026-07-20, taking the library from 4242 to 5097 entries. Every
sampled event through 2026-09-12 is still tagged **EX12**, including the BT26 release events of
2026-08-30 — so "current format" below means the EX12 tag, and BT26 cards are already inside it.

## Method

Per archetype, over EX12-tagged decklists only: `plays` / `share` / `tc` (top-cut count) /
`conv`, the union of distinct card ids, and four coverage columns kept apart on purpose:

- `yaml%` — a YAML spec exists (an upper bound on coverage; existence, not correctness)
- `verd%` — `IMPLEMENTED` or `AUDITED-OK` in `validated_cards_dsl.json` (the honest number)
- `wverd%` — the same, weighted by copies across lists (how much of an actual deck is playable)
- `dcgo%` — DCGO has a card-effect class, i.e. the oracle can answer for it

`exam` = cards with any exam verdict; `miss` = cards with no YAML; `miss_sets` = where the missing
cards live. `score = tc × (1 − verd%)`, the same editorial form as the August doc.

## The field since 2026-07-20 (964 decklists, 81 archetypes)

| Archetype | plays | share | tc | conv | distinct | yaml% | verd% | wverd% | dcgo% | miss | miss_sets |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Glowing Dawn | 88 | 9.1 | 30 | 0.34 | 47 | 70 | 70 | 98 | 79 | 14 | BT26:11 BT6:1 BT9:1 P:1 |
| Toho Braves | 80 | 8.3 | 44 | 0.55 | 37 | 81 | 19 | 5 | 84 | 7 | BT26:6 BT8:1 |
| TS Jupitermon | 70 | 7.3 | 24 | 0.34 | 82 | 71 | 52 | 69 | 77 | 24 | BT26:21 BT24:1 BT9:1 BT25:1 |
| **Three Musketeers** | 69 | 7.2 | 24 | 0.35 | 52 | 58 | 46 | 53 | 94 | 22 | EX7:7 BT25:2 BT26:2 BT7:2 |
| Styracomon | 40 | 4.1 | 13 | 0.33 | 39 | 87 | 85 | 99 | 97 | 5 | LM:3 ST12:1 BT24:1 |
| TS Angels | 37 | 3.8 | 22 | 0.59 | 58 | 79 | 55 | 87 | 91 | 12 | BT26:4 LM:1 BT25:1 P:1 |
| Vemmon | 37 | 3.8 | 21 | 0.57 | 21 | 76 | 76 | 82 | 100 | 5 | BT21:2 P:1 EX11:1 BT11:1 |
| Saiyu Warriors | 35 | 3.6 | 12 | 0.34 | 24 | 96 | 17 | 1 | 96 | 1 | BT26:1 |
| TS Vulcanusmon | 35 | 3.6 | 12 | 0.34 | 57 | 86 | 61 | 74 | 91 | 8 | BT26:4 P:2 ST12:1 BT24:1 |
| Apps Reboot | 25 | 2.6 | 8 | 0.32 | 44 | 73 | 73 | 92 | 95 | 12 | BT23:5 BT26:2 P:1 EX1:1 |
| Data Squad | 25 | 2.6 | 6 | 0.24 | 61 | 64 | 59 | 74 | 77 | 22 | BT26:14 BT13:3 LM:1 EX4:1 |
| Titans | 24 | 2.5 | 7 | 0.29 | 68 | 34 | 24 | 7 | 81 | 45 | BT24:13 BT26:13 BT6:3 BT4:3 |
| Metal Empire | 23 | 2.4 | 6 | 0.26 | 44 | 48 | 27 | 4 | 98 | 23 | EX12:12 EX9:2 BT18:2 BT22:2 |

(Toho Braves' low `verd%` is a tracker artefact — its pool was oracle-examined directly, 107 of
166 clauses confirmed, without the verdict tracker being backfilled. See
`qa/qa-reports/toho-braves-exam-report.md`.)

## Findings

1. **BT26 is now the binding constraint for the top of the field, not our coverage.**
   `data/cards.json` has **zero** BT26 cards, `code/digimon-engine/cards/bt26/` does not exist,
   and the base DCGO checkout (`f32d380f2`, 2026-08-26) carries only **5** BT26 scripts. Glowing
   Dawn (11 of 14 missing are BT26), TS Jupitermon (21 of 24), Data Squad (14 of 22), Titans and
   Chronomon all want BT26 first. Those need `/author-set BT26` after a card-data refresh **and** a
   DCGO submodule bump before the oracle can answer — a different job than an archetype campaign.
2. **Toho Braves is done as a campaign** (core 69/74 adjudicated, 0 diverged); its only new work is
   6 BT26 cards, blocked on the same thing.
3. **Three Musketeers is the best implement-plus-exam target that the oracle can actually serve
   today.** Top-4 deck (7.2% of the current field, 7.4% across all EX12), DCGO covers 94% of its
   pool, and its missing cards are an EX7-centred slice rather than BT26. Resolver plan
   (`tools.clause_coverage.campaign`): 55 cards, core 15 (in ≥72 of 102 lists), **implement 31**
   (5 of them core: BT21-074, BT25-005, BT25-085, EX7-008, EX7-051), **exam 125** clauses (35
   core), skipped 6. Only BT26-078 and BT26-083 are unimplementable (no card data, no DCGO
   script). Bonus: its 8 EX7 gaps are a strict superset of Beelstar's.
4. Runners-up, for the next dispatch: **TS Angels** (best conversion in the top ten at 0.59, 12
   missing, only 4 BT26) and **Saiyu Warriors** (fully written, 47 clauses unexamined, campaign
   already half-run at 44 confirmed).

## Decision

Dispatch `/archetype-campaign "Three Musketeers"` (job `three-musketeers-1`). Finish line per the
skill: every core clause adjudicated and zero untriaged `diverged`; pool coverage reported, not
gated.
