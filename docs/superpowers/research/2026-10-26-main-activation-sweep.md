# Frozen training-pool Main activation sweep

Scope: the running mixed-continuation campaign's exact training and evaluation
snapshots, not every list in the deck library. Training has 26 weighted slots
representing 21 distinct lists; evaluation has 13 lists. Their union contains
280 unique card IDs. Snapshot hashes and the complete clause inventory are in
`2026-10-26-main-activation-sweep.json`.

The scan includes top-level effects and nested dual-card Option faces. It found
62 Main clauses on 62 cards: seven field Main effects, two ordinary Digimon
hand Main effects, and 53 Option/dual-Option Main effects. All 280 cards had an
authored YAML source, including the embedded `_examples` pack entries. No Main
clause contains an explicit `refund_opt` after the prior Gekomon correction.

## Repeatable field declarations

| Card | Limiter | Cancellation evidence |
| --- | --- | --- |
| EX12-043 Hakubamon (SW) | Once per turn | New original/clone decline regression passes |
| EX12-013 BetelGammamon (Virus Busters) | Once per turn | New original/clone decline regression passes |
| BT26-012 Manekimon (Toho Braves) | Once per turn | Prior fix plus new original/clone regression passes |
| BT26-021 Gekomon (Chronomon) | Once per turn | Prior fix plus new original/clone regression passes |
| BT26-070 NightChiropmon (Glowing Dawn) | Once per turn and two face-down cards | New original/clone regression passes with enough material remaining to pay again |
| BT25-092 Asuna Shiroki (Three Musketeers) | Suspend self and trash an Option | Existing target/result decline tests preserve the paid cost; all 12 card tests pass |
| BT26-096 Kosuke Misono (Chronomon) | Return self to deck | Existing decline test preserves the paid self-removal; all six card tests pass |

The five new OPT regressions use the real action mask and decoder, then clone
the installed selection before declining it. Both original and clone return
to Main with the spent action unavailable and the second copy still available.
The four free-choice effects preserve hand/trash counts and memory. The
NightChiropmon case starts with four face-down cards, pays two, and retains two:
its unavailable action therefore proves OPT consumption rather than merely
insufficient remaining material.

Printed images were checked for Hakubamon, BetelGammamon, NightChiropmon, Asuna
and Kosuke. CR 4.3 15-14-1-1 through 15-14-1-5 distinguish activation counting,
independent copies and reset timing. The local DCGO Hakubamon and BetelGammamon
implementations refund cancelled processing; the simulator correctly follows
the rules' activation-counting behavior here. No additional refund was removed
from those cards. Triggered effects declined before activation retain their
separate semantics.

## Other Main paths and limits

Rhihimon BT18-081 and Lamiamon BT24-016 have hand Main effects. Their required
material gates and existing successful execution regressions were checked;
all nine Rhihimon and 27 Lamiamon tests passed. The prior Lamiamon fix still
requires its Dimetromon trash material before offering the declaration.

Option effects use the Option lifecycle rather than a reusable, free field
declaration. The scan covers all nested Option faces as well, including the
Arts cards. ST24-07's nine tests passed, including declining optional processing
after paying for the Option. This is a focused cancellation/refund sweep, not
a new full-fidelity certification of all 62 effects or proof that every possible
multi-card gameplay loop is absent.

Validation: all 87 selection tests passed, including the five new cloned-state
cases; 63 focused existing card tests passed; three audit-tool tests and all
13 tier-0 repository checks passed.
The audit regression pins the frozen pool's card-ID set and rejects newly
introduced Main refunds or missing source coverage. It separately tests nested
dual faces and distinguishes Main refunds from unrelated triggered refunds.

Reproduce the inventory with `python -m tools.cloud_search.audit_main_activations`
and `--snapshot` for each frozen training/evaluation snapshot, plus `--out`.
The current training process and native image were left running unchanged;
this sweep added evidence and regression coverage, with no new gameplay fix
or image deployment required.
