# Thomas / DATA SQUAD (BT25 slice) — Model

Slice cards (the six named for this slice): **BT25-087** Thomas H. Norstein,
**BT25-096** Mirage Beast Knight, **BT25-002** Wanyamon, **BT25-027**
MachGaogamon, **BT25-029** MirageGaogamon, **BT25-104** ShineGreymon: Burst Mode.

This is the **Thomas H. Norstein "[DATA SQUAD]"** sub-engine of BT25: a Tamer
(Thomas) that stashes face-down deck cards under itself, and a Gaogamon-line
payoff (MachGaogamon → MirageGaogamon) that *spends* those face-down stash cards
as a recurring cost to bounce the opponent's board and to refuse to leave play.
Wanyamon is the inherited draw engine that fires off DATA SQUAD-Tamer plays.

## Implementation status (Phase-4 precondition gate)

Read from `qa/qa-reports/validated_cards_dsl.json` (verdicts dated 2026-06-06):

| Card | Name | Status | Why |
|------|------|--------|-----|
| BT25-002 | Wanyamon | **IMPLEMENTED** | inherited [Your Turn][OPT] DATA SQUAD-Tamer-played → both players draw 1 |
| BT25-027 | MachGaogamon | **IMPLEMENTED** | alt-digivolve onto DS lvl4 cost3; WD/WA bounce + trash-FD-under-Tamer → unsuspend self; leave-prevention by trashing FD (main + inherited) |
| BT25-087 | Thomas H. Norstein | **BLOCKED (engine)** | clause 2 needs `OnAddToHand` trigger (inert); clause 3 hits BeforePayCost selection-bearing pay_cost Parked-drop |
| BT25-096 | Mirage Beast Knight | **BLOCKED (engine)** | use-cost −2 by trashing FD hits BeforePayCost Parked-drop (multi-Tamer choice) |
| BT25-029 | MirageGaogamon | **BLOCKED (engine)** | `OnAddToHand`-half of the unsuspend clause is inert |
| BT25-104 | ShineGreymon: Burst Mode | **BLOCKED (engine)** | cross-side Option-Main activation gap + treated-as-Digimon name-overlay aura gap |

Only **BT25-002** and **BT25-027** are IMPLEMENTED. Per the skill's Phase-4
gate, interaction tests are authored **only** for combos whose pieces are all
implemented; every combo naming a BLOCKED card is reported as blocked-on-that-card
and not authored (see "Blocked combos" below).

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| BT25-002 Wanyamon | engine (inherited) | DATA SQUAD Tamer played on your turn → both players draw 1 (OPT). |
| BT25-027 MachGaogamon | payoff / enabler | Alt-evo onto a DS lvl-4; bounces opp lvl≤4 + self-unsuspends by trashing a face-down stash; refuses to leave by trashing a face-down stash. |
| BT25-087 Thomas H. Norstein | engine (stash builder) | Sets memory to 3; stashes top-2 deck face-down under itself when opp gains cards; discounts DS digivolves by trashing a stash card. |
| BT25-096 Mirage Beast Knight | enabler (Option) | Trash-FD discount; places Gaogamon+MachGaogamon from trash as sources so a Gaomon free-digivolves into MirageGaogamon. |
| BT25-029 MirageGaogamon | payoff | Reboot/Blocker/Evade; double-bounce by trashing FD; unsuspends when cards added to opp hand or stash trashed. |
| BT25-104 ShineGreymon: Burst Mode | payoff (DUAL) | Raid/Piercing/SA+1/Blocker/Barrier; activates its Option-side Main; buffs Marcus Damon. |

## Digivolution lines

- **Gaogamon (Blue/Green DATA SQUAD):** Gaomon → Gaogamon → **MachGaogamon**
  (BT25-027, lvl5, alt-evo onto any DATA SQUAD lvl-4 for cost 3) → **MirageGaogamon**
  (BT25-029, lvl6). Mirage Beast Knight (096) can shortcut Gaomon → MirageGaogamon
  by planting Gaogamon + MachGaogamon from trash as sources.
- **Wanyamon (Blue, lvl2):** an inherited-source body that travels under the
  Gaogamon line, granting the "both players draw 1 on DATA SQUAD-Tamer play"
  trigger from the bottom of the stack.
- **ShineGreymon line (Marcus side):** off-colour DUAL payoff; not on the
  Gaogamon stash spine.

The unifying resource is the **face-down stash under a Tamer**: Thomas (087)
*builds* it; MachGaogamon (027), MirageGaogamon (029) and Mirage Beast Knight
(096) *spend* it. Because 087/029/096 are BLOCKED, the only IMPLEMENTED stash
*consumer* is MachGaogamon (027) and the only IMPLEMENTED stash-adjacent engine
is Wanyamon's draw.

## Named combos

### Mach stash-engine line (C1 / C2)
- Cards: **BT25-027** (+ synthesized DATA SQUAD lvl-4 base + a Tamer carrying a
  face-down stash source + an opponent lvl≤4 Digimon).
- Expected outcome: alt-digivolve MachGaogamon onto a DATA SQUAD lvl-4 base for
  cost 3; on attack, bounce an opp lvl≤4 Digimon to hand **and** (paying the
  face-down stash) unsuspend MachGaogamon. The unhappy mirror: with no face-down
  stash, the bounce still resolves but the self-unsuspend cost goes unpaid → it
  stays suspended.
- Rules/keyword basis: `general_rule.pdf` §16 digivolve-cost + WD/WA timing;
  DCGO `BT25_027.cs` (alt-evo `AddSelfDigivolutionRequirementStaticEffect(level 4,
  DATA SQUAD, cost 3)`; shared WD/WA Mode.Bounce then trash-FD `if (trashed)`
  unsuspend).
- Rank: 1 (the core engine of the implemented half).

### Wanyamon draw engine under a real Gaogamon stack (C3)
- Cards: **BT25-002** (buried inherited source) + **BT25-027** (the board carrier
  it sits beneath) + a synthesized DATA SQUAD Tamer (name/trait only).
- Expected outcome: with Wanyamon (002) as the bottom source under a real
  MachGaogamon (027), playing a DATA SQUAD Tamer on your turn makes **both**
  players draw 1 — the inherited trigger fires from inside a real two-card
  digivolution stack.
- Rules/keyword basis: DCGO `BT25_002.cs` (`OnEnterFieldAnyone`, inherited,
  owner-turn, DATA SQUAD-Tamer condition, draw loops over `Players_ForTurnPlayer`);
  `general_rule.pdf` §16 inherited-effect activation.
- Rank: 2.

### Mach stash contention: bounce-unsuspend vs leave-prevention (C4)
- Cards: **BT25-027** alone (two of its own clauses) + one face-down stash + an
  opp lvl≤4.
- Expected outcome: the WA self-unsuspend and the leave-prevention replacement
  draw from the *same* single face-down stash. After the WA chain consumes the
  only face-down source, a subsequent leave attempt has no stash to pay and
  MachGaogamon leaves — the cross-clause resource-contention fact a single-trigger
  per-card test never exercises.
- Rules/keyword basis: DCGO `BT25_027.cs` (both the shared WD/WA tail and the
  `WhenRemoveField` replacement trash a bottom face-down source — they compete
  for the same `IsFlipped` sources); `general_rule.pdf` §16 replacement timing.
- Rank: 3.

## Blocked combos (named a BLOCKED card → not authored, logged)

- **Thomas stash-build → Mach/Mirage stash-spend** (BT25-087 + 027/029): Thomas
  builds the face-down stash that 027/029 spend. **Blocked on BT25-087** (its
  stash-building clause needs the inert `OnAddToHand` trigger; its DS-digivolve
  discount hits the BeforePayCost Parked-drop gap).
- **MirageGaogamon double-bounce + add-to-hand-unsuspend loop** (BT25-029 +
  087): **Blocked on BT25-029** (the `OnAddToHand` half of its unsuspend clause
  is inert) and **BT25-087**.
- **Mirage Beast Knight shortcut into MirageGaogamon** (BT25-096 → 029):
  **Blocked on BT25-096** (use-cost −2 trash-FD hits BeforePayCost Parked-drop)
  and **BT25-029**.
- **ShineGreymon: Burst Mode Option-Main + Marcus aura** (BT25-104): **Blocked
  on BT25-104** (cross-side Option-Main activation gap + name-overlay aura gap).

All four gaps are already recorded against the individual cards in
`docs/RUST_ENGINE_GAPS.md` (OnAddToHand inert; BeforePayCost selection-bearing
pay_cost Parked-drop; Option-Main cross-side activation; treated-as-Digimon
name-overlay aura) per the per-card BLOCKED verdicts — no new gap is introduced
by this capstone pass.

## Playstyle

Tempo/combo control. Thomas sets memory to 3 at the start of your turn and feeds
a face-down "fuel tank" the Gaogamon line drains for repeated free bounces and
near-unkillable bodies; Wanyamon turns each Tamer play into card advantage (a
symmetric both-players draw). Closes with MirageGaogamon's repeated tempo bounce
or the ShineGreymon Burst Mode finisher.

## Win conditions

- Lock the opponent's board out of attacks via MachGaogamon/MirageGaogamon
  recurring bounce while their own bodies refuse to die (FD-stash leave-prevention
  / Evade), then push security.

## Ranked interactions to test (authored)

1. **C1** Mach alt-evo + WA bounce + stash-trash self-unsuspend (the core engine). — authored
2. **C2** Mach WA with no stash: bounce resolves, self-unsuspend unpaid (the gate). — authored
3. **C3** Wanyamon inherited draw firing from under a real MachGaogamon stack on Tamer play. — authored
4. **C4** Mach stash contention: WA consumes the only stash → later leave-prevention can't pay. — authored

Dropped (blocked, logged above): Thomas stash-build chain; MirageGaogamon
double-bounce/unsuspend loop; Mirage Beast Knight shortcut; ShineGreymon BM
Option-Main + Marcus aura.
