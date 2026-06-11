## Context

A card's keyword set has two parts: (1) **innate** keywords it always has (printed as attributes), and (2) keywords its **effects grant** under conditions. The engine already models (2) via DSL effects (`grant_keyword` auras, `security_attack_fn` formulas). The bug is that (1) is *inferred from effect prose* by `parse_printed_keywords`, which scans the whole text for `＜…＞` tokens and so also picks up the granted/conditional/filter tokens that belong to (2).

Observed grammar of Digimon card text (sampled across BT/ST/AD sets):

| Card | Field text (head) | Token role |
|------|-------------------|------------|
| Monmon BT1-031 | `＜Blocker＞ (This Digimon can block…)` | **innate** (position 0) |
| MetalGreymon BT2-063 | `＜Reboot＞ (Unsuspend…)` | **innate** |
| Greymon AD1-001 (inh) | `＜Raid＞ (When this Digimon attacks…)` | **innate** |
| WarGreymon ST1-11 | `[Your Turn] For every 2… it gains ＜Security A. +1＞` | granted/formula |
| SkullGreymon BT1-023 | `[On Play] Delete… with ＜Blocker＞` | target filter |
| Flarerizamon BT1-018 | `[Your Turn] While 3+ memory… gains ＜Security A. +1＞` | conditional grant |

Innate tokens sit in a **keyword line at the very start**, before any `[Timing]` bracket or `Inherited Effect` / `Security Effect` header. Granted/filter tokens always come **after** a `[Timing]`/header. This is a structural regularity, not a probabilistic one.

## Goals / Non-Goals

**Goals**
- Innate keywords reflect only what a card prints as an attribute.
- Parametric keywords (`Security A.`, `Draw`, `De-Digivolve`) never double-count or apply unconditionally from prose.
- Net behavior of implemented cards is correct-or-better (conditional grants instead of unconditional phantoms).
- One mechanism for WarGreymon's security attack (the parser), not two.

**Non-Goals**
- `spec.keywords`-authoritative sourcing (declaring innate keywords in YAML instead of parsing) — a larger re-architecture, deferred.
- Implementing unimplemented cards' grants — logged, not fixed.
- A `security_attack` assertion kind for the scenario evaluator.

## Decisions

### D1. Per-token left-context classifier (revised during implementation)
**Original plan** was a leading keyword-line tokenizer (innate = the `＜kw＞` run at the field start, stop at the first `[Timing]`/prose). **The Step-2 audit disproved it**: genuinely-innate keyword *abilities* are printed attached to a timing/location label — `[Hand] [Counter] ＜Blast Digivolve＞`, `[Main] ＜Digi-Burst 2＞` — so "stop at the first `[`" wrongly dropped them (68 Blast Digivolve + ~36 Digi-Burst). The behavioral suite missed it only because those cards lacked coverage. This is exactly why the audit-first gate (D3) was worth running.

The durable rule is **per-token left-context**: a `＜kw＞` token is innate iff its left context (back to field start, after stripping a leading `Inherited Effect`/`Security Effect`/`Rule Effect` header), trimmed of trailing whitespace, is empty or ends with `]` (a `[Timing/Location]` label), `)` (a prior keyword's reminder), or `＞` (a chained keyword). Anything else — a grant verb (`gains ＜…＞`), a filter preposition (`with ＜…＞`), or a DP-and-keyword comma (`+3000 DP, ＜…＞`) — is prose, so the token is granted/conditional/filter, not innate. This keeps bracket-attached abilities, drops grants/filters, and naturally handles keyword tokens that appear *inside a reminder* (their left context is prose → not innate), so no special reminder-skipping is needed.

Rejected alternatives: leading-line (drops bracket-attached abilities, above); grant-verb-only exclusion (fragile, English-dependent); cut-at-first-`[`.

### D2. Classifier shape
For each field: strip a leading ingest header; scan every `＜…＞` token; for each, look at the trimmed-right left context and emit it as innate only if that context is empty / `]` / `)` / `＞`-terminated; otherwise skip. Token classification (the prefix table + parametric `Security A.`/`De-Digivolve`/`Draw`/`Decoy`/… handling) is unchanged — only the innate/granted gate is new.

### D3. Diff-first gating (outcome recorded)
Step 2 ran before Step 3. Findings, after two corrections the gate forced:
1. **Mechanism flaw** (see D1): the leading-line parser dropped bracket-attached innate abilities; revised to the context-classifier.
2. **Partition bug**: "implemented" must mean a DSL `*.yaml` exists. The `*.json` files in `code/digimon-engine/cards/**` are per-card *metadata* (4099 of them), not implementations (586 `*.yaml`). Counting them inflated "implemented losers" to ~1094; the `.yaml`-only partition gives **167** implemented losers.
3. **Audit verdict**: of those, the regression-risk surface (meaningful keyword, grant/ambiguous role, grant not detected in YAML) narrowed to **20 cards** — and manual review found **all 20 are false-positives**: the lost token was inside an `＜Alliance＞` reminder ("…gains ＜Security A. +1＞ for the attack" — 8 cards), a Token's stat block (`…/6000 DP/＜Reboot＞ ＜Blocker＞…`), a condition/action reference, or an already-modeled grant. **No genuine silent regressions.** The diff + audit artifacts (`keyword-diff.md`, `keyword-audit.md`) are committed as the review record.

### D4. Correct-or-better for implemented regressors
A card that *loses* a parsed keyword had that keyword wrongly (unconditional/innate). The audit (D3) found **zero implemented cards that genuinely relied on the phantom** — every loss was a reminder/token/condition artifact or an already-modeled grant. So no per-card grant authoring is required by this change. (If a future card surfaces a real gap, the rule stands: add a conditional grant, strictly more faithful than the phantom.)

### D5. Remove the interim patch, keep its tests
The combat-site subtraction (`top_card_has_security_attack_formula` / `top_face_security_attack_keyword_bonus`) becomes dead once `face_keywords(WarGreymon)` returns `[]`. Two mechanisms for one fact is worse than one. The real-card-data regression tests stay and now exercise the durable path.

## Risks / Trade-offs

- **Diff larger than expected.** Mitigated by D3 (diff-first; reconvene on size). Boolean-keyword losses are low-risk (idempotent today; the change makes a conditionally-gained boolean correctly conditional once modeled, or simply removes a phantom innate the card never legitimately had).
- **A card relied on a "filter" token being parsed as its own keyword.** Extremely unlikely (filters describe targets), but the diff surfaces every such case for review.
- **Tokenizer misclassifies an unusual leading layout.** Mitigated by unit tests over the sampled grammar + the full behavioral suite + the reviewed diff. Any card whose keyword line genuinely sits after a header is covered by the header-skip step (D2.1).
- **Unimplemented regressors lose a keyword.** Acceptable and logged — they were not correctly implemented anyway; the phantom gave a wrong (unconditional) approximation.

## Migration Plan

1. Land the tokenizer + unit tests (no behavior depends on it until call sites recompute — they recompute live each query, so this is immediate).
2. Run the diff; commit the artifact; audit.
3. Model gaps for implemented regressors (data-gated).
4. Remove the interim patch.
5. Full verification (behavioral suite + WarGreymon real-data tests).
6. Docs.

Each step is independently reviewable; Step 3 is the only data-gated one.

## Open Questions

- **Diff size** — how many implemented cards regress? Resolved by running Step 2. Gates Step 3.
- **Header set** — the exact list of leading headers to skip (`Inherited Effect`, `Security Effect`, `Rule Effect`, others?). Enumerate from the diff/data during Step 1.
- **`spec.keywords` follow-up** — if the diff shows the parse remains brittle for some layouts, that strengthens the case for the deferred DSL-authoritative re-architecture; note but do not act here.
