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
