## ADDED Requirements

### Requirement: Edit the selected deck from the CHOOSE DECK screen

The CHOOSE DECK screen SHALL provide an EDIT control for the currently selected deck, available in every opponent mode (bot, quick-match, room), that opens the deck builder for that deck without leaving the play flow.

#### Scenario: EDIT is available for the selected deck

- **WHEN** a deck is selected on the CHOOSE DECK screen in any opponent mode
- **THEN** an EDIT control is shown for that deck
- **AND** activating it opens the deck editor for that deck

#### Scenario: No deck selected

- **WHEN** no deck is selected
- **THEN** the EDIT control is unavailable

### Requirement: Editing opens the full deck builder in an overlay

Activating EDIT SHALL render the full deck builder — including its filters, GRID/DETAIL/DECKLIST views, validation, and import — inside an overlay above the play flow, rather than navigating to a separate page. The overlay SHALL present save and cancel/close controls in place of the builder's page navigation chrome (HOME/LIBRARY/QUIT).

#### Scenario: Overlay shows the full builder

- **WHEN** the player activates EDIT
- **THEN** the deck builder appears in an overlay over the CHOOSE DECK screen with the selected deck loaded
- **AND** the builder's filters, view toggle, and card pool are usable within the overlay

#### Scenario: Overlay chrome replaces page navigation

- **WHEN** the deck builder is shown in the overlay
- **THEN** it presents save and cancel/close controls instead of the page HOME/LIBRARY/QUIT navigation

### Requirement: Saving in the overlay persists and returns in place

Saving in the overlay SHALL persist the deck, re-evaluate its legality for the queue's format, refresh the selected deck's summary on the CHOOSE DECK screen, keep that deck selected, and return the player to CHOOSE DECK without a full-page navigation.

#### Scenario: Save updates the deck and returns

- **WHEN** the player changes the deck in the overlay and saves
- **THEN** the deck is persisted and the overlay returns the player to CHOOSE DECK with the same deck still selected
- **AND** the selected deck's card counts and legality reflect the saved changes

#### Scenario: An edit that breaks legality is flagged before queuing

- **WHEN** a saved edit makes the deck illegal for the current format
- **THEN** CHOOSE DECK reflects the deck as not legal and the proceed/USE THIS DECK action is disabled until resolved

### Requirement: Cancelling discards changes with an unsaved-changes guard

Cancelling or closing the overlay SHALL leave the deck as it was before editing; if there are unsaved changes, the player SHALL be asked to confirm discarding them before the overlay closes.

#### Scenario: Cancel with no changes closes immediately

- **WHEN** the player opens the overlay and closes it without making changes
- **THEN** the overlay closes and the deck is unchanged

#### Scenario: Cancel with unsaved changes prompts to confirm

- **WHEN** the player has unsaved changes and attempts to cancel or close the overlay
- **THEN** the player is asked to confirm discarding the changes
- **AND** confirming closes the overlay and leaves the deck unchanged

### Requirement: Play-flow selections are preserved across editing

Opening, saving, or cancelling the edit overlay SHALL NOT change the player's current format or opponent-mode selection in the play flow.

#### Scenario: Format and mode survive an edit

- **WHEN** the player has chosen a format and opponent mode, opens the edit overlay, and then closes or saves it
- **THEN** the same format and opponent mode are still selected on return
