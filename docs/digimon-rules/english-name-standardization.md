# English name-substring exclusions

Authority: Bandai's [Language Standardization Rules](https://world.digimoncard.com/rule/lang-standardization-rules/), May 29, 2026, and its [reference PDF](https://world.digimoncard.com/rule/pdf/lang-standardization-01.pdf). This derivation covers the **12 English exact-name/query exclusion pairs**, not the PDF's multilingual mappings. The owner authorized this production follow-up on October 22, 2026.

| Exact name | Excluded substring query | PDF page |
|---|---|---|
| Pagumon | Agumon | 2 |
| DemiVeemon | Vee | 2 |
| KendoGarurumon | Garurumon | 2 |
| BurningGreymon | Greymon | 3 |
| DoruGreymon | Greymon | 3 |
| DexDoruGreymon | Greymon | 3 |
| Indramon | Dramon | 4 |
| BeelStarmon | Starmon | 10 |
| BeelStarmon (X Antibody) | Starmon | 10 |
| DemiVeemon | Veemon | 12 |
| Blimpmon | Impmon | 12 |
| MasterBlimpmon | Impmon | 12 |

Apply the exclusion case-insensitively to each name independently. A Rule or effect granting an eligible additional name can satisfy the query even when the base name is excluded. Combined DUAL names are evaluated per face for substring matching. Exact-name equality and generic printed-text searches keep their existing semantics.

The shared `card_data::rules_name_contains` helper is used by CardSource, synthesized permanent identities, game handle lookups, DSL card/source/field predicates, and live/deleted event identities. DNA material requirements and legacy DigiXros name matching must use the same helper. This follow-up preserves the existing identity inputs of DNA material requirements; it does not expand their static-alias or effective-overlay support.

PDF page 13 additionally includes MarineChimairamon in Kimeramon. That is an inclusion rule, outside this exclusion-only fix. Other rows express Chinese, Japanese, or Korean substrings and are not automatically English exclusions.

Behavioral evidence lives in `code/digimon-engine/tests/dsl/name_standardization.rs`: real hand selections and masks for all twelve pairs, positive controls, case folding, static/runtime aliases, effective permanent and deleted-event names, exact equality, DUAL faces, and DNA material legality/resolution. This proves simulator behavior; it is not a DCGO oracle or multilingual conformance claim.

Validation (October 22, 2026): before the fix, the focused suite had 2 passing controls and 5 expected behavioral failures. After the fix, all 7 tests pass. The freshly built DSL executable also passes 85 existing tests filtered to `constructs::identity_state`, `also_treated_as`, `dna_material_identity`, `digixros_aliases`, and `event_context_bindings` (0 failures).

The production-card regression passes 70 tests across EX4-061, EX13-067, BT19-014, BT17-007, and BT11-018 (0 failures). This is a focused regression boundary; the full engine suite and readiness-artifact refresh remain separate campaign tasks.
