# EX13 "Sukamon / beast" slice — Model

Slice of EX13 authored by the `author-set` workflow: EX13-027 Chuumon,
EX13-028 Sukamon (yellow Abnormal line) and EX13-038 Salamon, EX13-040 Mikemon
(green Beast line). No DCGO C# exists for any of the four cards (no
`EX13_027/028/038/040.cs` in the base-repo submodule); printed text from the
official Bandai DB bundles (`data/card_bundles/EX13-0xx.md`) plus
`general_rule.pdf` govern.

Interaction tests: `code/digimon-engine/tests/archetypes/sukamon_beast_ex13.rs`.

## Card pool & roles

| Card | Role | One-line function |
|---|---|---|
| EX13-027 Chuumon (Y Lv.3, cost 3) | enabler / engine | [On Play][When Moving] reveal 3: 1 [Sukamon]/[Etemon]-named card to hand, 1 to trash, rest to bottom. Inherited: [All Turns][OPT] leave-prevention paid by deleting another [Sukamon]-named Digimon. |
| EX13-028 Sukamon (Y Lv.4, cost 3, digivolve Y/B Lv.3 for 2) | engine / wall | <Blocker>. [On Deletion] reveal 3: may play a cost<=3 [Chuumon]/[Sukamon]-named Digimon free, trash the rest. Same inherited leave-prevention as Chuumon. |
| EX13-038 Salamon (G Lv.3, cost 3) | enabler | [On Play] reveal 3: 1 [Leopardmon]-text card + 1 [Beast]/[Animal]/[Sovereign] (not [Sea Animal]) Digimon to hand. (Rule) Beast. Inherited: your suspended Digimon +1000 DP. |
| EX13-040 Mikemon (G Lv.4, cost 4, digivolve G Lv.3 for 2) | tempo / control | [On Play][When Digivolving] 1 opp Digimon/Tamer can't unsuspend until their turn ends. [All Turns] when this suspends, suspend 1 opp Digimon/Tamer. Inherited: suspended Digimon +1000. |

Out-of-slice support referenced: EX13-043 Leopardmon (the Salamon search
target; IMPLEMENTED, only used as a reveal hit). EX13-031 KingSukamon is
**BLOCKED** in `validated_cards_dsl.json` — any combo naming it is skipped.

## Digivolution lines

- Yellow: (Lv.2 Y/B, cost 0) → **Chuumon** → **Sukamon** (Y Lv.3, cost 2) → [KingSukamon — BLOCKED].
- Green: (Lv.2 G, cost 0) → **Salamon** → **Mikemon** (G Lv.3, cost 2) → green Lv.5 → Leopardmon.

## Named combos

### C1 — Blocker wall: Sukamon-on-Chuumon blocks, sacrifices a 2nd Sukamon, which refills the board
- Cards: EX13-028 ×2, EX13-027 ×2 (one as digivolution source, one in deck).
- Expected mechanical outcome: opponent's 5000 DP Digimon attacks; Sukamon
  (<Blocker>, 2000 DP, Chuumon underneath) blocks and loses the battle.
  Chuumon's inherited replacement is offered (battle is "other than by your
  effects"); paying it deletes the OTHER Sukamon, so the blocker stays. The
  sacrificed Sukamon's [On Deletion] reveals Chuumon and plays it free;
  Chuumon's [On Play] reveal puts one Sukamon in hand and one in trash.
  Net: field count unchanged (lost Sukamon#2, gained Chuumon), +1 hand, own
  security unchanged, 0 memory paid.
- Rules basis: printed text EX13-027/-028 bundles; `general_rule.pdf` 16-4
  (<Blocker>: suspend to redirect the attack); a battle loss is "other than by
  your effects", so the inherited "would leave" replacement applies; the
  Sukamon deleted to pay that cost is still deleted, so its [On Deletion]
  triggers (pending until the current processing ends).
- Rank: 1 (the slice's core loop — resilient wall + card advantage).

### C2 — Lone Sukamon falls: [On Deletion] replays Chuumon, which re-tutors Sukamon (unhappy path of C1)
- Cards: EX13-028, EX13-027 ×2.
- Expected outcome: with no other [Sukamon]-named Digimon the replacement is
  NOT offered; the blocker (and its Chuumon source) is deleted, then
  [On Deletion] plays Chuumon free and Chuumon re-tutors a Sukamon.
- Rank: 2.

### C3 — Chuumon tutors its own digivolution
- Cards: EX13-027, EX13-028 ×2.
- Expected outcome: play Chuumon (3), reveal adds Sukamon, trashes the other
  Sukamon; digivolve Sukamon onto Chuumon (Y Lv.3, 2) and draw 1. The stack
  now carries Chuumon's inherited prevention: an opponent-effect deletion is
  offered the "delete another Sukamon" replacement.
- Rank: 3.

### C4 — Salamon tutors Mikemon + Leopardmon, Mikemon digivolves and locks
- Cards: EX13-038, EX13-040, EX13-043 (reveal hit only).
- Expected outcome: Salamon [On Play] adds Leopardmon (text bucket) and Mikemon
  (Beast bucket); digivolving Mikemon onto Salamon (2) draws 1 and its
  [When Digivolving] gives the chosen opponent Digimon CannotUnsuspend.
  Memory spent 3 + 2.
- Rank: 4.

### C5 — Mikemon attack: self-suspend → suspend the locked target; Salamon's inherited +1000
- Cards: EX13-040 on EX13-038.
- Expected outcome: [When Digivolving] locks Birdramon; Mikemon attacks, its
  suspend triggers "suspend 1 opp Digimon/Tamer" → Birdramon. While suspended
  Mikemon is 5000 + 1000 (Salamon's inherited) = 6000. In the opponent's
  unsuspend phase Birdramon stays suspended; an untouched Digimon does not.
- Rules basis: `general_rule.pdf` 6-2-1 (unsuspend phase unsuspends all of
  the turn player's Digimon — CannotUnsuspend overrides); "until their turn
  ends" expiry follows the EX12-063 idiom.
- Rank: 5.

## Playstyle
- Yellow side: grindy midrange/value — recursive bodies, Blocker walls,
  card advantage through reveal tutors. Green side: tempo/control via
  suspend-lock, with suspended-DP buffs rewarding attacking.

## Win conditions
- Out-grind removal with Sukamon/Chuumon recursion into KingSukamon/Etemon
  tops (blocked here); green side swings with locked-out blockers.

## Ranked interactions to test
1. C1 Blocker wall chain — longest cross-card chain (replacement → cost
   deletion → On Deletion → free play → On Play reveal).
2. C2 lone Sukamon fallback — unhappy path proving the replacement gate.
3. C3 Chuumon → Sukamon line — tutor + digivolve + inherited transfer.
4. C4 Salamon → Mikemon line — dual-bucket tutor + WD lock.
5. C5 Mikemon attack lock — on-suspend + lock persistence + inherited aura.

Not selected / blocked (logged):
- Sukamon → KingSukamon (EX13-031) line — BLOCKED card.
- Chuumon [When Moving] from breeding — single-card, covered per-card.
- Etemon-named hits for Chuumon (EX13-035 KingEtemon) — outside slice; the
  name filter is covered per-card.
- Salamon's Beast bucket finding Chuumon (cross-line) — low play value.
