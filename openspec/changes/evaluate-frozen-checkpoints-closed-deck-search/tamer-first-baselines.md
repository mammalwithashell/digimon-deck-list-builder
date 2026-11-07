# Tamer-first heuristic baselines

User requested a baseline that digivolves over a Tamer whenever possible. Add two separately named variants; do not change existing teachers, checkpoint weights, frozen schedules or training configuration.

| Variant | Preferred actions | Fallback / ties |
|---|---|---|
| `greedy_tamer_first_v1` | Every mask-legal direct Main-phase digivolution targeting the acting player's Tamer | Lowest eligible action ID; original greedy otherwise |
| `development_tamer_first_v1` | Same eligible set | Existing development score/cost/action tie-break among eligible actions; original development otherwise |

The preference is strict, including when evolution passes memory or competes with a winning attack. It is an intentionally simple hypothesis-testing baseline, not a claim that every such evolution is strategically correct. Ordinary battle evolution, raising evolution and DNA remain distinct. Pending effect selections and multi-step effect-triggered Tamer evolution are not inferred from card text. No actions are added or masked out in the engine.

The development variant retains independent per-seat memory and repeated-state guards. If a repeated identical state exhausts its eligible Tamer actions, it fails explicitly rather than looping or silently breaking priority. Native output records the variant version, direct eligible routes and whether priority applied. Existing `development_v1` output/behavior and its state remain independent. `development-fixed` is a separate frozen learned checkpoint and is not changed by these heuristic variants.

Python usage after building/qualifying the private binding:

```python
from digimon_gym.agents.tamer_first import TamerFirstOpponent
opponent = TamerFirstOpponent("development_tamer_first_v1")
# Compatible with OpponentWrapper's callable interface.
# Existing anchored evaluation can opt in explicitly:
evaluate_against_anchors(..., include_tamer_first=True)
```

The optional anchored-evaluation flag defaults to false. A new run must explicitly freeze both variant identities, runtime and source hashes, decks/seeds and game budget. Existing reports and anchors keep their original identities. The historical mixed-bootstrap teacher list stays unchanged; this addition does not start training or retrofit teacher data.

Native tests cover legal direct choices, mask filtering, phase-scoped action reuse, ordinary Digimon targets, source-state preservation, memory crossing, deterministic ties, fallback equivalence and retained repeated-state safeguards. Python tests cover version/legality/priority validation, old-binding rejection and opt-in anchor integration. Whole-pool rollout qualification and strength evidence remain separate from these implementation tests.
