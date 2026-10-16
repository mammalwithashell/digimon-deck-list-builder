## Context

DCGO's only format-like axis is `useBanlist`, a persisted boolean ([ContinuousController.cs:892](DCGO/Assets/Scripts/Script/ContinuousController.cs)) backed by a single banlist fetched from `https://www.dcgo.online/Banlist.json`. That boolean already behaves like a one-bit format field:

| Surface | Today |
|---|---|
| Persistence | `PlayerPrefs["UseBanlist"]` |
| Settings UI | `GameplayOption.cs:124`, self-locks while `PhotonNetwork.InRoom` (`:127`) |
| Deck builder | `DeckBuildingRule.MaxCount_BanList` / `CanAddCard` / `ModifiedDeckData` |
| Room match | encoded in the room name: `RoomName + "-" + useBanlist` → `"12345-True"` |
| Random match | **not encoded at all** — one mixed pool |

Three further facts from the codebase shape everything below:

1. **Decklists are already public.** `SignUpBattleDeckData()` ([ContinuousController.cs:1779](DCGO/Assets/Scripts/Script/ContinuousController.cs)) writes the full deck code into `PhotonNetwork.LocalPlayer.CustomProperties["BattleDeckData"]` while still in the lobby. Photon player properties are room-visible, and opponents already read this key (`CardObjectController.cs:183/300/380`, `TurnStateMachine.cs:470`). Enforcement therefore needs no server and no trust.
2. **The deck-fixup path is destructive.** `DeckBuildingRule.ModifiedDeckData()` does not flag over-limit cards — it removes them in a `while` loop — and it is run over the whole library and saved to disk ([ContinuousController.cs:605-614](DCGO/Assets/Scripts/Script/ContinuousController.cs)) and on import ([CreateNewDeckButton.cs:82](DCGO/Assets/Scripts/Script/CreateNewDeckButton.cs)).
3. **The two match modes have different chokepoints.** Room match already has a ready gate (`AllPlayerIsReady()`, `RoomManager.cs:397`, master starts at `:425`). Random match has none — it fires the moment `PlayerCount == MaxPlayers` ([LobbyManager_RandomMatch.cs:481](DCGO/Assets/Scripts/Script/LobbyManager_RandomMatch.cs)).

The overriding constraint is that this is intended as a **pull request to `DCGO2/DCGO`**, not a private fork. Diff size, idiomatic fit, and backwards compatibility are first-class design inputs, not polish.

## Goals / Non-Goals

**Goals:**
- Turn `useBanlist` into a format axis whose first two members are exactly its two existing values, so the change reads as widening an axis rather than adding a concept.
- Ship **Singleton** and **EDEN** end-to-end: builder → format select → per-format queue → mutual validation → match.
- Preserve byte-identical behaviour for unmodded clients: same room names, same random-match key, same pool.
- Enforce format legality against both players' real decklists, client-side, with no server.
- Make it impossible for a format selection to destroy a stored deck.
- Keep EDEN's rules data separable from code, so a rules revision is a data-only change.

**Non-Goals:**
- Any gameplay, rules-engine, board, or network-topology change. Every format here is 2-player, 50-card, standard play.
- The 4-player EDH/Commander format described in `docs/EDH_COMMANDER_MODE.md`. The format named here is **Singleton** — 1v1, no commander — and is unrelated to it.
- EDEN Singleton. It is one descriptor entry once this change lands, but it was not requested.
- Fixing the existing standard/no-banlist mixing in random match.
- Any dependency on this fork's `Assets/Scripts/Script/Recording/` harness layer. (Structurally enforced: that directory does not exist on `develop`.)
- **`LobbyManager_FriendMatch`.** It is a third matchmaking path with its own key (`"ランダムマッチ部屋"`) and inverted `!Contains` scan logic, so it would otherwise be a candidate for per-format keys. It is **dead code on `develop`**: no C# file references the class, and its script guid `243575478ae357c43a861a58ef802c05` is referenced by no scene, prefab, or settings asset anywhere under `Assets/` or `ProjectSettings/`. Adding format handling there would be untestable diff on an unreachable path. If upstream later revives friend match, it needs the same treatment as the other two paths.
- Parameterising deck size — all formats in scope are 50 cards.

## Decisions

### D1 — Format generalizes `useBanlist`; it does not sit beside it

A format is a descriptor `(id, displayName, rarityPolicy, banlist, maxCopies)`. The two legacy boolean values become the first two descriptors:

```
  useBanlist == true   ->  standard        (rarity: all,  banlist: official, maxCopies: 4)
  useBanlist == false  ->  no_restriction  (rarity: all,  banlist: none,     maxCopies: 4)
  new                  ->  singleton       (rarity: all,  banlist: none,     maxCopies: 1)
  new                  ->  eden            (rarity: eden, banlist: eden,     maxCopies: 4)
```

Every format in scope is a point in this space; none of them needs a bespoke code path. That is the property that makes EDEN Singleton a one-line addition later, and it is worth protecting when implementing: resist `if (format == Eden)` branches in favour of asking the descriptor.

*Alternative considered:* a separate `Format` field alongside `useBanlist`. Rejected — it creates two axes that can contradict each other (what is "Standard with the banlist off"?), doubles the room-name encoding space, and presents upstream with a redundant concept rather than a wider one.

### D2 — Encode the format in the room name; keep legacy strings byte-identical

Room match extends the existing suffix; random match extends the existing key:

```
  ROOM MATCH                             RANDOM MATCH
  "12345-True"      -> standard  (as-is)  "<rand8>randomMatchRoom"    -> legacy (as-is)
  "12345-False"     -> no_restr. (as-is)  "<rand8>singletonMatchRoom" -> singleton (new)
  "12345-singleton" -> singleton (new)    "<rand8>edenMatchRoom"      -> eden (new)
  "12345-eden"      -> eden      (new)
```

Because the legacy encodings are unchanged, unmodded clients match exactly as they do today. Because new formats use *different* tokens rather than a suffix appended to the old one, an unmodded client's `name.Contains("randomMatchRoom")` scan can never match a Singleton room — cross-version contamination is structurally impossible rather than merely unlikely.

*Alternatives considered:* `PhotonNetwork.JoinRandomRoom(expectedCustomRoomProperties, ...)` is the idiomatic Photon mechanism and would delete roughly a hundred lines of manual scan/leave/rejoin churn, but it replaces upstream's matchmaking core — the worst possible diff for a PR and for future rebases. A `TypedLobby` per format gives the strongest isolation and remains the better answer if the queue count ever grows, but it is a larger behavioural change than needed for three formats. Bumping `PhotonNetwork.GameVersion` ([ContinuousController.cs:1650](DCGO/Assets/Scripts/Script/ContinuousController.cs)) would isolate everything at once but partitions the entire client, not the format.

### D3 — Match format keys exactly, never by `Contains`

The existing scan uses `roomInfo[i].Name.Contains(RandomKey)`. With one key that is safe; with a family of keys it is not — a future `eden_singleton` key is a superstring of `singleton`, and relying on letter case to keep them apart is a latent bug. The new matcher parses the format token out of the room name and compares it for equality. Cost is trivial and it removes an entire class of future defect.

### D4 — Add queues; split none

Random match currently pools banlist-on and banlist-off players together. That is a real defect, but correcting it in this change would fragment a live player base and convert a mechanical PR into a product argument. Legacy rooms keep their exact current (mixed) semantics; only the new formats get their own queues. The defect is reported upstream separately.

### D5 — Mutual client-side validation, with mode-specific gates

Both clients independently parse the opponent's `BattleDeckData` deck code and validate it against the agreed format. Neither client trusts the other's claim, and there is no authority to add.

```
  ROOM MATCH                             RANDOM MATCH
  ready gate exists (RoomManager:397)    no gate (auto-fires at :481)
        |                                       |
        v                                       v
  refuse READY while either deck is      validate on join; on mismatch
  illegal; surface the reason            LEAVE + REQUEUE, imitating the
                                         existing retry at OnJoinRoomFailed:305
```

Validation must **fail closed on ambiguity**: an unparseable deck code or an unknown card ID is a failed validation, not a pass. A player who cannot be validated cannot enter a format match.

*Alternative considered:* trusting a self-declared format flag in player properties. Rejected — the user explicitly asked for validation, and the data needed to do it properly is already on the wire.

### D6 — Format is a read-only lens; only the official banlist may mutate stored decks

This is the load-bearing safety decision. The builder call sites split into two classes that must never be conflated:

```
  MAY be format-aware (they GATE - non-destructive)
    MaxCount_BanList()   CardPrefab_CreateDeck.cs:374   per-card cap in the picker
    CanAddCard()         CardPrefab:470, EditDeck:852/1011/1138/1176
    IsStandardValid      CEntity_Base.cs:74 (stub) -> CardPrefab:507 Cover_Standard grey-out

  MUST NOT see the selected format (they DELETE)
    ModifiedDeckData()   ContinuousController.cs:611  library-wide, then SaveDeckDatas()
                         CreateNewDeckButton.cs:82    import path
                         DeckData.cs:806
```

If the selected format reached `ModifiedDeckData` while set to Singleton, every saved 50-card deck would be silently stripped to its distinct-card count (roughly 15–20 cards) and written over the original, and any imported decklist would be shredded on arrival. Legality against a *selected* format is reported, never repaired. `ModifiedDeckData` continues to see only the official banlist, exactly as today.

*Alternative considered:* changing `ModifiedDeckData` to flag rather than delete. That is arguably the correct fix, but it changes long-standing upstream behaviour and belongs in its own PR; bundling it here would make this change contentious for reasons unrelated to formats.

### D7 — Two loosely-coupled format axes: a persisted builder format, a per-match format

The builder format is a persisted preference occupying the slot `UseBanlist` holds today — same convention, same shape, so it reads as a generalization. The match format is chosen per match in the battle flow and defaults to the builder format. They stay independent: a player may build in Singleton and still queue Standard.

*Alternative considered:* attaching the format to the deck as metadata. Rejected — DCGO deck codes are exchanged between players as strings, so adding a field is a compatibility break with every existing exported list.

### D8 — EDEN's rules data ships with the mod; the official banlist fetch is untouched

EDEN needs a second banlist and an Anomaly Protocol definition. The two candidate homes were a shipped data asset and a multi-format extension of `dcgo.online/Banlist.json`. **The data ships with the mod.** The endpoint option is strictly better for freshness but requires the upstream maintainers to change *their* service, which this change cannot do unilaterally and cannot block on. `LoadBanListOnline` is left exactly as it is, still serving the official list for `standard`.

To limit the staleness cost, EDEN's data is a plain data asset — banlist and Anomaly Protocol both — so a rules revision is a data-only pull request with no code review surface.

**Shape is constrained by `JsonUtility`.** DCGO parses its banlist with `JsonUtility.FromJson<BanList>` and the project carries no other JSON library (`Packages/manifest.json` has only `com.unity.modules.jsonserialize`; no Newtonsoft). `JsonUtility` cannot deserialize dictionaries, top-level arrays, or nested generic collections — so `data/deck_formats.json`'s `limited_to` object and its `choice_groups` list-of-lists-of-lists are **not parseable as written**. The DCGO asset therefore carries the same *content* in a flat, `JsonUtility`-compatible encoding:

| `data/deck_formats.json` | DCGO asset |
|---|---|
| `banned: [id, …]` | `restrictions: [{ id, limit: 0 }, …]` |
| `limited: [id, …]` | `restrictions: [{ id, limit: 1 }, …]` |
| `limited_to: { id: n }` | `restrictions: [{ id, limit: n }, …]` |
| `choice_groups: [[[a],[b]]]` | `bannedPair: [{ id: a, pairs: [b] }]` |
| `anomaly_protocol.categories[]` | `anomalyCategories: [{ cardKind, nameContains, rarities: [] }]` |

The correspondence is a **documented 1:1 mapping, not shape identity** — which is what the drift check must compare. Adding a dependency purely to preserve byte-shape symmetry would be a worse trade in a PR to someone else's project, and the flat encoding is the shape DCGO's existing `BanList` already uses.

*Consequence to accept:* two copies of EDEN's list now exist, here and in `data/deck_formats.json`. That is a real cost. It is bounded by keeping the shapes identical and by a task that diffs them, and it is preferable to making the PR depend on a third party changing a service.

### D12 — EDEN's rarity policy has a Digi-Egg exception, and unknown rarity fails closed

Per `docs/EDEN_FORMAT_RULES.md`: commons and uncommons are legal by default, **Digi-Eggs are legal at any rarity**, and rare-or-higher main-deck cards are illegal unless they qualify under the Anomaly Protocol. The egg exception is easy to miss and is not expressible as a plain rarity mask — the check must be `cardKind`-aware.

DCGO's `Rarity` enum carries values our registry does not model (`UR`, and `None` for missing data). "Rare or higher" is therefore defined as **anything that is not `C` and not `U`**, rather than as an enumerated list of high rarities. `None` consequently reads as rare-or-higher and is illegal unless it qualifies as an anomaly — failing closed, consistent with D5.

### D13 — The Anomaly Protocol cap is deck-wide, so it is enforced in `CanAddCard`, not in the per-card cap

At most 4 Anomaly Protocol cards may appear in a deck, counted as **copies, not distinct card IDs** — four copies of one rare Tamer exhausts the allowance. This is the first constraint in the codebase that depends on the whole deck rather than on one card's count, so it does not fit `MaxCount_BanList`, which returns a per-card maximum with no deck context. It goes in `CanAddCard`, which already receives the `DeckData`.

Anomaly qualification is evaluated from the descriptor's category rules — `cardKind` plus an optional case-insensitive English-name substring plus a set of qualifying rarities — never from a hardcoded card list, so appending a category or an explicit card ID stays a data change. This mirrors how our engine models the same rules and is what keeps `if (format == Eden)` out of the builder.

### D14 — EDEN reuses DCGO's existing banlist structures

DCGO's `BanList` already carries `Restrictions` (card id, limit) and `BannedPair` (card id, incompatible ids). EDEN's list maps onto it without new types: banned becomes limit 0 (which `MaxCount_BanList` and `CanAddCard` already handle correctly as "cannot add"), limited becomes limit 1, `limited_to` becomes an explicit limit N, and choice groups become banned pairs. The only change is that a descriptor now selects *which* `BanList` applies rather than a boolean selecting whether one applies.

### D9 — Singleton counts across main deck and egg deck

One copy of any card ID across both zones; an egg deck is therefore up to 5 *distinct* eggs. This matches this repo's own engine precedent ([deck_tools.rs:415-448](code/digimon-engine/src/deck_tools.rs) counts main and egg together) and the singleton rule described in `docs/EDH_COMMANDER_MODE.md`.

### D10 — Do not parameterise deck size

All three formats are 50 cards, so the `DeckCards().Count != 50` check at [DeckData.cs:692](DCGO/Assets/Scripts/Script/DeckData.cs) stays as-is. Parameterising it now would add diff for no behavioural gain; EDEN does not need it either.

### D11 — Branch from upstream, not from this fork

The implementation branch is cut from `origin/develop` (`DCGO2/DCGO` — upstream's default branch is `develop`, there is no `main`), not from the `fork/` branch carrying this repo's recorder-hook commits. Nothing in this change may reference `Assets/Scripts/Script/Recording/`. Because the shared base-repo DCGO checkout sits on the recorder branch with a large body of uncommitted card assets, the work happens in a dedicated **git worktree of the submodule** — which shares the object store and is therefore not the multi-GB per-worktree clone that repo rule 29 prohibits.

## Risks / Trade-offs

- **Silent library destruction via `ModifiedDeckData`** → The single most severe risk. A Singleton-aware fixup pass would irreversibly shred every saved deck to its distinct-card count; an EDEN-aware one would strip every rare-or-higher card from every deck. Mitigated by D6 as an architectural rule, and by explicit regression tests — one per format — asserting that switching the builder format never mutates a stored deck.
- **EDEN's data drifts from `data/deck_formats.json`** → Accepted cost of D8. Mitigated by keeping the two assets shape-identical and by a task that diffs them; revisited if upstream ever agrees to serve a multi-format banlist.
- **EDEN's rarity data may be incomplete in DCGO's card database** → Cards carrying `Rarity.None` are treated as rare-or-higher and become illegal in EDEN. This fails closed, but a systematically missing rarity field would make EDEN unplayable rather than merely strict. Mitigated by auditing rarity coverage across the card database before shipping EDEN, as an explicit task rather than an assumption.
- **Queue fragmentation on a small player base** → New formats may sit empty and feel broken. Mitigated by D4 (no existing pool is split, so nothing regresses), by surfacing live per-format population in the format-select view (free from the room list already delivered to `OnRoomListUpdate`, `LobbyManager_RandomMatch.cs:262`), and by room match working at any population since it is a room code rather than a pool.
- **Pre-merge cross-version contamination** → Before upstream merges, modded and unmodded clients share a Photon app. Mitigated structurally by D2: unmodded scans cannot match the new keys.
- **Upstream may not want hard-blocking validation** → Mitigated by scoping the block to non-legacy formats only; Standard and No Banlist behave exactly as today, so the blocking path is opt-in by format choice and can be softened to a warning without touching the rest of the design.
- **Rebase drift against upstream** → Mitigated by D2's minimal-diff choice and by concentrating changes at existing chokepoints (`MaxCount_BanList`, `CanAddCard`, the room-name construction sites, the ready gate) rather than spreading them.
- **PlayerPref migration** → `UseBanlist` is superseded by a format id. Mitigated by a one-time read-and-convert (`true`→standard, `false`→no_restriction) and by defaulting any unrecognised persisted format to Standard, which is also the rollback behaviour.
- **Validation adds a lobby round-trip** → Parsing an opponent's deck code costs a card-database lookup per distinct ID. Acceptable at lobby time; it must not run per-frame.

## Migration Plan

1. Cut the branch from `origin/develop` into a dedicated git worktree so the shared recorder checkout is untouched; confirm no `Recording/` dependency.
2. Land the format descriptor set with only `standard` and `no_restriction` populated, migrating the `UseBanlist` pref. At this point behaviour is provably identical — a useful bisection point.
3. Add `singleton` to the descriptor set and make the builder gating call sites format-aware (D6's safe list only). Singleton exercises the copy-limit axis alone, with no pool machinery.
4. Add the EDEN data asset, the rarity policy, and the Anomaly Protocol. EDEN exercises the pool axis, on a skeleton already proven by step 3.
5. Add the format-select view and the per-format room keys.
6. Add mutual validation, room match first (existing gate) then random match (new gate).
7. Verify an unmodded client still creates, finds, and joins legacy rooms unchanged.

Steps 3 and 4 are deliberately ordered so that the copy-limit and pool axes land separately; a bug in EDEN's rarity handling cannot then be confused with a bug in the descriptor plumbing.

**Rollback:** any unrecognised persisted or received format id resolves to Standard, so an older client, a reverted build, or a corrupted preference degrades to today's behaviour rather than failing.

## Open Questions

- **Will upstream take EDEN's data as a shipped asset**, or would they prefer to serve it from `dcgo.online/Banlist.json` alongside the official list? D8 chooses the asset because it is the only option this change can deliver alone, but if the maintainers are willing to extend the endpoint, moving to it later is a small change and removes the drift risk entirely. Worth asking during PR review rather than before.
- **Does upstream want the random-match standard/no-banlist pool split** as a follow-up, given it fragments their live population?
- **Should EDEN Singleton ship too?** It is EDEN's pool and banlist with `maxCopies: 1` — one descriptor entry once this change lands, no new machinery. Excluded because it was not requested; trivially addable if wanted.
- **Display naming** if EDEN Singleton ever ships — Singleton and EDEN Singleton need to be distinguishable at a glance in the format list, not just in the key space.
