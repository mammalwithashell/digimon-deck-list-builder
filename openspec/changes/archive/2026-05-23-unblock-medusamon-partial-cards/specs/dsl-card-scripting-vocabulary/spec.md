## ADDED Requirements

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
