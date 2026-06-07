## Context

The apply spike found Q5 is blocked at two layers — missing card data AND a missing engine executor. The DSL `AltPathKind::Assembly` compiles but `CompiledAltPathKind::Assembly` is matched in no engine execution path (verified: only `Digivolve`/`DnaDigivolve`/`BlastDnaDigivolve`/`BurstDigivolve` are wired in `dna_digivolve.rs`, `dsl_bridge.rs`, `game_actions.rs`). `cards/_examples/BT18-102.yaml` only proves the DSL compiles the kind; there is no production assembly card.

### DCGO reference (the faithfulness target)

Read during the spike — the implementation mirrors this flow:

- **`AD1_025.cs:214-255`** — `AddAssemblyConditionClass` builds an `AssemblyCondition` with two `AssemblyConditionElement`s (`[WarGreymon]` count 1, `[MetalGarurumon]` count 1) and `reduceCost: 6`. The element `CanSelectCard` conditions check owner + `IsDigimon` + card name.
- **`SelectAssemblyClass.cs`** — the executor:
  - `CanFulfillConditions` / `CanFulfillEachElementCondition`: the card is playable via Assembly iff, for each element, the count of matching cards in the **owner's `TrashCards`** ≥ `element.ElementCount` (multi-element uses a recursive distinct-assignment check so the same trash card isn't double-counted).
  - `Select` → `SelectTrashCard`: per element, a `SelectCardEffect` with `root: Trash`, `customRootCardList: owner.TrashCards (minus already-chosen)`, `maxCount: element.ElementCount`, `canEndNotMax: false` (must pick exactly the count), `selectPlayer: owner`. The selection is shown — not auto-resolved.
  - `AddDigivolutiuonCards`: only if `selectedAssemblyCards.Count == assemblyCondition.elementCount`, each selected trash card is `AddDigivolutionCardsBottom` — placed UNDER the played card.
- **RULES_CONTEXT §7-3:** materials from trash; exact number must be placed (7-3-2-4); not mandatory (7-3-2-9); play cost reduced by the specified amount.

### Engine substrate that already exists (reuse, don't rebuild)

- `cards/_examples/BT18-102.yaml` + `AltPathSpec`/`MaterialSpec` (`zones`, `stack_under`, `repeat`) — the YAML surface compiles.
- DNA-digivolve play execution (`dna_digivolve.rs`) — the closest sibling (places 2 materials under, sets cost) to model the Assembly executor on.
- Trash-source selection (`select_count_capped_multi`, source-from-trash selectors used by BT24-017 et al.) — reuse for the per-element trash pick.
- Digivolution-stack bottom insert (used by DNA/digivolve) — reuse for placement.
- `calculate_play_cost` + the existing `via_alt_path` reduction plumbing — integrate the −N reduction.

## Goals / Non-Goals

**Goals**
- Faithful Assembly play execution (eligibility from trash, surfaced per-element exact-count trash selection, bottom placement, cost reduction, declare-then-pay) mirroring DCGO `SelectAssemblyClass`.
- AD1-025 carries `[Assembly]` in data + an `assembly` alt_path; Q5 pins.

**Non-Goals**
- The other ~10 `[Assembly]` cards; the `DigiXros`/`AppFusion` siblings; the `ChangeCardLevelForAssembly` interaction.
- `fix-judge-quiz-engine-gaps` / `add-grant-triggered-effect-dsl`.

## Decisions

### D1 — Model the Assembly executor on the DNA-digivolve play path
DNA digivolve already "places N materials under a played/evolved card at a set cost." The Assembly executor reuses that shape, differing in: materials come from **trash** (not battle area), cost is the base play cost **minus** the reduction (not a fixed DNA cost), and there is no digivolve target (it is a hand play). Rationale: minimal new surface; the placement + cost machinery is proven. Add a `CompiledAltPathKind::Assembly` arm to the play execution alongside the existing kinds.

### D2 — Materials from trash; per-element exact-count surfaced selection
Per `AssemblyConditionElement`, install a trash selection (`maxCount = element count`, must pick exactly that many — DCGO `canEndNotMax: false`), candidate set = controller's trash matching the element filter minus already-chosen, selecting player = controller. The selection surfaces through `pending_selection` (no-approximations §17 — the choice of WHICH WarGreymon/MetalGarurumon reaches the RL action space). Multi-element distinctness mirrors DCGO's recursive check (a trash card can't satisfy two elements).

### D3 — Eligibility = each element satisfiable from trash, AND declare-then-pay legal
The Assembly play is offered (in the action mask) when, for each element, the controller's trash holds ≥ `element count` matching cards (DCGO `CanFulfillConditions`). Legality also honors declare-then-pay: the declaration is legal when the **reduced** cost can be made payable (Q5's rule — memory at 0, `−6` makes 15→9 payable). Confirm the existing play-legality path computes affordability against the reduced cost at declaration, not the base cost; if it gates on base cost, that is the specific fix the Assembly arm must apply.

### D4 — Placement at the digivolution-stack bottom; pay reduced cost
On resolution, place the selected materials at the BOTTOM of the played card's digivolution stack (DCGO `AddDigivolutionCardsBottom`), only when exactly `elementCount` were chosen (RULES §7-3-2-4). Reduce the play cost by the reduction amount and pay it. The played card's own `[On Play]`/`[When Digivolving]` and keyword grants then fire as for any play.

### D5 — `cost:` on an assembly alt_path is the REDUCTION amount
The `[Assembly] -6` notation and DCGO `reduceCost: 6` mean the play cost is reduced by 6 (15 → 9 for Omnimon). Encode `cost: 6` on the assembly alt_path as the reduction, and document this in the lowering (distinct from DNA-digivolve's `cost:` which is the absolute final cost). Confirm/define this in the lowering during implementation; if the existing compiled representation can't distinguish "reduction" from "absolute," add an explicit marker.

### D6 — AD1-025 YAML + data
`data/card_overrides.json`: add `[Assembly] -6 [WarGreymon] x [MetalGarurumon]`. YAML alt_path:
```yaml
  - kind: assembly
    materials:
      - { name_contains: "WarGreymon", zones: [trash], stack_under: true }
      - { name_contains: "MetalGarurumon", zones: [trash], stack_under: true }
    cost: 6   # reduction (D5)
```
Confirm `name_contains` matching avoids "(X Antibody)" alias pitfalls (memory note) — Omnimon's materials are plain WarGreymon/MetalGarurumon.

### D7 — TDD, engine-first then card
Order: (1) engine executor with synthetic test cards (a hand card with an assembly alt_path + 2 named trash materials) — prove eligibility/selection/placement/cost; (2) AD1-025 data + YAML; (3) AD1-025 per-card behavioral test; (4) un-ignore Q5.

### D8 — Assembly is a post-PLAY_HAND selection flow, NOT a new play action (DCGO-confirmed)

DCGO (user-provided screenshots, AD1-025): the player plays the card via the normal play action, THEN the engine prompts the Assembly flow: an optional "use the assembly pieces from trash?" gate (offered only when the pieces are present), then a per-element surfaced trash selection ("Select [WarGreymon] from trash", then "Select [MetalGarurumon] from trash"), then the pieces are placed under and the reduced cost is paid. So Assembly rides the existing `PLAY_HAND` action and the existing `pending_selection` action ranges — **no new action-space range, no `ActionSpace.cs` regen, no RL-contract change** (the earlier apply-spike concern is resolved). The only mask change is declare-then-pay legality (D3): offer `PLAY_HAND` for an assembly-capable hand card when the *reduced* cost is affordable. The Assembly gate is optional (RULES §7-3 / DCGO "No Selection" button).

## Risks / Open Questions

- **Decline-then-unaffordable edge.** If the play is offered only because the reduced cost is affordable (declare-then-pay) and the player then DECLINES the optional Assembly gate, they'd owe the full cost they can't pay. DCGO offers "No Selection" but the play is already committed; model the faithful resolution (most likely: the Assembly gate is auto-skipped-as-declined only when the full cost is affordable; when the play was only affordable via Assembly, the gate is effectively required to complete — confirm against DCGO `SelectAssemblyClass` `canNoSelect`). For AD1-025/Q5 the player uses Assembly, so the happy path is unaffected; pin the edge with a test.
- **Declare-then-pay vs base cost (open — D3).** Does the current legality path test affordability against the reduced cost at declaration? Q5 hinges on it. If not, the Assembly arm computes the reduced cost for the legality check.
- **`cost:` semantic (D5).** Reduction vs absolute for assembly — confirm in lowering; add a marker if the compiled form is ambiguous.
- **Scope creep.** The executor is reusable for ~10 cards + the DigiXros/AppFusion siblings; resist generalizing beyond Assembly in this change (Non-Goal) — but write the helpers so a later change can reuse them.
- **Multi-element distinctness.** Ensure a single trash card can't satisfy two elements (DCGO recursive check). AD1-025 has two distinct-name elements so the risk is low, but the executor must be correct for the general case it will serve.
