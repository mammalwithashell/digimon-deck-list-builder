## ADDED Requirements

### Requirement: Infoset extraction never exposes hidden information
The system SHALL extract, for a given viewing player, an `Infoset` containing only information that player is entitled to observe: the public state, the viewer's own hand (exact), the viewer's own deck and security as composition multisets without order, and the opponent's hidden zones as per-zone counts plus proven ("pinned") cards plus a deck prior — never the opponent's concealed card identities. This is the inverse of `server/state_filter.py`'s redaction.

#### Scenario: Opponent concealed identities are absent from the infoset
- **WHEN** an `Infoset` is extracted for player P from a game where the opponent holds concealed hand and face-down security cards
- **THEN** the `Infoset` contains the opponent's hand and face-down security *counts* but none of their concealed card identities

#### Scenario: Face-up security is treated as known
- **WHEN** some of the opponent's security cards are recorded in `face_up_security`
- **THEN** those specific cards appear as known/pinned in the `Infoset` and are excluded from the sampled (face-down) portion

### Requirement: Sampled worlds are consistent and materialized
The system SHALL produce, from an `Infoset` and a seed, a concrete fully-known `Game` (a "determinized world") in which all hidden zones are filled with sampled card identities that respect: per-zone counts, deck-construction copy limits, pinned cards, and exclusion of cards already known to be elsewhere. The materialized world SHALL have no live `RevealSource` (all piles concrete and ordered).

#### Scenario: Sampled world honors counts and copy limits
- **WHEN** a world is materialized from an `Infoset`
- **THEN** each hidden zone has exactly its recorded count of cards, no card exceeds its copy limit across all zones, and no pinned card is placed elsewhere

#### Scenario: Materialized world needs no reveal source
- **WHEN** a determinized world is advanced through draws and security checks
- **THEN** card identities are read from the committed piles with no `RevealSource` consultation and no re-shuffle

### Requirement: Determinization round-trips to the source infoset
For the viewing player, re-extracting an `Infoset` from a sampled world SHALL equal the source `Infoset` (the sample is consistent with what the viewer knows).

#### Scenario: Re-extracted infoset matches the source
- **WHEN** a world is sampled from `Infoset` I for viewer P, and an `Infoset` I' is extracted from that world for P
- **THEN** I' equals I (same public state, same own hand, same hidden counts and pins)
