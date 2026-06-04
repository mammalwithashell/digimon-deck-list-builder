## Why

Judge-quiz Q5 ("Is using `[Assembly]` with AD1 Omnimon a legal game action?", judge: YES) is blocked by **two** layers, both discovered during the apply spike:

1. **Card data** — AD1-025 Omnimon's `[Assembly]` keyword is absent from `data/cards.json` (only `<Raid>/<Blocker>/<Partition>`, the `[On Play]` body, and `xros_req = [DNA Digivolve] … Cost 0` were ingested). The real card HAS it: DCGO `AD1_025.cs:214-255` (`AddAssemblyConditionClass`, elements `[WarGreymon]` ×1 + `[MetalGarurumon]` ×1, `reduceCost: 6`).
2. **Engine — no Assembly play execution.** The DSL has an `AltPathKind::Assembly` that *compiles* (only `cards/_examples/BT18-102.yaml` uses it), but `CompiledAltPathKind::Assembly` is matched in **no** engine play/digivolve execution path — only `Digivolve`/`DnaDigivolve`/`BlastDnaDigivolve`/`BurstDigivolve` are wired. There is no production assembly card and no executor that places materials from trash under the played card, applies the cost reduction, or offers the play. So authoring data + YAML alone would be a silent no-op and Q5 could not pass.

This change delivers the **whole faithful solution**, mirroring DCGO's Assembly framework: implement Assembly play execution in the engine, add the missing keyword to card data, author AD1-025's `assembly` alt_path, and pin Q5. The executor is reusable (the `[Assembly]` keyword family spans ~10 cards: AD1-009, AD1-012, BT22-078, BT24-062/081, EX9-047, EX11-036/045/046).

## What Changes

- **Engine — Assembly play execution (faithful to DCGO `SelectAssemblyClass.cs`).** Wire `CompiledAltPathKind::Assembly` into the play flow:
  - **Eligibility / declaration.** Offer the Assembly play for a hand card whose assembly materials are each satisfiable from the controller's **trash** (per element, count of matching trash cards ≥ the element count — DCGO `CanFulfillConditions` / `CanFulfillEachElementCondition`). The declaration is legal whenever the reduced cost can be made payable (declare-then-pay — the rule Q5 asserts). Assembly is **not mandatory** (RULES §7-3-2-9).
  - **Material selection (surfaced, not auto).** Per element, install a selection over the controller's trash (`maxCount = element count`, exact count required — RULES §7-3-2-4), so the choice reaches the RL action space (no-approximations §17). DCGO `SelectTrashCard` (`SelectCardEffect` root = Trash).
  - **Placement + cost.** Place the selected materials at the **bottom** of the played card's digivolution stack (under it — DCGO `AddDigivolutionCardsBottom`), and reduce the play cost by the specified amount before payment.
- **Card data — add the `[Assembly]` keyword to AD1-025** via `data/card_overrides.json` (survives re-ingest).
- **Card YAML — author AD1-025's `assembly` alt_path** (`materials: [WarGreymon, MetalGarurumon]`, `zones: [trash]`, `stack_under: true`, cost reduction 6) alongside the existing `dna_digivolve`.
- **Tests — pin Q5** + engine play-execution tests + an AD1-025 per-card behavioral test.

## Capabilities

### New Capabilities
- `assembly-play-execution`: The engine can play a hand card via its `[Assembly]` alt-path — offering the play when the materials are satisfiable from the controller's trash, surfacing a per-element trash selection (exact count, RL-visible), placing the chosen materials at the bottom of the played card's digivolution stack, and reducing the play cost by the specified amount with declare-then-pay legality. AD1-025 Omnimon faithfully carries its printed `[Assembly] -6 [WarGreymon] x [MetalGarurumon]` keyword and is playable via this path.

### Modified Capabilities
<!-- The Assembly executor is NEW surface (no execution exists today). If wiring it requires changing an existing play/alt-path capability's requirements rather than extending them, a MODIFIED delta will be added at that point. -->

## Impact

- **Engine (Rust):** the play/alt-path execution path (`game_actions.rs` play flow + `dsl_cards/` alt-path handling + `dna_digivolve.rs`/`dsl_bridge.rs` siblings) gains an `Assembly` arm; the action mask (`action/mask.rs`) offers the Assembly play; a trash-material selection (reuse `select_count_capped_multi` / source-from-trash selection) is installed per element; placement reuses the digivolution-stack-bottom insert; cost reduction integrates with `calculate_play_cost`.
- **Card data:** `data/card_overrides.json` (AD1-025 `[Assembly]`).
- **Card content:** `code/digimon-engine/cards/ad1/AD1-025.yaml` (`assembly` alt_path).
- **Tests:** new engine tests under `tests/dsl/` or `tests/selection/` for assembly eligibility/selection/placement/cost; un-ignore `judge_quiz` Q5; AD1-025 per-card behavioral test.
- **Trackers:** new `G-ASSEMBLY-PLAY-EXECUTION` entry resolved in `qa/archetype-qa/engine-gaps.md` → `qa/resolved-gaps.md`; update `judge-quiz.md`/`card-resolution.md` Q5 (BLOCKED-DATA → reclassified to engine + data, then PASS).
- **RL contract — NO action-space change (DCGO-confirmed flow).** Assembly rides the existing `PLAY_HAND` action: after the play, an optional "use assembly?" gate (only when pieces are in trash) and a per-element trash selection surface through the existing `pending_selection` action ranges. The only mask change is declare-then-pay legality (offer the play when the reduced cost is affordable). No new range, no `ActionSpace.cs` regen.

## Non-Goals

- Authoring the other ~10 `[Assembly]` cards — this change authors only AD1-025; the rest land via normal card authoring once the executor exists.
- Wiring the other unwired material-under-play siblings (`DigiXros`, `AppFusion`) — out of scope; only `Assembly` is implemented here (note the executor may share helpers they later reuse).
- The `fix-judge-quiz-engine-gaps` and `add-grant-triggered-effect-dsl` changes (separate, no overlap).
- The `ChangeCardLevelForAssembly` modifier interaction (cards that alter level for assembly) — not needed for AD1-025; deferred unless a consumer requires it.
