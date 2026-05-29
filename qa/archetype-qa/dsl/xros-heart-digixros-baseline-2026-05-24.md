# Xros Heart DigiXros Baseline

Date: 2026-05-24

Changes: `close-xros-heart-digixros-gaps`,
`author-xros-heart-reusable-primitives`

Assessment workflow: `.codex/skills/assess-rust-engine-archetype/`

## Verdict

`primitive-ready`

The first Xros Heart DigiXros substrate slice is now implementable in Rust YAML
DSL. The engine has a first-class DigiXros play transaction for material
selection, before-payment cost reduction, selected material source attachment,
transaction-scoped origin hooks, pre-attached materials, and deletion-timed
`<Material Save>` filtered through the carrier's DigiXros recipe.

The follow-up reusable-primitive slice now covers under-Tamer card flow,
generalized source movement and source rescue, turn-scoped DigiXros wildcard
substitution, and effect-created attack windows. The representative production
YAML cards are authorable without `raw_rust` placeholders. This does not mean
every historical Xros Heart, Blue Flare, Twilight, or Bagra Army card is
complete; broader readiness still depends on card-by-card authoring and any
non-Xros-specific residual primitives discovered by later cards.

## Acceptance Fixtures Added

These behavioral tests defined the first closure target and now pass with
production YAML:

| Card | Fixture | Current baseline failure | Gap expected to close |
| --- | --- | --- | --- |
| `BT10-009` Shoutmon X4 | `code/digimon-engine/tests/cards_behavioral/bt10/bt10_009.rs` | `code/digimon-engine/cards/bt10/BT10-009.yaml` | DigiXros recipe declaration, material selection, cost reduction, source attachment, and `digixros_count`. |
| `BT10-087` Taiki Kudo | `code/digimon-engine/tests/cards_behavioral/bt10/bt10_087.rs` | `code/digimon-engine/cards/bt10/BT10-087.yaml` | Optional cast-time transaction modifier granting under-Tamer materials for one pending DigiXros play. |
| `BT12-112` Shoutmon X7: Superior Mode | `code/digimon-engine/tests/cards_behavioral/bt12/bt12_112.rs` | `code/digimon-engine/cards/bt12/BT12-112.yaml` | Pre-attached material selection before cost is fixed, trash material access, and selected-source attachment. |
| `BT10-013` Shoutmon X5 | `code/digimon-engine/tests/cards_behavioral/bt10/bt10_013.rs` | `code/digimon-engine/cards/bt10/BT10-013.yaml` | Deletion/removal-timed optional `<Material Save 3>` over recipe-eligible sources, Tamer destination selection, and no main-phase Material Save action. |
| `BT21-083` Taiki Kudo | `code/digimon-engine/tests/cards_behavioral/bt21/bt21_083.rs` | `code/digimon-engine/cards/bt21/BT21-083.yaml` | Start-main stash under self plus optional attack window for a just-played or just-digivolved Xros Heart/Hero Digimon. |
| `BT11-095` Taiki Kudo | `code/digimon-engine/tests/cards_behavioral/bt11/bt11_095.rs` | `code/digimon-engine/cards/bt11/BT11-095.yaml` | Start-main stash under self plus under-Tamer materials available to one DigiXros play. |
| `P-224` Shoutmon + Star Sword | `code/digimon-engine/tests/cards_behavioral/p/p_224.rs` | `code/digimon-engine/cards/p/P-224.yaml` | Hand/trash union stash under self plus reduced-cost play from under Tamers. |
| `BT19-090` Xros Heart option | `code/digimon-engine/tests/cards_behavioral/bt19/bt19_090.rs` | `code/digimon-engine/cards/bt19/BT19-090.yaml` | Modal play from under Tamer or unsuspend-and-attack option mode. |
| `BT21-092` Xros Heart option | `code/digimon-engine/tests/cards_behavioral/bt21/bt21_092.rs` | `code/digimon-engine/cards/bt21/BT21-092.yaml` | Move source cards under a Tamer, bind moved count, and reduce the follow-up hand play by that count. |
| `BT10-111` Shoutmon (King Version) | `code/digimon-engine/tests/cards_behavioral/bt10/bt10_111.rs` | `code/digimon-engine/cards/bt10/BT10-111.yaml` | Turn-scoped DigiXros wildcard requirement substitution without `raw_rust`. |
| `BT21-027` Shoutmon DX | `code/digimon-engine/tests/cards_behavioral/bt21/bt21_027.rs` | `code/digimon-engine/cards/bt21/BT21-027.yaml` | Trait-filtered leave-battle source rescue, distinct from recipe-filtered Material Save. |
| `BT19-061` Sparrowmon | `code/digimon-engine/tests/cards_behavioral/bt19/bt19_061.rs` | `code/digimon-engine/cards/bt19/BT19-061.yaml` | DigiXros-only treated-as alias, search/trash split, and hand-or-trash stash on deletion. |

Focused baseline commands:

```bash
cargo test -p digimon-engine --test cards_behavioral bt10_009 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt10_087 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt12_112 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt10_013 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt21_083 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt11_095 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral p_224 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt19_090 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt21_092 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt10_111 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt21_027 -- --nocapture
cargo test -p digimon-engine --test cards_behavioral bt19_061 -- --nocapture
cargo test -p digimon-engine --test dsl xros_reusable_primitives -- --nocapture
```

## Remaining Follow-Up Fixtures

- `code/digimon-engine/cards/_examples/BT12-112.yaml` now mirrors the
  production transaction-modifier pattern instead of marking the printed
  Shoutmon hook as omitted.
- `code/digimon-engine/cards/_examples/BT10-111.yaml` has been removed after
  `BT10-111` was promoted to production YAML with native wildcard-substitution
  DSL.

## Tracker Entries This Change Should Close or Narrow

- `docs/RUST_ENGINE_GAPS.md`: the Xros Heart DigiXros transaction, Material
  Save substrate, under-Tamer card flow, generalized source movement/rescue,
  wildcard substitution, and effect-created attack-window primitives are
  closed for the acceptance pools; arbitrary Apocalymon-style cast-time
  assembly remains a separate open gap.
- `qa/archetype-qa/engine-gaps.md`: Xros Heart narrows from reusable engine
  substrate to remaining card-specific authoring or later non-Xros residuals.
- `qa/dsl-vocab-gaps.md`: `kind: digixros`, transaction modifiers,
  pre-attached materials, material origin-zone extensions, Material Save
  recipe filtering, under-Tamer selectors/play, source-count formulas,
  wildcard substitution, and immediate attack prompts are no longer open
  Xros Heart DSL-vocabulary gaps.

## No-Approximations Constraints

- Do not add dedicated DigiXros action IDs or change `ACTION_SPACE_SIZE`.
- Material picks, pre-attach picks, Tamer destination choices, and optional
  accept/decline prompts must flow through pending selections and existing
  action ranges.
- Do not promote example YAML or raw-Rust placeholders as "implemented" until
  the production YAML and focused behavioral tests pass.
