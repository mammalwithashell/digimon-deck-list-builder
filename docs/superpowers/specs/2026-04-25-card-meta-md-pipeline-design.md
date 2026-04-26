---
title: Per-card metadata `.md` pipeline + xros_req parser
date: 2026-04-25
status: draft
related:
  - docs/superpowers/specs/2026-04-21-card-scripting-dsl.md (§9.4 — alt-path parsing locus)
---

# Per-card metadata `.md` pipeline + xros_req parser

## 1. TL;DR

To unblock sub-agent-driven authoring of Rust DSL card YAMLs, we generate
one focused `.md` file per card under `data/card_meta/<set>/<card_id>.md`,
checked into git. Each file contains:

- An H1 with `<card_id> — <card_name>` for grep / scan friendliness.
- A `## Alt paths (parsed from xros_req)` block — structured YAML matching
  the DSL spec's `alt_paths:` shape (§3 of the DSL spec).
- A `## Source record` block — the verbatim `cards.json` entry as a JSON
  fenced block (with `card_overrides.json` already applied at ingest time).
- (conditional) `## Unparsed xros_req` — the raw lines the parser couldn't
  fully cover, preserved verbatim so a sub-agent can hand-author the
  alt-path entry instead.

Generation lives in `tools/`. A new `tools/xros_req_parser.py` module is
the only piece of new logic; everything else extends existing aggregators
(`tools/resolve_deck.py`'s `CardEntry`).

This resolves DSL spec §9.4 in favor of **option (b)** — parse `xros_req`
at metadata-build time and promote to a structured field, but with the
permissive failure mode that lets unparsed cases coexist with parsed ones.

## 2. Goals and non-goals

**Goals:**

- One `.md` per card, ~50 lines, focused enough that a sub-agent's prompt
  can include it inline without grepping cards.json.
- Parsed `alt_paths:` block that a sub-agent can paste verbatim into a
  card YAML when the parser was confident.
- Verbatim card text via the `cards.json` record dump — no transcription
  step that could introduce drift between the .md and the source-of-truth.
- Diffable in PRs — when `cards.json`, `card_overrides.json`, or the
  parser changes, the .md diff shows every card whose metadata flipped.
- Permissive failure mode — the parser never blocks the build; cards
  with unparsed `xros_req` still get a usable .md.
- Coverage reporting — every CI run knows the parser's exact card-level
  coverage, so the grammar can grow incrementally.

**Non-goals:**

- DCGO C# discoverability is **not** embedded in the .md — the dispatcher
  skill greps `DCGO/Assets/Scripts/CardEffect/` for the card_id at
  agent-dispatch time.
- No engine API reference, no Pinecone-retrieved snippets, no similar-card
  index — those are dispatcher-side context, not card-side metadata.
- No localization. The .md is English-only (mirrors `cards.json`'s
  `*_eng` fields).
- No fandom URL embedding. The card_id is enough for an agent to construct
  the URL if needed; baking it in is dead weight.
- No on-demand generation. The .md tree is checked in; refresh is a
  manual-but-CI-enforced step.

## 3. File layout

```
data/card_meta/
├── ad1/
│   ├── AD1-001.md
│   ├── AD1-002.md
│   └── ...
├── bt17/
├── bt22/
├── ...
└── _coverage.md   # parser coverage report (regenerated each build)
```

Set-level subdirectories (lowercase set prefix) match the existing layout
in `digimon_gym/engine/data/scripts/<set>/`. Cards whose `card_id` doesn't
have a set prefix (e.g. promos) bucket into `_misc/`.

## 4. File format

```markdown
# BT17-007 — Agumon

## Alt paths (parsed from xros_req)

- kind: digivolve
  from: { name_is: "Koromon" }
  cost: 0

## Source record

​```json
{
  "card_id": "BT17-007",
  "card_name_eng": "Agumon",
  "card_kind": "Digimon",
  "level": 3,
  "play_cost": 3,
  "dp": 2000,
  "card_colors": ["Red"],
  "type_eng": "Reptile",
  "form_eng": "Rookie",
  "attribute_eng": "Vaccine",
  "evo_costs": [...],
  "xros_req": "[Digivolve] [Koromon]: Cost 0",
  "effect_description_eng": "[Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its name, return 1 card with [Garurumon], [Greymon] or [Omnimon] in its name from your trash to the hand.",
  "inherited_effect_description_eng": "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand.",
  "security_effect_description_eng": ""
}
​```
```

Sections are stable in order (H1, alt paths, source record, optional
unparsed). The `## Source record` JSON is the verbatim `cards.json` entry
post-overrides, pretty-printed with 2-space indent and sorted keys for
diff stability.

If `xros_req` was wholly absent from the source record, the alt-paths
block is `## Alt paths (parsed from xros_req)\n\n_(none)_`. If the parser
recognized only some lines, the recognized entries appear under
`## Alt paths` and the rest under `## Unparsed xros_req` as a fenced text
block.

## 5. xros_req grammar (parser scope)

Empirical survey of 1,158 cards with `xros_req`: four leading markers
account for 100% of lines:

| Marker             | Count |
|--------------------|-------|
| `[Digivolve]`      | 1068  |
| `[DNA Digivolve]`  | 62    |
| `[App Fusion]`     | 24    |
| `[Burst Digivolve]`| 4     |

(`[DigiXros]` is absent from `xros_req` strings — DigiXros is encoded
in `effect_description_eng` as a triggered clause, not as a printed
digivolution requirement. The DSL `kind: digixros` is reserved for the
declarative aura case. Out of scope for this parser.)

### 5.1 Recognized productions

Each `xros_req` is split on newlines (`\r\n` and `\n`). Each non-empty
line is matched against:

```
PROD_DIGIVOLVE     := "[Digivolve]"     CONSTRAINT ":" "Cost" INT
PROD_DNA           := "[DNA Digivolve]" MATERIALS  ":" "Cost" INT
PROD_APP_FUSION    := "[App Fusion]"    MATERIALS  ":" "Cost" INT
PROD_BURST         := "[Burst Digivolve]" CONSTRAINT ":" "Cost" INT
```

`CONSTRAINT` is one of:
- `Lv.<N> w/[<NAME>] in name` → `from: { level_eq: N, name_contains: NAME }`
- `Lv.<N> w/[<NAME>] in text` → `from: { level_eq: N, name_in_text: NAME }` *(new
  predicate; spec §3 has `name_contains` for top-card name only — `in_text`
  means anywhere in the digivolution-source stack's printed text. Parser
  emits this; engine support is a Tier-3 follow-up flagged in the coverage
  report.)*
- `Lv.<N> w/[<TRAIT>] trait` → `from: { level_eq: N, trait_has: TRAIT }`
- `[<NAME>] w/<N> or more [<TRAIT>] trait cards under` → DNA-stack constraint:
  `from: { name_is: NAME, materials: { trait_has: TRAIT, count_gte: N } }`
- combined with `or` → `any_of:` list
- combined with `/` (e.g. `[Xros Heart]/[Blue Flare]`) → `any_of:` list of
  trait variants

`MATERIALS` for DNA / App Fusion uses `&` as the conjunction:
- `[<NAME_A>] & [<NAME_B>]` → `materials: [{ name_is: NAME_A }, { name_is: NAME_B }]`

### 5.2 Failure mode (permissive + reporting)

- Each line is matched independently. A line that matches becomes a
  parsed alt-path entry. A line that doesn't match becomes a verbatim
  entry in the `## Unparsed xros_req` block of that card's .md.
- If a line matches the leading marker but fails on the constraint or
  cost suffix (e.g. AD1-005's post-line `If 2 such cards are linked
  together, stack the link card on top and digivolve.` — which has no
  marker and no `Cost N`), it is treated as unparsed.
- The parser never raises. The build never fails on parser errors.
- After every full build, `data/card_meta/_coverage.md` is regenerated:

  ```markdown
  # xros_req parser coverage

  Generated: 2026-04-25T...

  - Total cards with xros_req: 1158
  - Fully parsed: 1102 (95.2%)
  - Partially parsed: 41 (3.5%)
  - Wholly unparsed: 15 (1.3%)

  ## Cards with unparsed lines

  - AD1-005: `If 2 such cards are linked together, stack the link card on top and digivolve.`
  - ...
  ```

  CI asserts the coverage doesn't regress (green if ≥ previous run).

## 6. Tool surface

Three pieces:

### 6.1 `tools/xros_req_parser.py` (new)

```python
@dataclass(frozen=True)
class ParsedAltPath:
    kind: str   # "digivolve" | "dna_digivolve" | "app_fusion" | "burst_digivolve"
    from_: dict | None
    materials: list[dict] | None
    cost: int

@dataclass(frozen=True)
class XrosReqParseResult:
    parsed: list[ParsedAltPath]
    unparsed_lines: list[str]

def parse(xros_req: str) -> XrosReqParseResult: ...
```

Pure function, no I/O. Tests live at `tests/tools/test_xros_req_parser.py`
and exercise the production grammar against fixture strings drawn from
the empirical survey (one fixture per distinct shape).

### 6.2 `tools/resolve_deck.py` extension

Add:

```python
def build_card_meta_md(card_id: str) -> str:
    """Render the .md for a single card. Returns the file body."""
```

Reuses the existing `CardEntry`-load path so override merging stays in
one place. Returns the full file body string; caller is responsible for
writing.

### 6.3 `tools/build_card_meta.py` (new CLI)

```
python -m tools.build_card_meta                    # rebuild all 4085 .md files
python -m tools.build_card_meta --card BT17-007
python -m tools.build_card_meta --set bt17
python -m tools.build_card_meta --check            # CI mode: rebuild in tempdir, diff
python -m tools.build_card_meta --coverage-check   # CI mode: assert coverage didn't regress
```

`--check` is the CI hook. It regenerates the tree to a tempdir, then
diffs against `data/card_meta/`. Exit nonzero on any diff. This enforces
the "checked-in tree must match generator output" invariant without
forcing CI to commit anything.

## 7. CI integration

Two checks:

1. `python -m tools.build_card_meta --check` — must exit 0 (tree matches
   generator output).
2. `python -m tools.build_card_meta --coverage-check` — reads
   `_coverage.md`, asserts wholly+partially unparsed counts have not
   regressed beyond the committed baseline. Baseline lives in
   `data/card_meta/_coverage_baseline.json` and is updated alongside
   any intentional grammar regression.

No new GitHub Actions workflow file — these add as steps to the existing
test job.

## 8. Migration / rollout

1. Land `tools/xros_req_parser.py` + tests.
2. Land `tools/resolve_deck.py::build_card_meta_md` + tests.
3. Land `tools/build_card_meta.py` CLI.
4. Run the CLI once, commit the resulting `data/card_meta/` tree
   (~4,085 files).
5. Add the two CI checks.
6. Update `/implement-archetype` (and the forthcoming Rust DSL sibling
   skill) to read `data/card_meta/<set>/<card_id>.md` instead of
   re-aggregating from `cards.json`.

## 9. Open questions

1. **`name_in_text` predicate** — does the DSL spec want a new predicate
   for "in text" matching (matches anywhere in the printed text of a
   digivolution-source stack), or should the parser emit `unparsed:` for
   those lines until §3 of the DSL spec adds the predicate? Parser
   currently emits the predicate; engine support is a Tier-3 follow-up.
   Default: emit and let the engine catch up.

2. **Override layering** — should `card_overrides.json` apply to the
   `## Source record` block (current default — overrides ARE the
   source-of-truth), or should the .md show pre-override and
   post-override side-by-side? Default: overrides applied silently,
   matching how cards.json is consumed everywhere else.

3. **Promo / no-set-prefix cards** — bucket into `_misc/`, or dedicated
   per-promo-line subdirs (`p_/`, `bts_/` etc.)? Default: `_misc/`
   until a sub-agent run actually targets a promo set.

4. **Schema-version frontmatter** — the .md has no frontmatter today.
   If the format ever changes (e.g. adding a `## Validated by` block),
   downstream consumers need a version. Default: defer until format
   actually changes; the file is a sub-agent input, not a long-lived
   contract.

## 10. Out of scope (future work)

- Sub-agent dispatcher / prompt template for Rust DSL YAML authoring.
  This spec only defines the input artifact.
- A YAML round-trip validator that consumes `## Alt paths (parsed from
  xros_req)` and asserts it lowers cleanly via the DSL Phase 2 compiler.
  Useful, but lives in `digimon-engine/`'s test surface, not in
  `tools/`.
- A reverse tool that diffs an authored card YAML against its .md and
  flags drift (e.g. card YAML's `alt_paths:` no longer matches the
  parsed one because the source text changed). Future, once the first
  ~50 YAMLs land.
