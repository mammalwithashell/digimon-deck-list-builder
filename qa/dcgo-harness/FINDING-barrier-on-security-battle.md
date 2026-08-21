# FINDING: our engine offers `<Barrier>` on a security-check deletion; DCGO does not

**Status:** strong hypothesis, rules-grounded. NOT yet confirmed against the PDF's own §16-24 text, and NOT yet isolated to a minimal repro. Do not act on it as settled.

**Found by:** DCGO parity replay of a real player decklist (Virus Busters, EX12), first game of the new-schema corpus (seed 700000).

## The divergence

```
step 11  MOVE_FROM_BREEDING   -> EX12-024 Garurumon enters the battle area (srcs=2)
step 12  id 114 = ATTACK      -> P1 board empty, so an attack on the player -> SECURITY CHECK
step 13  recorded: play_hand_0 (Giant Meat)
         engine legal set:  {59, 62}  ONLY
```

`59` is **`REPLACEMENT_ACCEPT`** (`action/space.rs:50`) — phase-disambiguated, legal only in `GamePhase::EffectChoice` with a `SelectionKind::Replacement` prompt installed. `62` is `PASS` (decline).

So our engine has a **replacement prompt pending** that DCGO never raised. This is a state divergence, not a disagreement about one action's legality.

**Memory agreed (3 on both sides), which rules out memory drift as the cause.**

## Why this deck reaches it

Five VB cards carry replacement keywords: `EX12-013`, `EX12-037`, `EX12-040`, `EX12-042` (`<Barrier>`) and `EX12-035` (`<Evade>`). The attacking `EX12-024` had 2 digivolution sources, so inherited `<Barrier>` is plausibly live.

## The suspected cause is my own change from the same session

`a1f7f59e9` — *"Security battles are battle deletions, so `<Barrier>`/`<Evade>` can fire."*

`Game::infer_deletion_cause` tested `security_resolution.is_some()` before `pending_attack.is_some()`. During a security battle BOTH are set, so an attacker losing the DP compare had its deletion attributed to the CHECK rather than the BATTLE. `<Barrier>`/`<Evade>` gate on `ReplacementCause::Battle` and were filtered out. The commit forces `Battle`.

So our engine now offers Barrier exactly where DCGO does not.

## Who is right — the rules favour US, not DCGO

From the verified derivations (`docs/digimon-rules/keyword-semantics.md`, cited to §16):

- **`<Jamming>` (16-8):** "Not deleted as a result of **a battle with a Security Digimon**." The wording presupposes that a security battle *is* a battle. A keyword would not need to exempt itself from something that was not a battle.
- **`<Iceclad>` (16-34):** "Battle (**not vs Security Digimon**)." Proof that the manual states the security carve-out **explicitly** when it intends one.
- **`<Barrier>` (16-24):** "Immediate; this would be deleted **in battle**." **No carve-out.**

Expressio unius: Iceclad excludes security battles by name; Barrier does not. Therefore Barrier applies to a security battle, and our engine is correct.

Source priority also supports this: `general_rule.pdf` (#1) outranks DCGO (#2) for rules questions. DCGO is battle-tested, but it is not the authority on rules.

## What would settle it

1. Read §16-24 directly in `general_rule.pdf` (the derivations are verified, but this claim contradicts DCGO and deserves the primary source).
2. Find DCGO's `<Barrier>` implementation and confirm it gates on a non-battle security path — i.e. that this is a deliberate DCGO behaviour rather than a coincidence of this board state.
3. Minimise to a repro: one attacker with inherited `<Barrier>`, one lethal security Digimon.
4. Check the official Bandai Q&A for a ruling on Barrier vs Security Digimon.

## Why this matters beyond one card

If confirmed, **the oracle is wrong and our engine is right.** That is worth knowing structurally: the whole harness is built on treating DCGO as the reference, and this is the first concrete case where the reference itself must be overridden by the rules manual. The triage flow currently has no way to record "divergence confirmed, DCGO is the wrong one" — every divergence implicitly accuses our engine.
