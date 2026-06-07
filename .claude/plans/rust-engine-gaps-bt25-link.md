# BT25 Link-subsystem engine/DSL gap fix-plan

Scoped from the BT25 gap→card table. **Gap 1 (Link +N aura, `ChangeLinkMax`
hardcoded 0) is DONE** (commit `11298c63`). This plan covers the four remaining
gaps, each a net-new substrate subsystem confirmed by source investigation.
All share `code/digimon-engine/src/{game_actions,effect_context,modifiers}.rs`
and the `digimon-dsl` crate, so they must be implemented sequentially (no safe
parallelization). TDD per gap: failing `tests/option_flow/link_flow.rs` (or
`tests/cards_behavioral/bt25/`) first.

Source priority for behavior: DCGO C# (`$BASE_DCGO`) → `general_rule.pdf` →
card image (printed text already captured below).

---

## Gap 5 — predicated WhenWouldLink cost-reduce (facet #10 predicated)

**Cards:** BT25-004 Tapmon (Digi-Egg, **inherited**), BT25-045 Onmon.
**Printed:** "[Your Turn] [Once Per Turn] When a [Social], [Tool] or [Game]
trait card would link to this Digimon, you may reduce the cost by 1."

**Why the flat path is insufficient:** the existing flat `ModifierType::ChangeLinkCost`
(`link_cost_delta_for_player`) reduces *all* of a player's link costs. This
reducer is **predicated on (a) the linked card's traits and (b) the host = the
effect's own Digimon**, plus optional / once-per-turn / your-turn. Auto-applying
it would over-reduce (an approximation; violates rule 17 optionality too).

**DCGO model:** `ChangeLinkCostClass` (`_changeCostFunc(cardSource, permanent, cost, root)`
+ `_cardSourceCondition` + `_permanentCondition` + `_rootCondition`) consulted at
`CardSource.GetChangedLinkCost(permanent, root)`. The OPT/"you may" is registered
during the `WhenWouldLink` window (`Owner.UntilCalculateFixedCostEffect.Add(...)`).

**Blocker found:** the `WhenWouldLink` replacement window (`begin_digimon_link`
→ `try_replace(WhenWouldLink, subject = Card(linking_card))`) **does collect
host-side effects** (`replacement.rs::collect_candidates` step 2 scans every
battle-area permanent), but the replacement context does **not expose the link
target host**, so a host-side effect cannot verify "to *this* Digimon", and the
replacement has no cost-reduce outcome (only cancel/substitute/none).

**Design:**
1. Thread the link host into the `WhenWouldLink` window. Add `Game.pending_link_host:
   Option<PermanentHandle>` set in `begin_digimon_link` (and the option-link +
   `link_chosen_card_into_host` paths) before `try_replace`, cleared after. Expose
   via `EffectReadContext::pending_link_host()`.
2. Host-side predicated reducer effect: a `WhenWouldLink`-timed effect whose
   `condition` checks `event/subject card` traits (the linking card — already the
   replacement subject) AND `pending_link_host() == source_permanent`. Optional +
   `once_per_turn` already supported by the replacement candidate machinery
   (`OptionalCancelWouldLink` is the optional template; `max_per_turn` honored in
   `collect_candidates`).
3. Cost-reduce path: the accept-branch installs a one-shot `ChangeLinkCost -N`
   player modifier on the linking card's owner with an expiry that lasts exactly
   the imminent link (consumed by `commit_digimon_link`'s `link_cost_delta_for_player`
   read). Simplest faithful reuse — no new replacement outcome needed.
4. DSL: new timing `when: when_would_link_to_this` (host-POV, mirrors the
   facet-#6/#11 `when_card_linked_to_this` pattern but at `WhenWouldLinkBattleArea`)
   + a `reduce_link_cost: N` step (or a `link_cost_reduction` body) gated by a
   card-trait `filter`. Lower to the replacement effect in (2)/(3).

**Tests:** linking a [Social] card onto the host pays cost−1 once per turn; a
non-matching trait pays full; a sibling host's copy doesn't reduce; decline path
pays full; OPT lockout after first use.

---

## Gap 2 — link-N-cards-per-host DSL step (G-DSL-LINK-N-CARDS-PER-HOST)

**Cards:** BT25-075 Vulcanusmon ("link **up to 2** cards from your hand or trash
to **any** of your Digimon without paying the cost"), BT25-060 Rebootmon ("link
1 [Appmon] card from hand or this Digimon's digivolution cards to **this**
Digimon without paying the cost"), BT25-089 Kazuki & Itsuki ("link 1 [Appmon]
card from hand or your Digimon's digivolution cards to 1 of your Digimon with
the cost reduced by 2").

**Substrate ready:** `Game::link_chosen_card_into_host(host, card, LinkCardSource)`
+ `EffectContext::link_chosen_card_into_host` (shipped commit `0df5c67e`). Gap is
the **DSL authoring step** over it.

**Design:** new `StepSpec::LinkCardsToDigimon` (compiled variant + lowering in
`dsl_cards/step/`):
- `from: [hand | trash | digivolution_sources_of_self | digivolution_sources_any]`
- `filter: PredicateSpec` (card-level, e.g. `{ trait_has: Appmon, kind: digimon }`)
- `to: self | own_digimon` (host: the effect's own permanent, or a selected own Digimon)
- `count: { exactly: N } | { up_to: N }` (up_to ⇒ each pick optional/PASS-able)
- `cost: free | { reduce: N }` (reduce computes effective = printed link cost −
  reduction; "without paying the cost" = free)

**Selection flow (per pick, looped up to N):** install a card selection across
the `from` zones (filter-matched) → on resolve, if `to: own_digimon` install a
host selection → call `link_chosen_card_into_host`. Reuse the pending-selection
state machine; model the loop on existing repeated-select steps. Mind the
optional-on-mandatory pitfall (memory note `reference_dsl_...`): only mark PASS
optional for the genuine "up to N" tail.

Note BT25-075 also wants "for each of your link cards, De-Digivolve 1 …" — that
scaling rider is a *separate* clause (existing `de_digivolve` + a link-card count
binding); out of scope for this step.

---

## Gap 3 — Option self-as-link-source + link-card leave-replacement

**Two distinct primitives; cards BT25-066, 073 (leave-replacement only), 101 (both).**

### 3a. Link-card-trash leave-replacement
**Printed (066 / 073-inherited / 101):** "[All Turns] When this Digimon would
leave the battle area, by trashing 1 of its link cards, it doesn't leave."
- Timing `EffectTiming::WhenWouldLeaveBattleArea` **exists** + `lower_replacement.rs`
  handles it; DSL `when: when_would_leave_battle_area` maps. The replacement
  `cancel()` (prevent leave) exists.
- **Missing:** a replacement **cost** that trashes one of the *leaving permanent's
  own link cards* (`ReplacementCostBody` currently only models `delay_self`).
  Add a `trash_own_link_card` cost variant (+ condition gate: `linked_cards.len()
  >= 1`), wired so paying it ⇒ `cancel()` (stays on field). Optional ("by trashing"
  = you may). DCGO: `OnTrashLinkCard.cs` / `TrashLinkedCards.cs`.

### 3b. Option self-as-link-source
**Printed (101):** "[Main] … you may link **this card** or 1 [TS] trait card from
your trash to 1 of your Digimon …" + "[Link] [Vulcanusmon]: Cost 3 (Plug this
card from the hand or battle area sideways …)".
- An **Option** that links **itself** as a persistent link card (not trashed on
  resolve). This is the Plug-In (Shape-A) path but the Option *is* the linked card.
  `link_chosen_card_into_host` already moves a chosen card from a zone onto a host
  — extend `LinkCardSource` with a `SelfOption`/hand-option variant, or route the
  Option's own handle through it. The "or 1 [TS] card from trash" branch is
  facet-#9 from-trash (already supported by the primitive).
- The leave-replacement (3a) applies to the linked Option too.

---

## Gap 4 — App Fuse primitive

**Cards:** BT25-036 Craftmon, BT25-060 Rebootmon ("App Fusion [names] & [names]:
Cost N. If 2 such cards are linked together, stack the link card on top and
digivolve"), BT25-089 ("[End of Your Turn][OPT] 1 of your Digimon may app fuse
into a Digimon card in the hand").

**New mechanic** (digivolution-like over linked Appmon cards). DCGO:
`CardEffectFactory.AddAppfuseMethodByName(List<string> names, card)` — registers
an App-Fusion alt-play: when the named Appmon cards are present as link cards on
a host (or in hand for 089), you may "stack the link card on top and digivolve"
into the App-Fusion result for the listed cost.

**Design (sketch — largest gap, needs its own assessment):**
- An `alt_paths` entry `kind: app_fusion { names: [...], cost: N }` registering an
  App-Fusion play method, analogous to the existing DigiXros/assembly alt-path
  machinery (`ChangeCardNamesForDigiXros`, `material`/assembly lowering).
- Resolution: consume the named link cards (DCGO "stack the link card on top and
  digivolve") → the App-Fusion result becomes the new top; the consumed cards
  become its digivolution sources. Fire the normal digivolve triggers.
- 089's variant fuses a *hand* card; 036/060 fuse linked cards.
- Recommend running `/assess-archetype-rust` on the Appmon/App-Fusion set first
  to enumerate every App-Fusion primitive before implementing.

---

## Ordering recommendation
2 (cleanest, builds on shipped primitive) → 5 (extends facet #10) → 3a
(self-contained replacement-cost) → 3b → 4 (largest; assess first).
