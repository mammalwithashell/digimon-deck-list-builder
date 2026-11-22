# BT27 Virus Busters and MirageGaogamon restart

Base: `349816e5c45a120629ba668ffdd6810a67554642` (remote main checked 2026-11-22).
Branch: `codex/bt27-virus-busters-mirage`.
Remote: https://github.com/mammalwithashell/digimon-deck-list-builder.git

This is a fresh rebuild. The historical local implementation checkpoint
`707dda33b537a357aabb8dd79480f93e48613f98` was not recovered. Prior test counts
are not results for this branch. This first checkpoint contains an inventory
and verification plan only: zero newly implemented or admitted cards.

## Grounded scope and initial state

Official English labels are `VB` and `DATA SQUAD`; `DATS` is not the English
trait label. The package is not every card with the franchise trait.

| Batch | Card | Package | Current main state |
|---|---|---|---|
| 1 | BT27-019 Gaomon | Mirage core | No YAML or registered metadata |
| 1 | BT27-023 KausGammamon | Virus Busters | No YAML or registered metadata |
| 1 | BT27-024 Gaogamon | Mirage core | No YAML or registered metadata |
| 1 | BT27-047 WezenGammamon | Virus Busters | No YAML or registered metadata |
| 2 | BT27-030 MachGaogamon | Mirage core | No YAML or registered metadata |
| 2 | BT27-032 MirageGaogamon / Full Moon Blaster | Mirage core | No YAML or registered metadata |
| 2 | BT27-089 Thomas H. Norstein | Mirage core | No YAML or registered metadata |
| 2 | EX4-019 MachGaogamon | Exact Bandai sample prerequisite | Metadata exists; no YAML; gate rejects untested_lowering |
| 3 | BT27-035 MirageGaogamon: Burst Mode / Full Moon Meteor Impact | Mirage core | No YAML or registered metadata; focused assessment required |
| Separate blocker | BT27-062 Maildramon | Knightmon-text support | Do not claim support: prior cost-prohibition witness needs independent resolution |

Bandai's Mirage sample also uses older BT25/ST24 cards and EX4-019; BT13-097
is an additional recommendation. Preserve the exact sample separately from
substituted smoke candidates. Virus Busters support includes existing EX12
cards; existing Royal Knight EX12-037 remains another lane's ownership.

## Sources and BT27-035 discrepancy

- English set/list: https://world.digimoncard.com/cards/index.php?category=522040&search=true
- Japanese list: https://digimoncard.com/cards/index.php?category=503042&search=true
- Set guide: https://world.digimoncard.com/products/pack/ver27/
- EX4-019: https://world.digimoncard.com/cards/index.php?card_no=EX4-019&search=true

The English list explicitly gives the Japanese list priority. English and
Japanese printed images and Japanese Q7687 reduce the opponent's hand to seven;
English Q7687 alone says eight. Implement seven only with faithful tests and
preserve this source discrepancy in the evidence. The source resolution is not
proof that the blind-choice/order/security-transfer primitives are complete.

Required Burst tests: hands of seven/eight/nine; activating player's blind
selection and exact bottom order; owner visibility without activator leakage;
unrevealed top security to bottom; both simultaneous trigger orders; Option
bounce from seven to eight then optional own unsuspend; direct Option-effect
attribution without Option-use events/cost/Arts; actual use/Arts separately;
source departure, OPT, printed routes/keywords and Burst end-turn cleanup.

## Integration and verification boundaries

Use failing native behavioral tests before YAML and positive/negative choice
coverage. Shared primitive changes must be separate commits. Recheck the prior
Tamer-source host-identity/cleanup witness; any correction overlaps existing
BT25/ST24/BT26 consumers, including Glowing Dawn BT26-057. Flag before integration.
Do not fold Maildramon's Solarmon/Q7764 full-cost witness into the Burst slice.

Production canonical ingestion and stable indices remain separate. Never push
private scratch admission inputs, synthetic fixtures presented as production
data, bulk caches, raw page dumps, or private replays. Scoped behavioral tests
and temporary-data smoke are not production admission. Keep incomplete cards
blocked in the ledger; do not infer admission from YAML existence.

Run focused native tests, existing admission checks and `scripts/verify --tier 2`;
report required broad suites and any actual skips/failures. Test the exact sample
only when all prerequisites pass. No training, paid Oracle, PR, merge, main push,
force-push or unrelated publication. Non-force pushes of this dedicated lane's
source/tests/concise notes are explicitly authorized, with remote SHA verification.

## Rebuild checkpoint: first four cards

New definitions and native behavioral tests for BT27-019 (12), BT27-023 (14),
BT27-024 (13), and BT27-047 (8) pass: **47 passed, 0 failed**. Native missing-YAML
RED preceded authoring (46 failures; one additional test was added before GREEN).
The first GREEN attempt passed 46/47; the remaining ordinary-evolution test used
an incorrect choice-only API and was corrected to the normal action decoder.
Final run: 47/47, 33.21 seconds. Spec and code review found no shared primitive
changes needed. These are newly rebuilt implementations, not recovered objects.

Reproduce from repository root with a Rust toolchain and ONNX Runtime installed:

```sh
export ORT_LIB_LOCATION=/path/to/onnxruntime/capi
export ORT_PREFER_DYNAMIC_LINK=1 LD_LIBRARY_PATH="$ORT_LIB_LOCATION"
export CARGO_BUILD_JOBS=2 RUST_MIN_STACK=268435456
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export CARGO_PROFILE_DEV_OPT_LEVEL=0 CARGO_PROFILE_TEST_OPT_LEVEL=0
cargo test --manifest-path code/digimon-engine/Cargo.toml --test bt27_lane_b
```

This run used the isolated stable Rust toolchain, ONNX Runtime Python wheel 1.24.2,
and an isolated local toolchain/venv. No production metadata or admission status
was changed. Runtime deck smoke, full tier-2 verification and production training
admission are **not yet passed**. Batch 2, Burst Mode, and exact-deck prerequisite
coverage remain outstanding at this checkpoint.

The authorized non-force push failed before publication with:
`fatal: could not read Username for 'https://github.com': No such device or address`.
A following remote ref read found no lane branch. No remote SHA is claimed.
Portable private Git bundles are the preservation route until credentials work.
