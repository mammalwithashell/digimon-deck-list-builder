# Unimplemented winning decks — a prioritisation aid

**Date:** 2026-08-22
**Status:** prioritisation aid, NOT a verdict. The underlying columns are measurements; the
ranking is editorial.
**Scope:** which meta archetypes are worth implementing next in the Rust DSL, scored on
tournament success, how much is missing on our side, whether DCGO can serve as an exam oracle,
and era coherence.

---

## 1. Summary

The oracle is almost never the binding constraint. Across the 151 archetypes with
`times_played >= 5`, DCGO has a card-effect class for 2446 of 2477 distinct cards (98.7%), and
every archetype in the shortlist below is >= 96% oracle-covered. **What varies is our own
coverage**, which ranges from 12% to 98%. So "can DCGO answer for this?" should almost never
decide a target; it is a risk check, not a selector.

The second finding is that **era coherence varies enormously and is not visible in the
`era_sets` field**. Two archetypes with nearly identical top-cut counts can differ by 3x in how
concentrated their missing cards are. Hunters needs 42 cards spread over 7 sets with 79% of them
in just BT12+BT21; Sakuyamon needs 80 cards spread over 23 sets with no set holding more than
14%. That difference matters more for planning than the raw card count does.

The third finding is that **"missing" means two different things** and conflating them corrupts
the ranking. Some archetypes need cards *written* (no YAML spec exists). Others have essentially
every card written but almost none *verdicted* — Toho Braves is missing exactly 1 YAML file yet
only 16.7% of its pool carries a passing verdict. These are different kinds of work with
different costs and they are separated into different tiers below.

---

## 2. Methods

### 2.1 What I recomputed

I did not take either input measurement on faith. The two source agents disagreed on the YAML
file count (803 vs 822) and the verdict count (708 vs 679), so I re-derived every number in this
document. Worktree confirmed:

```
$ git rev-parse --show-toplevel
C:/Users/james/Documents/digimon-deck-list-builder-1/.claude/worktrees/bold-bassi-d34dc7
```

Script: `<scratchpad>/s1.py` (throwaway, outside the repo). No repo files were written except
this document; no engine code was changed; DCGO was read strictly read-only from the base repo
per rule 29 (`C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO`), with no
`git submodule update --init`.

```
YAML files: 828 distinct ids: 822
verdict entries: 708 pass(IMPLEMENTED|AUDITED-OK): 679
status counts: Counter({'IMPLEMENTED': 533, 'AUDITED-OK': 146, 'PARTIAL': 20, 'BLOCKED': 9})
DCGO .cs files: 4012 DCGO class ids: 4000
decklists: 4242 undecodable: 0 archetypes with cards: 392
times_played>=5: 151
GLOBAL distinct meta ids: 2917 yaml: 809 pass-verdict: 667 dcgo: 2848
```

**Card ids** come from the parsed `decklist` field on each decklist entry (a JSON-encoded string
array, one entry per copy). All 4242 decklists carry it; 0 were undecodable. I did **not** decode
`source_url`, and did not cross-check the two against each other.

**Coverage** is measured two ways, kept separate throughout:

- `yaml%` — a YAML spec file exists. This counts a file's *existence*, not its correctness.
- `verd%` — the id has status `IMPLEMENTED` or `AUDITED-OK` in `validated_cards_dsl.json`.
  `PARTIAL` (20) and `BLOCKED` (9) deliberately do **not** count as passing.

I rank on `verd%`, because that is the honest denominator. `yaml%` is an upper bound.

**Era coherence** is a new measure neither input agent produced. Both reported an `era_sets`
field listing every set any card came from; because competitive lists splash staples across the
whole history of the game, that field spans 20–35 sets for nearly every archetype and is close to
uninformative. Instead I compute the set distribution of the **missing** cards only, and report
`n_miss_sets` (how many distinct sets the work touches) and `top3%` (what share of the missing
cards live in the three most common sets).

### 2.2 Three corrections to the input measurements

**(a) The `_examples/` directory is live, shipped code — M1 undercounted by 6.** There are 828
YAML files but only 822 distinct ids: `code/digimon-engine/cards/_examples/` holds 13 files, 6 of
which duplicate a spec in a real set dir, and 6 of which exist *only* there (BT11-042, BT13-060,
BT18-102, BT7-107, EX11-027, EX6-072). M1 derived the set directory from the card id and looked
only there, so it never saw `_examples` and counted those 6 as unimplemented — BT11-042 appears in
M1's "missing" sample for Mastemon. But `code/digimon-engine/build.rs` compiles **every**
subdirectory of `cards/` ("Iterate every direct subdirectory of cards/ (e.g. \_examples, bt21,
ex11)"), and the behavioral tests confirm it: `bt17_007.rs` reads "the embedded DSL pack ships
BT17-007 from `_examples/`". Those 6 cards are genuinely shipped. This document uses the 822-id
union. (Side note, not chased here: 6 ids are compiled from two locations at once.)

**(b) M2's "679" was not a verdict count.** M2 reported 679 where M1 reported 708. Both are
right about different things: 708 is the total number of verdict entries, 679 is the
`IMPLEMENTED|AUDITED-OK` subset. Not a conflict.

**(c) M1 overstated the archetype-alias duplication.** M1 claimed Gallantmon/Dukemon,
Beelzemon/Beelze and Machindramon/Machindra are near-duplicate rows inflating the backlog, and
that `data/archetype_aliases.json` would collapse them. That file has 35 entries and does **not**
map any of those three pairs. Measuring card overlap directly:

```
Gallantmon     vs Dukemon        |A|=111 |B|= 96 inter= 64 jaccard=0.45 B-subset-of-A=32
Beelzemon      vs Beelze         |A|= 58 |B|= 48 inter= 41 jaccard=0.63 B-subset-of-A=7
Machindramon   vs Machindra      |A|=104 |B|= 85 inter= 43 jaccard=0.29 B-subset-of-A=42
```

Only Beelzemon/Beelze genuinely collapse (Beelze has just 7 cards outside Beelzemon).
Gallantmon/Dukemon are a related family sharing 64 cards but with 32 cards unique to Dukemon.
Machindramon/Machindra at 0.29 are **not** duplicates and should not be merged.

### 2.3 Mechanic-risk check

For each shortlisted archetype I extracted the printed keywords of its missing cards using
angle-bracket tokens (`<Kw>` / U+FF1C), not substring matching — a first pass using substrings
produced a large phantom "Ace" count that was matching "place"/"replace". Every keyword the
shortlist needs — Save, Alliance, Collision, Reboot, Raid, Iceclad, Retaliation, Blast Digivolve,
Material Save, Blocker, Piercing, Rush, Jamming — is present in `Keyword` at
`code/digimon-engine/src/enums.rs:436`, and Delay is modelled via `DelayEffect`/`DelayTrigger`.
DigiXros vocabulary was closed on 2026-05-24 per `qa/dsl-vocab-gaps.md:143`. So mechanic risk
across the shortlist is low, with the specific exceptions flagged per row.

### 2.4 Scoring

`score = top_cut_count * (1 - verd_pct/100)` — "top-cut finishes we cannot currently simulate".
The multiplicative form is a choice, not something in the data. It lets a big popular deck at
moderate coverage outrank a tiny near-perfect-conversion deck at low coverage. Rank by coverage
alone, or top-cut count alone, and the order changes materially. The final ranking below then
adjusts that score for era coherence and family overlap, which is a judgement call.

---

## 3. Ranked shortlist (implementation work)

`tc` = top_cut_count, `conv` = conversion_rate, `miss` = cards with no YAML spec,
`sets` = distinct sets those missing cards live in, `top3%` = share of missing cards in the
3 most common sets.

| # | Archetype | tc | conv | distinct | yaml% | verd% | DCGO% | miss | sets | top3% |
|---|-----------|----|------|----------|-------|-------|-------|------|------|-------|
| 1 | Hudiemon | 87 | 0.42 | 88 | 47.7 | 45.5 | 97.7 | 46 | 12 | 71.7 |
| 2 | Hunters | 21 | 0.78 | 65 | 35.4 | 29.2 | 98.5 | 42 | 7 | 85.7 |
| 3 | EX7 bundle: Three Musketeers + Beelstar | 21 + 31 | 0.53 / 0.84 | 48 / 53 | 50.0 / 58.5 | 39.6 / 49.1 | 97.9 / 100 | 28 union | 10 | 62.5 / 59.1 |
| 4 | Beelzemon (+ Beelze) | 45 (+21) | 0.63 / 0.84 | 58 | 15.5 | 13.8 | 98.3 | 49 | 18 | 44.9 |
| 5 | Sakuyamon | 65 | 0.54 | 111 | 27.9 | 25.2 | 96.4 | 80 | 23 | 35.0 |
| 6 | BG Imperial | 48 | 0.51 | 85 | 50.6 | 49.4 | 98.8 | 42 | 17 | 38.1 |
| 7 | Zephagamon | 55 | 0.55 | 81 | 25.9 | 19.8 | 97.5 | 60 | 25 | 35.0 |

### 1. Hudiemon — the best value-per-coherence trade in the dataset

**Worth doing:** 87 top-cut finishes is the highest of any archetype in the corpus, from 206
recorded entries. We can currently simulate 45.5% of its pool.

**Missing:** 46 cards, and **30 of them (65%) are in BT22 and BT23** — two adjacent recent sets.
Full distribution: `BT23 19, BT22 11, BT8 3, P 3, BT16 2, EX1 2`, tail across 6 more sets. Both
set dirs are already well populated (bt22 124 files, bt23 123), so this is card-level infill in
sets the substrate already understands, not a greenfield set.

**Oracle:** DCGO 97.7%. Only 2 of its 88 cards are DCGO-blind: BT3-061 (Chuumon) and ST13-08
(Chikurimon), both splashed memory/cost-lock commons rather than archetype pieces.

**Risk:** low. Keyword profile of the missing BT22/BT23 cards is DigiXros (14 carry `xros_req`),
Blocker (5), Delay (5), Alliance (4), Reboot (2), plus one Iceclad and one Raid — all supported.
The main caution is that BT23 at 19 cards is the single largest concentration, so if BT23 turns
out to lean on an unimplemented mechanic the estimate moves.

### 2. Hunters — the cleanest slice of work available

**Worth doing:** the tightest era coherence of any sizeable candidate. 42 missing cards across
only **7 sets**, with 85.7% in the top three and **24 in BT12 alone**; BT12+BT21 together account
for 33 of 42 (79%). Conversion rate 0.78 is high — when Hunters shows up it converts.

**Missing:** 42 cards. `BT12 24, BT21 9, BT10 3, BT8 2, EX10 2, BT11 1`.

**Oracle:** DCGO 98.5%; one blind card, BT3-077 (Gazimon).

**Risk:** low-moderate. The mechanic profile is unusually concentrated: **26 of the 33 BT12/BT21
missing cards carry `<Save>`**, and 18 carry a `xros_req` DigiXros requirement. Both are
supported (`Keyword::Save` in `enums.rs`; DigiXros vocabulary closed 2026-05-24). That
concentration is the reason this slice is cheap — one mechanic pattern repeated — but it also
means a single Save-semantics defect would affect most of the batch, so get Save exactly right
first. The honest caveat: 21 top cuts is a real step down from Hudiemon's 87. This ranks second
on *efficiency*, not on prize size.

### 3. EX7 bundle — Three Musketeers + Beelstar, one slice for two archetypes

**Worth doing:** these are two separate archetypes (52 combined top cuts, conversion 0.53 and
0.84) whose missing EX7 cards overlap almost completely:

```
Beelstar EX7 missing: EX7-005, EX7-008, EX7-040, EX7-043, EX7-044, EX7-051, EX7-059, EX7-066
3Musk    EX7 missing: EX7-005, EX7-008, EX7-010, EX7-011, EX7-013, EX7-040, EX7-043,
                      EX7-044, EX7-051, EX7-059, EX7-066
shared: all 8 of Beelstar's | union 11
```

**Beelstar's 8 missing EX7 cards are a strict subset of Three Musketeers' 11.** An 11-card EX7
slice therefore clears Beelstar's single largest gap entirely and most of Three Musketeers'.
Combined missing across both archetypes (all sets) is 28 cards.

**Oracle:** 97.9% and 100%. One blind card between them (ST13-08).

**Risk:** low. Keywords in the EX7 slice are Reboot, Piercing, Collision, Blocker, Retaliation,
Blast Digivolve, De-Digivolve — all supported; 9 of 11 carry `xros_req`. Caveat: Zephagamon also
has 6 missing EX7 cards, but they are **disjoint** from this set (EX7-004/031/032/034/036/064),
so there is no bonus third archetype here — a 17-card EX7 slice would touch three archetypes but
that is a bigger, less coherent unit of work.

### 4. Beelzemon (+ Beelze) — lowest coverage of any high-performing deck

**Worth doing:** 45 top cuts at 0.63 conversion, and our coverage is the lowest of any big deck
at **13.8% verdicted** (15.5% yaml). Beelze genuinely collapses into it (Jaccard 0.63; only 7 of
Beelze's 48 cards fall outside Beelzemon), adding 21 more top cuts at 0.84 conversion for very
little extra work.

**Missing:** 49 cards. `ST14 9, BT12 7, EX2 6, BT19 5, EX10 4, P 4`, tail across 12 more sets —
18 sets total, top3 44.9%. Middling coherence: better than Sakuyamon, well short of Hunters.

**Oracle:** DCGO 98.3%; one blind card (BT3-077).

**Risk:** low on mechanics — Delay 5, Rush 4, Blocker 4, Blast Digivolve 3, Retaliation 2, all
supported. The risk here is scatter: 18 sets means 18 contexts to load, and ST14 (a starter deck
dir with only 12 specs) is the largest single bucket, so a meaningful share of the work is in a
thinly-covered corner.

### 5. Sakuyamon — the biggest raw prize, the worst slice

**Worth doing:** 65 top cuts at 0.54 conversion with only 25.2% verdicted gives it the highest
raw score in the whole dataset (48.6). If the goal is purely "maximise top-cut finishes we can
simulate", this is the number-one pick.

**Missing:** 80 cards — the second-largest backlog here — spread across **23 sets** with only 35%
in the top three: `EX2 11, ST22 11, BT10 6, BT17 6, BT19 6, P 6`. There is no dominant set. This
is the explicit counterexample to era coherence: high value, poor slice.

**Oracle:** DCGO 96.4% — the lowest of the shortlist. 4 blind cards: BT14-009, BT3-077, BT5-041,
ST12-03.

**Risk:** moderate, mostly scheduling rather than mechanical. 80 cards over 23 sets is a
multi-batch campaign, not a slice, and it will not produce a visible playable-deck milestone until
most of it lands. Ranked 5th rather than 1st purely on that basis — reasonable people would rank
it 1st.

### 6. BG Imperial — good numbers, one known specific hazard

**Worth doing:** 48 top cuts at 0.51 conversion, 42 cards missing, and it is already half done
(49.4% verdicted) so it is closer to a finish line than most.

**Missing:** 42 cards across 17 sets, top3 38.1%: `BT16 7, P 5, BT12 4, BT8 4, BT14 2, BT17 2`.
Moderate scatter, no dominant set.

**Oracle:** DCGO 98.8%; one blind card (BT3-021, Veemon — a bare `<Jamming>` printing).

**Risk:** **the highest specific risk on this list.** There is a known cross-colour digivolution
defect in this archetype: off-colour `evo_costs` are missing from `cards.json`, which breaks
cross-colour digivolution, and `code/digimon-engine/tests/archetypes/bg_imperial.rs` already
codifies it (the file notes at line 517 that `card_data_from_compiled` sets
`evo_costs: Vec::new()`). That is a data/substrate problem, not a per-card scripting problem, and
it should be resolved before or alongside the card work rather than discovered during it.

### 7. Zephagamon — high prize, scatter comparable to Sakuyamon

**Worth doing:** 55 top cuts at 0.55 conversion, and only 19.8% verdicted — the third-highest raw
score (44.1).

**Missing:** 60 cards across **25 sets**, top3 35%: `EX11 8, ST18 7, EX7 6, P 5, BT1 3, BT14 3`.
The most scattered candidate on the list by set count.

**Oracle:** DCGO 97.5%; 2 blind cards (BT7-049, BT9-047).

**Risk:** scatter, as with Sakuyamon. Included because the tournament numbers are genuinely
strong; ranked last because 25 sets for 60 cards is the least coherent slice here. Its 6 EX7
cards do **not** overlap the EX7 bundle in row 3.

---

## 4. Separate tier — near-complete archetypes needing verdicts, not code

These would top a naive ranking, but the work is **exam and verdict work, not implementation**,
so they do not belong in the table above. They are listed because they are unusually cheap.

| Archetype | tc | conv | distinct | yaml | missing YAML | verd% | DCGO% | verdict-status breakdown |
|-----------|----|------|----------|------|--------------|-------|-------|--------------------------|
| Toho Braves | 28 | 0.62 | 42 | 41 | **1** | 16.7 | 100.0 | NO-VERDICT 34, IMPLEMENTED 7 |
| TS Jupitermon | 33 | 0.65 | 66 | 60 | 6 | 63.6 | 100.0 | IMPLEMENTED 41, NO-VERDICT 14, PARTIAL 4, AUDITED-OK 1 |
| XrosHeart | 17 | 0.89 | 41 | 32 | 9 | 14.6 | 100.0 | NO-VERDICT 26, IMPLEMENTED 6 |
| TS Olympos | 48 | 0.47 | 126 | 93 | 33 | 57.1 | 100.0 | IMPLEMENTED 70, NO-VERDICT 19, PARTIAL 2, AUDITED-OK 2 |

**Toho Braves is the single sharpest case in the whole corpus:** 28 top cuts, DCGO answers for
100% of its 42 cards, exactly **one** card lacks a YAML spec, and 34 of the 41 written specs have
never been verdicted at all. Nothing needs to be built; the pool needs to be examined. If the
goal this cycle is to raise trustworthy coverage rather than raw coverage, this is the cheapest
available win. XrosHeart is the same shape at smaller scale (0.89 conversion, 26 NO-VERDICT), and
its DigiXros mechanics were closed out on 2026-05-24, so the risk is low.

---

## 5. Confidence and limitations

This is a prioritisation aid. Specific reasons not to over-trust it:

- **The rank order is an opinion.** Every column is a measurement; the ordering is not. Ranking by
  coverage alone, or top-cut count alone, or without the era-coherence adjustment, materially
  reorders this list. Sakuyamon is 1st on raw score and 5th here.
- **`yaml%` is an upper bound on real coverage.** It counts file existence. Pool-wide, 20 cards
  are `PARTIAL` and 9 are `BLOCKED`, and many written specs carry no verdict at all.
- **The card union ignores copy counts and recency.** A 1-of tech card in one fringe list counts
  as much toward `distinct_cards` as a 4-of engine piece in every list, so coverage understates
  how playable an archetype actually is. Decklists span the whole scrape window with no
  format/date filter, so rotated-out builds contribute cards no current list would run.
- **`distinct_cards` is a union across all of an archetype's decklists**, not a single 50-card
  deck, so it is a superset of any one list. An effort estimate scoped to one representative
  decklist would be smaller.
- **Mechanic risk was assessed from printed keyword tokens only.** A card can be hard for reasons
  that never appear as a keyword. The keyword scan tells us nothing about effect-body complexity.
- **The `decklist` parsed field is trusted without cross-check.** If the upstream parser that
  produced it was lossy, every per-archetype card union here inherits that silently.
- **DCGO coverage is distinct-card fraction, unweighted.** `P` (238 promos) and `LM` (66) are
  large and appear in nearly every archetype, so high oracle coverage is partly carried by promos
  being well covered rather than by archetype cores.

---

## Appendix A — M1 (our-coverage) caveats, verbatim

> DECODE: 0 decklists were undecodable. All 4242 decklist entries carried a parsed `decklist` field (a JSON-encoded string array of card ids, one entry per copy). I did NOT need to decode the source_url 'count n cardid a' encoding at all, and did not cross-check the two against each other — so if the upstream parser that produced `decklist` was itself lossy, my per-archetype card unions inherit that silently.

> SET DIRS: 0 card ids failed to map to an existing set directory, and 0 ids had an unparseable shape. Observed prefixes across the whole corpus: AD1, BT1-BT25, EX1-EX12, LM, P, RB1, ST1-ST10, ST12-ST24 (no ST11 appears in any decklist). Every one of those has a matching lowercase dir under code/digimon-engine/cards/. So the 'no matching set dir' bucket is genuinely empty, not swept under a rug.

> ARCHETYPE DENOMINATOR: 432 archetypes exist; 40 of them have ZERO decklists and were dropped entirely (they contribute no cards, so coverage is undefined, not 0%). 392 had decodable lists; 151 cleared times_played >= 5 and were ranked.

> YAML != VERDICT, and this cuts both ways. Of the 803 meta-relevant cards with a YAML on disk, the validated_cards_dsl.json statuses are: IMPLEMENTED 521, AUDITED-OK 146, NO-VERDICT 114, PARTIAL 20, BLOCKED 2. So ~14% of cards I counted as 'implemented' have never been verdicted at all, and 22 more are explicitly PARTIAL or BLOCKED. My coverage_pct counts a file's EXISTENCE, not its correctness or completeness — treat it as an upper bound on real coverage. Per-archetype verdict counts were computed (field `with_verdict` in the script output) but the output schema has no slot for them; notable divergences: TS Olympos 93 yaml / 74 verdicted, Galaxy 40/36, Zephagamon 21/17, Numemon 26/23.

> era_sets IS NOT AN ERA SIGNAL for most of these rows. It is the set of every set code any card in any of the archetype's lists came from. Competitive lists splash staples and tech from the whole history of the game, so nearly every top archetype spans 20-35 sets and the field is close to uninformative. Read the `missing_sample` ids instead — those cluster much more tightly on the actual era of unimplemented work (e.g. Hudiemon -> BT22, Sakuyamon -> BT10/BT14-BT16, Machindramon -> BT11/BT12).

> THE RANK ORDER IS AN OPINION, NOT A MEASUREMENT. I ranked by top_cut_count * (1 - coverage), i.e. 'top-cut finishes we cannot currently simulate'. That multiplicative form is my choice, not something in the data — it deliberately lets a huge popular deck at 48% coverage (Hudiemon) outrank a tiny near-perfect-conversion deck at 12% coverage (Blue Ulforce, 14 top cuts from 15 entries). Rank by coverage alone, or by top_cut_count alone, and the order changes materially. Every underlying column (times_played, top_cut_count, distinct_cards, implemented, coverage_pct) is a real measurement; only the ordering is editorial.

> ARCHETYPE NAMES ARE NOT DEDUPLICATED and several rows are near-duplicates of each other, which inflates the apparent breadth of the backlog. 'Beelzemon' (71 played) and 'Beelze' (25) overlap heavily in missing cards (BT12-073/078/082/085/110, BT14-099 appear in both). Same for 'Machindramon' (20) vs 'Machindra' (28), and 'Gallantmon' (84) vs 'Dukemon' (53) are the EN/JP names of the same deck with near-identical missing lists (AD1-003, AD1-008, BT10-112, BT12-001, BT12-010, BT12-089). data/archetype_aliases.json exists to canonicalize these; I did NOT apply it. Fixing that would collapse roughly 3 pairs inside this top 25 and free slots for lower-ranked archetypes.

> THE CARD UNION IGNORES COPY COUNTS AND RECENCY. A card played as a 1-of in a single fringe list counts exactly as much toward distinct_cards as a 4-of core engine piece in every list, so coverage_pct undercounts how playable an archetype actually is. Likewise, decklists span the whole scrape window (entries dated back through 2025) with no format/date filter, so a rotated-out or pre-errata build contributes cards that no current list would run.

> POOL-WIDE CONTEXT for scale: 2917 distinct card ids appear across all decklists; 803 have a YAML (27.5%). There are 828 YAML files on disk total, so only 25 implemented cards are outside the meta corpus — implementation effort so far has been well aimed at meta cards, and the gap is breadth, not misallocation. Separately, 12 of the 708 verdict entries are for cards that appear in no meta decklist.

**Note on M1's caveats:** the "SET DIRS" and "POOL-WIDE CONTEXT" items are affected by correction
2.2(a) — M1's id-to-directory method silently skipped `cards/_examples/`, so its 803 figure should
be 809 and 6 cards it reported as missing are in fact shipped. The archetype-alias caveat is
partly wrong; see 2.2(c).

## Appendix B — M2 (DCGO oracle) caveats, verbatim

> FIELD MEANING: `implemented` and `coverage_pct` are DCGO's coverage, NOT ours. Reading them as our engine's coverage would be badly wrong -- e.g. Beelzemon reads 98.3% here but only 13.8% of its cards have a passing DSL verdict on our side.

> THE HEADLINE: the oracle is almost never the binding constraint. Across the whole times_played>=5 pool there are 2477 distinct cards; DCGO has a script for 2446 (98.7%). Every one of the top 25 archetypes is >=92.5% oracle-covered and 10 of 25 are at exactly 100%. So 'can DCGO answer for this archetype' is essentially never the reason to pick or reject an exam target -- OUR coverage is what varies.

> IDEAL EXAM TARGETS (oracle high, ours low), computed over all 151 archetypes with coverage_pct>=95 and our DSL verdict rate <40%, sorted by top_cut_count. Format: archetype (topcut) DCGO% / our-verdict%: Sakuyamon (65) 96.4/25.2 | Zephagamon (55) 97.5/19.8 | Beelzemon (45) 98.3/13.8 | Dukemon (40) 99.0/38.5 | Gallantmon (39) 98.2/29.7 | Mastemon (Tribal) (38) 98.1/32.7 | Toho Braves (28) 100.0/16.7 | Numemon (27) 96.5/25.6 | Purple Hybrid (21) 97.3/36.5 | Three Musketeers (21) 97.9/39.6 | Hunters (21) 98.5/29.2 | Beelze (21) 100.0/12.5 | Leviamon (20) 99.2/22.5 | Machindra (20) 98.8/20.0 | Diaboromon (18) 96.5/19.3. Toho Braves is the single sharpest case: DCGO 100%, our YAML files 97.6% but our DSL verdicts only 16.7% -- the specs exist and the oracle can answer for every card, so it is almost pure exam-and-verdict work with no implementation gap.

> OUR-SIDE NUMBERS for the same top 25 (two DIFFERENT things, per the task's warning -- a YAML file existing is not a verdict existing). Format: archetype -> YAML-file% / verdict%(IMPLEMENTED|AUDITED-OK in validated_cards_dsl.json): Hudiemon 47.7/45.5 | Medusamon 89.4/87.9 | Sakuyamon 27.9/25.2 | Royal Knights 88.8/84.3 | Jesmon 56.6/54.3 | Rocks 86.4/84.7 | DNA Omnimon 74.3/74.3 | Zephagamon 25.9/19.8 | TS Olympos 73.8/57.1 | BG Imperial 50.6/49.4 | Beelzemon 15.5/13.8 | Dukemon 40.6/38.5 | Gallantmon 30.6/29.7 | Millenniummon 43.4/42.5 | Mastemon 39.4/32.7 | Glowing Dawn 92.1/89.5 | TS Jupitermon 90.9/63.6 | Beelstar 58.5/49.1 | Galaxy 48.2/42.2 | TS Angels 84.2/63.2 | Toho Braves 97.6/16.7 | Magneticdra 92.5/92.5 | Numemon 30.2/25.6 | Dark Masters 45.5/40.3 | Shoes-Puppet 67.1/65.8. Basis: 822 YAML spec files under code/digimon-engine/cards/**, and 679 ids with status IMPLEMENTED or AUDITED-OK in qa/qa-reports/validated_cards_dsl.json. NOTE: I found 822 YAML files, not the 708 verdict entries the brief quoted -- those are different denominators and the file count has evidently moved since that number was taken.

> WHAT 'MISSING' MEANS AND HOW MUCH OF IT IS REAL. Across the whole times_played>=5 pool only 31 distinct cards lack a DCGO class. I audited all 31 against data/cards.json: 2 are bad ids in deck_library itself ('BT13-10', a padding typo for BT13-010 which DCGO DOES implement, 1 copy; and 'BT8-842', not in cards.json at all, 1 copy). 7 more have no printed effect text whatsoever (BT10-043 Mushroomon, BT10-047 RedVegiemon, BT10-062 Golemon, BT2-052 Hagurumon, BT2-056 Numemon, BT3-067 Tankmon, BT9-019 Crabmon) -- DCGO correctly needs no script for a vanilla card, so counting them as 'not implemented' understates the oracle. Re-scoring with vanilla cards treated as answerable moves nothing in the top 25 except Magneticdra (92.5% -> 95.0%); every other row is unchanged. That leaves 22 cards that print real text and have no DCGO class -- these are genuine oracle blind spots.

> THE 22 REAL ORACLE GAPS, with meta copy-counts, are heavily concentrated in early-set 'lock' commons that recur across many archetypes: BT3-077 Gazimon (171 copies, opponent can't gain memory except with Tamers), ST13-08 Chikurimon (167, players can't reduce play costs), BT14-009 Gotsumon (124, players can't play Digimon by effects), BT9-047 Pomumon (117, same), BT3-061 Chuumon (137, memory lock), RB1-001 Gurimon (80), ST12-03 Solarmon (51), ST10-02 Salamon (41), BT3-021 Veemon (38, Jamming), BT6-021 ModokiBetamon (36), BT5-033 Cutemon (25), BT14-069 Gazimon (12), plus BT11-013, BT12-049, BT13-066, BT3-048, BT4-001, BT5-041, BT7-049, ST16-06, ST5-03, ST6-08. Because these are splashed tech cards rather than archetype cores, they are exactly the cards an exam is most likely to trip over, and they are unanswerable by DCGO.

> A KEYWORD-ONLY CARD WITH NO SCRIPT IS GENUINELY KEYWORDLESS IN DCGO -- I checked this rather than assuming keywords are data-driven. Six of the 22 print only a bare keyword (BT3-021 Jamming; BT11-013, BT12-049, ST16-06, ST5-03, ST6-08 Blocker). CardSource.cs:2533 defines `HasBlocker` as a search for a `BlockerClass` among the card's attached cardEffects, and CardEffectFactory/KeyWordEffects/ is a helper library that card scripts CALL, not a text parser that runs automatically. So with no class attached, DCGO's Agumon ST5-03 has no Blocker at all. I did not chase whether these are stale/alt printings, so treat the six as 'DCGO answers, and its answer may itself be wrong' rather than as clean unreachables.

> DCGO IS NOT A FULL CARD POOL. 4000 DCGO ids vs 4294 entries in data/cards.json. BT26 in particular has only 5 classes (vs ~102 cards in a normal booster), so the doc's 'BT1-BT26' is literally true but BT26 is not usable as an oracle. Any exam target drawing on BT26 will be near-uniformly `unavailable`.

> ST11 IS ABSENT ENTIRELY -- no directory under CardEffect/. The claim in qa/dcgo-exams/README.md:150 ('ST1-ST24') and docs/superpowers/plans/2026-08-21-dcgo-exam-workflow.md:334 ('ST1-ST24') should be corrected to 'ST1-ST10, ST12-ST24'. No ST11 card appears in the top-25 archetypes' era_sets, so this does not affect the coverage numbers above, but it will silently produce `unavailable` verdicts if anyone exams an ST11 card.

> METHOD RISK I ACTUALLY HIT: deriving ids from FILENAMES is wrong and I nearly shipped it. BT25/Yellow/BT25_002.cs declares `class BT25_003`. Filename-derived totals were 3999 ids with a phantom BT25-002 duplicate and a false 'BT25-003 missing from DCGO' (4 meta copies). All numbers above use class-name derivation. Anyone re-deriving this set from `ls`/`find` on filenames will reproduce that error.

> `P` (238 promos) and `LM` (66) are large and appear in nearly every archetype's era_sets, so a high oracle coverage number is substantially carried by promo/limited cards being well-covered rather than by the archetype core. I did not weight coverage by copy-count -- these are DISTINCT-card fractions, so a 4-of staple and a 1-of tech card count equally.

> Distinct-card counts are the union across ALL of an archetype's decklists, not a single 50-card deck, so `distinct_cards` (88-129 for the big archetypes) is a superset of any one list and includes fringe one-ofs. An exam scoped to one representative decklist would face a smaller and probably better-covered pool.

> I wrote no repo files and changed no engine code; all scratch scripts live under the session scratchpad (dcgo_enum.py, dcgo_checks.py, m2.py, m2_miss.py, m2_vanilla2.py, m2_final.py, m2_adj.py). The DCGO submodule was read-only -- no `git submodule update --init` was run.

**Note on M2's caveats:** the 822-file figure is a distinct-**id** count, not a file count — there
are 828 files, 822 distinct ids (see 2.2(a)). M2's "679" is the `IMPLEMENTED|AUDITED-OK` subset,
not a total verdict count, which reconciles its apparent conflict with M1's 708 (see 2.2(b)).
