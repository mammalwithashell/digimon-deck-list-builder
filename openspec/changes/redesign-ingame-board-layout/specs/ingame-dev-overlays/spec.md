## ADDED Requirements

### Requirement: Debug overlays are gated to development builds

Developer-only debug overlays SHALL render only in development builds (e.g.
gated behind `import.meta.env.DEV`) and MUST NOT appear in production or desktop
release builds. This applies at minimum to the tensor-summary badge
(`profileId / Pn / Tn / An / Ln / phase`).

#### Scenario: Tensor badge absent in production build

- **WHEN** the app runs in a production/release build
- **THEN** the tensor-summary badge is not rendered anywhere in the in-game UI

#### Scenario: Tensor badge available in dev build

- **WHEN** the app runs in a development build
- **THEN** the tensor-summary badge is rendered (for QA/debugging)

### Requirement: Debug overlays do not overlap gameplay chrome

When a debug overlay is shown (dev builds), it SHALL occupy a dedicated region
that does not overlap gameplay chrome — specifically the hand, the hand-count
chip, and the action bar. Two debug/info elements MUST NOT be anchored to the
same screen position such that they render on top of each other (as the tensor
badge and hand-count chip previously both did at the bottom-right).

#### Scenario: Tensor badge clear of the hand-count chip

- **WHEN** the tensor badge is shown in a dev build
- **THEN** it does not overlap the hand-count chip or the hand cards

#### Scenario: Debug overlays clear of the action bar

- **WHEN** any debug overlay is shown in a dev build
- **THEN** it does not cover or obstruct the action bar controls
