## Why

DCGO has no concept of a *format*. It has one global boolean — `useBanlist` ([ContinuousController.cs:892](DCGO/Assets/Scripts/Script/ContinuousController.cs)) — and a single banlist fetched from one endpoint. Room match encodes that boolean into the room name so both seats agree; **random match ignores it entirely**, so banlist-on and banlist-off players are matched against each other today. Community formats (Singleton, EDEN) can only be played by agreeing out-of-band and trusting your opponent, because nothing validates the opposing decklist against anything.

We want a standalone, upstreamable DCGO mod that turns that lone boolean into a real format axis: a format is chosen per match, has its own matchmaking queue, is enforced against both players' actual decklists, and is respected by the deck builder. **Singleton** and **EDEN** both ship in this change.

## What Changes

**Scope discipline.** This change is deck-legality only. No gameplay-rules, engine, board, or network-topology changes; every format is 2-player, 50-card, standard play. It is a **standalone** mod intended to be offered upstream to `DCGO2/DCGO`, so it must be cut from `origin/main` and must not depend on this fork's `Assets/Scripts/Script/Recording/` harness layer.

- **Format axis (new).** Generalize the `useBanlist` boolean into a format descriptor set: `(rarity policy, banlist, max copies)`. The two existing boolean values become the first two formats — `useBanlist == true` → **Standard**, `useBanlist == false` → **No Banlist** — so this reads as widening an existing axis, not adding a parallel concept.
- **Singleton format (new).** 50-card main deck, 0–5 egg deck, **exactly one copy of any card ID across main and egg**, no banlist at all (no bans, no limits, no banned-pair/choice groups), all rarities, no commander.
- **EDEN format (new).** 50-card main deck, 0–5 egg deck, up to 4 copies. Common and uncommon cards legal by default; **Digi-Eggs legal at any rarity**; rare-or-higher main-deck cards illegal **unless** they are EDEN Anomaly Protocol cards (rare/promo Tamers; rare, super-rare or promo Memory Boost options; promo Training options; promo Scramble options), with **at most 4 Anomaly Protocol cards in a deck**. EDEN's own banned/limited/banned-pair list applies. Rules source: `docs/EDEN_FORMAT_RULES.md` (EDEN Format Rules & Guidance v1.1.1).
- **EDEN format data ships with the mod (new).** EDEN's banlist and Anomaly Protocol are carried as a shipped data asset mirroring the shape of this repo's `data/deck_formats.json`, so the two registries can be mechanically diffed and a rules revision is a data-only change. The existing official-banlist fetch is untouched and no new endpoint is required.
- **Format selection in the battle flow (new).** A format-select step between mode select and deck select, built on the existing `YesNoObject` window pattern ([SelectBattleMode.cs:70-115](DCGO/Assets/Scripts/Script/SelectBattleMode.cs)). It shows live per-format queue population, which is free from the room list already delivered to `OnRoomListUpdate`.
- **Per-format matchmaking (new).** Random match gains a per-format room key; room match extends its existing `RoomName + "-" + useBanlist` handshake ([RoomManager.cs:156](DCGO/Assets/Scripts/Script/RoomManager.cs), [EnterRoom.cs:90](DCGO/Assets/Scripts/Script/EnterRoom.cs)). **Legacy encodings are preserved byte-identically** (`"12345-True"`, `"12345-False"`, key `randomMatchRoom`); only new formats get new tokens. Unmodded clients therefore keep matching exactly as they do now and simply cannot see format rooms.
- **Explicitly NOT splitting the existing pool.** The standard/no-banlist mixing bug in random match is left as-is and reported separately. Fixing it would fragment a live player base and turn a mergeable PR into a debate.
- **Mutual deck validation (new).** Each client independently validates the *opponent's* decklist against the agreed format. This needs no server and no trust: `SignUpBattleDeckData()` already publishes the full deck code into the room-visible `PhotonNetwork.LocalPlayer.CustomProperties["BattleDeckData"]` before the battle scene loads ([ContinuousController.cs:1779](DCGO/Assets/Scripts/Script/ContinuousController.cs)). Room match gates on its existing ready check; random match needs a new gate.
- **Format-aware deck builder (new).** A persisted builder format selector drives per-card caps, add-refusal, out-of-pool grey-out, and the deck-wide EDEN anomaly cap, and reports per-deck legality in the deck list.
- **Safety rule — format is a read-only lens (new).** `DeckBuildingRule.ModifiedDeckData()` does not flag over-limit cards, it **deletes** them, and it is run across the entire deck library and saved to disk ([ContinuousController.cs:605-614](DCGO/Assets/Scripts/Script/ContinuousController.cs)) and on import ([CreateNewDeckButton.cs:82](DCGO/Assets/Scripts/Script/CreateNewDeckButton.cs)). Pointed at Singleton it would silently strip every saved deck to its distinct-card count; pointed at EDEN it would strip every rare-or-higher card. The selected format MUST NOT reach that path; only the official banlist may ever mutate stored decks.
- **BREAKING (local only): the `UseBanlist` PlayerPref is superseded** by a format-id preference, with a one-time migration (`true`→Standard, `false`→No Banlist). No network or room-code compatibility is broken.

## Capabilities

### New Capabilities
- `dcgo-format-axis`: The format descriptor set in DCGO, the Singleton format definition, the generalization of `useBanlist` into a format id with legacy-identical encoding, and format persistence/migration.
- `dcgo-eden-format`: The EDEN format — its rarity policy including the Digi-Egg exception, the EDEN Anomaly Protocol and its deck-wide cap, EDEN's banned/limited/banned-pair list, and the shipped data asset those are read from.
- `dcgo-format-selection-flow`: The format-select step in the battle flow, its placement between mode select and deck select, live per-format queue population, and deck-select integration.
- `dcgo-format-matchmaking`: Per-format room keys and queues for random match and room match, backwards compatibility with unmodded clients, and non-fragmentation of the existing pool.
- `dcgo-format-deck-validation`: Mutual client-side validation of both decklists against the agreed format, and the per-mode gate points at which an illegal deck blocks the match.
- `dcgo-format-aware-builder`: The builder format selector, the gating call sites it drives, per-deck legality display, and the read-only-lens rule that protects stored decks from destructive fixup.

### Modified Capabilities
<!-- None. This change lives entirely in the DCGO submodule (C#). It does not alter the
     requirements of this repo's `deck-format-registry` or `deck-builder-format-selection`
     specs, which govern the Rust engine and our own frontend. -->

## Impact

**Codebase:** the `DCGO/` submodule only — no Rust, Python, or frontend changes in this repo.

- **Format axis / persistence:** `ContinuousController.cs` (`useBanlist`, `BanList`, `LoadBanListOnline`), `DeckBuildingRule.cs` (`MaxCount_BanList`, `CanAddCard`, `ModifiedDeckData`).
- **EDEN data + rarity policy:** a new shipped format-data asset; `CEntity_Base.cs` (`rarity`, `cardKind`, `CardName_ENG`) is read but not modified.
- **Battle flow:** `SelectBattleMode.cs`, `SelectBattleDeck.cs`, plus a new format-select component reusing `YesNoObject`.
- **Matchmaking:** `LobbyManager_RandomMatch.cs` (room key, `GetRandomMatcingRoom`, auto-start gate), `RoomManager.cs`, `EnterRoom.cs`.
- **Validation:** reads the existing `ContinuousController.DeckDataPropertyKey` player property; gates in `RoomManager.cs` (`AllPlayerIsReady`) and `LobbyManager_RandomMatch.cs`.
- **Builder:** `CardPrefab_CreateDeck.cs`, `EditDeck.cs`, `DeckInfoPrefab.cs`, `DeckListPanel.cs`, `CEntity_Base.cs` (`IsStandardValid`, currently a hardcoded `true` stub already wired to a `Cover_Standard` grey-out overlay).
- **Settings:** `GameplayOption.cs` (the banlist toggle becomes a format selector; it already knows to lock itself while `PhotonNetwork.InRoom`).

**Dependencies:** none new. No server, no new endpoint, no Photon AppId or `GameVersion` change. EDEN's data ships with the mod.

**Out of scope (noted, not built):** EDEN Singleton (`docs/EDEN_FORMAT_RULES.md` defines it as EDEN's pool and banlist played highlander). Once both this change's EDEN pool policy and Singleton copy rule exist, it is one additional descriptor entry with no new machinery — left out because it was not requested, not because it is expensive.
