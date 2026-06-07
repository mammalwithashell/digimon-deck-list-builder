## ADDED Requirements

### Requirement: A Digimon carries a self link-condition

The engine SHALL let a `kind: digimon` card declare a self link-condition consisting of a link cost and a host filter. This condition MUST be readable when the card is evaluating its own link legality from hand or from the battle area, independent of any Option resolution.

#### Scenario: Link-condition gates legal hosts

- **WHEN** a Digimon with a self link-condition requiring an `Appmon`-trait host is eligible to link
- **THEN** only the controller's un-linked Digimon permanents that satisfy the host filter are legal link hosts
- **AND** a host that fails the filter is not a legal selection

#### Scenario: No link-condition means no link ability

- **WHEN** a Digimon has no self link-condition
- **THEN** no link ability is offered for it
- **AND** no host-selection prompt is installed

### Requirement: Linking a Digimon is a player-driven, action-mask-visible choice

The engine SHALL expose initiating a link as a legal action only when it is the controller's turn, the linking card is in a permitted origin zone, and at least one host satisfies the link-condition. Every link choice (initiate, host pick, source-zone pick) MUST be represented through pending selections and legal action masks within existing action ranges.

#### Scenario: On-field Digimon offers a link-activate ability

- **WHEN** an un-linked battle-area Digimon with a self link-condition has at least one legal host
- **AND** it is the controller's turn
- **THEN** a link-activate action is legal for that Digimon in the current action mask
- **AND** resolving it installs a host-selection pending selection

#### Scenario: No legal host hides the link ability

- **WHEN** a Digimon with a self link-condition has no host satisfying the filter
- **THEN** no link-activate action is legal for it
- **AND** no host-selection prompt is installed

#### Scenario: Action space is unchanged

- **WHEN** Digimon-link support is enabled
- **THEN** `digimon_engine.ACTION_SPACE_SIZE` remains unchanged
- **AND** existing action IDs keep their prior meaning

### Requirement: Linking attaches through the shared link back-half

After a host is chosen, the engine SHALL fire the `WhenWouldLink` window, pay the link cost adjusted by active `ChangeLinkCost` modifiers, attach the linked card into the host's linked cards, and dispatch `OnLink`. The attached card MUST follow the existing linked-card lifecycle: untargetable by attacks/deletion, and trashed when the host is deleted or returns to hand.

#### Scenario: Link pays adjusted cost and attaches

- **WHEN** a Digimon links to a legal host with a link cost of 1 and no cost modifiers
- **THEN** the controller's memory decreases by 1
- **AND** the linked card appears in the host's linked cards
- **AND** `OnLink` is dispatched after the attach

#### Scenario: WhenWouldLink cancel prevents the attach

- **WHEN** a `WhenWouldLink` effect cancels the pending link
- **THEN** no link cost is paid
- **AND** the card is not attached to the host

#### Scenario: Host deletion trashes the linked Digimon

- **WHEN** a host with a linked Digimon is deleted
- **THEN** the linked Digimon is trashed
- **AND** a single `OnLinkedCardTrashed` is dispatched for that host

### Requirement: Link source origins cover the card's permitted zones

The engine SHALL support linking a card from each origin zone permitted by the card text: hand, trash, under-stack (digivolution cards), another host's linked area (re-link), and as a whole standing battle-area permanent. A zone that the card does not permit MUST NOT offer the card as a link source.

#### Scenario: Standing permanent is absorbed whole

- **WHEN** a standing battle-area Digimon (with under-sources) is linked to a host
- **THEN** the entire permanent, including its under-sources, is removed from the controller's battle area
- **AND** it is placed as a single linked entry in the host's linked cards
- **AND** linking it later off the host, or trashing it, returns/moves the whole recorded stack

#### Scenario: Disallowed origin zone is not offered

- **WHEN** a Digimon's text permits linking only from hand or battle area
- **AND** a copy of an eligible card is in the controller's trash
- **THEN** the trash copy is not a legal link source

### Requirement: A linked Digimon's WhenLinked effects resolve on attach

The engine SHALL resolve the linked Digimon's own "when this Digimon gets linked" effects when it is attached to a host, attributed to the host's controller. (Per design D6 this may lower to a self-filtered `OnLink` rather than a new timing; the observable behavior below is the contract.)

#### Scenario: WhenLinked fires for the linked card only

- **WHEN** a Digimon with a `WhenLinked` effect is linked to a host
- **THEN** that effect resolves exactly once on attach
- **AND** a face-up Digimon that was not just linked does not fire a `WhenLinked` effect from the same attach

### Requirement: A linked Digimon grants its ESS to the host

The engine SHALL apply a linked Digimon's ESS-style grants (DP buff and granted keywords such as `Raid`) to its host while linked, attributed to the host's controller, and SHALL remove the grant when the link ends.

#### Scenario: Linked Digimon grants Raid to the host

- **WHEN** a Digimon whose link ESS grants `Raid` is linked to a host
- **THEN** the host is treated as having `Raid` for attack legality while the link persists
- **WHEN** the linked Digimon is trashed from the host
- **THEN** the host no longer has the granted `Raid`

#### Scenario: Linked Digimon grants DP to the host

- **WHEN** a Digimon whose link ESS grants +DP is linked to a host
- **THEN** the host's effective DP increases by the granted amount while the link persists
