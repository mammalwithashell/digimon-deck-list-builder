## ADDED Requirements

### Requirement: Complete physical source reachability
The engine SHALL make every eligible physical source reachable through its owned selection protocol without truncating or imposing a gameplay depth cap. All material, own-source and count-capped material consumers MUST use the same complete mechanism.

#### Scenario: Thirteenth and later sources
- **WHEN** eligible cards occur at indices twelve, twenty-four or later in a legal stack
- **THEN** the selecting player can reach and commit each exact card through legal actions, including Tamer-only and Digimon-only filters

#### Scenario: Count predicates remain independent
- **WHEN** a source-count condition is evaluated on the same stack
- **THEN** it reads the complete stack without requiring navigation or changing the active selection

### Requirement: Navigation has no gameplay side effects
Source paging SHALL change only owned prompt/page state. It MUST NOT pay a cost, move cards, consume OPT, reorder triggers, change RNG, or advance a turn.

#### Scenario: Page boundaries and cancellation
- **WHEN** a player navigates forward/backward or declines an optional source selection
- **THEN** cards, memory, OPT and resolution order remain unchanged until an authorized commit or decline continuation

### Requirement: Authoritative masks and typed source metadata
The current page SHALL expose legal commits/navigation/decline through the engine mask and typed metadata. Async clients MUST reject stale prompt/page generations and MUST dispatch supplied action IDs rather than reconstruct source offsets.

#### Scenario: Stale click after navigation
- **WHEN** a client submits a commit from an earlier page generation after navigation
- **THEN** the engine-facing surface rejects it without selecting a different current-page card

#### Scenario: Wrong selecting seat
- **WHEN** a non-selecting seat submits a source or navigation action
- **THEN** the action is rejected and its mask contains no permitted choice

### Requirement: Clone-safe identity and interruption
A parked source selector SHALL retain exact physical-card and carrier identities, prefix choices and independent page state through cloning and nested effect windows.

#### Scenario: Clone diverges after partial selection
- **WHEN** a game is cloned after navigation or one material choice
- **THEN** each clone can independently finish or decline without changing the other clone or rebinding a moved source index

### Requirement: Coordinated model and transport contract
The migration MUST update action schema version, observation schema/layout metadata, Rust/PyO3/RL/frontend exports, protocol DTOs, replay descriptors and model metadata in one dedicated change. Equal action or tensor sizes MUST NOT bypass semantic compatibility checks.

#### Scenario: Old same-size model
- **WHEN** an old checkpoint/ONNX artifact has the same shapes but an old or missing semantic version/hash
- **THEN** inference, resume, export and publication reject it rather than stamping current metadata

### Requirement: Selected-source placement must complete before reward
A security-placement cost SHALL expose a completed correlated receipt. BT18-042 MUST delete only after that receipt confirms its chosen source was processed into security under applicable rules.

#### Scenario: Pending placement is declined or redirected
- **WHEN** a replacement interrupts the source placement and then cancels, custom-handles or redirects it away from security
- **THEN** the deletion reward does not execute, including after cloning or replay reset

### Requirement: Evidence-gated card promotion
Affected card ledgers SHALL remain blocked until complete-source reachability, cost receipts, real-card behavior and generated training admission checks pass under the new compatible runtime.

#### Scenario: A shallow test passes while deep source selection is incomplete
- **WHEN** ordinary-card tests pass but a legal deep Tamer or Digimon remains unreachable
- **THEN** the card is not promoted merely because it has a module or lowering-covered constructs
