## Context

The `[Link]` audit (2026-06-06) split the keyword into two card shapes that share `Permanent.linked_cards` storage. Shape A (Plug-In Options) is implemented end-to-end through the Option play flow. Shape B (Appmon Link Digimon, ~34 cards) is not implementable because the engine's only link-initiation path is `OptionSubtype::Link` inside `Game::resolve_option_outcome` — a branch of Option play. A Digimon cannot activate a link ability, and a standing Digimon permanent cannot be absorbed as a link.

DCGO's Shape-B model is concrete and is the shape guide here:

- `CardEffectFactory.AddSelfLinkConditionStaticEffect(permanentCondition, linkCost)` attaches a structured `card.linkCondition { cost, digimonCondition }` to the Digimon (a `timing == None` static effect).
- `CardEffectFactory.LinkEffect(card)` is a player-activated `ActivateClass` ("Link (Cost: N)") legal when it is the owner's turn, the card is in hand **or** is an un-linked battle-area Digimon, and at least one host satisfies `linkCondition.digimonCondition`.
- `ILinkCard.LinkCard()` computes a source `root` (`Hand`/`Trash`/`DigivolutionCards`/`LinkedCards`/`None`), fires `WhenWouldLink`, pays `GetChangedLinkCost`, then either `IPlacePermanentToLinkCards` (root `None`: absorb the whole standing permanent) or `permanent.AddLinkCard` (other roots: attach the single card), and the attach fires `WhenLinked`.

Rust already owns the back half of this contract (`WhenWouldLink`, `ChangeLinkCost`, `attach_linked_card`, host-deletion/return cascades, linked-untargetability). The design adds the **front half** (self link-condition + player-activated initiation + multi-zone source) and confirms the `WhenLinked`/ESS seams, reusing the existing pending-selection and trigger infrastructure rather than porting DCGO's object graph.

## Goals / Non-Goals

**Goals:**

- Represent a Digimon's self link-condition (cost + host filter) as card-level metadata available to `kind: digimon` cards.
- Add a player-activated link initiation path (on-field activate + from-hand link) that flows host selection through existing pending selections and the `FIELD_EFFECT` action range.
- Support link source origins beyond just-played Options: hand, trash, under-stack, re-link from another host, and standing-permanent absorb.
- Reuse the existing `WhenWouldLink` → cost → `attach_linked_card` → cascade machinery for the attach back-half.
- Confirm (with tests) and wire as needed: the linked Digimon's `WhenLinked` self-trigger and its ESS grant (DP + keywords) to the host.
- Provide DSL vocabulary to author the initial Appmon acceptance pool without raw Rust.
- Keep `ACTION_SPACE_SIZE`, tensor profiles, PyO3 exports, and frontend action constants stable.

**Non-Goals:**

- Authoring every Appmon Link card. This change defines the substrate, DSL vocabulary, and first acceptance fixtures.
- Reworking the existing Shape-A Plug-In Option link flow except where helpers are naturally shared.
- Reworking normal digivolution, DNA, DigiXros, or Blast flows.
- Adding hidden auto-picks, UI-only choices, or per-card approximations (no-approximations policy, rule 17).
- Porting DCGO code verbatim; DCGO is a behavioral reference.

## Decisions

### D1 — Self link-condition is card metadata, not an Option effect

Add a `LinkCondition { cost: u16, host_filter }` to `CardData` (or an adjacent registry), populated for `kind: digimon` cards by the DSL. The Shape-A `link_requirement` clause stays as-is for Options; Shape B reads the Digimon's own metadata.

Rationale: DCGO's `linkCondition` is static self-data consulted by both the activate-ability legality check and `ILinkCard` cost math. Modeling it as metadata lets `link_host_candidates` and the link-activate mask consult it uniformly across hand/field sources. Alternative considered: reuse the `OptionMain` `link_cost`/`link_filter` effect slot — rejected because those are timing-scoped to Option resolution and don't exist for a standing Digimon evaluating its own legality.

### D2 — Link initiation is a player-activated ability in the `FIELD_EFFECT` range

An un-linked battle-area Digimon with a `LinkCondition` and ≥1 legal host exposes a "Link (Cost N)" activated ability through the existing `FIELD_EFFECT` action range (`space.rs:74`). Resolving it installs a host-selection pending selection that threads into `attach_linked_card`. The from-hand link case (a Digimon linked directly from hand) is offered as a play-time alternative routed through the same host-selection install.

Rationale: on-field activated abilities already live in `FIELD_EFFECT`; link-activate is one more such ability and needs no new action ID. Host pick reuses the field-selection mask already used by Shape A (`install_link_host_selection`). Alternative considered: dedicated link action IDs — rejected (would force `ACTION_SPEC.md`, tensor, PyO3, and frontend changes, breaking model compatibility).

### D3 — Reuse the Shape-A attach back-half

After host selection, route through the existing `WhenWouldLink` window → `link_cost_delta_for_player` cost math → `attach_linked_card(host)` → `OnLink` dispatch → existing host-deletion/return-to-hand cascade. Shape B differs only in *what* gets attached and *from where*, not in the attach/cascade semantics.

Rationale: the back-half is already tested (`tests/option_flow/link_flow.rs`). Reuse keeps one cascade contract. Alternative considered: a parallel Shape-B attach — rejected as duplication that would drift from Shape A's cascade rules.

### D4 — Standing-permanent absorb is a distinct source path

For root `None` (a standing battle-area Digimon being linked), remove the whole permanent (top card + its sources) from the owner's battle area and place it into the host's `linked_cards` as a single linked entry, then fire `OnLink`/`WhenLinked`. This is the one source path with no Shape-A analog.

Rationale: DCGO's `IPlacePermanentToLinkCards` moves the entire permanent, not just the top card; the linked card must remember its stack for unlink/trash cascades. Alternative considered: only move the top card — rejected as unfaithful (loses under-sources and breaks return semantics).

### D5 — Source-zone allowances are explicit per link

The link initiation records which origin zones are legal (hand, trash, under-stack, linked-area, standing-field) for that specific link, matching the card's printed text. Most Shape-B cards are hand-or-field; trash/under-stack/re-link origins are opt-in per card text.

Rationale: mirrors DCGO's `root` discrimination in `ILinkCard` and keeps masks tight (no over-exposed illegal picks, per the optional-on-mandatory-selection pitfall in memory). Alternative considered: always allow all zones — rejected as over-exposure of illegal actions to the RL mask.

### D6 — `WhenLinked` self-trigger: confirm before adding a new timing

The audit marked `WhenLinked` (the linked Digimon's own "when I get linked" trigger) PARTIAL: `EffectTiming::OnLink` exists as a global observer whose enum doc says it "Mirrors DCGO `WhenLinked`," and `link_flow.rs::on_link_observer_fires_on_both_sides_after_attach` shows both sides fire. Task 1 writes a confirming test: a linked card's own `OnLink` effect, self-filtered, must fire on attach. If it passes, `WhenLinked` lowers to `OnLink` + self-filter and **no new timing is added**. Only if it fails do we add a dedicated `WhenLinked` timing.

Rationale: avoid speculative enum growth; let a test decide. Alternative considered: add `WhenLinked` eagerly — rejected pending evidence (rule 28: widen the substrate only when a card forces it).

**VERDICT (2026-06-06, task 1.1 — `tests/option_flow/link_flow.rs::d6_when_linked_via_on_link_fires_for_self_but_overfires_on_sibling`):** Confirmed. FACT 1 — an `OnLink` + `.linked()` effect on the link card DOES fire when that card attaches (the `enqueue_from_permanent` linked-card scan at `effect_queue.rs:1725` reaches it). FACT 2 — `OnLink` fires via `TriggerSource::PlayerBattleArea` (`game_actions.rs:2765`) with no just-linked-card identity, and the scan re-fires ALL of a host's linked cards every attach, so the same effect over-fires when a sibling links later. **Decision: lower `when: when_linked` to `OnLink` + a self-filter on a new "just-linked card" field added to the `OnLink` trigger context. No dedicated `WhenLinked` timing.** (Task 6.1.)

### D7 — ESS grant to host (DP + keywords like `Raid`): confirm via `.linked()` scope

The `.linked()` sideways-inherit scope already fires linked-card effects attributed to the host's controller. Task 1 writes a confirming test that a linked Digimon granting `Raid` (and a DP buff) to its host resolves through `.linked()` + an existing keyword-grant/DP modifier. If a keyword-from-linked grant doesn't reach the host's combat predicates, wire that consult site; do not add a parallel grant system.

Rationale: DCGO's `RaidSelfEffect(isLinkedEffect: true)` is an inherited-style grant, which is exactly what `.linked()` models. Alternative considered: a bespoke "link ESS" subsystem — rejected as duplication of the existing inherited/linked scope.

**VERDICT (2026-06-06, task 1.2 — `tests/option_flow/link_flow.rs::d7_linked_ess_keyword_grant_reaches_source_host_but_not_linked_host`):** Gap confirmed. The `.linked()` scope dispatches TRIGGERED effects at host timings, but CONTINUOUS/STATIC grants do NOT reach the host: `Game::has_keyword` (`game.rs:3406-3430`) scans only `card_sources` for inherited keyword grants, never `linked_cards` (the test's control proves the same `<Raid>` ESS reaches a host as a digivolution source but not as a linked card). DP grants share this consult class. **Decision: extend the inherited-grant scan in `has_keyword` (and the DP computation) to also scan `linked_cards` for declarative linked grants, reusing the existing inherited-grant machinery — no parallel system.** (Task 6.2.)

### D8 — DSL authoring mirrors reusable concepts, not card names

Add YAML vocabulary: a `link_condition` block on a `kind: digimon` card (cost + host filter), `when: when_linked` trigger timing, and a `scope: linked` ESS grant (already present for Shape A). DSL lowers these to the metadata + timings above. Gaps the DSL can't yet express are logged to `qa/dsl-vocab-gaps.md` (vocab) or `docs/RUST_ENGINE_GAPS.md` (engine primitive), per rule 28.

Rationale: keeps the compounding-coverage flywheel — each Appmon card after the first is cheaper. Alternative considered: hand-write each Appmon Digimon as raw Rust — rejected (rule 28 last-resort only).

## Risks / Open Questions

- **D6/D7 unknowns are load-bearing.** If `WhenLinked` and ESS-grant turn out *not* to be expressible on `OnLink`/`.linked()`, the change grows (new timing and/or a keyword-grant consult site). The acceptance tests in Task 1 resolve this before substrate work commits.
- **From-hand link vs. play action overlap.** Offering "link this Digimon from hand" alongside normal play must not double-count or mis-mask the hand card. Needs a clear legality rule (link-activate is offered only when a legal host exists and the card's text permits hand-origin link).
- **Standing-permanent absorb and summoning sickness / attack state.** Absorbing a suspended or attacking permanent mid-turn must follow DCGO's eligibility (un-linked, owner's turn); edge cases around an absorbed permanent that was itself a host need a test.
- **Tamers as hosts.** A few Appmon Tamers (Swipemon, Tapmon) interact with linking; confirm whether any Shape-B Digimon may link onto a Tamer (Shape-A already allows Option-on-Tamer via `<Linked>`).
