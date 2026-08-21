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
    --clauses clauses.json --recordings-dir D:\dcgo-build\vb-corpus2 --out coverage.json
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
3. **`image-required`** — **security zone only**. See "Why security gets its
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
field — a documented, conservative tradeoff, not an oversight.

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
  its own clause, unconditionally — open-ended: any angle-bracket token
  that isn't a recognized timing-marker name becomes a keyword clause, so
  new keywords in future sets are picked up automatically without a code
  change.
- Square-bracket tokens that aren't recognized timing markers (`[VB]`,
  `[Gammamon]`, `[NSp]`, ...) are trait/card-name references, not clause
  boundaries — left as ordinary body text.
- `TIMING_MARKER_NAMES` is the task's given list plus **one** directly-
  observed addition: `"Start of Your Main Phase"` (real printed text on
  `EX12-021`, not in the task's illustrative list but structurally the same
  kind of marker as `"Start of Your Turn"`).

### Known limitation: keyword splitting is unconditional, not positional

`<Draw 2>` embedded mid-sentence inside a triggered effect (e.g. `EX12-005`
Agumon's `"[On Play] By trashing 1 card ..., ＜Draw 2＞ (Draw 2 cards from
your deck.)"`) is split into its own keyword clause exactly like a
standalone persistent-keyword grant (`EX12-018`'s `＜Progress＞ ＜Piercing＞
＜Security A. +1＞` prefix). This can leave the preceding timing clause's
captured text reading as an incomplete sentence and the keyword clause
reading as context-free — read the two clauses together for full context.
This is a deliberate simplification (uniform, position-independent rule) to
satisfy the task's literal splitting instruction without adding an
unrequested positional heuristic; it does not lose text (the union of a
field's clause texts, in order, reconstructs the original almost verbatim).

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

## The coverage report's honest limit — clause-level firing is not measurable today

`docs/DCGO_RECORDING_SCHEMA.md` rows carry `card_id`/`cost_paid` only on the
2026-08-20-and-later `action`/`action_detail` diagnostic fields, and even
those never record "this card's clause fired" — there is no
effect-activation row in the schema at all. So:

- **card-level** (which deck cards ever appeared on a battle area) IS
  measurable — from the union of `board_p0`/`board_p1` snapshots carried on
  every `action`/`selection` row, which are present in every recording
  regardless of schema vintage. But "never on board" is not "never played":
  `board_p0`/`board_p1` list only the TOP card of each stack, so a Digi-Egg
  that hatched and immediately digivolved further is buried as
  digivolution material and invisible to it, and a card whose own effect
  routes it somewhere other than the battle area never shows up even if it
  fired every game. Both are confirmed, not hypothetical, on the measured
  VB corpus: `EX12-001` (the deck's only Digi-Egg) and `EX12-069` (an
  Option whose `[Main]` effect places itself as a security card, never the
  battle area) both read `never_on_board` despite almost certainly being
  played in most/all of the 12 games.
- **prompt-level** (which `selection.prompt` kinds fired, and how often) IS
  measurable directly from the `selection` rows.
- **clause-level** is **NOT** measurable. A card's presence on the board is
  not evidence any specific clause of that card executed — a card can sit
  on the board the entire game without its `[On Deletion]` clause ever
  running, or with an optional `[On Play]` effect declined every time it
  was played. `coverage.py` reports every clause `"UNKNOWN"`, explicitly,
  with the reason spelled out in the JSON. **This is the correct output
  right now** — a report that quietly implies coverage it cannot measure is
  the exact failure this whole effort exists to prevent. The UNKNOWN count
  is the report's headline number for exactly that reason.

The measured 12-game `vb-corpus2` corpus (read from `D:\dcgo-build\vb-corpus2\`,
read-only, never written to) predates even the `action.card_id` /
`action_detail` fields — `coverage.py` detects this dynamically per corpus
(counts action rows carrying `card_id`, counts `action_detail` rows) and
says so in the report's `corpus.schema_note`, rather than assuming.

### The cheap next step (not built here — out of scope for this task)

DCGO's own player log already emits effect-activation lines, e.g.:

```
Activate_Optional_Effect_Execute: Siriusmon
```

Hooking that into `GameRecorder.cs` as a new JSONL row type (say,
`effect_activation`, with `card_id` + a resolved clause reference) would be
the direct path to real clause-level coverage — parallel to how
`action_detail` was added for the resolved-play diagnostic round
(`docs/DCGO_RECORDING_SCHEMA.md` §"Resolved-action detail row"). This is
explicitly **not** implemented by this task (YAGNI: extractor + coverage
report only, no DCGO integration) — it's recorded here so the next piece of
this pipeline knows where to start.

## Scope (YAGNI)

Exactly the two entry points above. No scenario running, no assertions
against expected coverage, no DCGO integration/build/launch. Never runs
`dcgo-harness`, never writes under `D:\` or
`C:\Users\james\AppData\LocalLow\DCGO\` (recordings are read-only input).
