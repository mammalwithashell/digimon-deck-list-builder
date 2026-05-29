## ADDED Requirements

### Requirement: Temporary lockouts can suppress timing-effect activation

The engine SHALL support temporary modifiers that prevent affected Digimon or
Tamers from activating specified timing-effect families until the modifier's
printed expiry.

#### Scenario: When-digivolving effects are locked out

- **WHEN** an affected Digimon would activate a When Digivolving effect while it
  has a modifier suppressing When Digivolving effects
- **THEN** that effect does not activate and no hidden player choice is created
  for it

#### Scenario: Unaffected timing family still activates

- **WHEN** an affected permanent has a lockout for one timing family and a
  different timing family would trigger
- **THEN** the different timing family remains eligible unless another active
  modifier suppresses it

### Requirement: Temporary lockouts can prevent unsuspension

The engine SHALL support temporary modifiers that prevent affected Digimon or
Tamers from unsuspending until the modifier's printed expiry.

#### Scenario: Locked permanent reaches unsuspend phase

- **WHEN** a suspended permanent with an active cannot-unsuspend modifier would
  unsuspend during an unsuspend step
- **THEN** it remains suspended

#### Scenario: Lockout expires

- **WHEN** the expiry point for a cannot-unsuspend modifier has passed
- **THEN** the affected permanent can unsuspend through normal rules again

### Requirement: Temporary lockouts respect explicit expiry

Temporary effect lockouts SHALL expire at the printed duration, including
durations such as until the end of the opponent's turn.

#### Scenario: Lockout remains during opponent turn

- **WHEN** a modifier lasts until the end of the opponent's turn and the opponent
  is still in that turn
- **THEN** the lockout remains active

#### Scenario: Lockout removed after expiry

- **WHEN** the game advances beyond the printed expiry point
- **THEN** the lockout modifier is removed or ignored for future legality and
  trigger checks
