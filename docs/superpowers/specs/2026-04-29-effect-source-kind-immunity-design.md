# Effect Source Kind Immunity Design

**Goal:** Add a stable source-kind contract so immunity and targeting rules can distinguish Digimon, Tamer, Option, Security, and rule-generated effects without guessing from printed card kind.

## Problem

Some effects need to know what kind of effect is affecting a Digimon, such as "unaffected by your opponent's Digimon effects" for a turn or until the end of a player's turn. The engine currently carries `source_card` and sometimes `source_permanent`, but that is not enough for DUAL cards or inherited effects:

- A DUAL card can produce an Option effect when used from hand or trash.
- The same DUAL card can later be a Digimon on the field after Arts Digivolve.
- Inherited effects are printed on source cards, but rules treat them as effects of the top Digimon.

The source-kind answer must therefore be contextual, not derived only from `CardKind`.

## Source Kind Model

Add an explicit effect source classifier:

```rust
pub enum EffectSourceKind {
    Digimon,
    Tamer,
    Option,
    Rule,
}
```

`EffectSourceKind` represents the game object currently producing the effect.
Security is an activation origin/timing, not a source kind by itself. If the
engine needs to distinguish "this effect activated from security," carry that as
separate origin metadata.

## Classification Rules

| Effect origin | Source kind |
|---|---|
| Top card of a Digimon stack | `Digimon` |
| Inherited effect from a source under a Digimon | `Digimon` |
| DUAL card used as an Option from hand/trash | `Option` |
| DUAL card after Arts Digivolve, now on a stack | `Digimon` |
| Tamer field effect | `Tamer` |
| Standard Option resolving from hand/trash | `Option` |
| Digimon card revealed in security with a security effect, e.g. AD01 LordKnightmon | `Digimon` |
| Option card resolving its security effect | `Option` |
| Tamer card resolving a security effect, if supported by card text | `Tamer` |
| Engine/rules bookkeeping with no card source | `Rule` |

Security effects should still retain enough origin/timing information for cards
that care about "security effects" specifically, but immunity to Digimon,
Option, or Tamer effects must use the source kind above. For example, a Digimon
revealed from security that resolves a security effect is still a Digimon effect.

## Runtime Contract

`QueuedEffect` should carry `source_kind: EffectSourceKind`, and `EffectContext`/`EffectReadContext` should expose it to effect code and replacement/immunity checks.

Existing helpers like `source_is_tamer()` should be migrated to this field instead of re-looking up `source_card` and `source_permanent`.

## Immunity Check

An immunity such as "this Digimon is unaffected by your opponent's Digimon effects" should block an effect only when all of these are true:

1. The effect is attempting to target or affect the protected Digimon.
2. The effect controller is the protected Digimon's opponent.
3. `source_kind == EffectSourceKind::Digimon`.
4. The immunity's expiry window is active.

The immunity should not use `CardKind::Dual` directly. DUAL cards are deliberately ambiguous at the data level; the queued effect context determines whether the current effect is an Option effect or a Digimon effect.

## Testing Requirements

Add tests that cover:

- Top-card Digimon effects are blocked by Digimon-effect immunity.
- Inherited effects are also blocked as Digimon effects.
- Tamer effects are not blocked by Digimon-effect immunity.
- Option effects are not blocked by Digimon-effect immunity.
- DUAL used as an Option is not blocked by Digimon-effect immunity.
- The same DUAL card after Arts Digivolve is blocked as a Digimon effect.
- A Digimon revealed in security with a security effect is blocked as a Digimon effect.
- Opponent-only immunity does not block the protected player's own effects.
- Turn-scoped and player-turn-scoped expiry behave independently from source classification.
