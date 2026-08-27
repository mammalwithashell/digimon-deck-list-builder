# `clause_coverage` — the (card, clause) denominator + DCGO coverage report

First two pieces of a card-validation workflow that uses DCGO recordings as a
parity oracle. "Card X works" is not checkable — a card has several
independent clauses (on-play, when-attacking, on-deletion, inherited,
security, each printed keyword, conditional variants) that fail separately.
The tracked unit is **(card, clause)**, not (card).

## Entry points

```bash
# extract: decklist / card-ID list -> the (card, clause) denominator
PYTHONPATH=code python -m tools.clause_coverage.extract \
    --decklist qa/dcgo-harness/vb_pool.json --out clauses.json

PYTHONPATH=code python -m tools.clause_coverage.extract \
    --card-ids EX12-073 EX12-018

# coverage: clause list + a directory of DCGO .jsonl recordings -> what fired
PYTHONPATH=code python -m tools.clause_coverage.coverage \
    --clauses clauses.json --recordings-dir D:\dcgo-build\vb-corpus3 --out coverage.json
```

Both commands print a JSON document (to `--out`, or stdout) plus a short
human-readable summary. When `--out` is given the summary goes to stdout;
when JSON itself goes to stdout (no `--out`), the summary goes to stderr so
it never corrupts piped JSON.

No new dependencies — standard library only (`argparse`, `json`, `re`,
`dataclasses`, `pathlib`, `collections.Counter`), matching the rest of
`code/tools/`.

## Source priority — read this before trusting an extracted clause

Per `CLAUDE.md`'s "Source priority for card / keyword / rules questions" and
"Printed card data" sections, this extractor resolves each card's printed
text in this order:

1. **`data/card_bundles/<ID>.md`** (official Bandai DB `world.digimoncard.com`).
   Parsed from its machine-readable twin, **`data/card_official.json`**'s
   `text_sections` list — same underlying scrape, both written by
   `code/tools/build_card_bundles.py`; the JSON is far cheaper to parse
   reliably than the markdown, so the extractor reads that instead of
   regexing the `.md` file. `card_bundles/<ID>.md` still exists per-card as
   the human-readable rendering and is what a human/vision agent should open
   to fill in an image-cache entry (below).
2. **`data/cards.json` + `data/card_overrides.json`** (overrides win,
   per-field — a clause's `source` is tagged `"card_overrides"` only for
   fields the override patch actually touches).
3. **DCGO's C# card script** — **security zone only**, and only as a
   *negative* oracle: it can prove a card has NO security clause, never
   supply the text of one. See "Why security gets its own fallback" below.
4. **`image-required`** — **security zone only**. See "Why security gets its
   own fallback" below.

### Why security gets its own fallback

Measured against the real EX12 set (verified, not re-derived here):

- `security_effect_description_eng` is populated for exactly 3 of ~4300
  cards pool-wide, and those 3 are synthetic `TEST` cards. It is **never**
  ingested for real cards. Confirmed against the card image: `EX12-073`
  Giant Meat plainly prints `[Security] Place this card in the battle
  area.` and `cards.json` reports 0 characters for that field.
- `card_overrides.json` does not backfill this (for `EX12-073` it patches
  only `type_eng`).

**Absence of text in a lossy source is not evidence of absence of a
clause.** So: when a card has no bundle, the extractor emits its security
clause slot with `"source": "image-required"` and an `image_path` pointing
at the card image, rather than silently concluding "no security effect".
The `image_required_count` printed by `extract` is deliberately the
headline number — it is the extractor's own honesty measure: how much of
the denominator it could NOT confirm from text sources alone.

#### What changed: DCGO as a second source (the principle survives intact)

The rule above is right, and it is unchanged. What it was missing is that
**a lossy source is not the only source available.** The reasoning "we have
no text, therefore we cannot know" silently assumed text was the only
evidence there is — but DCGO's C# script for a card is independent evidence
about whether the card *has* a security clause at all, even though it can
never tell us what that clause says.

Every DCGO card with a security effect declares it under
`EffectTiming.SecuritySkill`. Verified across the whole checkout: the
substring `SecuritySkill` occurs **only** as `EffectTiming.SecuritySkill`
(911 occurrences, zero other uses), and none of the neighbouring
security-ish timings (`OnAddSecurity`, `OnLoseSecurity`, `OnSecurityCheck`,
`OnDetermineDoSecurityCheck`, `OnDiscardSecurity`,
`OnFaceUpSecurityIncreased`) contain it — so a plain substring test is
exact.

The rule the extractor applies, in the security zone only, when no text
source produced a clause:

| DCGO state | Meaning | Extractor |
|---|---|---|
| script exists, no `SecuritySkill` | **positive evidence of absence** | emit **no** security clause |
| script exists, has `SecuritySkill` | card HAS a clause; text still unknown | emit the `image-required` slot |
| no script for this card | genuinely unknown | emit the `image-required` slot |
| no usable DCGO checkout | genuinely unknown | emit the `image-required` slot |

Grounding, three independent sources agreeing per card: `EX12-020` Gasamon's
card face prints no `[Security]` box, its DCGO script
(`Assets/Scripts/CardEffect/EX12/Blue/EX12_020.cs`) contains zero
`EffectTiming.SecuritySkill`, and `cards.json`'s security field is empty.
`EX12-061` Hanimon was re-verified the same way against its card image.
Contrast `EX12-071` Saneiketsu Invitation, which plainly prints a Security
Effect box **and** has a `SecuritySkill` block — its `image-required` slot
is correctly retained.

Why this matters more than it sounds: the denominator is the exam's honesty
mechanism ("N clauses: X confirmed, Y unmeasured"). A phantom slot is not a
conservative over-count — it is a clause that can *never* be measured,
because it does not exist, so it reads forever as permanently unreached and
drags every coverage figure down by a fixed, meaningless amount. Measured
on the 44-card Toho pool, 20 of 24 `image-required` security slots were
phantoms; pool-wide (4294 cards) 64 of them disappear.

**Failure direction is deliberate.** Every DCGO failure mode — no checkout
at all, a worktree's empty `./DCGO` placeholder (CLAUDE.md rule 29), no
script for the card, an unreadable file — answers `unknown`, which keeps
the `image-required` slot. Absent DCGO can only ever reproduce today's
behaviour; it can never silently delete a slot.

**Configuring the root.** `extract_card_clauses(..., dcgo_root=...)` takes
the checkout to consult. Omit it and `default_dcgo_root()` resolves the
**base-repo** `DCGO/` (following a worktree's `.git` pointer file, per rule
29); pass `dcgo_root=None` to skip DCGO entirely. The environment variable
`DIGIMON_DCGO_ROOT` overrides the default, and setting it to the empty
string disables the consultation — mirroring how `DIGIMON_CARD_IMAGE_DIR`
overrides the image directory.

### Ingestion artifacts are filtered before they become clauses

Some values in `cards.json` are scrape residue rather than printed card
text, and splitting them produces clauses that are not clauses:

- `inherited_effect_description_eng` is literally `|applinkdp =` — a
  MediaWiki template key — on **33** cards pool-wide (e.g. `EX12-076`,
  `EX12-019`).
- `effect_description_eng` is prefixed with the literal box label
  `Inherited Effect` on **10** cards (`BT25-001..006`, `EX12-001..004`),
  which the splitter emits as a content-free leading clause.

`card_sources.is_ingestion_artifact` drops these. It is a **hard-coded
exact-match blocklist, deliberately not a heuristic**: a clever "looks like
junk" rule risks eating real printed text, which corrupts the denominator in
the far more dangerous direction — a real clause silently stops being
tracked. Consequences of exact matching, all intentional:

- `EX12-001`'s whole printed text is one clause that merely *begins* with
  `Inherited Effect`; it survives untouched. A prefix rule would have
  truncated it.
- `"Effect"` alone is **not** blocklisted. It is an ordinary English word
  that legitimately ends real clause fragments (`ST1-15` splits a sentence
  into one), and eating those is exactly the silent loss this filter exists
  to avoid.
- Empty text is dropped **only** when the span also carries no timing and
  no keyword. A keyword clause legitimately has empty text — all its
  content is in `keyword` (`EX12-065`'s `＜Fortitude＞`) — and so does a
  marker-only timing clause; a blanket empty-text rule would delete both.

**A dropped clause does not consume an index**, so its surviving siblings in
the same zone renumber down. Zones are numbered independently, so dropping
the only clause in a zone (every `#security#0`, and every `|applinkdp =`
`#inherited#0`) renumbers nothing at all.

When a card **does** have a bundle, a missing `"Security"` section in that
bundle is treated as a **confirmed** absence (the official DB is
authoritative, not lossy) — no image-required slot is added in that case.
This is the one place bundle-absence and cards.json-absence are handled
differently, deliberately: the former is a trustworthy negative, the latter
is not.

### The "inherited" zone is skipped for Tamer/Option/Dual-kind cards

`cards.json`'s `inherited_effect_description_eng` field (`source_effect` in
the raw API) is legitimate for Digimon and Digi-Egg cards — "Inherited
Effect" is a real Digimon TCG concept that only exists for cards that can
become digivolution material. For Tamer/Option/Dual cards it structurally
doesn't apply, and the field is observed to hold something else entirely:

- `EX12-073` (Option): `inherited_effect_description_eng` =
  `"[Security] Place this card in the battle area."` — the card's actual
  security-face text, sitting under the wrong key.
- `EX12-066` (Tamer): same pattern — `"[Security] Play this card without
  paying the cost."`
- `EX12-069` (Option): same pattern.

This is a real, observed API-ingestion quirk, not documented behavior —
trusting it would silently produce a **correct-looking but unverified**
security clause, exactly the failure mode this tool exists to prevent (a
denominator that validates itself). So the extractor deliberately does
**not** read this field for Tamer/Option/Dual kinds. The consequence:
`EX12-073`, `EX12-066`, and `EX12-069` all get `image-required` security
slots even though the text is arguably sitting right there in a different
field — a documented, conservative tradeoff, not an oversight. (DCGO
independently agrees all three *have* a security clause: each script carries
a `SecuritySkill` block, so the new DCGO check keeps their slots rather than
dropping them.)

## Clause splitting rules

See `text_split.py`'s module docstring for the authoritative rule text and
implementation. Summary:

- Each bracketed **timing marker** (`[On Play]`, `[Main]`, `[Security]`,
  `[When Digivolving]`, ...) starts a clause. A run of markers separated
  only by whitespace (`[When Digivolving] [When Attacking] [Once Per
  Turn]`) is **one compound clause** carrying all three timings, not three
  clauses.
- A handful of marker names print via angle brackets in the raw text
  (`＜Use Req. (...)＞`, `＜Delay＞`, `＜Arts Digivolve＞`, and the other
  alt-digivolve-mechanism markers) despite being timing markers
  conceptually — `TIMING_MARKER_NAMES` special-cases them so both spellings
  behave identically.
- Each **angle-bracket keyword** (`<Progress>`, `<Security A. +1>`, ...) is
  its own clause — open-ended: any angle-bracket token that isn't a
  recognized timing-marker name becomes a keyword clause, so new keywords
  in future sets are picked up automatically without a code change.
- **Both marker families only open a clause at a clause boundary.** A
  marker printed *inside* a sentence is a reference to an ability, not the
  start of one, and stays inline. See "Positional boundary rule" below.
- Square-bracket tokens that aren't recognized timing markers (`[VB]`,
  `[Gammamon]`, `[NSp]`, ...) are trait/card-name references, not clause
  boundaries — left as ordinary body text.
- `TIMING_MARKER_NAMES` is the task's given list plus **one** directly-
  observed addition: `"Start of Your Main Phase"` (real printed text on
  `EX12-021`, not in the task's illustrative list but structurally the same
  kind of marker as `"Start of Your Turn"`).

### Positional boundary rule

*(This section replaces the former "Known limitation: keyword splitting is
unconditional, not positional". That rule was written for the standalone
keyword-grant case — `EX12-018`'s `＜Progress＞ ＜Piercing＞ ＜Security A. +1＞`
prefix — which it handles correctly. Applied to a marker printed **inside**
a sentence it cut that one sentence into unmeasurable pieces, which
inflated the denominator with clauses no scenario can ever exercise:
`ST1-15`'s printed `"[Security] Activate this card's [Main] effect."`
became `"Activate this card's"` + `"effect."`, `BT8-097` the same, and
`EX12-065#effect#5`'s entire body was `"."`. That is the honesty problem
this package exists to avoid, so the rule is now positional.)*

A marker opens a new clause only at a **clause boundary**. Given the body
text accumulated since the last accepted marker (`_is_clause_boundary` in
`text_split.py`), the marker opens a clause when that body:

- is **empty** — marker runs (`＜Progress＞ ＜Piercing＞`,
  `[When Digivolving] [When Attacking]`) and field-leading markers; or
- ends at a **line break**; or
- ends on a **clause-ending character** — `. ! ? 。 ！ ？`, a closing
  `) ） ]` (the end of a reminder-text span: `"...(Specified cards let you
  ignore color requirements.) [Main] ..."`), a closing `}` (the
  `{Hand}` / `{Security}` zone prefix printed immediately before its
  marker), or a quote character (a granting sentence opens a quote around
  the granted ability — `1 of your Digimon gains "[On Deletion] ..."` — and
  that ability keeps its own clause, because it is independently testable).

Otherwise the marker is mid-sentence and stays as ordinary body text of
the clause in progress. Consequences, in both directions:

- `"[Security] Activate this card's [Main] effect."` is **one** clause.
- `"All of your [Puppet] or [TB] trait Digimon gain ＜Blocker＞ and
  ＜Retaliation＞."` (`EX12-065`) is **one** clause; the granted keywords
  are that clause's effect, not two clauses of their own. Likewise
  `EX12-019`'s `＜Collision＞` reminder text mentions `＜Blocker＞`, which no
  longer manufactures a phantom Blocker clause on that card.
- `"[On Play] By trashing 1 card ..., ＜Draw 2＞ (Draw 2 cards from your
  deck.)"` (`EX12-005`) is **one** clause — the keyword is this clause's
  action. This is the case the old README told you to "read together"; it
  is now actually together, and it matches DCGO's `effect_activation`
  rendering, which prints the whole sentence.
- `EX12-018`'s `＜Progress＞ ＜Piercing＞ ＜Security A. +1＞` prefix still
  splits into **three** keyword clauses — each sits at a boundary.

**One exception, by marker name.** The digivolution-condition markers
(`[Digivolve]`, `[DNA Digivolve]`, `[Use Req.]`, `[Assembly]`,
`[DigiXros]`, `[Arts Digivolve]`, `[Blast Digivolve]`, `[Burst Digivolve]`
— `_ALWAYS_BOUNDARY_MARKER_NAMES`, mirroring
`activation_match.COST_LINE_MARKERS`) always open a clause. They head
*structured cost lines*, and consecutive lines run together with no
sentence punctuation (`"[Digivolve] Lv.6 w/[CS] trait: Cost 5
[DNA Digivolve] Lv.5 [Justimon] + ..."` — 30 cards pool-wide). These names
are never printed as inline nouns, so the exception is safe.

The rule still does not lose text: the union of a field's clause texts, in
order, reconstructs the original almost verbatim.

#### What the boundary rule still cannot see

It is punctuation-shaped, so a **field-label ingestion artifact glued to
the front of the text now absorbs the first marker** rather than sitting in
a separable clause of its own. `cards.json`'s
`effect_description_eng` carries a literal `"Inherited Effect"` /
`"Security Effect"` box label on 11 cards (`BT25-001`..`006`, `BT25-088`,
`EX12-001`..`004`); on those, the leading clause now reads e.g.
`"Inherited Effect [Your Turn]"` and the marker it swallowed no longer
tags its clause. The label is not card text and the fix belongs upstream of
this pure splitter — strip the residue from the raw field **before**
`split_clauses`, rather than dropping a content-free clause after it (see
`card_sources.is_ingestion_artifact`).

## Filling in image-required slots

Do not OCR the image. Read it (`Read` tool renders `.webp` natively) or hand
it to a vision-capable agent, then write the confirmed text into a JSON
cache file:

```json
{
  "EX12-073": {"security": "[Security] Place this card in the battle area."}
}
```

Pass `--image-cache path/to/cache.json` to `extract`; a cached zone is
consulted (and split through the same clause rules, tagged
`"source": "image-cache"`) before falling back to `image-required`. This
way the image-reading work is done once per card, not once per `extract`
run.

## Clause-level coverage — now measured from `effect_activation` rows

DCGO commit `8c4f98cb6` added `effect_activation` rows to the recording
schema (`docs/DCGO_RECORDING_SCHEMA.md` §"Effect-activation row"): `card_id`
+ the printed `effect_description` + `is_optional` + `executed`. This is
the "cheap next step" this README used to describe as future work — it has
landed, and `coverage.py` now consumes it for real, measured clause-level
coverage instead of a blanket `UNKNOWN` placeholder.

- **card-level** (which deck cards ever appeared on a battle area) is
  measurable from the union of `board_p0`/`board_p1` snapshots carried on
  every `action`/`selection` row. "Never on board" is still NOT "never
  played" — see the `card_level.method` note in the report JSON for the
  Digi-Egg-buried-as-material and effect-routes-elsewhere caveats,
  confirmed directly on the VB corpus (`EX12-001`, `EX12-069`).
- **prompt-level** (which `selection.prompt` kinds fired, and how often) is
  measurable directly from the `selection` rows.
- **clause-level** is now measured. Each clause in the denominator gets
  exactly one status:
  - `FIRED` — matched at least one activation with `executed: true`.
  - `OFFERED_ONLY` — matched, but every matching activation had
    `executed: false` (offered and declined — DCGO's recorder deliberately
    distinguishes this from "never fired"; see the schema doc).
  - `NOT_FIRED` — no activation matched anywhere in the corpus, and the
    clause is of a kind the hook can observe.
  - `UNOBSERVABLE` — the clause's KIND is a confirmed structural blind spot
    of the hook: angle-bracket keyword clauses (persistent/passive, not
    dispatched as an activated effect) and digivolve/Use-Req. cost-line
    timing clauses (`[Digivolve]`, `[DNA Digivolve]`, `[Use Req.]`, and the
    other alt-digivolve-mechanism markers). Calling these `NOT_FIRED` would
    assert a falsehood the instrument cannot back up. If the corpus happens
    to catch real matched evidence for one of these anyway, that evidence
    wins and reports `FIRED`/`OFFERED_ONLY` instead — the kind-based rule
    is only the fallback for absence of evidence, never a reason to discard
    evidence that exists. Separately, 25 DCGO cards register
    `SetIsBackgroundProcess(true)` and bypass the hook entirely regardless
    of clause kind; which cards those are cannot be determined from
    recordings alone, so this is called out in the report's
    `clause_level.known_limitations` text rather than reflected in any
    single clause's status.

  Matching an activation row to a denominator clause
  (`tools.clause_coverage.activation_match`) is fuzzy (normalized
  similarity ratio), not exact string equality: `effect_description`
  (DCGO's live rendering) and this package's extracted clause text come
  from two independent pipelines that agree on content but not
  byte-for-byte (full/half-width punctuation, curly vs. straight quotes,
  bullet glyph choice, small independent wording differences). Any
  activation that cannot be matched to any clause on its card at all is a
  genuine signal — a card missing from the denominator, an unresolved
  `image-required` clause, or (observed in the real corpus) a granted
  ability logged under its *recipient's* `card_id` rather than the
  granting card's — and is surfaced loudly in
  `clause_level.unmatched_activations`, never silently dropped. See
  `.superpowers/sdd/clause-coverage-v2-report.md` for the measured result
  against the real `vb-corpus3` corpus, including the specific unmatched
  findings.

`coverage.py` still detects corpora that predate the `action.card_id` /
`action_detail` fields and says so in `corpus.schema_note` rather than
assuming; the same applies for corpora with zero `effect_activation` rows
(every clause simply has no matching evidence, which — per the rules
above — resolves to `NOT_FIRED` or `UNOBSERVABLE` as appropriate, not a
special-cased blanket status).

## Scope (YAGNI)

Exactly the two entry points above. No scenario running, no assertions
against expected coverage, no DCGO integration/build/launch. Never runs
`dcgo-harness`, never writes under `D:\` or
`C:\Users\james\AppData\LocalLow\DCGO\` (recordings are read-only input).
