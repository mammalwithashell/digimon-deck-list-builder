## Context

`data/cards.json` is ingested from the digimoncard.io API. That API is lossy: it drops the second colour of multi-colour digivolve lines (recovered in commit `ded458ab`) and — the subject of this change — it drops the `(Rule) Trait: Has [X] Type.` / `Has [X] attribute.` grants entirely (neither in the `type_eng`/`attribute_eng` fields nor in the effect text).

The Rust engine builds `CardData.traits` solely from `form_eng + attribute_eng + type_eng` ([card_data.rs:347](../../../code/digimon-engine/src/card_data.rs#L347)). Every trait match — including the alt-digivolution `from: { trait_has: ... }` predicate ([predicate.rs:2112](../../../code/digimon-engine/src/dsl_cards/predicate.rs#L2112)) — reads `CardData.traits`. The matching logic is correct and case-insensitive; only the data is missing. DSL YAML authors hand-encoded the missing trait in each spec's `traits:` field, and behavioral tests build `CardData` *from the compiled YAML* ([debug_runner.rs:1252](../../../code/digimon-engine/src/debug_runner.rs#L1252)), so the tests pass while production — which builds `CardData` from `cards.json` and reconciles only `ace_overflow` / `digixros_aliases` / `also_treated_as` / `dna_costs` (not `traits`) via [dsl_bridge.rs:115](../../../code/digimon-engine/src/dsl_bridge.rs#L115) — stays broken.

The official Bandai global DB (`world.digimoncard.com`) **preserves** the `(Rule)` clause in its card text and is already wired into our tooling: `code/tools/build_card_bundles.py` fetches it and writes `data/card_official.json` (438 cards today) + `data/card_bundles/<ID>.md`. A scan of the existing scrape finds 26 cards carrying a dropped grant (Ice-Snow, Mineral/Rock LIBERATOR, Iliad/TS, `[Free]`-attribute Royal Knights); the full pool is larger.

## Goals / Non-Goals

**Goals:**
- Recover `(Rule) Trait: Has [X] Type/attribute` grants from the official Bandai DB and make them present in production `CardData.traits` for every consumer (engine effects, alt-digivolve matching, deck legality, tensor).
- Fix the reported defect: Rule-granted Ice-Snow Digimon (EX7-016/020/021/023, EX11-014, EX8-019, …) recognized as Ice-Snow so alt-digivolution / cost-reduction / search requirements work.
- Formally promote `world.digimoncard.com` above DCGO and `cards.json` in the documented source priority for the printed/structured data it authoritatively provides.
- Prevent recurrence of the YAML-vs-production trait divergence.

**Non-Goals:**
- Changing trait-matching logic (already correct).
- Implementing engine attribute-predicate matching (`predicate.rs` `attribute_is`) — recorded as a tracked follow-up so recovered `[Free]` grants become matchable later.
- Re-sourcing card *behavior*: DCGO stays the authority for how a card resolves; `general_rule.pdf` stays canonical for rules/keyword/timing.
- A live network dependency at engine build/run time — the scrape output is committed and consumed offline.

## Decisions

### D1 — Source authority: official Bandai DB over DCGO/cards.json for printed data
The official DB is the publisher's source of truth for *printed* card facts (traits incl. Rule grants, digivolve costs/conditions, effect/inherited/security text, official Q&A). We promote it above both `cards.json` (lossy API) and DCGO (a community re-implementation) **for those data classes**. DCGO remains authoritative for behavioral resolution (processing order, interaction edges); the PDF remains canonical for rules semantics. Rationale: prefer the publisher's structured data over a derived API or a third-party reimplementation when the question is "what does the card print," reserving DCGO/PDF for "how does it resolve / what does the rule mean." Alternative considered: keep DCGO above the official DB everywhere — rejected because DCGO does not carry printed metadata (traits/costs) and is itself downstream of the same official text.

### D2 — Recovery: parse the Rule clause from official text, not the `type` field
The official `type` field is still just the trait line (e.g. `Dragonkin`); the grant lives only in the effect text (`(Rule) Trait: Has [Ice-Snow] Type.`). Recovery parses `text_sections` with a tolerant regex (`\(?Rule\)?\s*Trait:\s*Has\s*\[([^\]]+)\]\s*(Type|attribute)`), classifying `Type` → trait and `attribute` → attribute. Refresh `card_official.json` over the full Digimon pool first so coverage isn't limited to the current 438. Alternative considered: card-image vision — rejected as unreliable and already deprecated by `ded458ab` for digivolution data.

### D3 — Propagation: via `card_overrides.json`, not direct `cards.json` edits
Emit recovered grants as `type_eng` / `attribute_eng` entries in `data/card_overrides.json`, then run `apply_overrides` ([ingest_cards.py:369](../../../code/tools/ingest_cards.py#L369)) to bake them into `cards.json`. This is the exact channel `ded458ab` used for the 191 evo-cost corrections, and overrides **survive re-ingestion**. Because `apply_overrides` does a wholesale `dict.update` per field (no deep merge), each override MUST carry the **full** `type_eng` (existing line entries **plus** the granted trait) so no existing trait is dropped — e.g. P-215 must become `["Ice-Snow","Mineral"]`, not `["Mineral"]`. Build the override value from the official `type` field split on `/` ∪ the recovered grant.

### D4 — Engine: data fix is primary; reconciliation + guard are defense-in-depth
Once `cards.json` carries the trait, production works with **no engine change** (matching already reads `CardData.traits`). Two engine-side safeguards:
- (Optional) thread `compiled.traits` into `CardData` as a union inside `enrich_card_data_with_dsl_alt_paths` so a YAML-declared trait is never silently inert — a belt-and-suspenders that also covers any DSL-only card. Must be a union (never overwrite) to avoid dropping `cards.json` traits the YAML omitted.
- (Required) a guard test asserting that, for every DSL card, the production `CardData` trait set (cards.json path) is a superset of the authored YAML `traits:` — turning the silent divergence into a build failure.

### D4b — Scope expanded to comprehensive trait reconciliation (2026-06-20)
Prototyping the D4 guard revealed the Rule grants are only one of *three* ways the API drops printed traits, and the guard fails for 33 cards, only 6 of which are Rule-grant: (1) Rule grants; (2) the **`form` field**, dropped wholesale or partially — so the **Appmon** mechanic trait (printed in form as `<grade>/Appmon`, *required* by ~37 cards via `[Appmon]`-trait effects) never reaches production; (3) `(App Name)` suffixes on app-types (`Search (App Name)` vs official `Search`). Per the user, scope was expanded to reconcile **all** authored-trait divergences from the official DB, not just Rule grants. Tooling generalized: `reconcile_traits.py` (supersedes the Rule-grant-only `recover_rule_traits.py`) reconciles `type_eng` + `attribute_eng` everywhere in-scope and `form_eng` **only for Appmon cards** — deliberately *not* injecting evolutionary stages (Rookie/Champion/…) into non-Appmon cards, to bound the tensor/deck-legality blast radius to gameplay-relevant traits. In-scope = has a Rule grant OR carries `Appmon` in official form OR its DSL `traits:` exceed production. Cards whose DSL declares a trait the official DB does *not* list (e.g. BT10-029 `Lesser` vs official `Major`; BT25-070 `Logging` vs `Logoff`) are corrected in the YAML, not injected into `cards.json`.

### D5 — Documentation is part of the contract
The source-priority promotion lands in `CLAUDE.md` ("Source priority"), `.claude/skills/digimon-card-lookup/SKILL.md` (extend the existing "official DB > image > cards.json for digivolution data" note to cover traits/text generally), and the source-priority feedback memory. This is a normative requirement, not a nicety: it changes which source contributors and sub-agents consult first.

## Risks / Trade-offs

- [Override wholesale-replaces `type_eng` → could drop base traits] → Build the override from the official trait line ∪ grant; add a check that every override's `type_eng` ⊇ the pre-override `type_eng`.
- [Official-text wording variance ("(Rule)" vs "Rule", "Type" vs "type")] → Tolerant case-insensitive regex; log/diff the parsed grant count and eyeball against the known 26 before applying.
- [Full-pool scrape is ~4000 polite fetches] → Reuse the existing 0.4s-delay + 3-retry fetcher; resumable via `--ids-file`; commit the refreshed `card_official.json` so downstream steps are offline/reproducible.
- [Adding traits changes the observation tensor's trait features → may perturb trained models] → Accept: faithfulness outranks model stability, and these are correctness fixes; note in the change so eval/retraining is aware.
- [`[Free]`-attribute grants recovered but still unmatchable] → Out of scope by D-Non-Goals; tracked follow-up on `predicate.rs` `attribute_is`. The data is correct and ready when matching lands.
- [Scrape diverges from the local image mirror] → The card image stays the authoritative *visual* tiebreak; discrepancies get manually adjudicated before override emission.

## Migration Plan

1. Refresh `data/card_official.json` (full Digimon pool) via `build_card_bundles.py`.
2. Parse Rule grants → candidate `type_eng`/`attribute_eng` corrections; diff vs current `cards.json`; review.
3. Emit corrections into `data/card_overrides.json`; run `apply_overrides`; regenerate `cards.json`.
4. Rebuild the engine (cards.json is `include_str!`'d) and run the trait/alt-digivolve guard + affected behavioral/archetype suites.
5. Land the docs/source-priority updates.
6. Rollback: revert the `card_overrides.json` + `cards.json` diff (and the optional `dsl_bridge.rs` change); no schema/runtime migration involved.

## Open Questions

- Long-term trait source of truth: keep `cards.json` as the single SoT (YAML `traits:` becomes a derived mirror, enforced by the D4 guard), or invert and let YAML drive? Leaning `cards.json` since non-DSL consumers (deck tools, tensor) read it directly.
- Full-pool refresh now vs. phased (Digimon-only, then the rest)? Leaning full Digimon pool for completeness in one pass.
- Implement the optional D4 `dsl_bridge` union now, or rely solely on the data fix + guard? Leaning guard-only first (smallest surface), add the union if a DSL-only card surfaces.
