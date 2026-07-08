# Proposal: Implement EX12 Shambala + Virus Busters slices

## Why

EX12 is the newest release set and its two headline archetypes — **Shambala** (33 cards: the SW/TB twin sub-engines, the Tentei Hachibushu Lv.6 cycle, Lv.7 Susanoomon) and **Virus Busters** (21 cards: the Adventure partner lines, DUAL Siriusmon, Omnimon, Lv.7 Proximamon, Hiro) — have **zero cards implemented** in the Rust engine. Implementing them keeps the simulator current with the live meta and extends RL training coverage to the newest mechanics. The set also introduces **two new printed keywords (＜Guard＞, ＜Engage＞)** the engine cannot express today, so the work must widen the substrate first (CLAUDE.md rule 28) rather than route around them.

**The DCGO behavioral oracle now covers EX12** (2026-07-07): the submodule was bumped to upstream `a5e66480b` (Beta 1.16.9+), which implements **all 77 EX12 cards** (`Assets/Scripts/CardEffect/EX12/<Color>/EX12_*.cs`) plus first-class ＜Guard＞ and ＜Engage＞ keyword machinery (`Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/`, `CardEffectFactory/KeyWordEffects/`). Standard source priority applies again: DCGO C# is the behavioral authority (source-priority #2), consulted per card by implementers and reviewers. Official-DB bundles remain unavailable (`world.digimoncard.com` mid-restructure), so printed text still grounds on card scans, with conservative BLOCKED verdicts over guesses. Caveat: DCGO's EX12 scripts are freshly authored (not yet battle-tested like older sets), so scan-grounded review remains mandatory rather than deferring to DCGO blindly.

## What Changes

- **New keyword: ＜Guard＞** — "When any of your other Digimon would leave the battle area by your opponent's effects, by deleting this Digimon, they don't leave." `Keyword::Guard` + printed-keyword parse + auto-emitted protect-others leave replacement (rides the existing `protect_others` replacement substrate: cost `delete_self`, outcome `prevent`, cause scoped to opponent effects). Consumers: EX12-056, EX12-057 (Paishu tokens), EX12-072.
- **New keyword: ＜Engage＞** — "At the end of your turn, this Digimon may attack." `Keyword::Engage` + printed parse + end-of-turn optional attack window (Vortex-sibling machinery; exact target rules confirmed against rulings before implementation — the reminder text omits Vortex's played-this-turn allowance and Digimon-target clause). Consumers: EX12-019, EX12-060 (out-of-slice consumer noted for the parse).
- **New token species** registered in the token registry: **[Paishu]** (Yellow / 6000 DP / ＜Blocker＞ ＜Guard＞, EX12-057) and any others the assessment surfaces (e.g. [Kotenken], EX12-034).
- **Gap assessment phase** (54 cards, 8 batches) — re-run of the authored-but-credit-killed audit workflow (`wf_6f7700f2-6c5`): per-clause decomposition against current DSL/engine vocabulary, consolidated capability-centric gap entries, per-card SUPPORTED/PARTIAL/BLOCKED verdicts.
- **Gap-closure rounds** for whatever the assessment surfaces beyond the two keywords (TDD engine/DSL widenings, clone-safe, tracker resolutions).
- **54 card implementations** as YAML DSL specs + per-card DebugRunner behavioral tests, in review-gated waves (implementer → Opus reviewer grounded on the scan **and the card's DCGO C#**), including the DUAL EX12-018 Siriusmon (ST23-09/BT25-043 dual-YAML shape) and the Lv.7s.
- **Verdict tracking** in `qa/qa-reports/validated_cards_dsl.json`; gap trackers (`qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md`) updated per rule 28.
- **Capstone**: archetype interaction tests for both slices (`/archetype-interaction-test-author` composition — combo tests + the four static archetype tests).

_Not in scope_: the other EX12 slices (Deep Savers, Metal Empire, Night Soldiers/NSp, WG — including DUALs EX12-033/EX12-052 beyond their already-landed data), deck-library meta ingestion for EX12 archetypes, and frontend changes (the DUAL/keyword UI already generalizes).

## Capabilities

### New Capabilities
- `keyword-guard`: ＜Guard＞ keyword semantics — printed parse, protect-others leave-replacement behavior (opponent-effect cause scope, delete-self cost, optional), RL action-space exposure of the accept/decline choice, and token-carried instances.
- `keyword-engage`: ＜Engage＞ keyword semantics — printed parse, end-of-your-turn optional attack window, interaction with summoning sickness/unsuspension per confirmed rulings.
- `ex12-shambala-cards`: faithful DSL implementations of the 33 Shambala-slice cards (per-card behavioral coverage, verdicts, SW/TB sub-engine interactions).
- `ex12-virus-busters-cards`: faithful DSL implementations of the 21 Virus Busters-slice cards (per-card behavioral coverage, verdicts, incl. the DUAL Siriusmon and Gammamon-line placement mechanics).

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: any new DSL verbs/predicates the assessment surfaces beyond the two keywords (delta spec added during gap closure if assessment confirms; keyword grants themselves ride the existing `grant_keyword`/printed-parse requirements).

## Impact

- **Engine**: `code/digimon-engine/src/` — `enums.rs` (Keyword variants), `card_data.rs` (printed-keyword parse), `cards/keyword_effects.rs` + replacement/attack machinery (Guard/Engage behavior), `cards/tokens/` (new species).
- **DSL**: `code/digimon-dsl/` only if the assessment surfaces missing vocabulary (tracked via `qa/dsl-vocab-gaps.md`).
- **Cards/tests**: `code/digimon-engine/cards/ex12/*.yaml` (54 new), `code/digimon-engine/tests/cards_behavioral/ex12/` (54 new suites + mod wiring), `tests/archetypes/` (2 interaction suites), keyword tests under `code/digimon-engine/tests/`.
- **QA artifacts**: `qa/qa-reports/validated_cards_dsl.json`, `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/` model docs.
- **Data**: already landed (commit `e53df0d8e`) — no further `data/` changes expected beyond possible trait/Q&A reconciliations found during review.
- **Dependencies/risk**: DCGO oracle available for all 77 EX12 cards (submodule at `a5e66480b`, Beta 1.16.9+) but freshly authored — trust it for behavior per source priority, verify against scans; official DB down (no bundles); usage-credit availability gates the agent-driven phases.
