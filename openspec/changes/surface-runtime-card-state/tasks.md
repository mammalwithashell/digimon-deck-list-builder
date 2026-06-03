## 1. Layer A — printed effect text from card_data

- [ ] 1.1 In `serialization.rs::to_ui_json`, populate each stack source's `mainEffectText` and `inheritedEffectText` from `card_data[c.data_index]` (printed text), replacing the hard-coded `""`.
- [ ] 1.2 Confirm the exact `card_data` fields to use for printed main vs inherited text and that overrides (`card_overrides.json`-derived data) are respected by reading from the same loaded card data the engine resolves from.
- [ ] 1.3 Rust unit test: a constructed permanent with a multi-card stack serializes non-empty `inheritedEffectText` for sources that have printed inherited effects, and empty (not missing) for those that don't.

## 2. Permanent inheritedEffects array

- [ ] 2.1 Build the permanent-level `inheritedEffects` from the non-top digivolution sources, excluding the top card, with each entry attributable to its source (decide structured `{sourceCardId, text}` vs text-only per design Open Questions).
- [ ] 2.2 Rust unit test: a stacked permanent lists its sources' inherited effects; a single-card permanent (or Tamer) lists none; the top card's main effect is never listed as inherited.

## 3. Layer B — runtime-accurate active set

- [ ] 3.1 Enumerate the engine mechanisms that change which inherited effects apply (substitution/removal/suppression; cf. `dsl-inherited-substitute-trash`) and decide the v1 in-scope set.
- [ ] 3.2 Make `inheritedEffects` reflect the in-force set under those mechanisms (not just printed text); add a Rust test exercising a substitution/removal case for the in-scope mechanism(s).

## 4. Guardrails and parity

- [ ] 4.1 Regression test pinning that `dp`, `dpBreakdown`, `keywords`, `keywordBreakdown`, and `sources[].dpContribution` are unchanged by this change (shape and values).
- [ ] 4.2 Update `docs/RUST_PYTHON_PARITY.md` if this closes a listed divergence; otherwise note the surface as aligned.

## 5. Verification

- [ ] 5.1 Rebuild the binding (`maturin develop`) and confirm `to_ui_json` returns populated text via a quick state fetch.
- [ ] 5.2 Manual/Playwright check: open the in-game stack inspector on a stacked permanent and confirm inherited-effect text renders per source.
