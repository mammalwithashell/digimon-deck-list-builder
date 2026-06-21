## Why

The digimoncard.io API that `data/cards.json` is ingested from silently drops printed card data that the **official Bandai global card DB** (`world.digimoncard.com`) preserves — most damagingly the `(Rule) Trait: Has [X] Type.` / `Has [X] attribute.` grants. These grants are a card's *printed* trait/attribute, but they live in a rules clause inside the effect box rather than the trait line, so the API captures neither the trait line entry nor the clause text. As a result the engine never tags these Digimon with the granted trait, and every requirement keyed on it fails in production: alt-digivolution paths (e.g. Skadimon EX11-017/EX8-028's printed "Digivolve Lv.5 w/[Ice-Snow] trait: Cost 3"), digivolve cost reductions (EX8-019), and search/play effects across the Ice-Snow line all stop recognizing their own archetype. A scan of the 438 already-scraped official cards finds **26** cards carrying a dropped grant, spanning Ice-Snow, Mineral/Rock LIBERATOR, the Iliad/TS "Tactician" cards, and `[Free]`-attribute Royal Knights — the real scope is larger across the full pool.

The official Bandai DB is the publisher's authoritative source and is **already wired into our tooling** (`build_card_bundles.py` → `data/card_official.json` + `data/card_bundles/`), established in commit `ded458ab` to recover the multi-colour digivolve costs the same API drops. We should finish what that session started — propagate the official data into the trait fields — and formally **promote `world.digimoncard.com` above DCGO and `cards.json` as our source for the printed/structured card data it authoritatively provides**, while keeping DCGO the authority for *behavioral* resolution and `general_rule.pdf` canonical for rules/timing.

## What Changes

- **Recover Rule-granted traits/attributes from the official DB.** Refresh `data/card_official.json` across the full card pool and parse `(Rule) Trait: Has [X] Type.` → granted *trait* and `Has [X] attribute.` → granted *attribute* from each card's official text.
- **Reconcile *all* authored-trait divergences from the official DB (scope expanded 2026-06-20).** The Rule grants are one of several ways `cards.json` drops printed traits: the API also drops the **`form` field** wholesale (so the Appmon mechanic's `Appmon` trait — which lives in form as `Stnd./Appmon` and is *required* by 37 cards — never reaches production) and decorates app-types with a `(App Name)` suffix. Every card whose authored DSL `traits:` exceeds production `CardData.traits` is reconciled against the official DB's `type`/`form`/`attribute` split (the official DB is authoritative); where the DSL declares a trait the official DB does **not** list, the DSL spec is corrected instead.
- **Propagate into our trait source of truth.** Emit `type_eng` / `attribute_eng` corrections into `data/card_overrides.json` (the same override channel the `ded458ab` session used for the 191 evo-cost corrections), so `apply_overrides` bakes them into `cards.json` and production `CardData.traits` carries them uniformly across every consumer (engine effects, the alt-digivolve `from: trait_has` match, deck legality, tensor).
- **Close the test-vs-production trait divergence.** Behavioral tests build `CardData` from the YAML `traits:` field, so a YAML-declared trait passes tests while production (built from `cards.json`) is missing it. Add a guard so this divergence can't silently recur, and reconcile the YAML `traits:` with the now-correct `cards.json`.
- **Promote `world.digimoncard.com` in the documented source priority.** Update `CLAUDE.md`'s "Source priority" section, the `digimon-card-lookup` skill's trust-order note, and the source-priority feedback memory: the official Bandai DB ranks **above DCGO and `cards.json`** for printed text, traits (incl. Rule-granted), digivolution costs/conditions, and official Q&A rulings. DCGO remains the authority for *how a card behaves/resolves*; `general_rule.pdf` remains canonical for rules/keyword/timing semantics.
- **Surface (not necessarily fix) the `[Free]`-attribute follow-up.** Engine attribute-predicate matching is currently unimplemented (`predicate.rs` `attribute_is` always returns false), so even with the data recovered, `Has [Free] attribute` requirements won't match yet. Scope the data recovery here; record the engine attribute-matching work as a tracked follow-up.

## Capabilities

### New Capabilities
- `official-card-data-sourcing`: Establishes the official Bandai DB (`world.digimoncard.com`) as the authoritative source for printed/structured card data, the recovery + propagation of Rule-granted traits/attributes into `cards.json` via overrides, the guard against YAML-vs-production trait divergence, and the documented source-priority hierarchy that promotes the official DB above DCGO and `cards.json` for the data it authoritatively provides.

### Modified Capabilities
<!-- No existing spec capability owns card-data sourcing or trait propagation; the
     CLAUDE.md source-priority hierarchy is documentation, captured as a requirement
     under the new capability. No spec-level requirement changes to existing specs. -->

## Impact

- **Data**: `data/cards.json` (corrected `type_eng` / `attribute_eng` for Rule-granted cards), `data/card_overrides.json` (new trait/attribute override entries), `data/card_official.json` + `data/card_bundles/<ID>.md` (refreshed over the full pool).
- **Tooling**: `code/tools/build_card_bundles.py` (full-pool refresh) plus a new parse/apply step (extract Rule grants from official text → emit overrides → re-run `apply_overrides`). Possibly factored alongside the existing `code/tools/audit_digivolve/` helpers.
- **Engine**: `code/digimon-engine/src/dsl_bridge.rs` (optional: thread `compiled.traits` into `CardData` as a defense-in-depth reconciliation) and a new guard test that production `CardData` traits match the authored YAML `traits:` for DSL cards. No change to the trait-matching logic itself (it already reads `CardData.traits` correctly).
- **Docs / process**: `CLAUDE.md` "Source priority" section, `.claude/skills/digimon-card-lookup/SKILL.md` trust-order note, and the source-priority feedback memory — all updated to promote `world.digimoncard.com`.
- **Follow-up (out of scope here, tracked)**: engine attribute-predicate matching (`predicate.rs` `attribute_is`) so recovered `[Free]`-attribute grants become matchable.
- **Risk / non-goals**: no gameplay-rules change; this is a data-fidelity + sourcing-policy change. DCGO's authority over behavioral resolution and the PDF's canonical rules role are unchanged.
