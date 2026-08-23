# Clause-denominator correction — measured impact on the Toho Braves exam

**Date:** 2026-08-23 · **Lane:** C (measurement only — this lane wrote no
`clause_coverage` source and no verdict store) · **Subject:** the phantom-security,
scrape-residue, and splitter-fragment corrections to `code/tools/clause_coverage/`
· **Baseline report:** `qa/qa-reports/toho-braves-exam-report.md`

## Headline

**The 44-card Toho pool denominator falls from 239 clauses to 170 — a 28.9%
over-count removed.** Every one of the 69 removed slots is either a clause that
does not exist on the printed card, or a fragment of a sentence already counted by
the clause it was cut from. No printed clause left the denominator: all 47
invalidated verdicts that pointed at real text retarget onto a surviving clause
with an **exact** text match (the old text is a literal substring of the new one in
every case).

The verdict store takes real damage — 63 of 138 entries are invalidated — but
**16 of those are `unreachable` verdicts on slots that no longer exist**, which is
the correction working as intended, not a loss.

## Method, and why these numbers are trustworthy

The two fixes landed in the same worktree, so a naive before/after cannot attribute
anything. Four variants of the package were built in the scratchpad from
`git show HEAD:` plus the working tree, and each was run over the identical card
list in its own subprocess:

| Variant | `card_sources.py` | `text_split.py` |
|---|---|---|
| `head` | HEAD | HEAD |
| `onlyA` | working tree | HEAD |
| `onlyB` | HEAD | working tree |
| `wt` | working tree | working tree |

Two harness details that materially affect the result:

- **Namespace-package resolution.** `code/tools/` has no `__init__.py`, so `tools`
  is a namespace package whose portions merge in `sys.path` order. A first attempt
  put `code/` ahead of the variant directory and silently measured the working tree
  four times (all four printed 170). The variant directory must be `sys.path[0]`.
- **DCGO root.** `default_dcgo_root()` walks up from `__file__` to find the base
  repo. From a scratchpad copy that walk fails and returns `None`, which — by the
  package's deliberate fail-safe direction — disables the DCGO check and keeps every
  phantom slot. Measured that way the security fix appears to do nothing. All runs
  therefore pin `dcgo_root` explicitly to
  `C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO`, the same path the
  in-tree default resolves. Verified in-tree beforehand: `EX12-020 -> absent`,
  `EX12-071 -> present`, `EX12-073 -> present`, matching the README's own grounding
  examples.

**Pool derivation.** The report's pool is its own results table: 44 card IDs.
Parsing them back yields 44 cards summing to exactly **239** clauses and
**37 / 12 / 46 / 144**, reproducing the published headline line for line. The `head`
variant then independently re-derives 239 from `cards.json` + `card_official.json`,
so the pool and the harness are both validated against a number neither produced.

Cross-checks on that derivation: the union of `card:` values across
`qa/dcgo-exams/*/*.yaml` is 35 cards, all but one (`ST1-08`, the ST1 selection gate,
not a pool card) inside the 44. `qa/dcgo-exams/EX12/toho_pool.json` contributes 59
distinct cards, which is a **different** set — it carries the generic ST1/ST19 deck
filler and omits 10 of the report's cards. That distinction matters; see "A
correction to the README" below.

**Verdict store.** `qa/qa-reports/dcgo_exam_verdicts.json` is being written by a live
background campaign and was **not** written by this lane. It was snapshotted to the
scratchpad at **138 entries** (`last_updated` `2026-08-23T20:02:53Z`) and every count
below is against that pin. It had grown to 141 entries by the end of this analysis,
so treat the counts as a measurement of a moving target, correct as of the pin.

## Before / after, attributed by fix

| Measurement | Clauses | Δ vs HEAD |
|---|---|---|
| **HEAD (published baseline)** | **239** | — |
| + phantom-security & scrape-residue fix only (`card_sources.py`) | 204 | −35 |
| + splitter boundary fix only (`text_split.py`) | 203 | −36 |
| **Both (working tree)** | **170** | **−69** |

−35 and −36 sum to −71, not −69. The +2 discrepancy is a real interaction, not
rounding — see "The residue fix is defeated in combination".

### What each fix removed

| Fix | Slots removed | Renumbers? |
|---|---|---|
| Phantom `#security#0` slots (DCGO negative oracle) | **27** | No — sole slot in its zone |
| `\|applinkdp =` MediaWiki residue (`#inherited#0`) | **6** | No — sole slot in its zone |
| `Inherited Effect` field label (`#effect#0`) | **0 in combination** (2 in isolation) | Would renumber; see below |
| Splitter fragments merged into their parent sentence | **36** | Yes — 27 zones shrink |
| **Total** | **69** | |

Zone and source totals move accordingly:

| | HEAD | AFTER |
|---|---|---|
| effect / inherited / security | 175 / 33 / 31 | 143 / 23 / **4** |
| bundle / cards_json / image-required | 53 / 155 / **31** | 45 / 121 / **4** |

**`image-required` falls 31 → 4 (87% of it was fiction).** That number is the
extractor's own honesty measure, and it was overwhelmingly noise.

The 27 phantom-security cards: `EX12-002`, `EX12-004`, `EX12-006`, `EX12-009`,
`EX12-011`, `EX12-012`, `EX12-015`, `EX12-019`, `EX12-020`, `EX12-022`, `EX12-025`,
`EX12-026`, `EX12-029`, `EX12-031`, `EX12-034`, `EX12-036`, `EX12-039`, `EX12-043`,
`EX12-045`, `EX12-046`, `EX12-047`, `EX12-056`, `EX12-057`, `EX12-061`, `EX12-062`,
`EX12-063`, `EX12-076`.

The 4 survivors — `EX12-070#security#0`, `EX12-071#security#0`,
`EX12-074#security#0`, `EX12-075#security#0` — are an independent corroboration:
they are precisely the four cards for which a hand-authored
`qa/dcgo-exams/EX12/<CARD>-security0.yaml` scenario exists. A human authored a
security scenario for exactly those four and for none of the 27, having read the
card faces. The DCGO oracle and the scenario author agree without having consulted
each other.

### The named defects, verified fixed

| Clause | HEAD text | AFTER |
|---|---|---|
| `ST1-15#effect#1` | `"Activate this card's"` | `"Activate this card's [Main] effect."` |
| `ST1-15#effect#2` | `"effect."` | id no longer exists (merged) |
| `BT8-097#effect#2` | `"Activate this card's"` | `"Activate this card's [Main] effects."` |
| `BT8-097#effect#3` | `"effects."` | id no longer exists (merged) |
| `EX12-065#effect#5` | `"."` | id no longer exists (merged) |
| `EX12-076#inherited#0` | `"\|applinkdp ="` | id no longer exists (dropped) |

### The residue fix is defeated in combination — a residual defect

`EX12-004#effect#0` is `"Inherited Effect"` at HEAD. Under the residue fix alone it
is dropped. Under **both** fixes it reads:

```
"Inherited Effect [Your Turn] This Digimon with the [TB] trait gains ＜Execute＞ (At the end of yo…"
```

The boundary rule now glues the following `[Your Turn]` marker onto the label, so the
residue blocklist — an exact-match list, deliberately — no longer matches, and the
clause survives. Worse, the marker it absorbed no longer tags its own clause: that
clause's `timings` went from `['Your Turn']` to `[]`. Same on `EX12-002`
(`['Your Turn', 'Once Per Turn']` → `['Once Per Turn']` on the sibling).

This is exactly the case the README already anticipates under *"What the boundary
rule still cannot see"*, and its prescription is right: **strip the label from the raw
field before `split_clauses`, not after.** Until that lands, 2 slots in this pool
(3 pool-wide, per Lane A: `EX12-002`, `EX12-003`, `EX12-004`) remain non-card-text and
carry a stolen timing. This is the one place the combined change is *worse* than
either fix alone, and it should not be left implicit.

### Per-card

| Card | Name | HEAD | +sec/residue | +splitter | AFTER | Δ |
|---|---|---|---|---|---|---|
| BT11-089 | Akiho Rindou | 4 | 4 | 3 | 3 | −1 |
| BT20-037 | Chaosmon: Valdur Arm | 5 | 5 | 4 | 4 | −1 |
| BT8-084 | Kimeramon | 3 | 3 | 3 | 3 | |
| BT8-097 | Crimson Blaze | 4 | 4 | 3 | 3 | −1 |
| EX1-066 | Analog Youth | 3 | 3 | 3 | 3 | |
| EX12-002 | Mococomon | 4 | 2 | 4 | 3 | −1 |
| EX12-004 | Onibimon | 5 | 3 | 3 | 2 | −3 |
| EX12-006 | Kakamon | 5 | 4 | 4 | 3 | −2 |
| EX12-009 | Wankomon | 4 | 3 | 4 | 3 | −1 |
| EX12-011 | Seasarmon | 5 | 4 | 5 | 4 | −1 |
| EX12-012 | Apemon | 6 | 5 | 5 | 4 | −2 |
| EX12-015 | Gokuumon | 7 | 6 | 5 | 4 | −3 |
| EX12-019 | Nezhamon | 11 | 9 | 10 | 8 | −3 |
| EX12-020 | Gasamon | 5 | 4 | 4 | 3 | −2 |
| EX12-022 | Kamemon | 5 | 4 | 4 | 3 | −2 |
| EX12-025 | Gawappamon | 6 | 5 | 5 | 4 | −2 |
| EX12-026 | Shellmon | 6 | 5 | 5 | 4 | −2 |
| EX12-029 | Sagomon | 7 | 6 | 5 | 4 | −3 |
| EX12-031 | MarineBullmon | 5 | 4 | 5 | 4 | −1 |
| EX12-034 | Erlangmon | 7 | 5 | 6 | 4 | −3 |
| EX12-036 | Ryugumon | 9 | 7 | 8 | 6 | −3 |
| EX12-039 | Takinmon | 4 | 3 | 4 | 3 | −1 |
| EX12-043 | Hakubamon | 4 | 3 | 4 | 3 | −1 |
| EX12-045 | Sanzomon | 6 | 5 | 5 | 4 | −2 |
| EX12-046 | Shishimamon | 6 | 5 | 5 | 4 | −2 |
| EX12-047 | Amaterasumon | 8 | 6 | 8 | 6 | −2 |
| EX12-048 | SeitenGokuumon | 8 | 8 | 8 | 8 | |
| EX12-056 | Cho-Hakkaimon | 8 | 7 | 6 | 5 | −3 |
| EX12-057 | Takutoumon | 8 | 6 | 5 | 3 | −5 |
| EX12-061 | Hanimon | 6 | 5 | 5 | 4 | −2 |
| EX12-062 | Kokeshimon | 5 | 4 | 5 | 4 | −1 |
| EX12-063 | Karakurumon | 5 | 4 | 5 | 4 | −1 |
| EX12-065 | Kaguyamon | 7 | 7 | 5 | 5 | −2 |
| EX12-070 | Sanmyojin Arrival | 6 | 6 | 4 | 4 | −2 |
| EX12-071 | Saneiketsu Invitation | 6 | 6 | 4 | 4 | −2 |
| EX12-074 | Genshi Continent & Ashin | 4 | 4 | 4 | 4 | |
| EX12-075 | Kunlun's Imperial Decree | 4 | 4 | 4 | 4 | |
| EX12-076 | Susanoomon | 9 | 7 | 8 | 6 | −3 |
| EX4-074 | ShineGreymon: Ruin Mode | 4 | 4 | 3 | 3 | −1 |
| P-130 | Lui Ohwada | 3 | 3 | 3 | 3 | |
| ST1-12 | Tai Kamiya | 2 | 2 | 2 | 2 | |
| ST1-15 | Giga Destroyer | 3 | 3 | 2 | 2 | −1 |
| ST16-14 | Matt Ishida | 3 | 3 | 3 | 3 | |
| ST19-14 | Arisa Kinosaki | 4 | 4 | 3 | 3 | −1 |
| **TOTAL** | **44 cards** | **239** | **204** | **203** | **170** | **−69** |

Nine cards are untouched. `EX12-057` Takutoumon loses the most (8 → 3).

## Damage to the verdict store

Against the 138-entry pin, evaluated the way `exam_binding` evaluates it — does the
`clause_id` still exist, and does the stored `text_sha256` still match:

| | Count | confirmed | diverged | unreachable |
|---|---|---|---|---|
| **still valid** | **75** | 43 | 9 | 23 |
| **id vanished** | **34** | 14 | 0 | 20 |
| **text drifted** | **29** | 19 | 7 | 3 |

63 entries invalidated. Per the store's design a drifted entry degrades to
`unmeasured`, never to a false pass, so nothing is mis-pointed — but the measured
work behind it has to be re-established.

### Split the 63 by what actually happened

- **16 are deletions of slots that never existed** — every one an `unreachable`
  verdict, i.e. the store's record of "we could never measure this". Losing them
  costs nothing; removing them *is* the correction: 13 phantom `#security#0`
  (`EX12-004`, `-009`, `-011`, `-020`, `-026`, `-031`, `-036`, `-046`, `-047`,
  `-061`, `-062`, `-063`, `-076`) and 3 `\|applinkdp =` `#inherited#0` (`EX12-036`,
  `EX12-047`, `EX12-076`).
- **47 retarget exactly.** For all 47 the old clause text is a literal substring of a
  surviving clause's text (similarity 1.00). These are recoverable.

### The 33 confirmed verdicts at risk

All 33 retarget exactly, onto **23 distinct** surviving clauses.

| Lost verdict | Cause | Retargets to | Scenario |
|---|---|---|---|
| `BT11-089#effect#3` | id vanished | `BT11-089#effect#2` | `qa/dcgo-exams/EX12/BT11-089-effect3.yaml` |
| `BT8-097#effect#2` | text drifted | `BT8-097#effect#2` *(same id)* | `qa/dcgo-exams/EX12/BT8-097-effect2.yaml` |
| `EX12-004#effect#2` | id vanished | `EX12-004#effect#0` | `qa/dcgo-exams/EX12/EX12-004-effect2.yaml` |
| `EX12-006#effect#0` | text drifted | `EX12-006#effect#0` *(same id)* | `qa/dcgo-exams/EX12/EX12-006-effect0.yaml` |
| `EX12-006#effect#1` | text drifted | `EX12-006#effect#0` | `qa/dcgo-exams/EX12/EX12-006-effect1.yaml` |
| `EX12-006#effect#2` | id vanished | `EX12-006#effect#1` | `qa/dcgo-exams/EX12/EX12-006-effect2.yaml` |
| `EX12-012#effect#1` | text drifted | `EX12-012#effect#1` *(same id)* | `qa/dcgo-exams/EX12/EX12-012-effect1.yaml` |
| `EX12-012#effect#2` | text drifted | `EX12-012#effect#1` | `qa/dcgo-exams/EX12/EX12-012-effect2.yaml` |
| `EX12-012#effect#3` | id vanished | `EX12-012#effect#2` | `qa/dcgo-exams/EX12/EX12-012-effect3.yaml` |
| `EX12-015#effect#1` | text drifted | `EX12-015#effect#1` *(same id)* | `qa/dcgo-exams/EX12/EX12-015-effect1.yaml` |
| `EX12-015#effect#4` | id vanished | `EX12-015#effect#2` | `qa/dcgo-exams/EX12/EX12-015-effect4.yaml` |
| `EX12-019#effect#8` | id vanished | `EX12-019#effect#7` | `qa/dcgo-exams/EX12/EX12-019-effect8.yaml` |
| `EX12-020#inherited#0` | text drifted | `EX12-020#inherited#0` *(same id)* | `qa/dcgo-exams/EX12/EX12-020-inherited0.yaml` |
| `EX12-020#inherited#1` | id vanished | `EX12-020#inherited#0` | `qa/dcgo-exams/EX12/EX12-020-inherited1.yaml` |
| `EX12-022#inherited#0` | text drifted | `EX12-022#inherited#0` *(same id)* | `qa/dcgo-exams/EX12/EX12-022-inherited0.yaml` |
| `EX12-022#inherited#1` | id vanished | `EX12-022#inherited#0` | `qa/dcgo-exams/EX12/EX12-022-inherited1.yaml` |
| `EX12-025#inherited#0` | text drifted | `EX12-025#inherited#0` *(same id)* | `qa/dcgo-exams/EX12/EX12-025-inherited0.yaml` |
| `EX12-025#inherited#1` | id vanished | `EX12-025#inherited#0` | `qa/dcgo-exams/EX12/EX12-025-inherited1.yaml` |
| `EX12-026#inherited#0` | text drifted | `EX12-026#inherited#0` *(same id)* | `qa/dcgo-exams/EX12/EX12-026-inherited0.yaml` |
| `EX12-026#inherited#1` | id vanished | `EX12-026#inherited#0` | `qa/dcgo-exams/EX12/EX12-026-inherited1.yaml` |
| `EX12-036#effect#4` | text drifted | `EX12-036#effect#4` *(same id)* | `qa/dcgo-exams/EX12/EX12-036-effect4.yaml` |
| `EX12-036#effect#5` | text drifted | `EX12-036#effect#4` | `qa/dcgo-exams/EX12/EX12-036-effect5.yaml` |
| `EX12-036#effect#6` | id vanished | `EX12-036#effect#5` | `qa/dcgo-exams/EX12/EX12-036-effect6.yaml` |
| `EX12-046#effect#0` | text drifted | `EX12-046#effect#0` *(same id)* | `qa/dcgo-exams/EX12/EX12-046-effect0.yaml` |
| `EX12-046#effect#1` | text drifted | `EX12-046#effect#0` | `qa/dcgo-exams/EX12/EX12-046-effect1.yaml` |
| `EX12-046#effect#3` | id vanished | `EX12-046#effect#2` | `qa/dcgo-exams/EX12/EX12-046-effect3.yaml` |
| `EX12-061#effect#0` | text drifted | `EX12-061#effect#0` *(same id)* | `qa/dcgo-exams/EX12/EX12-061-effect0.yaml` |
| `EX12-061#effect#1` | text drifted | `EX12-061#effect#0` | `qa/dcgo-exams/EX12/EX12-061-effect1.yaml` |
| `EX12-061#effect#2` | id vanished | `EX12-061#effect#1` | `qa/dcgo-exams/EX12/EX12-061-effect2.yaml` |
| `EX12-065#effect#4` | text drifted | `EX12-065#effect#3` | `qa/dcgo-exams/EX12/EX12-065-effect4.yaml` |
| `EX12-070#effect#1` | text drifted | `EX12-070#effect#1` *(same id)* | `qa/dcgo-exams/EX12/EX12-070-effect1.yaml` |
| `EX12-070#effect#2` | text drifted | `EX12-070#effect#1` | `qa/dcgo-exams/EX12/EX12-070-effect2.yaml` |
| `EX12-070#effect#3` | id vanished | `EX12-070#effect#2` | `qa/dcgo-exams/EX12/EX12-070-effect3.yaml` |

### 10 merge collisions — two confirmations, now one clause

These pairs were separately confirmed at HEAD and are one clause after the fix.
Nothing is lost; the surviving clause is confirmed by two independent scenarios
instead of one, which is *stronger* evidence than before. It does mean the confirmed
count cannot simply be carried over.

`EX12-006#effect#0`, `EX12-012#effect#1`, `EX12-020#inherited#0`,
`EX12-022#inherited#0`, `EX12-025#inherited#0`, `EX12-026#inherited#0`,
`EX12-036#effect#4`, `EX12-046#effect#0`, `EX12-061#effect#0`, `EX12-070#effect#1` —
each absorbing the clause immediately after it.

### 2 conflicting merges — triage these by hand

Two surviving clauses inherit **both** a `confirmed` and an `unreachable` verdict,
because a reachable clause and an unreachable one merged:

- `EX12-004#effect#0` ← `confirmed` from `EX12-004#effect#2`, `unreachable` from
  `EX12-004#effect#0` and `#effect#1`.
- `EX12-070#effect#2` ← `confirmed` from `EX12-070#effect#3`, `unreachable` from
  `EX12-070#effect#4`.

A merged clause is reachable if any part of it is, so `confirmed` should win — but
that is a judgement about the merged clause, and it should be recorded by re-running,
not by assertion.

### 21 scenario files whose `clause:` field no longer resolves

These need a one-line `clause:` retarget before their next run, or they will bind to
nothing. The mapping is mechanical (each retarget is an exact text match):

`BT20-037-effect4` → `BT20-037#effect#3` · `BT11-089-effect3` → `#effect#2` ·
`EX12-004-effect1` / `-effect2` → `EX12-004#effect#0` · `EX12-006-effect2` →
`#effect#1` · `EX12-012-effect3` → `#effect#2` · `EX12-015-effect3` → `#effect#1` ·
`EX12-015-effect4` → `#effect#2` · `EX12-019-effect8` → `#effect#7` ·
`EX12-020-inherited1` / `EX12-022-inherited1` / `EX12-025-inherited1` /
`EX12-026-inherited1` → `#inherited#0` · `EX12-036-effect6` → `#effect#5` ·
`EX12-046-effect3` → `#effect#2` · `EX12-061-effect2` → `#effect#1` ·
`EX12-065-effect5` → `#effect#0` · `EX12-065-effect6` → `#effect#4` ·
`EX12-070-effect3` / `-effect4` → `EX12-070#effect#2` · `ST19-14-effect3` →
`ST19-14#effect#2`.

Note `EX12-004-effect1` and `-effect2` now both target `EX12-004#effect#0`, as do
`EX12-070-effect3` and `-effect4` on `#effect#2` — the collisions above, seen from the
scenario side.

## Re-measurement plan

**Nothing needs re-deriving from the card face, and nothing needs Unity.** The 47
retargetable entries — 33 of them `confirmed` — all point at text that still exists
verbatim.

The decisive fact: **a scenario's scripted line is unchanged by this correction.** The
deck, the seed, the actor sequence, and the recorded oracle state are all untouched —
only the `clause:` *label* moves. So the sidecar a scenario already produced is still
the correct oracle for it.

`docs/DCGO_EXAM.md` makes this explicit: `exam --sidecar` needs Unity *"(to produce the
sidecar)"* only; replaying against a preserved sidecar is a local, Unity-free diff.

```bash
cargo run -p dcgo-harness -- --root "$ROOT" exam \
    --scenario qa/dcgo-exams/EX12/<SCENARIO>.yaml \
    --sidecar <recording>.state.jsonl --cards-json data/cards.json
```

**Sidecar inventory (verified on disk):**
`C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings` holds **171
`.state.jsonl` sidecars** paired with 171 `.jsonl` recordings (342 files). All 33
at-risk confirmed verdicts name a scenario under `qa/dcgo-exams/`. The
scenario→sidecar mapping is not recorded inside the recordings themselves (a
`game_start` row carries `game_id` and the post-shuffle deck, not a scenario path), so
it must come from the campaign's own job bookkeeping — that association should be
captured before the campaign's scratch state is cleaned up.

**Ordered plan:**

1. **Free — take no action on 16.** The `unreachable` verdicts on deleted phantom
   slots need no recovery. Drop them from the store when it is next rewritten.
2. **Cheap — retarget the 21 scenario `clause:` fields** using the mapping above. One
   line per file, mechanical, no engine run.
3. **Cheap — re-run the 47 retargetable verdicts against their preserved sidecars.**
   No Unity. `docs/DCGO_EXAM.md` quotes ~40 s per scenario for a sidecar diff, so the
   whole set is well under an hour of wall time, versus a full oracle re-run which
   would need Unity in Play for every line.
4. **By hand — adjudicate the 2 conflicting merges** (`EX12-004#effect#0`,
   `EX12-070#effect#2`) rather than letting a re-run pick a winner by ordering.
5. **Only then** re-issue the headline. Do not carry the old `37 confirmed` forward:
   10 merge collisions mean the confirmed *clause* count and the confirmed *verdict*
   count are no longer the same number.

**Projected corrected headline, retarget-only, no re-running** — over the 170-clause
denominator, against the 138-entry pin:

> **170 clauses: 66 confirmed · 16 diverged · 28 unreachable · 62 unmeasured.**

Coverage of the denominator rises from 95/239 (39.7%) to 108/170 (63.5%) — **almost
entirely because the denominator stopped counting fictions**, not because more was
measured. That is the whole point: the earlier figure was not conservative, it was
wrong. Treat this projection as an estimate to be replaced by step 3's real output,
and note the campaign was still appending verdicts (138 → 141) while this was measured.

## A correction to the README's own numbers

`code/tools/clause_coverage/README.md` states:

> Measured on the 44-card Toho pool, 20 of 24 `image-required` security slots were
> phantoms

Both numbers are correct measurements — **of the wrong pool.** 24 → 4 (20 dropped) is
what the 59 distinct cards in `qa/dcgo-exams/EX12/toho_pool.json` produce. The 44-card
pool that the exam report actually publishes gives **31 → 4, 27 dropped**. The two
pools are not the same set: `toho_pool.json` carries generic ST1/ST19 deck filler and
omits 10 cards the report scores (`BT8-084`, `EX12-029`, `EX12-034`, `EX12-039`,
`EX12-045`, `EX12-048`, `EX12-056`, `EX12-057`, `EX12-076`, `EX4-074`).

The README sentence should read "on the 59-card `toho_pool.json` deck pool", or be
restated as 27-of-31 for the 44-card report pool. The pool-wide figure it quotes (64
dropped across 4294 cards) is unaffected.

## Verification

```
PYTHONPATH=code python -m pytest code/tests/tools/ -k clause_coverage -q
  93 passed, 139 deselected

PYTHONPATH=code python -m pytest code/tests/tools/ -q
  232 passed
```

No collateral in the wider tools suite.

**Artifacts** (scratchpad, not committed): `lanec_pool.py`, `lanec_reportpool.json`,
`lanec_extract.py`, `lanec_{head,onlyA,onlyB,wt}.json`, `lanec_damage.json`,
`lanec_retarget.json`, `lanec_badscenarios.json`, `verdicts_snapshot.json` (the
138-entry pin), `var_{head,onlyA,onlyB,wt}/`.
