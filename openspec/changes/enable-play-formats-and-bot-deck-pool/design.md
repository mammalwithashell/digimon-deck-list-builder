## Context

The deck builder and the play queue currently get format data through different paths. The deck builder uses `deckApi.listFormats()`, which maps to `/decks/formats` or `rust_list_formats` and is backed by the engine registry in `data/deck_formats.json`. The play queue uses `playApi.listFormats()`, which maps to `/formats` or Tauri `formats_list`; those surfaces still hardcode Standard and disabled concept placeholders, so supported formats like EDEN can be missing or locked in the queue.

Bot launch already receives both player deck payloads. `DeckSelectPage` currently passes the selected deck as both `deck` and `opponentDeck`, so the greedy bot always mirrors the player in the new play flow. Saved desktop decks are loaded through `deckLibraryAdapter`, while browser/hosted decks come through the existing deck APIs. Starter deck product lists exist in local data, but the implementation should pin exactly ST1 through ST6 so duplicate manual deck-library entries do not create duplicate opponent candidates.

## Goals / Non-Goals

**Goals:**
- Make the play format queue show the same playable engine formats as the deck builder on desktop and browser/hosted paths.
- Keep disabled concept formats available as presentational placeholders only when they are not engine-playable.
- Replace bot mirror behavior with a random opponent deck selected from ST1-ST6 and valid saved decks.
- Exclude the player's selected saved deck from bot opponent candidates.
- Keep the change frontend/API-contract scoped; no engine gameplay behavior, action masks, or tensor contracts change.

**Non-Goals:**
- Add new game formats beyond those already in the engine registry.
- Make starter decks themselves format-validated before entering the bot pool; ST1-ST6 are built-in practice opponents and remain eligible in any selected format.
- Add user-facing controls for choosing a specific bot deck or weighting starter versus saved decks.
- Change the greedy bot policy or pacing behavior.

## Decisions

1. **Use the registry-backed deck-format API for play format selection.**
   The play queue should consume the same live registry source as the deck builder, then overlay existing play-page presentation metadata such as taglines and population percentages. This avoids maintaining a second stale `/formats` list. The implementation can either update `playApi.listFormats()` to call `/decks/formats` / `rust_list_formats`, or make `/formats` / `formats_list` delegate to the same registry-backed mapping; the important contract is that the queue is no longer sourced from hardcoded Standard-only data.

2. **Keep bot opponent selection in the play-flow/frontend layer.**
   The selected saved deck list is already available in `DeckSelectPage`, and desktop/browser/hosted storage surfaces differ. Choosing the bot opponent in a shared frontend utility avoids adding a cross-runtime backend command that would need to understand Tauri local JSON, browser-dev desktop fallback storage, and hosted DB rows.

3. **Represent ST1-ST6 as pinned built-in bot decks.**
   The bot pool should use an explicit list of the first six starter deck ids/compositions rather than "all deck-library entries with format starter." The current deck library contains duplicate or variant names for some starter entries, so explicit ST1-ST6 fixtures keep the pool stable and predictable.

4. **Saved deck candidates remain format-aware; starter candidates are always eligible.**
   Saved decks should pass the same `canUseDeckForFormat` gate used for player selection and must not be the selected player deck. ST1-ST6 are special built-ins for practice and are included for any selected format, per product-deck eligibility decision.

5. **Randomness is launch-local and non-authoritative.**
   The first version can choose with `Math.random()` at bot launch. The shuffle seed continues to control game setup/shuffle behavior, not opponent-pool selection. If reproducible opponent selection becomes important, a later change can thread a seeded chooser without altering the pool contract.

## Risks / Trade-offs

- [Starter deck data drift] -> Pin ST1-ST6 by explicit ids and add tests that assert the pool contains exactly six starter candidates.
- [Format API split persists] -> Add tests around `playApi`/format catalog mapping so the queue reports the five registry-backed playable formats, not only Standard.
- [Bot pool has no saved valid decks] -> ST1-ST6 remain eligible in every format, so bot matches still launch without mirroring.
- [User expects selected format to constrain starter decks] -> The UI/spec treats starter decks as built-in practice opponents; saved decks remain format-constrained.
- [Random choice makes E2E tests flaky] -> Unit-test the pool construction separately and inject or mock the random picker in launch-path tests where needed.
