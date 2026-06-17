## Why

The deck builder already understands the engine-backed playable formats, but the play queue still uses stale format metadata, so EDEN and other supported formats are not consistently available from the play flow. Bot matches also mirror the player's selected deck, which makes the greedy bot less useful for practice and hides whether saved decks and starter decks can actually drive varied local games.

## What Changes

- Route the play format selection queue through the same engine registry-backed format catalog used by the deck builder.
- Enable all playable engine formats in the play queue, including No Banlist, Pauper, EDEN, and EDEN Singleton.
- Add a greedy-bot opponent deck pool that randomly selects from the first six starter decks and eligible saved player decks.
- Exclude the player's selected deck from saved-deck bot opponent candidates.
- Treat the first six starter decks as built-in practice opponents that are eligible in any selected play format.
- Keep saved-deck opponent candidates format-aware: they must be valid for the selected format before entering the bot pool.

## Capabilities

### New Capabilities
- `bot-opponent-deck-pool`: Local bot matches can choose a non-mirrored opponent deck from built-in starter decks and valid saved player decks.

### Modified Capabilities
- `deck-builder-format-selection`: The play format queue must consume the engine registry-backed playable format catalog consistently across desktop and hosted/browser surfaces.

## Impact

- **Frontend play flow:** format selection, deck selection, bot-match launch, and focused tests under `code/frontend/src/features/play/` and `code/frontend/src/pages/`.
- **Format APIs:** stale `/formats` and Tauri `formats_list` behavior may be replaced or bridged to the registry-backed `/decks/formats` / `rust_list_formats` path.
- **Deck data:** the first six starter deck compositions must be available to the frontend bot-deck pool from a deterministic source, pinned to ST1 through ST6 to avoid duplicate starter entries.
- **Tests:** unit tests for format catalog and bot opponent selection, plus play-flow coverage that bot launch no longer sends the selected deck as both players.
