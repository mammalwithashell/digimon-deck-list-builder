# Archetype DSL Implementation: BT25 link-finish-aura slice
Date: 2026-06-07
Total cards in pool: 3
Processed this run: 3
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 0
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 2
- BLOCKED (dsl): 1
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0  (all 3 had prior BLOCKED verdicts → re-attempted)

All three cards were previously BLOCKED on `G-ENGINE-AURA-GRANT-LINK-MAX`
(aura `modifier: ChangeLinkMax` applied a hardcoded +0). That gap is now
**RESOLVED** (2026-06-07): the DSL aura body carries `modifier_value`
(`clause.rs:339`) and `lower_aura.rs` threads `modifier_value.unwrap_or(0)`
into both the self-aura and target-set modifier-application paths. The new
`link_cards` step (landed 2026-06-07, doc-named for these very cards) also
closes the link-from-source half. After re-attempt, each card remains BLOCKED
on a *different, newly-narrowed* core clause.

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-102 | Factorial Area | IMPLEMENT | BLOCKED (engine) | n/a | 0/0 | security-zone-sourced field aura |
| BT25-060 | Rebootmon | IMPLEMENT | BLOCKED (engine) | n/a | 0/0 | App Fuse primitive |
| BT25-075 | Vulcanusmon | IMPLEMENT | BLOCKED (dsl) | n/a | 0/0 | own-link-card-count De-Digivolve formula |

## Engine-Gap Blocked Cards

### BT25-102 Factorial Area
- Effect text: "[Security] [All Turns] All of your Black or Red [TS] trait Digimon gain ＜Blocker＞ ... While you have [Vulcanusmon], they also gain ＜Link +1＞."
- Missing engine API: a continuous static aura **sourced from a face-down Option in the security zone** that grants keywords/modifiers to battle-area Digimon. DCGO `BT25_102.cs` registers the Blocker/ChangeLinkMax static effects at `EffectTiming.None` gated on `IsExistInSecurity(card, false)`. The DSL/engine aura tick only emanates from battle-area carriers + digivolution sources; nothing lets a security-zone card register a field aura.
- New gap: `G-ENGINE-SECURITY-ZONE-SOURCED-FIELD-AURA` (qa/archetype-qa/engine-gaps.md).
- Note: the Link+1 value-carry (former blocker) is now fine; the [Main] bottom-security swap + play-with-reduction and the inherited [Security] play-from-hand/trash are individually expressible. Only the always-on security-zone board buff blocks.

### BT25-060 Rebootmon
- Effect text: "App Fusion (Bootmon, Shutmon)" — `AddAppfuseMethodByName` alt-play path; plus Security+1, Reboot, Link+1, link-to-self→unsuspend, when-linked/on-unsuspend OPT Piercing+Blocker+effect-immunity.
- Missing engine API: **App Fuse** — no `app_fuse` primitive anywhere in the engine/DSL (`AltPathKind::AppFusion` parses but resolves to nothing). Tracked in `docs/RUST_ENGINE_GAPS.md` (Gap 4 / App Fuse entry; BT25-060 added to card list 2026-06-07).
- All other Rebootmon clauses are now expressible (link_cards `to: self`, when_linked + on_unsuspend tokens, Link+1 via modifier_value, security_attack, Reboot, grant_effect_immunity). App Fuse is the sole blocker — but it is a printed alt-play option, so omitting it violates no-approximations → BLOCKED, not PARTIAL.

## DSL-Vocab-Gap Blocked Cards

### BT25-075 Vulcanusmon
- Effect text: "[On Play][When Digivolving] link up to 2 cards from hand/trash to any of your Digimon free. **Then, for each of your link cards, ＜De-Digivolve 1＞ all of your opponent's Digimon.**"
- Missing DSL verb: a `FormulaSpec`/`PerSelector` source counting **own link cards** (Σ `permanent.linked_cards.len()` over the controller's battle-area Digimon). DCGO `BT25_075.cs`: `degenerationCount = own Digimon → LinkedCards → Flat().Count()`, then `IMassDegeneration(enemy, 1)` × N.
- Lowers to engine API: substrate exists (`Permanent.linked_cards`, counted at `game_actions.rs:1494` / `tensor_v1.rs:267`); only a DSL formula selector + evaluator is missing.
- New gap: `G-DSL-FORMULA-OWN-LINK-CARD-COUNT` (qa/dsl-vocab-gaps.md). Suggested `{ own_link_card_count: { of: you } }` formula + a `repeat_n: <formula>` wrapper (DCGO applies mass De-Digivolve-1 N separate times, not De-Digivolve-N once).
- The link half (`link_cards` from:[hand,trash] to:own_digimon count:{up_to:2} cost:free) and the [All Turns] Rush + Link+1 aura are now expressible; the De-Digivolve magnitude is the wall.

## New Patterns Discovered
- None requiring RUST_DSL_TEST_API.md changes (all three BLOCKED before YAML/tests).

## Notes on the gap-landscape shift
The slice's name ("link-finish-aura") captures the three mechanics it stresses:
link-from-source (now closed via `link_cards`), a finishing board action
(De-Digivolve / bottom-security swap), and a Link+/keyword aura (Link+1
value-carry now closed). The aura gap that originally blocked all three closed
this cycle; the residuals are now cleanly separated: one engine substrate gap
each for security-zone auras (102) and App Fuse (060), and one DSL formula gap
for link-card-count (075).
