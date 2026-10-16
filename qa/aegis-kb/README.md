# Aegis rules KB vs our corpora (2026-10-08)

Source compared: Aegis simulator (MIT), <https://github.com/vinicius3333/aegis-digimon-tcg> at
`ec0cd22f0`, `data/kb/`. Compared against `data/card_official.json` + `data/card_bundles/`, the
errata guard (`code/tests/test_cards_json_errata.py`, **on branch
`claude/suspicious-thompson-504ad4` — not on main**), the judge-quiz suite
(`code/digimon-engine/tests/judge_quiz/`), and `data/deck_formats.json`.

Scripts that produced the numbers live in the session scratchpad; the reproducible pieces are
`code/tools/import_official_qa.py` (importer) and this directory's backlog JSON.

## 1. What the Aegis KB holds

| File | Holds | Comes from | Fetched |
|---|---|---|---|
| `qa.json` | dict card ID → list of `{qno, date, question, answer, related[]}`. 2,686 cards, 7,173 entries, 6,622 distinct Q-numbers (a ruling about two cards is listed under both). | Official Q&A pages `https://world.digimoncard.com/rule/?card_no=<ID>` (4,375 cards scanned). | Full crawl 2026-08-19; EX13 refresh 2026-09-20 (77 cards); a 224-card refresh 2026-09-13. 42 promos failed (`P-053`–`P-094`). Aegis flags its own currency as `"unverified"`. |
| `errata.json` | dict card ID → `{name, date, changes[{before, after}], notes, source}`. 91 cards, 92 changes. | Official errata page `https://world.digimoncard.com/rule/errata_card/` | 2026-08-19 |
| `banlist.json` | `events` (68 restriction events) + `current` (58 cards: 51 restricted, 4 banned, 3 `banned_pair`). | Official `https://world.digimoncard.com/rule/restriction_card/` | 2026-09-10 |
| `rules/comprehensive.md`, `manual.md`, `glossary.md` + `rules-index.json` (373 chunks) | Text conversions of the official PDFs. **Comprehensive Rules Ver. 4.3 (2026-09-18).** Glossary is marked archived. | `world.digimoncard.com/rule/pdf/{general_rule,manual,glossary}.pdf` | 2026-09-23 |
| `rule-obligations.json` | 7,837 "obligations" (1,119 rule, 6,622 card-QA, 92 errata, 4 interaction) with Aegis's own `status` (7,770 `gap`, 67 `proven`), `branch`, `mechanisms`, `reason`, `owner`, scenario scopes and an effect-play route matrix. | **Aegis's own engineering tracker** built over the official texts. The `text` fields cite official sources; everything else is Aegis's interpretation. | — |
| `manifest.json` | Source URLs, counts, fetch times, failed cards, refresh scopes. | — | — |
| `raw/` | Empty at this commit. | — | — |

Spot check: the Aegis entries for Ukkomon (BT16-082; Q2668–Q2671) and Growlmon (EX13-010;
Q7231–Q7234) match the live official pages in Q-number, date and answer.

## 2. Comparison

### 2a. Q&A rulings — we were missing almost all of them (now imported)

Our mirror's `qa` field is a lossy scrape of the official **card-list** page, not the Q&A
pages. `build_card_bundles.parse_official` stores, per card:

- **only one ruling**: 2,593 answer strings on 2,593 cards. Aegis has 7,173 rulings on 2,686 cards.
  Our string is Aegis's first ruling on 2,231 cards; a later one on 58.
- **the answer only**: no question, Q-number or date. 205 cards' entire "Official Q&A" was
  `Yes, you can.`
- **with `<Keyword>` tokens deleted** as if they were HTML tags (BT1-063: "It only grants ,
  no matter…"), and the page's "Related Cards" label appended.

On the remaining 291 cards our string matches no Aegis answer. I checked a sample, and every
mismatch traced to the two scrape artifacts above. **No real ruling conflicts were found.**
(This is a sample, not a proof. After the import the old strings are gone from the bundles; the
originals are still in `card_official.json`.)

Coverage gaps either way:
- Aegis only: 106 cards with rulings we had none for; 57 Aegis cards have no mirror entry at all
  (54 EX12, 3 promos).
- Ours only: 13 cards with a ruling Aegis lacks (BT26-020/035/041/058/086/087/088/092/096/097,
  RB1-004/006/007). These were published after Aegis's crawl.
- Neither side has the 42 promos `P-053`–`P-094` that Aegis failed to fetch (11 of them have our
  one-answer string).

Our corpus never cited Q-numbers: no behavioral test names one, but 79 test files quote
"Official Q&A" by text from the old one-answer bundles. One gap note cites Q5216
(`qa/archetype-qa/engine-gaps.md`).

**Action taken:** `code/tools/import_official_qa.py` → `data/card_official_qa.json` (7,173
rulings, each with its official URL and the crawl it came through) and the "## Official Q&A"
section of 2,629 bundles rewritten as `**Q####** (date) Q: … — A: …`. `build_card_bundles.py`
renders from the sidecar, so a rebuild keeps them. Provenance gate: the crawl's manifest must
name `https://world.digimoncard.com/rule/`; entries must be exactly the official fields
(`Q<digits>`, ISO date, question, answer, related IDs). Extra fields are dropped and reported,
and a Q-number whose text differs between cards is rejected everywhere. The run rejected 0,
dropped 0 fields, and found 0 inconsistent Q-numbers. `rule-obligations.json` is never read.
Guarded by `code/tests/test_import_official_qa.py`.

**Aegis data issue, corrected on import: cross-listed rulings lose their card.** A ruling about
card S that names card P is listed on both official pages. On P's page it sits under S's
heading, but Aegis drops the headings and strips the page's own ID from `related`. Without a
fix, 551 entries (449 Q-numbers) read as rulings on the page's card. For example, Angemon
(BT23-027) would carry Shakkoumon (BT23-032)'s Q6250; I verified this one on the live page. The
importer adds `about`: the copy whose `related` names every other page it is listed on. All 449
resolve to exactly one card, and the bundles label these rulings "a ruling on <ID>". 216 of the
2,386 rulings on implemented cards are cross-listed. Anyone consuming Aegis's `qa.json`
directly should know about this.

### 2b. Errata — ours is the superset; nothing to import

Our `ERRATA` has 99 rows on 92 cards, plus `PRINTED` for non-text errata. Aegis has 92 changes on 91 cards.

- **Aegis only (4):** BT3-111 (digivolve cost 3→5), BT4-041 (Attribute/Type
  Unidentified→Unknown), BT21-023 (yellow segment on cost ring) and EX11-025 (emblem art). All
  four are already in our guard's `PRINTED` table or its notes, and `cards.json` already carries
  BT3-111 and BT4-041.
- **Ours only (5):** BT13-089, EX4-058, EX4-071 and LM-013 (the 2025-04-25 "at the *next* end of
  your opponent's turn" group errata), plus P-030. EX4-063 also has two errata (2024-03-08 and
  2025-04-25); Aegis kept only the older one. `errata.json` is keyed by card, so it holds one
  errata per card and loses multi-card group entries. It is lossy.
- **Text:** with Aegis's undecoded HTML entities normalized (`&lang;Raid&rang;` → `<Raid>`), every
  shared After text matches. **No conflicts.**
- Our guard is not on main yet. It is on `claude/suspicious-thompson-504ad4` (`e2cb6d846`), with
  EX4-058 and EX4-072 still `PENDING`.

### 2c. Judge quiz — disjoint corpora

`tests/judge_quiz/` is the TCG-Judges Discord quiz: 30 questions, 29 PASS. Its questions are
community-authored multi-card scenarios, not Bandai Q-numbers, so there is no overlap to diff.
The official Q&A is the much larger corpus (2,386 rulings on our implemented cards) and is now
seeded below.

### 2d. Rules manual — ours is 7 revisions stale (not imported; action needed)

Our base-repo `general_rule.pdf` and `docs/digimon-rules/` derivations are **Ver. 3.6
(2025-12-25)**. Official is **Ver. 4.3 (2026-09-18)**. Changes since 3.6 (from the 4.3 update
history):
- 3.7: §13 Security Checks rewritten; 16-6 (`<Piercing>`) updated, 16-6-6 deleted; 17-1-3-7.
- 4.0: Arts Digivolve (2-11, 4-19), DUAL cards (4-5), Option cards in the battle area (4-27),
  "with XX in their texts" (4-22), `[X Per Turn]` (15-14-1), effects that reveal cards (15-15-3),
  many keyword sub-rules deleted, **new `<Use Req.>` (16-42) and `<Ascension>` (16-43)**,
  17-1-3 rule checks.
- 4.1: declaring use of a card (1-3-11), `(Rule)` (2-3-4), digivolution 8-x, **`<Engage>` (16-44),
  `<Guard>` (16-45)**.
- 4.2: Cost (4-2), 16-7 (`<Draw>`), **`<Detach>` (16-46), `<Succession>` (16-47)**.
- 4.3: "Also" (4-29), 2-3-3, 10-1-2-1, 11-5-1-4, 15-1-2, 15-3-3, 15-7-4, 15-13-3, 15-15-3-9.

Recommendation: download the 4.3 PDF from the official URL into the base repo's
`Digimon TCG resources/`, re-verify `keyword-semantics.md` §16 (the SessionStart table still
says Ver. 3.6), and bump `rules-index.json`. Use the official PDF, not Aegis's markdown
conversion. Rule numbers moved (e.g. 15-7…15-15-7-4 became 4-24), so citations in our docs and
tests should be re-checked.

### 2e. Banlist — ours comes from digimoncard.io and has drifted (report only)

`data/deck_formats.json` `official_eng` is sourced from digimoncard.io/DCGO. Compared with the
official `current` list (2026-09-10):
- Banned on official, not ours: **BT15-003 Nyaromon** (banned 2026-09-01).
- Restricted on official, not ours: BT10-080, BT14-033, BT23-032, BT3-092, EX1-066, EX5-059,
  EX5-061, EX8-012.
- Restricted on ours, not official (probably lifted): BT9-099, EX4-019, P-029, P-030.
- Pair bans (EX2-007/EX7-064 and BT20-037 with BT17-035/EX8-037) agree in shape.

`deck_formats.json` is compiled into the engine (`include_str!`), so changing it was out of
scope for this pass. It needs its own change, checked against the official page.

### 2f. What we deliberately did not import

- `rule-obligations.json`: Aegis's statuses, branches, expected results and reasons are its own
  interpretation.
- `errata.json`: lossy compared with ours (see 2b).
- `banlist.json` and `rules/*.md`: these are pointers to official sources. Take them from the
  official pages and PDF directly.

## 3. Test seeds

- **`test-seeds.md`**: curated seeds for the 30 most-played implemented cards (by meta-decklist
  frequency in `data/deck_library.json`), up to 3 non-boilerplate rulings each. Each seed gives
  the card, Q-number, a paraphrase, current test coverage, a reading of the YAML, and a
  suggested DebugRunner assertion. Of 75 seeds, 11 are already covered, 22 are partial, and
  42 are missing or blocked. Ten suspect YAML or engine readings are listed at the top
  (unrun). The strongest are the optional-on-mandatory deletes on Gallantmon (P-186) and
  ShineGreymon: Ruin Mode (EX4-074), the alt-digivolve missing from Agumon - Bond of Bravery
  (LM-021), and the missing memory check after a breeding-phase move (Digimon Emperor
  (BT8-094) Q1770).
- **`implemented-rulings.json`**: the full backlog. 2,386 official rulings on 754 of our 1,058
  DSL-implemented cards (753 of them are on `tested_cards.json`, so that list barely
  discriminates; deck usage is the real priority signal). Each row has a polarity-based
  assertion template. 454 rulings are flagged `generic` (definitional boilerplate repeated on
  15 or more cards, such as "a card with [X] in its text"). 87 rulings sit on cards with no
  behavioral test file. No existing test cites any of these Q-numbers.
