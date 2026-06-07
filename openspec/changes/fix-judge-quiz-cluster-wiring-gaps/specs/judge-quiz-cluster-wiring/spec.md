## ADDED Requirements

### Requirement: `<Blast Digivolve>` consults effect-immunity

When a Digimon would be digivolved via `<Blast Digivolve>` or `<Blast DNA Digivolve>` (the base Digimon being digivolved into the blast result), the engine SHALL treat that digivolve as a Digimon effect and SHALL NOT allow it if the base Digimon is unaffected by its own controller's Digimon effects. Concretely, a base Digimon for which `permanent_is_unaffected_by_effect(base, base.player, Digimon)` holds SHALL NOT be offered as a Blast (or Blast DNA) counter candidate, and any effect-driven blast path SHALL abort for such a base.

#### Scenario: Digimon immune to all effects (incl. its own) is not a Blast candidate

- **WHEN** a defender holds a `<Blast Digivolve>` card whose only valid base is a Digimon carrying an unconditional `CannotBeAffected` (the `Any` controller filter), and that base is attacked
- **THEN** no Counter window / Blast candidate is offered for that base
- **AND** the attack resolves without a Blast Digivolve

#### Scenario: Blast DNA field base honors immunity

- **WHEN** the Blast DNA field-material targets are enumerated for a hand card
- **THEN** a field Digimon unaffected by its own controller's Digimon effects is excluded from the field-base targets

#### Scenario: Non-immune base is unaffected

- **WHEN** a base Digimon has no effect-immunity (or only `OpponentOnly` immunity)
- **THEN** it is still offered as a Blast candidate and can blast-digivolve as before

### Requirement: Digivolve-target restriction

A base permanent carrying a digivolve-target restriction (`CanOnlyDigivolveInto`, carrying an allowed card name) SHALL offer a digivolve route ONLY into a card whose name matches the allowed name. This SHALL be honored by every digivolve route — the digivolve action mask, the Blast counter path, hand-digivolve execution (all via `normal_digivolve_route_for_card`), and the arts-digivolve path. When no such restriction is installed, digivolve legality SHALL be unchanged.

#### Scenario: Base restricted to a name cannot digivolve into another card

- **WHEN** a base Digimon carries `CanOnlyDigivolveInto("Apocalymon")` and a hand card named other than "Apocalymon" would otherwise be a legal digivolve target
- **THEN** no digivolve route is offered for that non-matching card
- **AND** a hand card named "Apocalymon" that is otherwise legal IS still offered

#### Scenario: No restriction is a no-op

- **WHEN** a base Digimon carries no `CanOnlyDigivolveInto` modifier
- **THEN** its digivolve routes are exactly as before (existing cards unaffected)
