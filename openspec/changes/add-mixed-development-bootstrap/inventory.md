# Native teacher metadata inventory

Audited against the training branch based on main 054a06b12. This inventory is
about policy feature coverage, not card implementation readiness.

| Decision | Native source | Coverage / limitation |
| --- | --- | --- |
| Ordinary, alternate, App Fusion evolution | `normal_digivolve_route_for_hand_card` | Legal route base cost; final optional reducers unpriced. No mutating cost scan. |
| DNA | `dna_route_for_hand_card` | Pair legality/base cost; material body loss included. Pending picks remain separate actions. |
| Main activation | shared `main_effect_select` | Exact first eligible effect, not any text on the card. |
| On Play / Option Main draw | typed compiled steps | Literal unconditional own draws supported; costs, branches and formula draws unknown. |
| Normal evolution draw | game rule + public deck count | One spent/one drawn is replacement, zero net gain. |
| Hand payment / trash retrieval | typed `RunTail` continuation | Direct bound discard/add operation supported; arbitrary closures and nested continuations unknown. |
| Other search/filter/effect evolution | existing effect categories | Intent only, not guaranteed net hand gain or cost. |
| Physical-card menus | observer-filtered `OwnedCardChoice::view` | Legal page controls and visible candidate identity; no raw hidden candidates inspected. |
| Numeric menus | `CountChoiceMetadata` | Commit distinguished from navigation; unpriced consequences remain unknown. |
| Outcome/route events | existing RL state and event stream | Terminal target and separate normal/DNA counters available; full resource event aggregation still needs qualification checks. |

No observation/action contract changes. Unknown feature cases are serialized
with every candidate. Policy scores are not training rewards or value targets.

The conservative initial cost feature is explicitly `base_memory_cost`, not an
effective-payment claim. Payment parity tests apply to fixtures without reducers.
Any extension to effective costs must reuse read-only native queries and add
payment parity tests rather than simulate the true hidden state.


### Frozen development_v1 fallback scores

The implemented initial weights retain the design table. Additional bases:
rookie play +7; other Digimon/Tamer play +2; Option play 0; Main pass -6;
empty-security/empty-enemy-board attack +100 (priority, not a guarantee against
hidden responses); other security attack +2; favorable DP trade +3, otherwise
-4. Pending pass -0.5. Unsupported pending choices have no resource bonus.
These are initial hypotheses, not tuned playing-strength claims. Cost values
are native route base costs before optional reducers and are labeled as such.
