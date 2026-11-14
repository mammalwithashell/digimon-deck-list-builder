# Sourcery review follow-up

Review evaluated against PR #766 at `34edea34c035f9d7936b74bbcc6780378abf1740`.
The admission policy and source decklists remain unchanged.

| Comment | Assessment and action |
| --- | --- |
| [Old compiled packs](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/766#discussion_r4570969315) | Valid missing version bump. Pack format is now 0.33.0. The existing loader policy explicitly rejects incompatible formats and requires rebuilding external packs; it does not migrate bincode layouts. Tests reject 0.32.0 before decoding its payload and round-trip the new predicate, ForceAttack scope, and color-count IR fields. |
| [Inventory binary](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/766#discussion_r4570969323) | Valid. Reproduced four E0027 errors. Both predicate walkers and both ForceAttack walkers now explicitly traverse the added fields. A real-card CLI test verifies authored and compiled graph paths. The existing DSL workflow now runs inventory and pack-version tests. |
| [Top ACE Overflow](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/766#discussion_r4570969332) | Not applicable: `Permanent::top_card()` is `card_sources.last()`, so the top card is already included. Adding it again would charge twice. The existing Overflow witness now tests the ACE both on top and underneath, asserting exactly one charge. |
| [More than 24 sources](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/766#discussion_r4570969336) | Not applicable to the current selector: material choices use `OwnedCardChoice::material` and 28-row physical-card pages; action IDs do not encode raw source indices. A production BT13-112 test navigates to physical source 28 in a 29-source stack, completes all 13 distinct names on both the original and clone, and checks every chosen action's mask. Duplicate names keep the fixture within the separate 14-permanent battle-area limit. No action-space change or source cap was introduced. |
| [Clone continuation](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/766#discussion_r4570969344) | The previous `runner.game = runner.game.clone()` did resume a clone, so a broken cloned continuation was not untested. Nevertheless, the test now explicitly completes both original and clone independently and checks isolation and final memory/pending state. |
| [Magnamon recovery identity](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/766#discussion_r4570969348) | Valid coverage improvement. A non-Magnamon decoy is present before MAGNA in trash. The test requires exactly one legal recovery choice, MAGNA in hand, the decoy still in trash, and BT9-098 returned, including the no-match branch. |
| [Exact evolution source](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/766#discussion_r4570969355) | Valid coverage improvement. A second evolution card has a compatible route for the host rejected by the effect-source card. The test proves the competitor's route is legal while the exact-source predicate remains false, alongside the existing legal-host and CannotDigivolve cases. |

## Local validation

- Inventory compilation reproduced the four missing-field errors before the fix.
- Inventory CLI: 5 tests passed, including both new-field reference graphs.
- Pack-version gate: 6 tests passed, including pre-payload rejection and new-IR round-trip.
- Royal Knights DSL witnesses: 9 passed, including top/under Overflow, independent clone continuation, and competing evolution routes.
- Magnamon security recovery: 1 passed with positive, decoy, and no-match cases.
- BT13-112 production regressions: 12 passed, including the 29-source page/clone witness.
- Tier zero: all 13 checks passed, including 183 Python tests; admission and coverage artifacts remain current.

The pack and inventory tests are now part of the DSL CI workflow. No gate
exceptions, skipped regressions, or changes to forced-attack semantics were used
for this follow-up.
