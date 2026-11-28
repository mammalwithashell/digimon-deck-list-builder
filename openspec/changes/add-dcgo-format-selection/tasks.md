## 1. Branch and baseline

- [x] 1.1 Add a dedicated git worktree of the DCGO submodule (shares the object store; NOT a fresh clone) on a branch cut from `origin/develop` — upstream's default branch — leaving the shared `add-recording-mod-r2` checkout and its uncommitted card assets untouched
- [x] 1.2 Confirm the branch builds in Unity before any change, and record the baseline so later regressions are attributable
- [x] 1.3 Add a guard note to the branch README (or PR description draft) stating that nothing in this change may reference `Assets/Scripts/Script/Recording/`
- [x] 1.4 Capture the current room-name strings produced for `useBanlist == true` and `== false` verbatim, to assert byte-identity against later
- [x] 1.5 Audit rarity coverage in DCGO's card database: count cards whose rarity is absent or unrecognised, since EDEN treats those as rare-or-higher and they would become illegal

## 2. Format axis (behaviour-identical checkpoint)

- [x] 2.1 Add a format descriptor type carrying id, display name, rarity policy, banlist reference, and max copies per card ID
- [x] 2.2 Define the `standard` and `no_restriction` descriptors so they reproduce the existing `useBanlist == true` / `== false` semantics exactly
- [x] 2.3 Add descriptor lookup that resolves an unknown id to `standard` rather than failing
- [x] 2.4 Replace the `useBanlist` boolean with a persisted format id in `ContinuousController`, keeping the existing preference slot
- [x] 2.5 Implement the one-time preference migration (`true` → `standard`, `false` → `no_restriction`) and make it idempotent across restarts
- [x] 2.6 Route `DeckBuildingRule.MaxCount_BanList` and `CanAddCard` through the descriptor instead of reading the boolean
- [x] 2.7 Make the descriptor select *which* banlist applies, so a format can carry its own list rather than toggling the official one on and off
- [x] 2.8 Verify behaviour is unchanged at this checkpoint: same deck-building limits, same room names, same matchmaking, for both legacy values

## 3. Singleton format (copy-limit axis)

- [x] 3.1 Add the `singleton` descriptor: all rarities, no banlist, max 1 copy per card ID
- [x] 3.2 Implement the singleton copy check counting main deck and Digi-Egg deck together
- [x] 3.3 Add a deck-legality evaluation entry point that reports legal/illegal plus the offending card IDs, without mutating the deck
- [x] 3.4 Confirm banned, limited, and banned-pair cards are all legal as single copies under `singleton`
- [x] 3.5 Confirm a 50-card distinct main deck plus five distinct Digi-Eggs evaluates as legal

## 4. EDEN format data

- [x] 4.1 Add a shipped format-data asset mirroring the structure of this repo's `data/deck_formats.json` (`restrictions` with `banned` / `limited` / `limited_to` / `choice_groups`; `anomaly_protocol` with `categories` / `max_total` / `extra_card_ids`)
- [x] 4.2 Populate EDEN's banned, limited, explicitly-limited, and choice-group entries from `data/deck_formats.json`, preserving the `EX02-007` → `EX2-007` normalisation
- [x] 4.3 Populate the Anomaly Protocol categories: rare/promo Tamers; rare, super-rare, promo Memory Boost options; promo Training options; promo Scramble options; cap of 4
- [x] 4.4 Map EDEN's list onto DCGO's existing `BanList` structures — banned as limit 0, limited as limit 1, `limited_to` as an explicit limit, choice groups as banned pairs
- [x] 4.5 Confirm loading EDEN's data requires no network call and leaves `LoadBanListOnline` and the official banlist path untouched
- [x] 4.6 Add a check that diffs the shipped EDEN data against this repo's `data/deck_formats.json` so drift between the two copies is detectable

## 5. EDEN format (pool axis)

- [x] 5.1 Add the `eden` descriptor: EDEN rarity policy, EDEN banlist, max 4 copies
- [x] 5.2 Implement the EDEN rarity gate as "anything that is not common and not uncommon is rare-or-higher", so unrecognised and absent rarities fail closed
- [x] 5.3 Implement the Digi-Egg exception: eggs are legal at any rarity and remain subject to EDEN's banned and limited list
- [x] 5.4 Implement Anomaly Protocol qualification from the data's category rules — card kind, optional case-insensitive English-name substring, qualifying rarities — plus the explicit card-ID list
- [x] 5.5 Implement the deck-wide anomaly cap counting total copies rather than distinct card IDs, and confirm Digi-Eggs do not consume the allowance
- [x] 5.6 Verify against the worked cases: a rare Tamer qualifies, a non-promo Training option does not, a rare Digimon does not, four copies of one rare Tamer exhausts the cap
- [x] 5.7 Review the implementation for `if (format == Eden)` branches and replace them with descriptor queries

## 6. Format-aware builder and the read-only-lens guard

- [x] 6.1 Replace the banlist toggle in `GameplayOption` with a format selector, preserving the existing lock-while-in-room behaviour
- [x] 6.2 Make the per-card cap in `CardPrefab_CreateDeck` reflect the selected builder format, bounded by the card's printed maximum
- [x] 6.3 Make the add-refusal paths in `EditDeck` reflect the selected builder format
- [x] 6.4 Enforce the EDEN anomaly cap in `CanAddCard`, which already receives the deck, since the cap is deck-wide and cannot be expressed as a per-card maximum
- [x] 6.5 Surface the current anomaly count against the cap in the builder while the format is `eden`
- [x] 6.6 Drive the existing `Cover_Standard` card-cover affordance from a format-aware legality predicate, replacing the hardcoded `IsStandardValid` stub
- [x] 6.7 Confirm the cover applies to rare-or-higher non-anomaly main-deck cards under `eden` and does not apply to Digi-Eggs on rarity grounds
- [x] 6.8 Show per-deck legality against the selected builder format in the deck list and deck info panel
- [x] 6.9 **Guard:** confirm `DeckBuildingRule.ModifiedDeckData` still reads only the official banlist and cannot observe the selected format, at all three call sites (library load, import, `DeckData`)
- [x] 6.10 Add a regression test asserting that switching the builder format to `singleton` and reloading the library leaves every saved deck byte-identical
- [x] 6.11 Add a regression test asserting the same for `eden`, covering decks holding rare-or-higher cards and EDEN-banned cards
- [x] 6.12 Add a regression test asserting that importing a multi-copy decklist under `singleton`, and a rare-heavy decklist under `eden`, preserves every card and reports the deck illegal rather than reducing it

## 7. Format selection in the battle flow

- [x] 7.1 Add a format-selection window between mode selection and deck selection, reusing the existing `YesNoObject` command-list pattern
- [x] 7.2 Default the selection to the persisted builder format without writing back to it
- [x] 7.3 Wire the chosen format through to Random Match, Room Match, and Bot Match setup
- [x] 7.4 Make dismissing the window return to mode selection without connecting or creating a room
- [x] 7.5 Derive per-format waiting-room counts from the room list already delivered to `OnRoomListUpdate` and display them per format
- [x] 7.6 Show counts as unknown (not zero) when no room list has been received yet
- [x] 7.7 Distinguish illegal decks in deck selection and refuse selecting one for the chosen format, surfacing the reason
- [x] 7.8 Report clearly when the player has no legal deck for the chosen format

## 8. Per-format matchmaking

- [x] 8.1 Introduce a room-name format token with legacy encodings preserved byte-identically (`"-True"`, `"-False"`, key `randomMatchRoom`)
- [x] 8.2 Give `singleton` and `eden` their own random-match keys, each distinct from `randomMatchRoom` and from each other, so an unmodded substring scan cannot match either
- [x] 8.3 Replace the room-list `Contains` scan with exact format-token parsing and equality comparison
- [x] 8.4 Make room creation in random match use the queued format's key
- [x] 8.5 Extend room-match creation and joining to carry the format token, and keep the displayed shareable room code free of it
- [x] 8.6 Confirm joining a room code under a mismatched format fails with a clear message
- [x] 8.7 Confirm two clients queuing in different new formats never pair with each other
- [x] 8.8 Confirm the legacy mixed `standard` / `no_restriction` random-match pool is preserved and not split

## 9. Mutual deck validation

- [x] 9.1 Add a validator that parses an opponent's published `BattleDeckData` deck code into a decklist and evaluates it against a format
- [x] 9.2 Make the validator fail closed on unparseable deck data, unknown card IDs, and missing deck data
- [x] 9.3 Confirm the validator applies EDEN's rarity policy and anomaly cap to an opponent's deck, not only the copy limits
- [x] 9.4 Room match: block signalling readiness while the local deck is illegal, surfacing the reason
- [x] 9.5 Room match: block the match start while the opponent's published deck is illegal, surfacing the reason
- [x] 9.6 Room match: re-run validation when either player changes deck
- [x] 9.7 Random match: validate the opponent after pairing and before the battle scene loads
- [x] 9.8 Random match: on failure, leave the room and resume queuing in the same format, informing the player why
- [x] 9.9 Random match: ensure the requeue does not immediately rejoin the rejected room and cannot spin tightly on repeated failures
- [x] 9.10 Confirm `standard` and `no_restriction` matches never newly fail to start, including against an unmodded opponent

## 10. Compatibility verification and PR preparation

- [ ] 10.1 Verify an unmodded client creates, discovers, and joins legacy random-match rooms unchanged against an updated client
- [ ] 10.2 Verify an unmodded client scanning the room list does not match a `singleton` or `eden` room
- [ ] 10.3 Verify an updated client queuing `standard` can still be paired with an unmodded client
- [ ] 10.4 Verify rollback behaviour: an unrecognised persisted or received format id resolves to `standard` and the client starts normally
- [ ] 10.5 Build a legal EDEN deck in the builder end to end and confirm the pool, anomaly cap, and banlist all behave as specified
- [ ] 10.6 Play a full `singleton` match and a full `eden` match end to end, through both random match and room match
- [ ] 10.7 Review the final diff for size and idiomatic fit, confirming changes sit at existing chokepoints rather than spreading
- [ ] 10.8 Draft the upstream PR description covering the legacy-identical encoding guarantee, the deliberate non-splitting of the existing pool, the read-only-lens rule, and EDEN's data-asset choice
- [ ] 10.9 Raise with the maintainers whether EDEN's data should later move to a multi-format `Banlist.json`, which would remove the two-copy drift risk
- [ ] 10.10 File the random-match banlist-mixing defect upstream as a separate report, not bundled into this PR
