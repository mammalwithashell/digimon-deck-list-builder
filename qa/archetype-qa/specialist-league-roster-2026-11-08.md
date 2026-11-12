# Specialist league roster: EX13 lists and training-gate status (2026-11-08)

Spec: `docs/superpowers/plans/2026-11-08-specialist-league.md`.

**Source.** DigiLab API, `format=EX13`, all event types, collected 2026-11-08.
Qualified means field >= 8, placement in the top third, at least 2 wins and
wins >= losses. Selection reuses `prepare_ex13_pool`'s placement/field/recency
score plus its distinct-event and copy-diversity bonuses. Up to 30 candidates
were checked per archetype.

**Gate.** Every card must be admissible in the committed
`data/card_admissibility.json` (CLAUDE.md rule 34, layer 2). Deck legality and
the engine-binding registry check (`prepare_ex13_pool prepare`) are not run
yet; they run when the frozen pool is built.

Raw data: `D:/ex13-data/specialist-league/sources/`
(`roster.json`, `gate_report.json`, `cache/`).

## Ready: 4 gate-passing EX13 lists (15 of 21)

| Archetype | DigiLab slug | Picked lists (place/field, date) |
|---|---|---|
| Examon | examon | 3/202 Nov 1 regional; 8/56; 1/44; 1/9 |
| Jupitermon | jupitermon | 7/202 regional; 2/52; 3/40; 2/42 |
| Glowing Dawn | glowing-dawn | 13/202 regional; 2/40; 9/48; 1/31 |
| TS Mervamon | ts-minervamon | 16/202 regional; 3/48; 14/56; 8/42 |
| Chronicle | chronicle | 1/10; 9/43; 5/14 online; 4/16 online |
| DATA SQUAD Ravemon | data-squad-ravemon | 20/202 regional; 1/48; 4/18 online; 1/25 |
| Beelstarmon (3M) | beelstarmon-3m | 14/202 regional; 1/18 online; 5/56; 1/42 |
| Toho Braves | toho-braves | 4/202 regional; 2/48; 7/40; 1/45 |
| Dynasmon | dynasmon | all online: 2/11; 7/19; 1/11; 5/23 |
| Medusamon/Styracomon | medusamon | 28/202 regional; 1/14; 12/44; 11/43 |
| Chronomon | chronomon | 2/56; 68/202 regional; 1/25; 8/45 |
| TS Gear Forest | ts-gear-forest | 2/202 regional; 6/56; 14/40; 4/43 |
| Virus Busters | virus-busters | only 5 EX13 lists exist; 2 qualify. Picked 2/8, 1/8, 14/22, **36/38 (weak)** |
| SW | saiyu-warriors | 1/8; 2/19; 7/28; 4/11 |
| Venmon | galacticmon | 3/18 online; 4/18; 1/8; 4/13 (15-17 Vemmon-named cards each) |

## Blocked (6 of 21)

48 distinct cards block these archetypes: 41 have no implementation at all
(no YAML spec, no raw_rust), 6 have an untested DSL construct, and 1 has a
failed verdict.

| Archetype | EX13 lists | Passing | To reach 4 lists |
|---|---|---|---|
| Imperialdramon (UG) | 29 | 0 | **Test 1 construct:** `timing:before_pay_cost_observe` (ExVeemon BT12-022, Stingmon BT12-050). Unblocks all 6 qualified lists. |
| Jesmon | 22 | 0 | Implement Huckmon (BT20-008) and Sistermon Blanc (Awakened) (BT7-082); test `verb:substitute_replacement` (Sistermon Blanc ST12-12). |
| UlforceVeedramon | 81 | 0 | Implement Veedramon (BT11-027) and Veemon (EX1-013); test `predicate:can_unsuspend` (BT12-029, BT22-023); clear the failed verdict on Blue Scramble (LM-028, in 17/19 lists). |
| Light Hybrid | **0** | n/a | No EX13 lists exist on DigiLab (newest are BT25, June). Options: keep the current pool's list(s), or implement Strabimon (BT7-019), which alone unlocks 2 BT25/AD01 lists. Wanyamon (BT18-003), Neemon (BT18-031) and Salamon (EX6-016) unlock 2 more. |
| Gallantmon | 38 | 0 | Implement 13 cards: the whole Guilmon/Growlmon/WarGrowlmon/Gallantmon line and both Takato Matsuki tamers (AD1-003, AD1-008, BT12-089, BT17-010, BT17-080, BT19-007, BT19-009, BT21-064, EX2-008, EX8-009, EX8-012, ST7-03, ST7-05). |
| ADVENTURE | 25 | 0 | Implement 20 cards (the ST20/ST21 starter lines, BT21-061/067/075, AD1-022, ST21-14) and test `predicate:distinct_tamer_colors_gte` (Agumon ST20-10). |

## Suggested order

1. **Imperialdramon (UG):** one construct test, no new cards.
2. **Jesmon:** two cards plus one construct.
3. **UlforceVeedramon:** two cards, one construct, and the LM-028 verdict.
   LM-028 is a shared card, so check why it failed first.
4. **Light Hybrid:** decide whether the old lists are enough. If not, BT7-019
   is the cheapest unlock.
5. **Gallantmon, ADVENTURE:** full archetype implementations; run
   `/implement-rust-dsl-archetype` or `/archetype-campaign` for each.

After any of these lands, regenerate the gate artifacts (`tools.lowering_coverage`,
`card_admissibility`) and re-run the collector. The raw API responses are
cached, so a re-run only re-checks admissibility.

## Update: re-run against origin/main (later 2026-11-08)

Re-gated with origin/main's `data/` artifacts (6e175a96d, after ADVENTURE
landed in #756). Added Millenniummon and Red Hybrid. Light Hybrid,
Millenniummon and Red Hybrid may use older formats (newest first) when EX13 has
too few lists. One Red Hybrid list must carry the Volcdramon/Volcanicdramon +
Metallicdramon top end. Script: `D:/ex13-data/specialist-league/sources/league_lists2.py`;
output `roster_v2.json`, `gate_report_v2.json`.

- **ADVENTURE is ready on main** (7 passing lists).
- **Ready: 16 of 23.**
- **Needs authoring: 7** (Imperialdramon (UG), Jesmon, UlforceVeedramon,
  Light Hybrid, Gallantmon, Millenniummon, Red Hybrid): one file each in
  `qa/archetype-qa/specialist-league/needs-work/`. Red Hybrid is an even
  split: 2 Volcdramon/Volcanicdramon + Metallicdramon lists (BT25 regionals)
  and 2 EX13 lists.
- **Training lists:** all 23 archetypes, 4 lists each, in
  `qa/archetype-qa/specialist-league/training-lists/` (`roster.json` is the
  machine-readable copy for the league driver). The 7 needs-work archetypes'
  lists pass once their cards are fixed.
