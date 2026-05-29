## Acceptance Fixture Baseline

This baseline records the current state after adding the red Rust behavioral
tests for tasks 1.1-1.8.

### Current card authoring state

- `BT21-083`: JSON card data exists under `code/digimon-engine/cards/bt21/`,
  but no embedded production DSL YAML exists. New tests cover start-main
  hand-to-self stash with draw/memory and the played-Digimon optional attack
  window.
- `BT11-095`: JSON card data exists under `code/digimon-engine/cards/bt11/`,
  but no embedded production DSL YAML exists. New tests cover start-main
  hand-to-self stash and current-transaction access to under-Tamer DigiXros
  materials.
- `P-224`: JSON card data exists under `code/digimon-engine/cards/p/`, but no
  embedded production DSL YAML exists. New tests cover hand/trash stash under
  itself and cost-reduced play from under Tamers.
- `BT19-090`: JSON card data exists under `code/digimon-engine/cards/bt19/`,
  but no embedded production DSL YAML exists. New tests cover both option
  modes: low-DP Xros Heart play from under a Tamer and unsuspend-then-attack.
- `BT21-092`: JSON card data exists under `code/digimon-engine/cards/bt21/`,
  but no embedded production DSL YAML exists. New tests cover moving all
  matching source cards under a Tamer, binding the moved count, and applying
  the count to a follow-up play-cost reduction.
- `BT10-111`: JSON card data exists and an example YAML exists under
  `code/digimon-engine/cards/_examples/BT10-111.yaml`, but the example still
  uses a `raw_rust` wildcard hook. New tests cover replacing one missing
  DigiXros requirement for the turn through a real transaction prompt.
- `BT21-027`: JSON card data exists under `code/digimon-engine/cards/bt21/`,
  but no embedded production DSL YAML exists. New tests cover trait-filtered
  source rescue from the pre-removal stack snapshot.
- `BT19-061`: JSON card data exists under `code/digimon-engine/cards/bt19/`,
  but no embedded production DSL YAML exists. New tests cover DigiXros-only
  `Sparrowmon` aliasing and hand-or-trash deletion stash under a Tamer.

### Observed red state

Focused test filters compile. The expected red failures are:

- Missing embedded production DSL YAML for `BT21-083`, `BT11-095`, `P-224`,
  `BT19-090`, `BT21-092`, `BT21-027`, and `BT19-061`.
- `BT10-111` loads from the existing example path, but the later DigiXros
  transaction does not install a material prompt when only the wildcard card can
  satisfy the missing requirement.

These failures are the intended baseline for the reusable primitive work in
sections 2-6. The tests should turn green by adding primitives and declarative
YAML, not by adding card-specific raw Rust placeholders.
