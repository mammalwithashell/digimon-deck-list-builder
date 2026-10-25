# Deleted-token choice serialization in the mixed-seed evaluation

The comparison stopped at 21:14:48 UTC on October 25 during development seed
checkpoint 0, after its complete 3,200-game meta panel and 206 completed pet
games. Pet game index 206 (seed 56522462) was Medusamon versus SW, against the
greedy anchor. The candidate hash was
`5f1b8ec03057bcf9af0f7d01556a28a7d3c1c42b37cbfeab4390fabad310fee8`.

## Reproduction and cause

The original immutable image
`sha256:05d93754f049591d7cd0d5b6d27ca59f3d1ff7d185e264de0532662c00aa0364`
reproduces the panic with the frozen candidate and balanced deck schedule.
Replay evidence is preserved at
`/opt/digimon-search/token-eval-investigation-20261025` on the experiment host.
The committed fixture contains both decks, the seed, and all 102 legal actions
through the failing state; no model weights are required for the native test.

Four Petrification Tokens have left play, with their mandatory On Deletion
effects waiting in a TriggerOrder prompt. The four physical handles no longer
resolve in any live zone. The pending choices and queued effect identities are
valid, but `serialization::pending_selection_data` called the infallible
`Game::card` accessor to obtain optional card-face metadata. This panicked from
`DigimonEnv.step` while collecting per-card telemetry. The desktop DTO builder
contained the same unsafe lookup.

This is a serialization failure, not a decision-cap loop. A removed token must
not be restored to a zone to satisfy a presentation lookup.

## Repair and validation

Both serializers use the fallible live-zone lookup. Missing card ID/name
metadata is null; labels, action IDs, prompt order and legal masks are retained.
The observation encoders already use fallible lookups. No tensor, action,
reward, search-budget, or model contract changes are needed.

The native fixture failed on the original accessor, then passed after repair.
It checks four preserved choices, unavailable/live/absent source metadata,
unchanged masks, serialization in cloned worlds, and resolution of the mandatory
effects for each possible first choice. The selection suite passed 82 tests;
the FFI/serialization suite passed 30 tests.

A separate diagnostics regression exposed that PyO3 panics inherit
`BaseException`, bypassing the evaluator's `Exception` handler. The evaluator
now records incomplete-game/error and replay evidence before re-raising native
panics, and cloud evaluation workers set a separate failure directory per panel.
No failed game is scored as a draw or admitted to training.

The original experiment remains stopped. Its models, successful reports and
failed report are preserved. A rebuilt runtime and full Python game replay must
be validated before retrying the evaluation.
