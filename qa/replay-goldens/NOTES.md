# Replay-golden change notes

Why a golden's recorded actions changed (a digest-only re-bless keeps the
actions and needs no note here). `replay_corpus` reads only `*.replay.json`.

## refresh_v2_generated_greedy_002_..._seed_40521420 -- regenerated 2026-10-20

- **Command:** `replay_corpus regenerate --goldens qa/replay-goldens --cards <LF copy of data/cards.json> --id greedy_002_`
  on the main-merge branch (`p4-main-merge`, after merge `e724c083c`). Same
  inline decks, seed 40521420 and 64-action budget; written as engine schema 2
  (the other nine are archived schema 1).
- **Why:** main's Security-check observer capture (433bd379c; Comprehensive
  Rules §13-1-8-2, §15-16-10-2, official Q&A Q6813). The check and removal
  observers trigger at the reveal and resolve after the immediate `[Security]`
  effect but BEFORE the security battle (previously after it, at disposal).
- **Where it bites:** step 27. Seat 0's attack reveals P1's BT24-008. Seat 0's
  breeding-area BT21-001 inherited effect ("[Your Turn] [Once Per Turn] When
  your opponent's security stack is removed from, 1 of your Digimon may
  digivolve into a Digimon card with the [Reptile] or [Dragonkin] trait in the
  hand with the digivolution cost reduced by 1") now resolves before the
  battle, while the attacker is still on the field, so its first prompt is the
  Digimon to digivolve (legal [62, 102]); the old recording picked a hand card
  (action 0) after the attacker had been deleted by the battle. The recorded
  step-27 action became illegal, so `bless` could only retire the golden.
- **Result:** actions 0-26 are unchanged; from step 27 the greedy policy plays
  the new game. Verified 10/10 with `replay_corpus verify`.

## Digest-only re-bless 2026-10-20 (no action changed)

Goldens 001, 003, 005 and 008 (`bb13f5b81`): the same Q6813 capture moves a
security-removal observer to `OnSecurityCheckDrain` (before the battle); every
recorded action stays legal.

## Goldens 002, 008 regenerated, 001 migrated -- 2026-10-21 (declined BT21-001 keeps its [Once Per Turn])

Step numbers below are 0-based action indices (`verify` prints the 1-based
digest count, so its "step N" is the digest after action N-1). Diagnosed by
token-tracing `verification_digest` on 811137a22 (before Plan 2 Task 6, 10/10
green) and b45e52db2 and diffing the per-action token chunks.

- **Commands:** 002 and 008: `replay_corpus regenerate --goldens qa/replay-goldens --cards <LF copy of data/cards.json> --id greedy_002_`
  (and `--id greedy_008_`) on `p2-triage` at b45e52db2. Same inline decks,
  seeds and 64-action budgets; 008 is now engine schema 2 like 002.
  001: hand-migrated like b750db5d9, not regenerated -- its path is curated
  (b750db5d9 inserted the step-44 accept) and `tests/replay_empty_replacement_window.rs`
  pins it, while a greedy regeneration leaves it at action 44 (the current
  greedy declines that BT24-082 trigger; it does so on 811137a22 too). A
  decline (62) is inserted at each re-offer, then `replay_corpus bless`
  re-records the digests; schema 1 kept, 65 -> 67 actions.
- **Why:** 9888747b0 (Plan 2 Task 6): a declined folded "you may" no longer
  spends the [Once Per Turn] (CR 4.3 §15-14-1-4, official Q&A Q1818: "If you
  choose not to, the effect's [Once Per Turn] requirement won't be used up").
  BT21-001 Gigimon's inherited "1 of your Digimon may digivolve ..." became
  `optional: true` with the hand pick gated on the Digimon pick (DCGO
  `BT21_001.cs` `isOptional: true`, `canNoSelect: true`), so passing the
  Digimon pick refunds the use and the effect is offered again on the next
  security removal that turn. The first digest change is representation only
  (the re-lowered clause's use key and resume-tail index, same prompt); the
  first state change is the refunded count after the decline.
- **Where it bites:** in all three, seat 0 has BT23-005 on BT21-001 in the
  breeding area and greedy declines the BT21-001 pick.
  - 001: decline at action 60 (turn 10); the action-61 attack's security
    removal re-offers it, so recorded action 62 (144, the next attack) is
    illegal (legal [62, 100, 102, 103, 104, 107]); the following attack
    re-offers it again. Declines inserted at actions 62 and 64; actions 0-61
    and digests 0-58 unchanged.
  - 002: decline at action 27 (turn 7); in turn 9 the action-42 removal
    re-offers it, so recorded action 43 (0) is illegal (legal [62, 102]). The
    pre-Task-6 engine did not offer it in turn 9 only because the breeding
    permanent's use count is never reset (G-ENGINE-BREEDING-AREA-INHERITED-TRIGGERS-ACTIVE).
  - 008: decline at action 28 (turn 5); the second attack (actions 29-30)
    re-offers it, so recorded action 31 (0) is illegal (legal [62, 100]).
- **Result:** 002 and 008 keep actions 0-42 / 0-30 and then play the greedy
  game; a regeneration of the same seeds on 811137a22 first differs from them
  exactly there (greedy declines the re-offer). 001 keeps its recorded path
  plus the two declines. Verified 10/10 with `replay_corpus verify`;
  `replay_empty_replacement_window` passes (it was red at b45e52db2: "recorded
  digest at 60") with its length pin moved to 67.
- **Known wrong in these games (pre-existing, not re-blessed away):** the
  effect should never trigger from the breeding area (CR 4.3 §3-4-7-4; DCGO
  requires `IsExistOnBattleAreaDigimon`). Filed as
  G-ENGINE-BREEDING-AREA-INHERITED-TRIGGERS-ACTIVE in `docs/RUST_ENGINE_GAPS.md`;
  fixing it will move these three goldens again.

## Digest-only re-bless 2026-10-21 (no action changed)

Golden 009: the same 9888747b0 change marks BT21-001's queued trigger optional
(digest from action 47, the trigger-order prompt) and refunds the declined use
at action 53; no later security removal happens that turn, so every recorded
action stays legal. Here BT21-001 is under a battle-area Digimon (legitimate).
