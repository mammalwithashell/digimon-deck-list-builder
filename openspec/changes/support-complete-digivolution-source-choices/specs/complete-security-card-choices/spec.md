## ADDED Requirements

### Requirement: Complete eligible security-card reachability
The engine SHALL expose every eligible physical security card through an owned legal selection protocol without imposing a ten-card gameplay cap. Top-card-only effects MUST remain fixed operations and SHALL NOT gain extra choices.

#### Scenario: Eleventh or later eligible card
- **WHEN** BT18-037's eligible card is at security index ten or later
- **THEN** the selecting player can navigate and commit that exact card through legal masked actions

### Requirement: Permission-preserving typed paging
Security pages SHALL retain zone owner, physical card identity and prompt/page generation. Metadata MUST reveal only what the active effect permits that selecting seat to inspect. Navigation MUST NOT move, recover, shuffle cards, consume RNG or advance resolution.

#### Scenario: Clone and stale page after a shuffle
- **WHEN** a selection is cloned or its security stack changes while parked
- **THEN** clone pages remain independent and stale commits cannot bind the card now occupying an old index or leak another seat's hidden cards

### Requirement: Coordinated migration and card promotion
Security paging MUST ship with the same versioned Rust/PyO3/RL/frontend, observation, replay and model compatibility transaction as complete source choices. BT18-037 MUST remain blocked until public boundary, clone, visibility and environment/transport tests pass and generated admission agrees with the rebuilt wheel.

#### Scenario: Old model with unchanged shapes
- **WHEN** an old model has matching action/tensor sizes but lacks the new semantic version or layout hash
- **THEN** inference, resume and export reject it rather than relabelling its metadata
