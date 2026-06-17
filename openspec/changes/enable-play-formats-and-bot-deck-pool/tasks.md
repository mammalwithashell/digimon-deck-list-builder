## 1. Format Catalog Unification

- [x] 1.1 Update the play-flow format loader so desktop and hosted/browser play selection consumes the engine registry-backed format list.
- [x] 1.2 Preserve the existing play-page presentation metadata while deriving playable format ids, names, descriptions, deck labels, and enabled state from the registry-backed data.
- [x] 1.3 Remove or bridge stale Standard-only `/formats` and Tauri `formats_list` behavior so tests cannot pass against the old hardcoded catalog.
- [x] 1.4 Add or update tests proving the play format queue enables Standard, No Banlist, Pauper, EDEN, and EDEN Singleton when those registry formats are playable.

## 2. Built-In Starter Bot Decks

- [x] 2.1 Add a deterministic frontend-accessible source for exactly the first six starter deck compositions: ST1, ST2, ST3, ST4, ST5, and ST6.
- [x] 2.2 Ensure each built-in starter candidate exposes a concrete `DeckResponse`-compatible payload with egg and main arrays in the order expected by `createBotGame`.
- [x] 2.3 Add tests that the built-in starter pool contains exactly one candidate for each ST1-ST6 deck and does not include duplicate manual deck-library variants.

## 3. Bot Opponent Pool Selection

- [x] 3.1 Create a shared play-flow utility that builds the bot opponent pool from ST1-ST6 plus saved decks that pass `canUseDeckForFormat`.
- [x] 3.2 Exclude the selected player deck from saved-deck opponent candidates while keeping all six starter decks eligible in every selected format.
- [x] 3.3 Add a random selection helper that chooses one concrete opponent deck at bot launch time and can be controlled or mocked in tests.
- [x] 3.4 Update `DeckSelectPage` bot launch to pass the selected player deck as player one and the chosen pool candidate as player two instead of mirroring the selected deck.

## 4. Verification

- [x] 4.1 Add unit tests for saved-deck filtering by format, selected-deck exclusion, starter fallback, and launch-local random choice.
- [x] 4.2 Update play-flow tests so bot launch payloads no longer use the same deck for both players.
- [x] 4.3 Run the focused frontend tests for format catalog, play-flow store/utility, and deck-selection launch behavior.
- [x] 4.4 Run `openspec status --change enable-play-formats-and-bot-deck-pool` and confirm the change remains apply-ready.
