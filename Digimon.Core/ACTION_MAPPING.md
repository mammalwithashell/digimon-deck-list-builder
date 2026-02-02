# Action Mapping Decoder

The `ActionDecoder` translates a discrete integer Action ID (~2000 size) into a concrete Game Action. The interpretation is **Context-Sensitive** based on the current `GamePhase`.

## Context 1: Main Phase (Standard)

| Action ID Range | Action Type | Description | Formula / Notes |
| :--- | :--- | :--- | :--- |
| **0 - 29** | **Play Card** | Play card from Hand Index `X`. | `HandIndex = ID` |
| **30 - 59** | **Trash Card** | Trash card from Hand Index `X`. | `HandIndex = ID - 30` |
| **60** | **Hatch** | Hatch Digitama in Breeding Area. | |
| **61** | **Move** | Move Digimon from Breeding to Battle Area. | |
| **62** | **Pass Turn** | Pass turn to opponent. | Sets Memory to Opponent's +3. |
| **63** | **Unsuspend** | Unsuspend All (Refresh Phase). | |
| **100 - 399** | **Attack** | Attack with Field Digimon. | `ID = 100 + (AttackerIndex * 15) + TargetIndex` <br> `AttackerIndex`: 0-11 (My Battle Slot) <br> `TargetIndex`: 0=Security, 1-12=Enemy Slot 0-11 |
| **400 - 999** | **Digivolve** | Digivolve Hand Card onto Field Digimon. | `ID = 400 + (HandIndex * 15) + FieldIndex` <br> `HandIndex`: 0-29 <br> `FieldIndex`: 0-11 (Battle), 12 (Breeding) |
| **1000 - 1999** | **Activate Effect** | Activate [Main] Effect of Field Card. | `ID = 1000 + (SourceIndex * 10) + EffectIndex` <br> `SourceIndex`: 0-11 (Battle Slot) <br> `EffectIndex`: 0-9 (Effect choice) |

## Context 2: Pending Selection (Target/Material)

When the game enters a `SelectTarget` or `SelectMaterial` phase (e.g., triggered by playing an Option card or DNA Digivolving), the actions re-map to selection pointers.

| Action ID Range | Target | Description |
| :--- | :--- | :--- |
| **0 - 29** | **Hand Card** | Select card at Hand Index `X`. |
| **100 - 111** | **My Field** | Select my Field Slot `X` (ID - 100). |
| **115 - 126** | **Opponent Field** | Select opponent Field Slot `X` (ID - 115). |
| **62** | **Pass / Cancel** | Cancel selection or Finish sequence (if allowed). |

## Context 3: Block Timing (Opponent Turn)

When the opponent attacks, the game enters `BlockTiming` phase.

| Action ID Range | Action | Description |
| :--- | :--- | :--- |
| **100 - 111** | **Block** | Block with Digimon at Field Slot `X` (ID - 100). |
| **62** | **Pass** | Decline to Block. |

## Context 4: Counter Timing (Opponent Turn)

Triggered when opponent attacks, before block timing.

| Action ID Range | Action | Description |
| :--- | :--- | :--- |
| **400 - 999** | **Blast Digivolve** | Hand Index `X` onto Field Index `Y`. (Same formula as Digivolve) |
| **62** | **Pass** | Decline Counter. |

## Context 5: Selection (Trash / Source)

| Action ID Range | Target | Description |
| :--- | :--- | :--- |
| **0 - 59** | **Trash Card** | Select Trash Card at Index `ID`. (Only in `SelectTrash` phase) |
| **2000 - 2119** | **Source Card** | `ID = 2000 + (FieldsSlot * 10) + SourceIndex`. (Only in `SelectSource` phase) |
