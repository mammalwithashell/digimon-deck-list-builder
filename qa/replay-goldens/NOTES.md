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

## Goldens 002, 008 regenerated, 001 migrated -- 2026-10-21 (breeding-area BT21-001 no longer triggers)

Step numbers are 0-based action indices, as in the entry above.

- **Commands:** 002 and 008: `replay_corpus regenerate --goldens qa/replay-goldens --cards <LF copy of data/cards.json> --id greedy_002_`
  (and `--id greedy_008_`) on `p2-triage` at 27009fb8a. Same inline decks,
  seeds and 64-action budgets; engine schema 2 kept.
  001: hand-migrated (its path is curated and pinned by
  `tests/replay_empty_replacement_window.rs`, see the entry above): the three
  declines (62) of the breeding-area BT21-001 pick at actions 60, 62 and 64
  are removed, then `replay_corpus bless` re-records the digests; schema 1
  kept, 67 -> 64 actions; the test's length pin moved to 64.
- **Why:** 27009fb8a (Plan 2 Task 9, G-ENGINE-BREEDING-AREA-INHERITED-TRIGGERS-ACTIVE):
  effects on a card in the breeding area trigger only when they explicitly
  specify or reference the breeding area (CR 4.3 §3-4-7-4 / §3-4-7-6; DCGO
  `BT21_001.cs` requires `IsExistOnBattleAreaDigimon`). Seat 0's BT21-001
  Gigimon under BT23-005 in the breeding area ("[Your Turn] [Once Per Turn]
  When your opponent's security stack is removed from, 1 of your Digimon may
  digivolve ...") has no breeding reference, so a security removal no longer
  offers it. The same commit resets the breeding permanent's [Once Per Turn]
  count at each turn change (§15-14-1-5-1), which no longer matters here.
- **Where it bites:** the first diverging digest is the attack whose security
  removal used to offer the breeding-area pick; the next recorded action was
  the greedy decline (62) of that pick, which no longer has a prompt to answer.
  - 001: digest after action 59 (attack 114); declines at 60, 62, 64 dropped.
  - 002: digest after action 26 (attack 144); the recorded decline at action
    27 (and the turn-9 re-offer's decline at action 43) dropped.
  - 008: digest after action 27; the declines at actions 28 and 31 dropped.
- **Result:** each new game is exactly the old recorded game with every
  breeding-area BT21-001 decline removed (002 and 008: the first 62 actions
  match the old path minus those declines; the 64-action budget then adds two
  greedy actions; 001 keeps its curated path). Digests before the first
  removed decline are unchanged. Verified 10/10 with `replay_corpus verify`
  (with `data/cards.json` and with the LF copy);
  `replay_empty_replacement_window` passes.
