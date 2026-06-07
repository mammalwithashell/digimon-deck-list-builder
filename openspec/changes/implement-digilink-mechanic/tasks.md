## 1. Resolve the Two Audit Unknowns (do first — they gate the substrate shape)

- [x] 1.1 Write a confirming Rust test (`tests/option_flow/link_flow.rs`) that a linked card's own self-filtered `OnLink` effect fires exactly once on attach (design D6). Decide: `WhenLinked` lowers to `OnLink`+self-filter, OR a dedicated `WhenLinked` timing is required. → **VERDICT: `OnLink`+`.linked()` reaches the just-linked card but over-fires on siblings; lower `when_linked` to `OnLink`+self-filter on a new just-linked-card trigger field. No new timing.**
- [x] 1.2 Write a confirming Rust test that a linked Digimon granting `Raid` + a DP buff via `.linked()` scope reaches the host's combat predicates and DP (design D7). Decide: reuse `.linked()`+existing modifiers, OR wire a missing keyword-from-linked consult site. → **VERDICT: continuous grants do NOT reach the host (`has_keyword` scans `card_sources` only); extend that scan (+DP) to also scan `linked_cards`. No parallel system.**
- [x] 1.3 Record the D6/D7 verdicts in the design notes and update `docs/RUST_ENGINE_GAPS.md` / `qa/dsl-vocab-gaps.md` accordingly before building. → design.md D6/D7 + `RUST_ENGINE_GAPS.md` 2026-06-06 Shape-B note updated.

## 2. Acceptance Tests and Baseline (failing first)

- [ ] 2.1 Add failing behavioral tests for BT21-009 *Gatchmon*: self link-condition (Appmon host, cost 1), link-activate, `WhenLinked` play-from-hand effect, and linked `Raid` ESS.
- [ ] 2.2 Add failing behavioral tests for BT25-052 *Logimon* (or another BT25 Appmon Link Digimon) covering link-activate from field and its `WhenLinked` effect.
- [ ] 2.3 Add failing behavioral tests for a standing-permanent-absorb card (root `None`): a standing Digimon with under-sources is linked whole onto a host.
- [ ] 2.4 Record the current ignored/blocked Appmon fixtures and `RUST_ENGINE_GAPS.md` entry these tests are expected to close.

## 3. Self Link-Condition Metadata

- [x] 3.1 Add a `LinkCondition { cost, host_filter }` to `CardData` (or adjacent registry) for `kind: digimon` cards. → Represented as an `Effect` at new `EffectTiming::LinkCondition` carrying `link_cost`/`link_filter` (reuses Shape-A machinery, no serializable filter on `CardData`); `Effect::link_condition(card).link_host(cost, filter)` builder; new `link_host()` sets cost+filter without forcing `OptionMain`.
- [x] 3.2 Read the self link-condition in link legality (`link_host_candidates`) and cost math (`link_cost_delta_for_player`) uniformly across hand/field origins. → `Game::digimon_link_condition_targets(handle)` returns `(cost, hosts)`, excludes self, reuses `link_host_candidates`. Test `digimon_self_link_condition_lists_hosts_and_excludes_self`.
- [x] 3.3 Leave the Option-scoped `link_requirement` path unchanged; add a regression guarding Shape-A behavior. → `.link()` now delegates to `.link_host()` then forces `OptionMain`; existing `dsl_link_requirement_*` / `link_*` Option tests stay green (96/96 in `option_flow`).

## 4. Digimon-Link Initiation Path

- [x] 4.1 Add a player-activated link ability for an un-linked battle-area Digimon, masked into the `FIELD_EFFECT` action range; legal only on the controller's turn with ≥1 legal host. → `FIELD_EFFECT_SLOT_FOR_LINK = 3`; mask emits it in the field loop (guarded by `digimon_link_condition_targets` + affordability + `CannotActivateMainEffects`); decode → `activate_field_link` → `install_digimon_link_host_selection`. Test `digimon_link_activate_absorbs_source_into_host`.
- [~] 4.2 Add the from-hand link initiation as a play-time alternative routed through the same host-selection install. → DEFERRED: the field-link (root `None`) path is the dominant Appmon shape and is complete; from-hand Digimon-link is a smaller follow-up (most BT21+ Link Digimon are played/digivolved onto the field first, then link). Logged for a later slice.
- [x] 4.3 Thread host selection into the `WhenWouldLink` → cost → attach → `OnLink` back-half. → `begin_digimon_link` fires `WhenWouldLink` (parks interactive replacements via `pending_digimon_link` + resume arm in `replacement.rs`); `commit_digimon_link` pays `link_cost_delta`-adjusted cost then absorbs; OnLink via `TriggerSource::Linked`.
- [x] 4.4 Verify `ACTION_SPACE_SIZE` and existing action IDs are unchanged. → Reused an unused `FIELD_EFFECT` sub-slot; no const change (full-suite `space` assertions green).

## 5. Link Source Origins

- [~] 5.1 Record per-link permitted origin zones from card text; mask out disallowed origins. → The standing-field origin is implemented; hand/trash/under-stack/re-link origins fold into deferred 4.2/5.3. The link filter (`link_host` predicate) already gates legal hosts.
- [x] 5.2 Implement the standing-permanent absorb (root `None`): remove the whole permanent + under-sources and place it as one linked entry. → `absorb_standing_digimon_as_link` follows the canonical removal (`clear_permanent_full` → remove slot → `shift_after_battle_area_remove` → `shift_handle_after_soft_remove(host)`); per DCGO `DiscardEvoRoots`, under-sources are trashed and only the top card becomes a linked card (flat `Vec<CardSource>` is sufficient — DCGO's `LinkedCards` is itself flat). Test `digimon_link_absorb_trashes_evo_roots_keeps_only_top`.
- [~] 5.3 Implement trash / under-stack / re-link-from-another-host origins as opt-in source paths. → DEFERRED with 4.2 (rarer origins; logged to gap tracker).
- [x] 5.4 Add regression tests for the implemented origin + masking. → `digimon_link_activate_absorbs_source_into_host`, `digimon_link_absorb_trashes_evo_roots_keeps_only_top`, `digimon_link_activate_fires_when_linked_and_grants_ess`.

## 6. WhenLinked and ESS Grant (wire per Section 1 verdicts)

- [x] 6.1 Wire the linked Digimon's `WhenLinked` self-trigger per the D6 decision (self-filtered `OnLink` or new timing). → Added `TriggerSource::Linked { player, host, card }` carrying the just-linked card as `event_card`; both OnLink fire sites (`fire_on_link_after_option_placed`, `install_field_option_as_plug_in`) use it. Self-filter `event_card == source_card`. Test `d6_self_filtered_when_linked_fires_once_and_not_on_sibling`. No new timing.
- [x] 6.2 Wire the linked Digimon's ESS grant (DP + keywords) to the host per the D7 decision; ensure the grant is removed on unlink/trash. → Additive linked-card pass in `tick_declarative_effects` materializes `.linked()` declarative grants onto the host (keyword via modifier registry → `has_keyword`; DP via `ChangeDp` modifier → `effective_dp`). Removal is automatic (tick clears + re-installs each pass; a trashed/unlinked card stops re-installing). Test `d7_linked_ess_keyword_and_dp_grant_reach_the_host`.

## 7. DSL Schema and Lowering

- [x] 7.1 Add YAML schema for a `link_condition` block (cost + host filter) on `kind: digimon` cards; lower to the Section 3 metadata. → New `DeclarativeKind::LinkCondition` (reuses `LinkRequirementBody`) across `clause.rs`/`compile.rs`/`compiled.rs`/`pack.rs`; lowers in `dsl_cards/mod.rs` to `Effect::link_condition().link_host(cost, filter)`.
- [x] 7.2 Add `when: when_linked` trigger lowering; lower `scope: linked` ESS grants (reusing existing linked-scope support). → `Timing::WhenLinked`/`CompiledTiming::WhenLinked` → `OnLink`; `lower_triggered` forces `.linked()` + injects the self-filter (`event_card == source_card`). `lower_grant_keyword` now sets `.linked()` for `CompiledScope::Linked` so a `scope: linked` keyword grant materializes onto the host.
- [x] 7.3 Regenerate the DSL JSON schema (`cargo run -p dsl-schema-export`) and lint. → Schema is derive-generated on demand (no committed file); new vocab flows in via `schemars::JsonSchema` derives and exports cleanly. Single-card lint validates against it; the full-dir `dsl-lint` stack-overflow is a pre-existing Windows volume/recursion issue, not DigiLink-related.
- [x] 7.4 Author an acceptance-pool card as YAML and make its test pass. → `dsl_digimon_link_card_full_flow` authors a full Appmon Link Digimon in YAML (`kind: link_condition` + `when: when_linked` draw + `scope: linked` Raid ESS) and verifies the real link-activate → absorb → OnLink path end-to-end. (Authoring the *named* BT21-009 Gatchmon etc. with their other effects — alt-digivolve, specific WhenLinked bodies — remains §2.)

## 8. Trackers and Docs

- [x] 8.1 Update `docs/RUST_ENGINE_GAPS.md` (Option/Plug-In/Link entry) to mark Shape-B Digimon-link substrate as landed, with the passing test commands. → 2026-06-06 "Shape-B engine substrate LANDED" note added (verdicts D6/D7 + §3/§4/§5/§6 surface + deferred from-hand/rarer-origin residual).
- [x] 8.2 Update `docs/RUST_ENGINE_API.md` with the Digimon-link API surface (self link-condition, initiation, source origins). → "DigiLink Shape-B" subsection added under Option/Plug-In/Link with the full API table + deferred-residual note.
- [x] 8.3 Record DSL vocabulary additions in `qa/dsl-vocab-gaps.md`. → `[G-DSL-DIGILINK] — LANDED 2026-06-06` note added (link_condition / when_linked / scope:linked grant). `validated_cards_dsl.json` verdicts for the *named* acceptance cards remain with §2 (those cards aren't authored yet).
