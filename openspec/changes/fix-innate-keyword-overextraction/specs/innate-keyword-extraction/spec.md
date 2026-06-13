## ADDED Requirements

### Requirement: Innate keywords come only from the leading keyword line

A card's innate (printed-attribute) keywords SHALL be extracted only from the **leading keyword line** of each text field (effect, inherited, security): the run of `＜keyword＞` tokens (each with an optional immediately-following `(reminder)`) that appears at the start of the field, before the first `[Timing]`/label bracket or prose sentence. Keyword tokens that appear after a `[Timing]`/header — whether granted, conditional, formula units, or target filters — MUST NOT be treated as innate. Field headers (`Inherited Effect`, `Security Effect`, `Rule Effect`) at the start are skipped before the keyword line.

#### Scenario: Leading token is innate

- **WHEN** a card's effect text begins `＜Blocker＞ (This Digimon can block in the blocker timing.)` (Monmon BT1-031)
- **THEN** the card's innate keywords include `Blocker`

#### Scenario: Token after a timing bracket is not innate

- **WHEN** a card's effect text is `[Your Turn] For every 2 digivolution cards … it gains ＜Security A. +1＞` (WarGreymon ST1-11)
- **THEN** the card has NO innate `<Security A.>` keyword (the grant is modeled as the `security_attack_fn` effect, not an innate attribute)

#### Scenario: Target-filter token is not innate

- **WHEN** a card's effect text is `[On Play] Delete 1 of your opponent's Digimon with ＜Blocker＞` (SkullGreymon BT1-023)
- **THEN** the card has NO innate `Blocker` keyword (the token describes the deletion target, not this card)

#### Scenario: A reminder's inner trait bracket does not terminate the keyword line

- **WHEN** a card's leading keyword unit is `＜Decoy ([Bagra Army] trait)＞`
- **THEN** `Decoy` is parsed as innate and the inner `[Bagra Army]` (inside the keyword's own reminder) does NOT end keyword-line parsing

#### Scenario: Multiple leading keywords

- **WHEN** a field begins with two consecutive keyword units before any `[Timing]` (e.g. `＜Retaliation＞ ＜Scapegoat＞`)
- **THEN** both are parsed as innate

### Requirement: Parametric keyword grants do not double-count or apply unconditionally

A parametric keyword (`Security A. +N`, `Draw N`, `De-Digivolve N`) that a card's effect grants or computes MUST contribute through the modeled effect only, never additionally as an innate keyword parsed from the grant text.

#### Scenario: Formula card checks the correct security count

- **WHEN** WarGreymon (ST1-11) has 4 digivolution cards on its controller's turn
- **THEN** its effective security strike is exactly 3 (`1 + floor(4/2)`), and on the opponent's turn it is 1 — with no extra `<Security A. +1>` from its own reminder text

#### Scenario: External grants remain additive

- **WHEN** a separate effect grants `<Security A. +1>` to WarGreymon (e.g. Tai Kamiya BT1-085's aura, or a buried source's inherited `<Security A.>`)
- **THEN** that grant is still added on top of WarGreymon's formula (e.g. total 4), because only the card's own duplicated innate parse is removed — modeled grants are untouched

### Requirement: Pool-wide regression audit with modeled gaps

The change SHALL produce a committed before/after innate-keyword diff over the full card pool, partitioned into implemented (has a DSL spec) and unimplemented cards. Every **implemented** card that loses a keyword under the new rule MUST end with that keyword's grant modeled as a DSL effect (existing or newly added as a conditional grant) so that net behavior is correct-or-better. Unimplemented regressors MUST be recorded in the engine-gap tracker rather than silently dropped.

#### Scenario: Implemented regressor is modeled

- **WHEN** the diff shows an implemented card losing a previously-parsed granted keyword
- **THEN** the card's grant is verified present as a DSL effect, or a conditional grant is added, before the change lands

#### Scenario: Unimplemented regressor is tracked

- **WHEN** the diff shows an unimplemented card losing a keyword
- **THEN** it is logged to `docs/RUST_ENGINE_GAPS.md` (the phantom unconditional keyword is removed; the faithful conditional grant is left as tracked future work)
