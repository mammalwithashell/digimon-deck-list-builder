# Spec: keyword-guard

## ADDED Requirements

### Requirement: Guard keyword parse and representation
The engine SHALL model ＜Guard＞ as a first-class `Keyword` variant, and the printed-keyword parser SHALL recognize `＜Guard＞` (and the ASCII `<Guard>` form) from card text so that any card or token whose text carries the keyword gains it without per-card scripting.

#### Scenario: Printed Guard parses onto the card
- **WHEN** a card whose effect text contains `＜Guard＞` is loaded into the card registry
- **THEN** the card's parsed keyword set contains `Keyword::Guard`

#### Scenario: Token-carried Guard
- **WHEN** a token species whose registry text carries `＜Guard＞` (e.g. [Paishu]) is played
- **THEN** the token permanent has `Keyword::Guard` active identically to a printed card

### Requirement: Guard protect-others leave replacement
A Digimon with ＜Guard＞ SHALL offer an optional leave replacement when another of its owner's Digimon would leave the battle area **by an opponent's effect**: by deleting the Guard carrier, the subject does not leave. The choice MUST surface through the pending-selection surface (RL action space), MUST be declinable, and the machinery MUST be clone-safe (resumable data-VM, no closure-based parks).

#### Scenario: Accepting Guard protects the subject
- **WHEN** an opponent's effect would delete one of the controller's other Digimon and a Guard carrier is in play, and the controller accepts the Guard offer
- **THEN** the Guard carrier is deleted, the subject remains in the battle area, and the subject's would-leave event is prevented

#### Scenario: Declining Guard lets the leave complete
- **WHEN** the same situation arises and the controller declines the offer
- **THEN** the Guard carrier remains in play and the subject leaves as the original effect dictates

#### Scenario: Guard does not trigger on own effects or battle
- **WHEN** the controller's own effect (or a battle deletion) would remove the subject from the battle area
- **THEN** no Guard offer is installed

#### Scenario: Guard does not protect the carrier itself
- **WHEN** an opponent's effect would delete the Guard carrier itself and no other own Digimon is leaving
- **THEN** no Guard offer is installed (the keyword protects only OTHER own Digimon)

#### Scenario: Clone-safety mid-offer
- **WHEN** the game is cloned while a Guard offer is pending
- **THEN** the clone can resolve the offer (accept or decline) without panicking
