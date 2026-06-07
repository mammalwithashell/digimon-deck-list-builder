# Design: Release-Set Authoring Workflow

## Context

This design was developed in an explore session against live `digimoncard.io` data and the base-repo DCGO checkout. The numbers below are empirical, gathered during exploration, and are recorded here as the grounding for the architecture — re-measure on a real run rather than trusting these as fixed.

## The core reframe: a release set is a union of slices, not a flat list

A booster set (~102 cards) is designed as:

```
RELEASE SET = ⋃ (archetype/evolution slices the set seeds) + (orphan staples)
```

This is the load-bearing insight. It means:

- **Authoring** is already solved by `batch-implement-cards-rust-dsl` (`--cards <ids>`); the missing piece is set resolution + dependency-ordered batching.
- **Combo testing** runs **per slice**, not per set — `archetype-interaction-test-author` researches "an archetype as a system," and a booster has no single system. The set is the *authoring* unit; the slice stays the *testing* unit.
- **Orphan staples** (generic tech, vanilla draft-fodder, cross-archetype singletons) belong to no slice. They are authored but have no combo to test, so they are labeled and skip Phase 5.

## Empirical findings (BT17 and neighbors)

**Snapshot drift (motivates Phase 1):** live API vs `data/cards.json` — BT17 102=102 (exact), ST23/ST24 15=15, but **EX12 43 live vs 3 local**. The newest set is the stalest.

**Cross-set dependency is a category error for authoring (shapes Phase 5):** BT17 has 218 resolved named-card references in effect text; 47 point only to other sets; a one-level closure of "referenced cards not yet implemented" is **104 cards** across 12+ sets, and transitively the closure is ~the whole game. But:
- Evolution prerequisites are **generic** ("digivolve from Lv5 Red") — no specific card; DebugRunner **synthesizes** the lower card as a fixture (see memory `reference_debugrunner_empty_evo_costs`). Cross-set pull for chains: **zero**.
- Named references ("play `[Diaboromon]` from trash") are **self-contained for authoring** — the carrier plays whatever object exists; the referenced card's own effect is irrelevant to authoring the carrier. Cross-set pull for authoring: **zero**.
- A referenced card's *implementation* matters only when a **combo test** fires that card's behavior. So pull is **lazy and test-driven**, bounded to a handful, not an eager closure.

**New-keyword rate (motivates Phase 2):** bracket-token scan minus {card-names (1867), traits, timings, grammar, existing 34 Rust keywords} leaves ~1–3 genuine new mechanics per set, and they **recur**:

```
BT17: Plug-In
BT22: App Fusion, Link
BT24: App Fusion, Link, Petrification, Assembly(DigiXros-var)
BT25: App Fusion, Burst Digivolve
EX11: Petrification
```

The residual is dominated by **trait false-positives** when the trait lexicon is incomplete (130 traits sampled vs ~400+ real). Denoising requires (a) a complete trait lexicon and (b) a positional rule: a trait in effect text is almost always followed by the literal word "trait" (`[Aqua] trait Digimon`), whereas a keyword stands alone or carries a numeric param (`App Fusion -4`).

## Decision 1 — The keyword gate is a DCGO-oracle three-way, and a hard barrier

DCGO is the battle-tested behavioral reference (CLAUDE.md source priority #2). It has **no central keyword enum**; keyword behavior is scattered across:
- `I…Effect` interfaces in `Script/CardEffectInterfaces.cs` (74 interfaces; keyword ones include `IRushEffect`, `IRebootEffect`, `IBlockerEffect`, `IVortexCanAttackPlayersEffect`),
- `CardEffectFactory/Add*.cs` factory methods (e.g. `AddAppfusionMethod.cs`),
- core mechanism files (`BurstEffectObject.cs`, `ContinuousController.cs`, `AutoProcessing.cs`).

Verified against the residual:

| Candidate | DCGO evidence | Action |
|---|---|---|
| App Fusion | `AddAppfusionMethod.cs` | AUTO-INGEST |
| Link | `AutoProcessing.cs`, `CardController.cs` | AUTO-INGEST |
| Petrification | `ContinuousController.cs`, `CardEffectCommons.cs` | AUTO-INGEST |
| Assembly | `CardController.cs` (DigiXros family) | AUTO-INGEST |
| Burst Digivolve | `BurstEffectObject.cs` | AUTO-INGEST |
| **Unchained** | **none** | **FLAG-FOR-HUMAN** |

Triage logic per candidate:

```
already a Rust Keyword variant?     → COVERED (skip)
resolves to trait / card-name?      → LEXICON-MISS (patch list, continue)
in DCGO keyword manifest?           → AUTO-INGEST  (port C#→Rust, TDD, BARRIER)
not in DCGO?                        → FLAG-FOR-HUMAN (halt, report, await direction)
```

**Why a barrier, not a footnote:** auto-ingest writes core engine surface (a `Keyword` enum variant + DSL lowering + `keyword_effects.rs` wiring + a green DebugRunner test against DCGO behavior). Authoring ~90 dependent cards on top of a missing primitive violates no-approximations. So auto-ingest is its own gated sub-pipeline that must fully land before Phase 4. This deterministic barrier is the primary reason this is a **Workflow** (script-driven control flow) rather than a prompt-driven skill.

**Why a checked-in manifest, not a live grep:** DCGO's scattered surface makes "is it in DCGO?" too flaky to grep per run. `data/dcgo_keyword_manifest.json` is extracted once from the canonical surface identified in the fidelity audit below, refreshed on DCGO rebase alongside the rule-27 hook-verification checklist. It is the oracle. It also unifies Thread H: one maintained place holds {Rust-covered keywords, DCGO-available keywords, complete trait lexicon, card-name lexicon}, and the gate maintains the lexicons as it triages lexicon-misses.

### Fidelity audit — where DCGO's keyword surface actually lives

An initial guess ("`I…Effect` interfaces + `CardEffectFactory/Add*.cs`") was audited against the base-repo DCGO checkout and found **badly incomplete**. Corrected findings:

- **The canonical keyword registry is the UNION of two directories**, one file per keyword:
  - `Script/CardEffectFactory/KeyWordEffects/*.cs` (32 files)
  - `Script/CardEffectCommons/KeyWordEffects/*.cs` (29 files)
  - **Neither alone is complete** — `MindLink` exists only in Commons; `Link`, `ArtsDigivolve`, `BlastDigivolution`, `BlastDNADigivolution` exist only in Factory. Union = **33 keywords**. The original `Add*.cs` glob would have matched **zero** of these (files are `Rush.cs`, `Jamming.cs`, …), and the interface list covers only ~7 of the 33.
- **Some keywords are core-modeled and invisible to directory scanning.** Five Rust keywords appear in neither `KeyWordEffects/` dir — `SecurityAttackPlus/Minus`, `DrawX`, `DeDigivolve`, `DigiBurst` — because they are stat-modifier / draw / cost-marker mechanics handled directly in core processing (attack/security/draw), not as keyword-effect files. These can never be discovered by scanning, so the manifest must carry a **hand-curated core-keyword allowlist** for them. (Note: `Iceclad`, the keyword that motivated this worry, *does* have both a `KeyWordEffects/Iceclad.cs` file and an `IIcecladEffect` interface — the genuine directory-invisible cases are the stat/draw/cost keywords, not Iceclad.)
- **The manifest extractor therefore has three inputs:** (1) PRIMARY — union of both `*/KeyWordEffects/*.cs` dirs (the keyword name list); (2) SECONDARY — `I…Effect` interface names from `CardEffectInterfaces.cs` (cross-check + behavior-mapping hints); (3) TERTIARY — a curated allowlist of core-modeled keywords. Name normalization maps DCGO spellings to Rust enum spellings (`Pierce`→`Piercing`, `BlastDigivolution`→`BlastDigivolve`, …).

**Bonus the audit surfaced — the manifest is proactive, not just per-set reactive.** Diffing the 33-keyword DCGO registry against the 34-variant Rust enum shows DCGO implements three keywords the engine lacks: **`Link`** (confirmed from the BT22/BT24 set scan) **plus `Ascension` and `Blast DNA Digivolution`**, which no per-set scan had flagged yet. A manifest-based oracle thus surfaces pre-existing engine gaps that per-set grepping would only catch when a future set happens to use them. This is a direct argument for the checked-in manifest over live per-run greps.

**Auto-ingest is for keyword PRIMITIVES, not arbitrary effects.** The gate only auto-ingests printed keyword mechanics (the `<…>`/`[…]` keyword vocabulary). Card-specific effects remain the job of the per-card DSL pipeline in Phase 4. If a "keyword" turns out to be card-specific behavior masquerading as a keyword, it is reclassified as a normal card clause, not ingested as a primitive.

## Decision 2 — Multi-signal clustering with a labeled orphan bucket

No single signal recovers slices cleanly:
- **Trait** → membership, but noisy (multi-trait cards belong to 2 slices; generic traits like `Lesser` leak; data artifacts like a literal `"3"` appear).
- **Name-reference graph** → connectivity (links a tamer/option to its Digimon, chains evo lines through named digivolve conditions). Reference *frequency* surfaces marquee themes (BT17: Pulsemon ×15, Eosmon ×15, Argomon ×12, Diaboromon ×8).
- **Color + level** → intra-slice ordering (babies → megas) for dependency-correct authoring.

The clusterer combines these and emits slices plus an explicit `orphan-staples` bucket. A human-approval checkpoint on the slice partition is cheap insurance against mis-clustering before ~100 cards are dispatched. Slices that intersect an existing `deck_library.json` archetype inherit that archetype's name (so `archetype-interaction-test-author` can target it); brand-new slices (no meta data yet) get a synthetic name from their dominant trait + marquee Digimon.

## Decision 3 — Phase shape and barriers

```
PHASE 1  INGEST-DIFF      deterministic   pull ?card=<SET> → diff cards.json → ingest_cards.py merge
PHASE 2  KEYWORD GATE     1 agent + barrier   detect → DCGO triage → auto-ingest sub-pipeline | FLAG halt
PHASE 3  CLUSTER          deterministic + 1 agent confirm   multi-signal → slices + orphan bucket (user approves)
PHASE 4  MASS-IMPLEMENT   pipeline() over slices   each slice → batch-implement-cards-rust-dsl, Lv2→Lv7 order
PHASE 5  COMBO-TEST       pipeline() over non-orphan slices   archetype-interaction-test-author; lazy cross-set pull here
PHASE 6  SET GATE         deterministic   all cards IMPLEMENTED + cargo test green → set verdict tracker + report
```

- Phases 4 and 5 are `pipeline()` fan-outs (slice A combo-tests while slice B still implements — no inter-slice barrier).
- Phase 2 is the only hard barrier: a `FLAG-FOR-HUMAN` keyword halts the whole run; an auto-ingest must land before Phase 4 begins.
- Phases 1 and 6 are deterministic, no agents.

## Reuse boundary

Genuinely new code: Phase 1 ingest-diff wrapper, Phase 2 keyword gate + manifest extractor + lexicon maintenance, Phase 3 clusterer, Phase 6 set gate + tracker, and the Workflow script that sequences them. Phases 4 and 5 are existing skills invoked as stages — this change does not reimplement card authoring or interaction testing.

## Risks and open questions

- **Clusterer quality** is the main quality risk; mitigated by the Phase 3 approval checkpoint. A bad partition mis-targets combo tests but does not corrupt authoring (per-card YAML is slice-independent).
- **Manifest staleness** — if DCGO rebases and a keyword's representation moves, the manifest can go stale and mis-route a candidate. Mitigated by refreshing the manifest on rebase (rule-27 checklist) and by the gate erring toward `FLAG-FOR-HUMAN` on ambiguous matches (a false flag costs a human glance; a false auto-ingest ships a wrong primitive).
- **Auto-ingest faithfulness** — porting a keyword from DCGO C# to Rust must be TDD'd against DCGO behavior, not just compiled. The auto-ingest sub-pipeline reuses the `assess-archetype-rust` → widen-substrate discipline and must produce a green behavioral test before the barrier lifts.
- **Network dependency** — Phase 1 needs `digimoncard.io`. If unreachable, the workflow should fall back to the local snapshot with a loud warning rather than fail, since settled sets match exactly.
- **Action-space drift** — a keyword that exposes a new player choice changes the action space; the auto-ingest sub-pipeline must trigger the rule-27 `ActionSpace.cs` codegen and the drift-CI gate, not silently add an action.
