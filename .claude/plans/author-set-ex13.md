# EX13 — author-set hand-off (Phases 1–3 done, 2026-09-30)

Card data for EX13 is ingested and reconciled, the keyword gate is clear, and
the slice partition below was **approved as-is (2026-09-30)**. `/author-set
EX13` can go straight to the Phases 4–6 Workflow.

Re-run the dry run any time:

```bash
PYTHONPATH=code python -m tools.author_set.report_set EX13
```

## What is in place

| Step | State |
|---|---|
| Official Bandai DB mirror | `data/card_bundles/EX13-001..077.md` + `data/card_official.json` (77/77) |
| Ingest | 77 cards in `data/cards.json`, indices 4325..4401 |
| Reconcile vs official DB | `data/card_overrides.json`: 5 Rule-granted traits, 26 evo-circle sets, 4 colour sets, 13 missing Assembly lines, 2 DUAL blocks, printed-text fixes |
| Keyword gate | clear. Every keyword EX13 prints already exists (`Engage`, `Guard`, `ArtsDigivolve`, `Decode`, ...) |
| DCGO oracle | **32/77** scripts at the submodule (`b9a0638cd` = fork + DCGO2 develop 541bc287a); oracle player not yet rebuilt |

## Slice partition (approved as-is)

From the dry run (10 slices + 21 orphan staples):

| Slice | Cards |
|---|---|
| Examon / holy warrior (15) | 020, 014, 015, 023, 036, 037, 043, 060, 061, 062, 064, 016, 045, 077, 001 |
| Dracomon / dragon (6) | 008, 018, 039, 021, 041, 044 |
| Veedramon / CS (6) | 017, 019, 022, 067, 069, 074 |
| Mutant (6) | 050, 053, 054, 031, 059, 063 |
| Richard Sampson / DATA SQUAD (5) | 003, 026, 030, 032, 071 |
| Chronicle (5) | 006, 049, 055, 057, 072 |
| Sukamon / beast (4) | 027, 038, 028, 040 |
| Guilmon / reptile (3) | 007, 048, 068 |
| Puppet (3) | 065, 066, 035 |
| Witchelny (3) | 025, 029, 033 |
| Orphan staples (21) | 002, 004, 005, 009, 046, 047, 010, 011, 051, 052, 012, 013, 034, 042, 056, 058, 024, 076, 070, 073, 075 |

The "Examon / holy warrior" slice is really the **Royal Knights** Assembly
pool; it was kept as one slice.

## What authors need to know

- **Assembly is the set mechanic.** 17 cards: 014, 015, 016, 020, 023, 024,
  031, 036, 037, 043, 044, 060, 061, 062, 063, 076, 077. The DSL has
  `kind: assembly` (EX12, BT24-081 are prior art). New requirement shapes to
  check against the vocabulary:
  - `Lv.5 × Lv.4 × Lv.3, all w/…`: one material per level (10 cards)
  - `… w/different colors` (077): is there a `distinct_by: color`?
  - `[WarGreymon]×[MetalGarurumon]×[Agumon]×[Gabumon]` (016): four named materials
  - `3 [Huckmon] text Digimon cards w/different names` (061)
- The Assembly text is in `xros_req` in the corpus's existing
  `Assembly Requirements [Assembly -N] …` shape. The engine does not read
  `xros_req`; author it in YAML.
- **Rule lines are not in card text** (corpus convention). They must be
  authored as `also_treated_as` or traits:
  - EX13-029 FlameWizardmon: Name, also treated as [Wizardmon]
  - EX13-053 Thundermon: Name, treated as including [Mamemon]
  - EX13-066 Sistermon Noir (Awakened): also treated as Name
    [Sistermon Ciel (Awakened)] + [Data] attribute (the attribute is already in
    `attribute_eng`)
  - Trait grants 025/038/054/056/076 are already in `type_eng`/`attribute_eng`.
- DUAL cards: 065, 066 (both White Option face, use cost 5, `<Arts Digivolve>`).
- Tokens: [Atho, René & Por] (014 and the Sistermon line), [Hinukamuy] (061).

## Open items

1. **DCGO bump: done in git; the oracle player still needs a rebuild.**
   Upstream DCGO2 develop (541bc287a, 87 commits past the last merge) merged
   cleanly into the fork's `add-recording-mod-r2` as `b9a0638cd` (pushed), and
   the submodule here points at it. EX13 scripts go from 20 to 32. Recorder hooks
   are verified untouched (no hook file or `GameRecorder`/`__rec` call site in
   any upstream-changed path), and so is the Mono pin in `HarnessBuild.cs`. The
   keyword dirs only saw edits (Ascension/Progress/Execute/Succession, still
   "simple"), so `data/dcgo_keyword_manifest.json` stays valid.
   **Still to do on the base-repo machine:** `git submodule update DCGO`, then
   rebuild the oracle player (scripted-v18) against `b9a0638cd` before any EX13
   exam. The full exam corpus has not been re-measured against it.
   Still no DCGO script anywhere for 45 cards: 003 014 016 025 026 027 028 029
   030 031 032 033 034 035 036 037 038 040 042 043 046 048 049 050 052 053 054
   055 057 058 059 060 061 063 064 065 066 067 068 070 071 072 073 074 077.
   For those, the official card text and `general_rule.pdf` are the references.
2. ~~EX13-067 source conflict~~ **Resolved: the official DB wins.** Its text is now
   "... [Greymon] in its name **or** 1 [Agumon] ..." (the card face prints
   "and").
3. ~~The ingest's DigiXros parser can't read the `Lv.5 × Lv.4 × Lv.3` Assembly
   shape~~ **Resolved 2026-10-01:** `tools/xros_cost_parser.py` parses all 17
   EX13 Assembly requirements (per-level slots, name/text/trait alternatives,
   keyword, colour, "different colors"), and an overridden `xros_req` now
   re-derives `dna_costs` / `digixros_costs`. Tests in
   `code/tests/tools/test_xros_cost_parser_assembly.py`.
