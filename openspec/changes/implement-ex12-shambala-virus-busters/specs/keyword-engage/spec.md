# Spec: keyword-engage

## ADDED Requirements

### Requirement: Engage keyword parse and representation
The engine SHALL model ＜Engage＞ as a first-class `Keyword` variant distinct from ＜Vortex＞, and the printed-keyword parser SHALL recognize `＜Engage＞` (and the ASCII `<Engage>` form) from card text.

#### Scenario: Printed Engage parses onto the card
- **WHEN** a card whose effect text contains `＜Engage＞` is loaded into the card registry
- **THEN** the card's parsed keyword set contains `Keyword::Engage` and does NOT contain `Keyword::Vortex`

### Requirement: Engage end-of-turn optional attack
A Digimon with ＜Engage＞ SHALL be offered an optional attack at the end of its controller's turn ("At the end of your turn, this Digimon may attack."). The offer MUST be declinable and exposed through the pending-selection surface; the attack, if taken, resolves through the standard attack state machine. Per the DCGO behavioral oracle (`Engage.cs`, cross-checked against official rulings — a contradicting ruling wins), the attack MAY target the opponent player or an opponent's Digimon, and the carrier receives NO played-this-turn allowance (normal summoning-sickness rules apply, unlike ＜Vortex＞).

#### Scenario: Engage offers an attack at end of controller's turn
- **WHEN** the controller's turn reaches the end-of-turn window and an unsuspended Engage carrier is able to attack
- **THEN** an optional attack offer for the carrier surfaces in the action space

#### Scenario: Declining Engage ends the turn normally
- **WHEN** the Engage offer is declined
- **THEN** no attack occurs and turn end proceeds; the carrier is not suspended by the declined offer

#### Scenario: Engage respects attack legality
- **WHEN** the Engage carrier cannot legally attack at the end-of-turn window (e.g. suspended, or attack-prevented by an effect)
- **THEN** no offer is installed

#### Scenario: No Engage window on the opponent's turn
- **WHEN** the opponent's turn ends
- **THEN** the controller's Engage carriers receive no attack offer
