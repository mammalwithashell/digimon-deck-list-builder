## ADDED Requirements

### Requirement: Deletion provenance survives attack and security continuations

The engine SHALL distinguish deletion caused by an effect from destruction by battle, including effects resolving while an attack or security check is pending. Replacement eligibility MUST use the initiating mutation's cause and controller throughout suspended selections and batch processing.

#### Scenario: Chronomon effect offers Decode during an attack

- **WHEN** BT26-016's When Attacking effect deletes EX12-031 with a qualifying source
- **THEN** MarineBullmon's Decode is an available immediate effect
- **AND** the pending attack does not classify that effect deletion as Battle

#### Scenario: Actual battle destruction excludes Decode

- **WHEN** MarineBullmon is deleted by actual normal or security battle
- **THEN** its non-battle Decode is not offered

#### Scenario: Security effect differs from security battle

- **WHEN** a resolving security effect deletes a Digimon with a non-battle leave effect
- **THEN** eligibility uses effect provenance rather than the surrounding security battle state

### Requirement: Delay leave observers resume the original removal window

A Delay effect that observes an impending departure and pays self-trash to perform a hand/union reward SHALL finish that body within the original immediate window. It MUST preserve the interrupted subject incarnation, cause, destination, remaining candidates, and activation consumption. It MUST NOT independently restart or convert the observed departure.

#### Scenario: Arrival and Decode can resolve in either legal order

- **WHEN** Sanmyojin Arrival and MarineBullmon's Decode are both eligible for the same effect-driven leave
- **THEN** the controller can choose their legal order through masked actions
- **AND** Arrival's successful self-trash and optional reward resolve before the original departure commits
- **AND** the remaining immediate effect retains its original eligibility and consumption state

#### Scenario: Original hand departure stays a hand departure

- **WHEN** Arrival resolves while its qualifying Digimon is being returned to hand
- **THEN** continuation retains the hand destination
- **AND** the observer does not turn the event into deletion

#### Scenario: Clone preserves a parked Delay window

- **WHEN** the game is cloned during Arrival payment, reward, or another immediate choice
- **THEN** applying the same legal choices yields the same masks, visible prompts, final zones, and playable continuation

### Requirement: Removal guards use stable incarnation and event identity

Replacement recursion and commit guards for a permanent SHALL follow its battle-area incarnation through field compaction and digivolution. Deletion consumption MUST remain scoped to the interrupting cause. A different permanent occupying the same slot, a replayed card with a new incarnation, or an independent deletion cause MUST NOT be suppressed by stale slot-based state.

#### Scenario: Field compaction cannot suppress a surviving subject

- **WHEN** Arrival trashes itself before MarineBullmon's departure resolves
- **THEN** the legal Decode choices and final outcomes are identical whether MarineBullmon compacts into Arrival's former slot or another slot
- **AND** the already consumed immediate effect cannot activate again for that same cause

#### Scenario: Replayed card is a new subject

- **WHEN** a physical card leaves and re-enters the battle area while an older removal window is suspended
- **THEN** the older removal cannot target the new incarnation
- **AND** a new removal attempt on the new incarnation has its own eligible replacement effects

#### Scenario: Independent cause can activate an effect again

- **WHEN** an immediate effect has activated for one resolved deletion cause and a surviving permanent faces another independent deletion cause
- **THEN** the first cause's replacement guard does not suppress an otherwise legal activation for the new cause
