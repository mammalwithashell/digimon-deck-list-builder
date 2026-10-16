# EX13 DCGO catalog repair package

This is an unapplied repair, not proof of a working Unity player. It supplies
nine missing CEntity_Base records and rebuilds the serialized catalog scene.
It does not alter card effect implementations or mark oracle exams confirmed.

## Evidence and review

`evidence/manifest.json` binds each saved official Bandai response, primary PNG,
and parsed JSON by SHA-256. Acquisition used the current `/cards/` endpoint,
not the retired `/cardlist/` link. All nine images were visually reviewed on
2026-10-10. No EX13 corrections apply in the captured reviewed correction ledger;
its complete-file digest is retained. The package deliberately has its own
snapshot schema and does not pretend these are published canonical records.
`acquire.py` uses the base checkout's official source parser read-only, never
writes the canonical pipeline, and fails if a future EX13 correction needs
adjudication. Existing compatibility JSON/bundles were not used as authority.

The pool is EX13-005, -008, -018, -021, -024, -039, -041, -044, -045.
Special evolution, DNA, and Assembly descriptions remain intact; behavioral
implementation remains in each existing DCGO.CardEffects.EX13 class.
Digi-Egg absent play cost maps to DCGO's -1 sentinel. Standard evolution circles
expand each listed color into its own EvoCost. Enum values match CEntity_Base.cs.

## Local verification (no Unity or DCGO mutation)

Run from the worktree root:

```powershell
python -m pytest code/tests/tools/test_dcgo_catalog_repair.py -q --confcutdir=code/tests/tools
python code/tools/dcgo_catalog_repair/generate.py
```

The second command prints a dry-run payload. `--write NEW_FILE` explicitly
creates a new file and refuses overwrite. `payload.json` is the reviewed output.
Tests first failed because the generator did not exist; all 12 now pass. Tests
cover mapping, multiple evolution colors, egg cost, incomplete evidence,
unsupported layouts, index preservation/collisions, image/JSON hash tampering,
real captured metadata and default no-write behavior. Unity compilation and
runtime acceptance remain required; Python tests cannot establish either.

## Application and separate build

Prefer a dedicated working copy of the base DCGO project (including its current
harness modifications), not the shared source/player. Review the copy's git
status and source identity before proceeding. Do not initialize another large
submodule merely to inspect it. Creating a separate project/build and launching
Unity are operator actions, intentionally not performed by this package.

1. Copy only `EX13CatalogRepair.cs` into the approved project's `Assets/Editor/`.
   Keep `payload.json` outside Assets and supply its absolute path.
2. Run Unity 2021.3.45f2 against that project in batch mode for validation:

```powershell
& 'C:/Program Files/Unity/Hub/Editor/2021.3.45f2/Editor/Unity.exe' -quit -batchmode -nographics -projectPath '<APPROVED_DCGO_PROJECT>' -executeMethod DCGO.CatalogRepair.EX13CatalogRepair.Execute -repairPayload '<PACKAGE>/payload.json' -logFile '<NEW_LOG_PATH>'
```

3. After reviewing the dry-run result, repeat with `-repairApply` and
   `-repairOutput '<ABSOLUTE_NEW_DIRECTORY_OUTSIDE_PROJECT>'`. The output
   directory must not already exist. It creates a new `DCGO.exe` and
   `DCGO_Data/`, never replaces an existing player and never launches it.

The importer rejects missing effect classes, occupied target paths, existing
primary target assets, duplicate/negative existing indices, insufficient deck
code capacity, a wrong card pool, missing scene/controller, or build failures.
It preserves every existing asset/index and allocates above the maximum.
A read-only scan on 2026-10-10 found max index 10465 (8461 records, no duplicates),
so the expected next nine are 10466..10474; the importer recomputes them live.
A second apply intentionally refuses existing targets instead of overwriting.

The new player uses company `DCGO-CatalogRepair` and a fresh `EX13-<GUID>` product
name. Its expected Windows LocalLow path is recorded in `repair-receipt.json`.
The previous company/product/backend settings are restored in `finally`.
A forced editor/process termination can bypass finally; use the settings backup
and verify the source diff before another build. A new product name separates
Unity persistentDataPath from the shared player's jobs directory. Do not point
an external launcher, environment override, or harness queue argument back at
shared jobs. Verify the actual player log path before submitting fresh jobs.

## Exact project writes and rollback

If an operator elects to use the shared base project, these are the required
base writes; none have been executed by the authoring agent:

- Add `Assets/Editor/EX13CatalogRepair.cs` and Unity-generated `.meta`.
- Add `Assets/CardBaseEntity/EX13/EX13_005.asset` (likewise 008, 018, 021, 024,
  039, 041, 044, 045), their `.meta` files, and EX13 folder metadata if new.
- Rewrite `Assets/Scenes/ContinuousControllerScene.unity` CardList and
  SortedCardList using AttachCardData.cs's established exclusions and sort rules.
- Temporarily change ProjectSettings company, product and scripting backend,
  then restore them; Unity also creates its normal Library/Temp/log caches.

The importer copies the original scene and ProjectSettings file into the NEW
output directory before catalog mutation, and writes `repair-receipt.json`
before creating assets. On failure it exits nonzero and leaves reviewable
partial changes with these backups; it does not delete existing assets.

Rollback with Unity closed: restore the scene and ProjectSettings from that
specific build's `.before` files. Compare against the pre-apply git diff first
so concurrent work is preserved. Remove only newly created asset paths listed
in that receipt and their `.meta` files after verifying their absolute paths
remain inside the approved project. Remove the copied importer and its `.meta`
if they were new. Keep the receipt, logs, and failed build for inspection. Never
use a broad reset/clean or delete any original CardBaseEntity asset. With a
dedicated project copy, simply retain/quarantine the whole copy instead.

## Acceptance before using the replacement oracle

Require Unity compilation/build success, then inspect the saved scene for all
nine exact CardIDs/classes/indices and preservation of prior catalog indices.
Launch only the new executable, verify its isolated persistentDataPath from
logs, then run the nine existing minimal catalog probes with a fresh harness
root. Each card must instantiate by exact ID without fallback substitution.
Rerun the previously blocked clause exams only after those checks; catalog
availability is not a behavioral verdict. Do not update the shared build
pointer or shared queues as part of this package.

## Scratch-copy feasibility

A read-only size scan on 2026-10-10 found approximately **0.94 GiB** for the
source tree excluding Library, Temp, Logs, obj and .git (33,222 files). Assets
accounts for approximately 1.01 GB decimal. Copy Assets (including every .meta
and current harness source), Packages, and ProjectSettings into a NEW ordinary
scratch directory. This is enough for a Unity project; UserSettings is optional.
Do not copy the base `.git` pointer, use junctions/hardlinks, or share Library:
all would weaken isolation or risk writes reaching the original project.
Library, Temp, Logs and obj are regenerable and can be omitted. Preserve the
package lock and source project version; imported/generated Library and build
size require additional disk space and are not covered by this estimate.

No absolute local `file:` dependencies were found in the current package
manifest/lock. A fresh Library may require package-network access (Unity
registry, the pinned WebP Git package, and the NuGet scoped registry). Unity
license availability was not tested. The installed editor version alone does
not establish successful compilation, licensing, or a usable player.
