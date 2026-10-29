## ADDED Requirements

### Requirement: Printed End of Attack identifies the attacking carrier
Printed End of Attack effects SHALL trigger only for the carrier that performed
the declared attack, as specified by CR 4.3 section 15-16-15-1. Top-card and
inherited effects MUST use stable carrier identity and preserve authored gates.

#### Scenario: Another ally or opponent attacks
- **WHEN** an attack ends for another Digimon
- **THEN** the unrelated printed End of Attack effect does not activate, pay a
  resource, unsuspend its carrier, delete it or recover security

#### Scenario: Carrier attacks and survives
- **WHEN** the carrier's own declared attack reaches its end timing and its
  authored conditions are satisfied
- **THEN** its printed effect remains available through the normal legal choice
  and trigger-ordering contracts

#### Scenario: Clone and field compaction
- **WHEN** a game is cloned or lower-index permanents leave during an attack
- **THEN** eligibility follows the original carrier identity rather than a reused
  field index, with no shared mutable continuation state between clones

### Requirement: Broad attack-end events remain available
The engine SHALL preserve legitimate broad observers, delayed processing and
attack-end cleanup independently of printed carrier-scoped timing.

#### Scenario: Broad observer at another Digimon's attack end
- **WHEN** a broad native observer or scheduled callback is eligible
- **THEN** printed timing qualification does not suppress that observer/callback

#### Scenario: No new observation or action contract
- **WHEN** the repair is applied
- **THEN** every existing legal decision remains player-visible and the model
  observation/action schema remains unchanged

### Requirement: Simultaneous start-turn Delay effects share ordering
Eligible scheduled start-turn Delay effects SHALL join ordinary start-turn
effects in the same timing window, with all legal ordering choices preserved.
Decline, unavailable processing and cloning MUST preserve the recurring window
and cost semantics without repeated offers in the same timing window.

#### Scenario: Red Scramble and Asuna are both due
- **WHEN** LM-027 and BT24-088 have eligible effects at the start of the same turn
- **THEN** the controller can choose either effect first before either body runs

#### Scenario: Delay is declined
- **WHEN** a scheduled Delay is declined in the shared start-turn window
- **THEN** its carrier remains, it can recur at its next legal window, and the
  current timing finishes without offering the same activation again

#### Scenario: Native and DSL cost ownership
- **WHEN** a native scheduled Delay or a DSL Delay that owns its trash cost resolves
- **THEN** its applicable cost/cleanup is performed exactly once and selection
  continuations resume without advancing the phase prematurely

#### Scenario: A simultaneous sibling enables processing
- **WHEN** a Delay's processing condition is false at trigger capture but becomes
  true after another effect in the same start-turn window
- **THEN** that processing condition does not prematurely erase its triggered
  entry, and availability is rechecked before activation

#### Scenario: Native Delay replaces its physical card
- **WHEN** a native Delay body removes its carrier and places the same physical
  Option as a new permanent, immediately or across a cloned selection
- **THEN** old-carrier cleanup follows the captured stack incarnation and cannot
  delete the newly placed permanent

### Requirement: Per-turn usage resets before start-turn effects
Per-turn usage MUST reset once at the beginning of the new turn before its
start-turn triggers or derived events can activate effects. Uses during that
window MUST remain spent throughout the rest of the turn.

#### Scenario: Prior-turn observer use
- **WHEN** BT25-077 used its once-per-turn effect on the preceding turn and
  BT24-088 plays a qualifying Digimon at the new turn's start
- **THEN** BT25-077 can activate for the new turn

#### Scenario: Start-turn use remains spent
- **WHEN** BT25-077 uses its once-per-turn effect during the start-turn window
- **THEN** entering Main does not reset that use or permit another activation
