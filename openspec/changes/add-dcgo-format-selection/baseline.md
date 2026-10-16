# Pre-change baseline

Captured from `origin/develop` @ `bf59c6e6c` in the dedicated worktree, **not** from the
recorder branch. Every string and count below is the "before" side of a byte-identity or
coverage claim made in `design.md`.

## Environment

| | |
|---|---|
| Upstream default branch | **`develop`** — there is no `main` |
| Branch | `format-selection`, tracking `origin/develop` @ `bf59c6e6c` |
| Worktree | `D:/dcgo-worktrees/format-selection` (git worktree of the submodule; shares the object store) |
| Shared base checkout | `DCGO/` on `add-recording-mod-r2`, untouched — verified same branch, same 8,349 dirty files, `Recording/` intact |

**`core.worktree` trap.** The submodule's shared config sets `core.worktree = ../../../DCGO`,
which a new worktree inherits — git then resolves its toplevel to the *base* checkout and
reports every file deleted. Fixed with `extensions.worktreeConfig=true` plus a per-worktree
`core.worktree`. Shared config left alone so the base still resolves correctly. **Any future
DCGO worktree needs the same fix**, or edits land in the wrong tree.

**LFS.** `filter.lfs.process = git-lfs filter-process --skip`, so `.asset` / `.psd` / `.ttf`
check out as pointers and `git lfs pull` alone does not materialize them. `.cs` is not LFS, so
all code work is unaffected. This is also why the base checkout permanently shows ~8.3k modified
assets: it holds the real files while git compares them against pointer blobs. **Never commit
those.** Payload is 682 MB / 8,750 files if a Unity import needs it.

## Task 1.4 — room-name strings, verbatim

Verified identical on `develop` and on the recorder branch.

| Path | Source | Constructed name |
|---|---|---|
| Room match — create | `RoomManager.cs:156` | `GeneratePassword_Num(5) + "-" + useBanlist` |
| Room match — join | `EnterRoom.cs:90` | `RoomIDInputField.text + "-" + useBanlist` |
| Random match — create | `LobbyManager_RandomMatch.cs:366-372` | `GeneratePassword_AlpahabetNum(8) + "randomMatchRoom"` |
| Random match — scan key | `LobbyManager_RandomMatch.cs:42-46` | `"randomMatchRoom"` |

C# `bool.ToString()` yields **`"True"` / `"False"`** — capitalised. Byte-identity therefore means
exactly:

```
  standard        "12345-True"
  no_restriction  "12345-False"
  random legacy   "<8 alphanumeric>randomMatchRoom"
```

Any change that produces `"12345-true"` breaks cross-version room joining.

## Task 1.5 — rarity coverage audit

`Rarity` enum (`CEntity_Base.cs:392`): `C=0, U=1, R=2, SR=3, UR=4, SEC=5, P=6, None=7`.

Measured across all 8,328 files under `Assets/CardBaseEntity` (base checkout, which holds the
materialized assets):

| Value | Rarity | Cards |
|---|---|---|
| 0 | C | 1,998 |
| 1 | U | 2,197 |
| 2 | R | 2,005 |
| 3 | SR | 1,259 |
| 4 | UR | 202 |
| 5 | SEC | 558 |
| 6 | P | 108 |
| 7 | **None** | **0** |
| — | field absent | **1** — `CardEntity_JSONLoader.asset`, a loader, not a card |

**Result: rarity coverage is complete.** All 8,327 real card assets carry a rarity and none is
`None`. The design's fail-closed treatment of unrecognised rarity (D12) is therefore correct but
currently inert — it guards future data, not present data, and EDEN is not at risk of being
unplayable for want of rarity information.

Derived expectations for EDEN, usable as implementation assertions:

- Legal by rarity alone (C or U): **4,195** cards.
- Rare-or-higher, hence gated behind the Anomaly Protocol: **4,132** cards.
- Promo (`P`) pool feeding three of the four anomaly categories: **108** cards.

`UR` (202) and `SEC` (558) exist in DCGO but not in this repo's `data/deck_formats.json` rarity
vocabulary — which is exactly why D12 defines rare-or-higher as *"not C and not U"* rather than as
an enumerated list. An enumerated list would have silently admitted 760 cards into EDEN.

## Task 1.2 — Unity baseline compile

Unity 2021.3.45f2 batchmode (`-batchmode -quit -nographics`) on unmodified `develop`:

| | |
|---|---|
| Exit code | **0** |
| `Assembly-CSharp.dll` | produced, 20,411,904 bytes |
| `error CS` count | **0** |
| Licensing | connected; a `Code 10` signature-verification warning is emitted but is non-fatal — compilation succeeded |

Card `.asset` files are LFS pointers here (see above), so asset-import errors appear in the log.
They do **not** affect script compilation, which is why the check greps `error CS` specifically.

**Repeatable check for every later task:**

```
"/c/Program Files/Unity/Hub/Editor/2021.3.45f2/Editor/Unity.exe" \
  -batchmode -quit -nographics \
  -projectPath D:/dcgo-worktrees/format-selection \
  -logFile D:/dcgo-worktrees/unity-check.log
grep -c "error CS" D:/dcgo-worktrees/unity-check.log   # baseline: 0
```

Any non-zero count is attributable to this change, not to `develop`.

## Task 2.8 — behaviour-identity check, and one deliberate deviation

Verified unchanged for the two legacy formats:

- `standard` resolves to the official banlist with `maxCopies: 4`, so `MaxCount_BanList` and
  `CanAddCard` compute exactly what `useBanlist == true` computed.
- `no_restriction` resolves to no banlist, matching the old early-return on `useBanlist == false`.
- Room naming, matchmaking, and `ModifiedDeckData` were not touched in this group.
- Unity compile: 0 `error CS`, same as baseline.

**Deviation — the format preference now actually loads.** Upstream calls `SaveUseBanlist()` from
the options toggle but the matching `LoadUseBanlist()` call is **commented out**
(`ContinuousController.cs:563`), so the setting was written and never read: every launch started
at the field default `true`. Persisting the format (required by the `dcgo-format-axis` spec, and
necessary for a format-aware builder that does not forget your choice each launch) means
`LoadDeckFormat()` is now called where that commented line was.

Consequence: a player who last used No Banlist now reopens in No Banlist rather than Standard.
This is a **local preference only** — it does not touch room names, the matchmaking key, or any
wire format, so none of the compatibility guarantees are affected. Flagged here rather than
silently absorbed, and worth calling out in the PR description as a fix to an evidently intended
behaviour.
