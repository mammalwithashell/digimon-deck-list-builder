# dsl-card-scripting-vocabulary Specification

## Purpose
TBD - created by archiving change unblock-medusamon-partial-cards. Update Purpose after archive.
## Requirements
### Requirement: Declinable `trash_self` activation cost

The card-scripting DSL SHALL accept `trash_self: true` as an `activation_cost` for a triggered clause, alongside the existing `suspend_self` and `return_self_to_deck_bottom` costs. The three cost kinds MUST be mutually exclusive — a clause specifying more than one is a compile error. A `trash_self` activation cost SHALL be declinable: when the trigger fires, the controller is offered an accept/decline choice, declining skips the entire clause body, and accepting trashes the source card and then runs the body. This makes `<Delay>` "by trashing this card" a true declinable cost per Comprehensive Rules 16-16-2.

#### Scenario: Author declares a `trash_self` activation cost

- **WHEN** a card YAML declares a triggered clause whose first body step is `activation_cost: { trash_self: true }`
- **THEN** the clause compiles and the cost is lifted onto the clause's activation cost rather than treated as a mid-body step

#### Scenario: Controller declines the activation cost

- **WHEN** a clause with a `trash_self` activation cost triggers and the controller declines
- **THEN** the source card is not trashed and the clause body does not run

#### Scenario: Controller accepts the activation cost

- **WHEN** a clause with a `trash_self` activation cost triggers and the controller accepts
- **THEN** the source card is moved to its owner's trash and the clause body resolves

#### Scenario: Mutually exclusive cost kinds

- **WHEN** a clause declares `activation_cost` with `trash_self` set together with `suspend_self` or `return_self_to_deck_bottom`
- **THEN** compilation fails with an error stating the cost kinds are mutually exclusive

### Requirement: Alt-path digivolution requirements can gate on a source card's printed text

An alternate-digivolution-path `from:` source filter SHALL be able to gate on whether a candidate source card has a given keyword (such as `<Save>`) printed in its effect text, so a printed digivolution requirement of the form "Lv.N w/<Keyword> in text" can be expressed.

This capability is provided by the DSL's existing `effect_text_contains` predicate — a case-insensitive substring scan of the candidate card's printed text (effect, inherited, and security text). When an alt-path `from:` filter is matched, the engine evaluates the filter against the candidate source permanent via `eval_predicate`, so `effect_text_contains` works there with no new predicate. (Implementation note: the change's design first proposed a dedicated `keyword_in_text` predicate; implementation found `effect_text_contains` already covers the need — and "w/<Keyword> **in text**" is itself worded as a text-presence check — so no redundant verb was added.)

#### Scenario: Source card has the keyword in its text

- **WHEN** an alt-path `from:` filter uses `effect_text_contains` for a keyword marker and a candidate source permanent's top card has that marker printed in its effect text
- **THEN** the candidate satisfies the filter and the alt-path is offered for that source

#### Scenario: Source card lacks the keyword

- **WHEN** the same filter is evaluated against a candidate source permanent whose top card does not have the marker in its text
- **THEN** the candidate does not satisfy the filter and the alt-path is not offered for that source

#### Scenario: Combined with other `from:` predicates under OR

- **WHEN** an alt-path `from:` filter combines the text-presence check with another predicate (such as `trait_has`) under an `any_of`
- **THEN** a candidate satisfying either branch is offered the alt-path

### Requirement: DSL supports material-count aggregate predicates

The DSL SHALL provide a permanent predicate that evaluates whether a candidate permanent's material count is tied for an aggregate material count among a referenced player's battle-area Digimon. Material count means digivolution stack size minus the top card. The predicate SHALL support at least `fewest_materials`, SHALL compose with existing filters such as `kind: digimon`, and SHALL include all tied candidates.

#### Scenario: All Digimon tied for fewest materials match

- **WHEN** a filter uses `materials_count_matches_aggregate: { selector: fewest_materials, of: opponent }`
- **AND** the opponent has Digimon with 0, 0, 1, and 2 materials
- **THEN** both 0-material Digimon satisfy the predicate
- **AND** the 1-material and 2-material Digimon do not satisfy the predicate

#### Scenario: Non-Digimon candidates are excluded by composed filter

- **WHEN** the aggregate predicate is composed with `kind: digimon`
- **THEN** opponent Tamers and other non-Digimon permanents do not satisfy the composed filter

### Requirement: DSL supports formula-valued De-Digivolve amounts

The `de_digivolve` step SHALL accept a formula-valued amount in addition to the existing literal amount. The formula SHALL evaluate at effect resolution time using the resolving effect context, and the resulting amount SHALL be passed through the normal De-Digivolve caps, immunity checks, and configured stop-at-level floor. DSL-authored `de_digivolve` steps that omit `stop_at_level` SHALL default to the normal level 3 floor, so card YAML that represents standard printed `<De-Digivolve N>` text preserves the floor even when using `amount_fn`. Non-standard stack-clearing effects that intentionally ignore the level 3 floor SHALL use a raw Rust/helper path that explicitly calls the engine primitive with no floor.

#### Scenario: De-Digivolve amount equals own Digimon count

- **WHEN** a `de_digivolve` step uses `amount_fn` based on the controller's Digimon count
- **AND** the controller has three Digimon when the effect resolves
- **THEN** the engine attempts to De-Digivolve the selected target by 3
- **AND** normal stop-at-level and available-source caps still apply

#### Scenario: Formula-valued standard De-Digivolve preserves the level 3 floor

- **WHEN** a standard printed `<De-Digivolve>` effect is authored with `amount_fn`
- **AND** the target stack contains a Digi-Egg under a level 3 card
- **THEN** the YAML-authored step SHALL preserve the standard level 3 floor
- **AND** resolving the effect SHALL NOT trash the level 3 card or expose the Digi-Egg

#### Scenario: Literal De-Digivolve remains supported

- **WHEN** a `de_digivolve` step uses the existing literal `amount` field
- **THEN** it compiles and resolves with the same behavior as before this change

#### Scenario: Non-standard unbounded stack trash remains expressible outside default DSL lowering

- **WHEN** a card's printed text requires trashing digivolution cards without the standard De-Digivolve level 3 floor
- **THEN** a raw Rust/helper implementation MAY call the engine De-Digivolve primitive with no stop-at-level floor for that non-standard effect
- **AND** that usage SHALL remain distinct from standard DSL-authored printed `<De-Digivolve>` text

### Requirement: DSL supports predicate-scoped timing suppression

The DSL SHALL allow card authors to suppress activation of specific effect timings for permanents matched by a predicate-scoped modifier. The suppression SHALL support `[When Attacking]` and `[When Digivolving]` timings and SHALL apply through the shared timing-dispatch path so face-up, inherited, and granted effects from affected permanents are blocked consistently.

#### Scenario: Affected permanent cannot activate When Attacking

- **WHEN** a permanent is affected by a modifier that suppresses `[When Attacking]`
- **AND** that permanent attacks
- **THEN** its `[When Attacking]` effects are not enqueued or activated
- **AND** unaffected permanents still activate their legal `[When Attacking]` effects

#### Scenario: Affected permanent cannot activate When Digivolving

- **WHEN** a permanent is affected by a modifier that suppresses `[When Digivolving]`
- **AND** that permanent digivolves
- **THEN** its `[When Digivolving]` effects are not enqueued or activated
- **AND** global observer effects from other unaffected sources are not suppressed unless their own source permanent is affected

### Requirement: `choose_from_reveal { optional: true }` requires printed-text "may"

The DSL primitive `choose_from_reveal` accepts an `optional: bool` field that, when `true`, lets the player decline the pick via the standard PASS action even when eligible candidates exist in the revealed pool. Card authors SHALL set `optional: true` ONLY when the printed card text explicitly grants the player permission to decline at that specific pick (printed wording variants include "you may add", "you may place", "may choose to add/place", and similar "may" formulations applied to the pick itself).

When the printed card text states the pick as an unconditional add (e.g., "Add 1 card with the [X] trait..."), the pick is mandatory and the YAML SHALL either omit `optional` (the default is `false`) or set it explicitly to `false`. The "no eligible candidates" case SHALL be handled by the engine's natural fizzle path — the bucket auto-skips when zero candidates match the filter — and SHALL NOT be modeled as a player-driven optional decline.

This rule applies to every `choose_from_reveal` invocation in `code/digimon-engine/cards/**/*.yaml`. Authors faced with a mandatory two-pick "Add 1 X and 1 Y" reveal-search pattern SHOULD prefer the `select_reveal_buckets` primitive (see BT24-031 Elecmon as the canonical reference), which surfaces a single combined bucket prompt and forbids `optional` by design.

The cost-payment surrounding a `choose_from_reveal` is orthogonal to the pick's `optional` field — a top-level effect clause MAY be `optional: true` (modeling a "by paying X..." optional activation) while the inner `choose_from_reveal` that follows the cost payment is mandatory. The two flags express different player choices: whether to activate the effect at all, versus whether to decline a specific pick once the effect is already mid-resolution.

#### Scenario: Mandatory "Add 1 trait card" pick rejects PASS

- **WHEN** a DSL clause uses `choose_from_reveal` with `optional: false` (or omitted) and the revealed pool contains at least one card matching the filter
- **THEN** the engine SHALL surface a pending selection whose `options` list contains the eligible card slots and SHALL NOT accept a PASS action (action_id 62) as a decline path — submitting PASS leaves the selection in place or returns an `ok: false` selection rejection

#### Scenario: Mandatory pick with zero candidates fizzles silently

- **WHEN** a DSL clause uses `choose_from_reveal` with `optional: false` and the revealed pool contains zero cards matching the filter
- **THEN** the engine SHALL skip the pick step without raising a pending selection, and any subsequent process steps (e.g., `order_remainder`) SHALL execute against the unchanged revealed pool

#### Scenario: Optional pick honors PASS decline

- **WHEN** a DSL clause uses `choose_from_reveal` with `optional: true` reflecting a printed-text "may" pick, and the revealed pool contains eligible candidates
- **THEN** the engine SHALL surface a pending selection with the eligible candidates AND SHALL accept PASS as a valid decline, after which subsequent process steps execute as if the pick produced no card

#### Scenario: Optional cost wrapping a mandatory pick

- **WHEN** a top-level effect clause is `optional: true` (modeling a "by paying X..." optional activation) and its `process` includes a `choose_from_reveal` step with `optional: false` after the cost is paid
- **THEN** declining the top-level activation SHALL skip the entire clause (no cost, no pick), while accepting the activation SHALL pay the cost and then surface the mandatory pick — declining the inner pick via PASS SHALL NOT be accepted in this case

### Requirement: `may_dna_digivolve_now` step verb for inline DNA digivolve at trigger fire

The DSL SHALL provide a `may_dna_digivolve_now` step verb that, when executed inside a triggered clause's body, surfaces the DNA digivolve UI inline at trigger fire time and (on player accept) merges two on-field permanents plus a target hand card into a new Digimon. The step's contract is:

- `anchor`: PermanentRef (defaults to `source`) — one DNA material is fixed to this permanent.
- `partner_filter`: PermanentFilter — predicate over own-field permanents for the OTHER DNA material; the anchor SHALL be excluded automatically by the step implementation regardless of whether the filter mentions the exclusion.
- `target_filter`: CardFilter — predicate over the controller's hand for the result Digimon card.
- `cost`: u16 (defaults to 0) — memory cost paid before the merge.
- `ignore_requirements`: bool (defaults to false) — when true, bypasses the digivolve target's normal requirement checks.
- `optional`: bool (defaults to false) — when true, the step prompts the controller accept/decline before any material selection.
- `prompt`: Option<String> — optional override for the accept/decline prompt copy.

The step SHALL call `EffectContext::effect_initiated_dna_digivolve(anchor, partner, target_hand_card, cost, ignore_requirements)` after both selections resolve. The post-merge trigger cascade (`WhenDigivolving → OnDnaDigivolve → OnDigivolve` per the existing primitive's docstring) executes as part of the step's resolution, so the new Digimon's own enter-field effects fire before control returns to the surrounding trigger batch.

#### Scenario: Step prompts accept/decline when `optional: true`

- **WHEN** a triggered clause's body executes `may_dna_digivolve_now` with `optional: true`
- **THEN** the controller is prompted accept/decline via the engine's standard optional-step surface
- **AND** picking decline resolves the step with no state mutation

#### Scenario: Step selects partner from own field excluding the anchor

- **WHEN** the controller accepts the optional prompt (or the step has `optional: false`)
- **AND** the anchor permanent exists on the controller's battle area
- **THEN** the next pending selection is a `SelectionKind::SelectPermanent` over own-field permanents matching `partner_filter`
- **AND** the anchor permanent is excluded from the selection candidates regardless of whether `partner_filter` references the exclusion

#### Scenario: Step selects target from controller's hand

- **WHEN** the controller has selected a partner permanent
- **THEN** the next pending selection is a `SelectionKind::Hand` over the controller's hand matching `target_filter`
- **AND** only Digimon cards in the controller's hand are eligible (the verb's printed-text contract presumes a Digimon target)

#### Scenario: Step calls `effect_initiated_dna_digivolve` after both selections

- **WHEN** both partner and target selections resolve
- **THEN** the engine calls `EffectContext::effect_initiated_dna_digivolve(anchor, partner, target_hand_card.handle(), cost, ignore_requirements)`
- **AND** the post-merge trigger cascade (`WhenDigivolving → OnDnaDigivolve → OnDigivolve`) fires and drains as part of the step's resolution
- **AND** the new merged Digimon's own `[On Play]` / `[When Digivolving]` effects resolve before the outer trigger batch resumes

#### Scenario: Step is a clean no-op when no eligible partner or target exists

- **WHEN** the step executes with no own-field permanent matching `partner_filter` (other than the anchor)
- **OR** with no hand card matching `target_filter`
- **THEN** the step does NOT install a pending selection — no accept/decline prompt, no partner prompt, no target prompt
- **AND** the surrounding trigger resolves with no body effect (silent skip)

### Requirement: `alt_path_registration { kind: dna_digivolve }` is deprecated for `[End of Your Turn]` printed-text patterns

When a card's printed text reads "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand" (or a structurally equivalent inherited end-of-turn "may DNA digivolve" clause), the card YAML SHALL use `may_dna_digivolve_now` inside a triggered `end_of_your_turn` clause and SHALL NOT use `alt_path_registration { kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn }` to express the same printed text. The `alt_path_registration` mechanism remains valid for cross-turn registrations or for alt-paths whose printed-text semantic genuinely deferred-availability; it is deprecated only for the inline at-EoT printed-text pattern.

#### Scenario: New card with [EoT] DNA digivolve inherited authors via `may_dna_digivolve_now`

- **WHEN** a card author adds a new YAML for a card whose printed inherited text reads "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand"
- **THEN** the YAML's inherited end-of-turn clause uses `may_dna_digivolve_now`
- **AND** the YAML does NOT use `alt_path_registration { kind: dna_digivolve }` for this clause

#### Scenario: Migration of legacy alt_path_registration cards

- **WHEN** a previously authored card uses `alt_path_registration { kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn }` for the printed inherited EoT DNA digivolve pattern
- **THEN** the card's YAML is migrated to `may_dna_digivolve_now` as part of this change (or a follow-up audit)
- **AND** the card's behavioral test is updated to assert the new clause shape

### Requirement: DSL supports no-choice bottom-source trash

The DSL SHALL provide a step for trashing the bottom N digivolution source cards from a resolved permanent target without presenting a source-card choice to the player. The step SHALL accept a target binding and a positive count, SHALL trash sources from the bottom of the target's stack in bottom-up order, SHALL cap naturally at the number of available source cards, and SHALL route each trashed source card to its owner's trash.

This primitive is for printed text such as "Trash the digivolution card at the bottom of 1 of your opponent's Digimon" and "Trash 2 digivolution cards at the bottom of 1 of your opponent's Digimon." It SHALL NOT replace `select_own_sources`, `select_opponent_sources`, or other player-choice source selectors when printed text says the player chooses source cards.

#### Scenario: One bottom source is trashed with no source prompt

- **WHEN** a DSL process selects an opponent Digimon and then executes bottom-source trash with count 1
- **THEN** the bottom source card under that opponent Digimon is moved to its owner's trash
- **AND** no pending source-selection prompt is installed

#### Scenario: Two bottom sources are trashed in order

- **WHEN** a target permanent has three source cards under its top card
- **AND** a DSL process executes bottom-source trash with count 2
- **THEN** the two lowest source cards are moved to their owners' trash in bottom-up order
- **AND** the remaining source and top card stay on the permanent

#### Scenario: Count caps to available sources

- **WHEN** a target permanent has one source card under its top card
- **AND** a DSL process executes bottom-source trash with count 2
- **THEN** the one available source card is trashed
- **AND** the top card is not trashed
- **AND** the engine does not panic

#### Scenario: Player-choice source selectors remain distinct

- **WHEN** printed text requires the controller to choose a source card
- **THEN** the card YAML SHALL use a source-selection primitive rather than bottom-source trash
- **AND** the action mask SHALL expose the legal source choices

### Requirement: DSL can evaluate the opposing battled Digimon's source count

The DSL SHALL provide a battle-context predicate usable by inherited or aura-style effects to test the currently opposing battled Digimon's source count. The predicate SHALL only evaluate as true while a Digimon-vs-Digimon battle context exists, SHALL inspect the opposing battle participant relative to the source carrier, and SHALL be false during security checks, player attacks, and other non-Digimon-battle contexts.

#### Scenario: Opposing battler has no sources

- **WHEN** a Digimon carrying an inherited effect battles an opponent Digimon whose stack contains only its top card
- **AND** the inherited effect condition checks that the opposing battled Digimon has no source cards
- **THEN** the condition evaluates true for that battle

#### Scenario: Opposing battler has sources

- **WHEN** the opposing battled Digimon has one or more source cards
- **THEN** the no-source battled-opponent predicate evaluates false

#### Scenario: No battle opponent context exists

- **WHEN** the carrier attacks a player or performs security checks
- **THEN** the no-source battled-opponent predicate evaluates false
- **AND** any DP or keyword grant gated by that predicate is not applied for that non-battle context

#### Scenario: Predicate resolves relative to the carrier

- **WHEN** both players have Digimon involved in the battle
- **THEN** the predicate inspects the opponent of the carrier permanent, not merely any no-source Digimon on either battle area

### Requirement: DSL can gate inherited effects on source-carrier battle deletion survival

The DSL SHALL provide a reusable predicate or helper that allows an inherited effect to detect that its source carrier deleted that carrier's battle opponent in battle and that the source carrier survived the battle. The predicate/helper SHALL compose with existing timing, owner, cause, and once-per-turn gates. It SHALL NOT match unrelated battle deletions caused by another friendly Digimon, attacks on players, effect deletion, or battles where the source carrier does not remain in the battle area.

#### Scenario: Predicate matches source carrier deleting its battle opponent
- **WHEN** an inherited effect uses the battle-deletion-survivor predicate/helper
- **AND** its source carrier deletes the opposing battle participant by battle
- **AND** the source carrier remains in the battle area after battle resolution
- **THEN** the predicate/helper evaluates true for that trigger context

#### Scenario: Predicate rejects unrelated friendly battle deletion
- **WHEN** an inherited effect uses the battle-deletion-survivor predicate/helper
- **AND** another friendly Digimon deletes an opponent Digimon by battle
- **THEN** the predicate/helper evaluates false for the source carrier that was not a participant in that battle

#### Scenario: Predicate rejects mutual destruction
- **WHEN** an inherited effect uses the battle-deletion-survivor predicate/helper
- **AND** its source carrier and the opponent battle participant are both deleted by battle
- **THEN** the predicate/helper evaluates false because the source carrier did not survive

#### Scenario: Predicate rejects non-battle deletion
- **WHEN** an inherited effect uses the battle-deletion-survivor predicate/helper
- **AND** an opponent Digimon is deleted by an effect rather than by battle
- **THEN** the predicate/helper evaluates false

### Requirement: DSL supports player Digimon attack-history predicates

The DSL SHALL provide a predicate or condition that evaluates whether a referenced player attacked with at least one Digimon during the current turn. The predicate SHALL be usable in triggered effect conditions, including inherited end-of-opponent-turn clauses, and SHALL support normal DSL negation so card authors can express "the opponent did not attack with a Digimon this turn."

#### Scenario: Predicate is true after referenced player attacks with a Digimon

- **WHEN** a player attacks with one of their Digimon during the current turn
- **THEN** evaluating the attack-history predicate for that player returns true

#### Scenario: Predicate is false when referenced player has not attacked with a Digimon

- **WHEN** a player reaches an end-of-turn timing without attacking with any Digimon during that turn
- **THEN** evaluating the attack-history predicate for that player returns false
- **AND** a negated form of the predicate can be used to authorize effects that require no Digimon attack

#### Scenario: Predicate resets across turn boundaries

- **WHEN** a player attacked with a Digimon on a previous turn
- **AND** a later turn begins
- **THEN** evaluating the attack-history predicate for that player reflects only the later turn's attack history

#### Scenario: Predicate can be referenced from inherited end-of-opponent-turn effects

- **WHEN** a card author writes an inherited `end_of_opponents_turn` clause conditioned on the opponent not having attacked with a Digimon this turn
- **THEN** the DSL compiles the clause
- **AND** the engine evaluates the condition at trigger resolution time using authoritative game attack history

### Requirement: DSL can express when-this-Digimon-is-blocked effects
The DSL SHALL allow a card author to express an effect that triggers only when the source permanent or inherited carrier is the attacking Digimon and that attack becomes blocked by a declared blocker. The trigger SHALL expose enough event context to distinguish the blocked attacker from other battle-area observers and from the original attack target. The effect SHALL work for face-up and inherited source scopes without hidden auto-resolution.

#### Scenario: Inherited source triggers when its carrier is blocked
- **WHEN** a Digimon attacks with an inherited source carrying a "when this Digimon is blocked" clause
- **AND** the defender declares a legal blocker
- **THEN** the inherited clause is enqueued for that attacking carrier
- **AND** resolving the clause runs its process body exactly once

#### Scenario: Other allied Digimon do not trigger
- **WHEN** a Digimon attacks and is blocked
- **AND** another allied Digimon has a "when this Digimon is blocked" clause but is not the attacker
- **THEN** the other allied Digimon's clause is not enqueued

#### Scenario: Unblocked attacks do not trigger
- **WHEN** a Digimon attacks and the defender declines to block or has no legal blocker
- **THEN** "when this Digimon is blocked" clauses do not trigger

#### Scenario: Non-block attack target changes do not trigger
- **WHEN** an attack target changes for a reason other than a declared blocker
- **THEN** "when this Digimon is blocked" clauses do not trigger

### Requirement: All selection verbs accept an inline `then:` action tail

Every selection verb (`select_opponent_permanent`, `select_own_permanent`, `select_hand`, `select_trash`, `select_security`, in addition to the source/budget/union selectors that already support it) SHALL accept an inline `then: <step list>` that runs after the selection resolves, with the selection's binding in scope. Each step in the tail SHALL still surface its own player choices through `pending_selection` (the tail is sugar over the existing select-then-act sequence, never an auto-pick). The longhand two-step form SHALL remain valid.

#### Scenario: select-then-act collapses into one clause

- **WHEN** a card uses `select_opponent_permanent { filter: {kind: digimon}, bind_as: t, then: [ delete_permanent: { target: t } ] }`
- **THEN** it behaves identically to the longhand `select_opponent_permanent { bind_as: t }` followed by `delete_permanent { target: t }`

#### Scenario: tail selections remain RL-visible

- **WHEN** a `then:` tail contains a step that itself requires a choice
- **THEN** that choice is surfaced as its own `pending_selection`, not auto-resolved

#### Scenario: longhand still works

- **WHEN** a card uses the separate select step and act step (no `then:`)
- **THEN** it parses and behaves as before

### Requirement: `reveal_search` composite for the reveal-search idiom

The DSL SHALL provide a `reveal_search { of, count, buckets: [{ filter, to, max, optional, prompt }...], remainder }` verb expressing reveal-top-N → pick into one or more destination buckets → place the remainder, lowering to the same selection sequence as the longhand idiom. Each bucket SHALL be an independent player-visible pick honoring its `optional` flag, and `add_to_hand_from_reveal` (and siblings) SHALL accept multiple cards so an "add up to N" bucket is a single `max: N`.

#### Scenario: single-bucket search

- **WHEN** a card uses `reveal_search { of: you, count: 3, buckets: [{ filter: {trait_has: X}, to: hand, max: 1, optional: true }], remainder: bottom }`
- **THEN** it reveals 3, offers a may-pick of one matching card to hand, and places the rest on the bottom — identical to the four-step longhand

#### Scenario: multi-card add in one bucket

- **WHEN** a bucket specifies `to: hand, max: 2`
- **THEN** up to two revealed cards may be added to hand through that one bucket (no per-card verb repetition)

#### Scenario: empty / all-declined behavior matches longhand

- **WHEN** the revealed pool has no bucket matches, or all optional buckets are declined
- **THEN** the remainder placement still runs, exactly as the hand-rolled idiom did

### Requirement: Security placement supports a chosen position and a single source-polymorphic verb

Security-placement steps SHALL support `position: choice` (a player-elected top/bottom installed as a `pending_selection`), and the family of per-source/per-disposition place-on-security verbs SHALL be consolidated into one verb whose `source:` accepts a hand binding, a permanent/material binding, or self, with replacement behavior expressed as an orthogonal disposition (`none | cancel | handle | observed`). The replacement dispositions SHALL remain usable only within `kind: replacement` clauses.

#### Scenario: top-or-bottom collapses into the step

- **WHEN** a card places a card "as the top or bottom of security" using `position: choice`
- **THEN** a single binary `pending_selection` offers top/bottom and no `select_effect_choice` + paired `if`-arms are needed

#### Scenario: one verb covers the prior sources

- **WHEN** a card places a hand card, a field permanent, or itself onto security
- **THEN** the same consolidated verb expresses all three via its `source:` parameter

#### Scenario: disposition is orthogonal to position

- **WHEN** a replacement-flow card needs "place on the bottom of security and handle the replacement"
- **THEN** that combination is expressible (fixing the prior bottom+handle asymmetry)

### Requirement: `link_card_to_self` is removed in favor of `link_cards`

The deprecated `link_card_to_self` verb SHALL be removed once `link_cards` covers single-card self-host and chosen-host linking, and the cards using it SHALL be migrated to `link_cards` with behavioral parity.

#### Scenario: migrated card behaves identically

- **WHEN** a card previously using `link_card_to_self` is authored with `link_cards`
- **THEN** the link is established with the same host, cost, and source-zone behavior

#### Scenario: deprecated verb is gone

- **WHEN** the vocabulary is enumerated
- **THEN** `link_card_to_self` is absent and `link_cards` is the single link-a-card verb

### Requirement: Link primitives cover relinking, heterogeneous choice, host filtering, and leave-prevention

The DSL/engine SHALL provide link primitives sufficient to faithfully express EX11-027 Maquinamon (folded in from `fix-dsl-substrate-rot-and-bugs`): moving a standing battle-area permanent to become a link card on a chosen own Digimon; a single player selection that chooses between two distinct link operations (with a branch offered only when its precondition holds); host filtering with link-requirement enforcement that excludes the source permanent; and a replacement cost that places a chosen link card as the carrier's bottom digivolution card to cancel a would-leave. Once EX11-027 carries no test-only raw_rust, the `dsl-substrate-integrity` loader guard SHALL be a hard error.

#### Scenario: a standing permanent is relinked as a link card

- **WHEN** EX11-027's `[On Play]` chooses to link this Digimon onto another of the controller's Digimon
- **THEN** the standing permanent becomes a link card on the chosen host, with the host's link requirement enforced and the source excluded as a host

#### Scenario: leave-prevention by placing a link card as a bottom source

- **WHEN** EX11-027 would leave the battle area and the controller elects the link-card replacement
- **THEN** a chosen link card is placed as the carrier's bottom digivolution card and the leave is cancelled

#### Scenario: guard promotes to hard error once no unregistered refs remain

- **WHEN** EX11-027 has migrated off raw_rust and the embedded pack has zero unregistered raw_rust references
- **THEN** the loader guard rejects any future unregistered reference as a hard load error (no warn-mode fallback)

### Requirement: Magnitude fields accept literals or formulas through one canonical type

Every DSL field expressing a numeric magnitude — memory deltas (`gain_memory`/`lose_memory`/`set_memory`), De-Digivolve amount, aura `dp_modifier` and `security_attack`, cost-reduction amount, and cost deltas — SHALL accept either a literal integer or a `FormulaSpec`, parsed through one canonical type. The previous parallel encodings (the `_fn` twin verbs/fields, `ModifierValueSpec`, and the `CostDelta` literal-vs-formula split) SHALL be removed. A bare integer in any of these positions SHALL continue to parse unchanged.

#### Scenario: Bare integer still parses after retype

- **WHEN** a card uses `gain_memory: 2` (or `dp_modifier: 3000`, `de_digivolve` amount `1`, etc.)
- **THEN** it parses and behaves exactly as before the unification

#### Scenario: Formula accepted in a position that previously required a `_fn` twin

- **WHEN** a card uses `gain_memory: { base: 0, per: ally_count, delta: 1 }` (a formula directly in the magnitude field)
- **THEN** it parses and resolves the formula at effect resolution — without a separate `gain_memory_fn` verb

#### Scenario: Retired `_fn` twins no longer exist

- **WHEN** the vocabulary is enumerated
- **THEN** `gain_memory_fn`, `lose_memory_fn`, `dp_modifier_fn`, `security_attack_fn`, the `cost_reduction.amount_fn` twin, `ModifierValueSpec`, and `CostDelta::ReduceFn` are absent (their function is subsumed by the canonical magnitude type)

### Requirement: Numeric predicate thresholds use a uniform, complete comparator

Numeric predicate comparisons SHALL be expressed through a uniform `Comparator { op: eq | gte | lte, value: FormulaSpec }` shape that is available for every numeric metric (DP, level, play cost, stack size, materials count, security count, and the event-payload metrics) and supports all three operators for each. Legacy key spellings (e.g. `dp_lte: N`, `level_eq: N`) SHALL continue to parse via deserialize aliases that lower to the same compiled comparator.

#### Scenario: Legacy threshold key still parses

- **WHEN** a card uses `filter: { dp_lte: 5000 }`
- **THEN** it parses and filters identically to before, lowering to the canonical comparator

#### Scenario: Operator completed for a metric that previously lacked it

- **WHEN** a card needs "play cost equal to N" (an `_eq` the legacy surface lacked for `play_cost`)
- **THEN** it is expressible through the uniform comparator without a new bespoke predicate field

#### Scenario: Threshold value may be a formula for any metric

- **WHEN** a card needs "DP ≤ (a runtime formula)" on any metric position
- **THEN** the comparator's `value` accepts a `FormulaSpec` and resolves it read-safely in the evaluation context where the predicate is checked

### Requirement: A single metric-parameterized budget-selection verb

Player-visible "delete/target up to a budget of total <metric>" selections SHALL be expressed by one verb parameterized by metric axis (DP or play cost), replacing the per-axis verb pair, with the budget value typed as `FormulaSpec`. The merged selection SHALL present the identical action mask and observation-tensor encoding as the per-axis verbs it replaces.

#### Scenario: DP budget via the merged verb

- **WHEN** a card uses the merged budget verb with `axis: dp` and a budget
- **THEN** it offers the same legal targets and consumes budget identically to the former `select_opponent_dp_budget`

#### Scenario: RL encoding is unchanged

- **WHEN** a budget selection is active under the merged verb
- **THEN** the action mask and observation tensor are byte-identical to the encoding produced by the former per-axis verbs (no action-space or tensor contract change)

