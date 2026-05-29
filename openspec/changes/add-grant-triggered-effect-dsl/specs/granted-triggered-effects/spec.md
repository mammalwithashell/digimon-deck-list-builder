## ADDED Requirements

### Requirement: Grant a triggered effect to another permanent with turn-scoped expiry

The engine SHALL provide a typed granted-trigger modifier slot that carries a triggered clause (a timing plus a process body) and an expiry, and the DSL SHALL provide a `grant_triggered_effect` step that installs such a slot onto a selected set of permanents. The installed clause's body SHALL be lowered against the GRANTED permanent (its controller is the grantee), and the slot SHALL expire per the declared expiry using the existing modifier expiry mechanism.

#### Scenario: Granted clause fires on the granted permanent

- **WHEN** a card grants "[When Attacking] lose 2 memory" to an opponent's Digimon and that Digimon later attacks
- **THEN** the granted clause resolves, subtracting 2 memory from the granted Digimon's controller
- **AND** the granted clause is gone once its expiry boundary passes

#### Scenario: Grant targets only the snapshot set

- **WHEN** "all opponent Digimon gain <clause> until <expiry>" resolves and the opponent later plays an additional Digimon within the expiry window
- **THEN** the later-played Digimon does NOT carry the granted clause (the eligible set is snapshotted at grant time)

#### Scenario: Grant installs nothing when no target matches

- **WHEN** the grant's target selector matches no permanent
- **THEN** no slot is installed and resolution continues

### Requirement: Granted effects are attributed to the grantee for cause and immunity

An effect resolved from a granted-trigger slot SHALL be attributed to the GRANTEE's controller. Consequently, a deletion the granted clause causes counts as the grantee's OWN-effect departure, and the granted slot is subject to the grantee's effect-immunity (it is removed/suppressed when the grantee becomes immune).

#### Scenario: Partition does not fire on a granted self-delete

- **WHEN** an opponent grants "[End of Your Turn] Delete this Digimon" to a Digimon that has `<Partition>`, and that delete resolves
- **THEN** the Digimon leaves the battle area by its OWN effect
- **AND** `<Partition>` does NOT trigger

#### Scenario: Immunity removes a granted effect

- **WHEN** a Digimon carrying a granted "[End of Your Turn] Delete this Digimon" gains immunity to opponent effects (e.g. a `[When Digivolving]` immunity)
- **THEN** the granted clause is removed/suppressed and the end-of-turn delete does NOT activate

#### Scenario: Progress excludes a granted opponent effect on the attacker

- **WHEN** an attacking Digimon with `<Progress>` carries an opponent-granted "[When Attacking] lose 2 memory"
- **THEN** `<Progress>` makes the attacker immune to that opponent-granted effect and the memory loss does NOT occur

### Requirement: Ice Wall! and Lilithmon authored; Q2/Q16/Q17 pinned

EX1-068 Ice Wall!'s `[Main]` grant and EX6-057 Lilithmon's `[On Play]` grant SHALL be authored faithfully using the new step, the judge-quiz tests they blocked SHALL be un-`#[ignore]`-d and pass, and the gap SHALL be archived.

#### Scenario: Q2 pins

- **WHEN** the primitive and EX1-068's `[Main]` clause land
- **THEN** `a_immunity_scope::q2_medusamon_progress_blocks_ice_wall_memory_loss` is un-ignored and passes (Medusamon loses no memory)

#### Scenario: Q16 and Q17 pin

- **WHEN** the primitive, cause attribution, and EX6-057's `[On Play]` clause land
- **THEN** the Q16 (`<Partition>` does not trigger) and Q17 (immunity removes the granted delete) tests are un-ignored and pass

#### Scenario: Gap archived

- **WHEN** the change completes
- **THEN** `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` is moved from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md` with a resolution note and test commands
