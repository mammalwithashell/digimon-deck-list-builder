# Ice-Snow (Suzune Kazuki) — Model

Blue/Yellow **digivolution-card-denial control**. The deck strips the
*digivolution cards* (the sources stacked under a Digimon) off the opponent's
board, then leverages two payoffs: **&lt;Iceclad&gt;** (compare digivolution-card
counts instead of DP in battle, so a stripped Digimon loses) and a family of
"**while your opponent has no Digimon with digivolution cards**" bonuses
(Piercing, Security A. +1, suspend-locks). **Suzune Kazuki** is the engine Tamer
that converts each Ice-Snow play/digivolve into more trashing (and memory).

Sources: card images (authoritative for printed text) + `data/cards.json`; DCGO
C# crosschecks live inline in each card's YAML header
(`code/digimon-engine/cards/<set>/<ID>.yaml`); `general_rule.pdf` §16 for keyword
semantics (Iceclad, Jamming, Blocker, Piercing, Security Attack ±N, Barrier,
Blast Digivolve, Delay).

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| EX11-002 Hiyarimon (Lv.2) | enabler (inh) | While opp has no Digimon w/ digivo cards, Ice-Snow may attack unsuspended Digimon |
| EX7-016 Bulucomon (Lv.3) | enabler | [On Play] reveal 3, add 1 Paledramon/Hexeblaumon + 1 Ice-Snow; inh [WA] OPT trash top opp source |
| EX11-014 Penguinmon (Lv.3) | enabler | [On Play] reveal 3, add 1 Suzune Kazuki + 1 Ice-Snow; &lt;Jamming&gt; |
| EX8-019 Penguinmon (Lv.3) | **cost ramp** | **[Your Turn] when digivolving into an [Ice-Snow] Digimon, reduce the digivolution cost by 1**; inh [WA] give opp &lt;Security A. -1&gt; |
| EX7-020 Paledramon (Lv.4) | engine | [WD] trash bottom 2 opp sources; then if opp sourceless gains &lt;Jamming&gt;+&lt;Blocker&gt; |
| P-215 Icemon (Lv.4) | tech | [When Moving][On Play][WD] place a Lv.4↓ Ice-Snow/Mineral/Rock as bottom source → protect own from bounce / De-Digivolve |
| EX11-015 Frigimon (Lv.4) | engine | [WD] if ≤1 Tamer, play 1 Suzune Kazuki free; &lt;Jamming&gt; |
| EX8-022 Frigimon (Lv.4) | engine | &lt;Iceclad&gt;; [On Play][WD] trash bottom 2 opp sources, then if opp sourceless gain 1 memory |
| EX7-021 CrysPaledramon (Lv.5) | payoff | &lt;Iceclad&gt;; [WD] trash any 2 opp sources, then if opp sourceless unsuspend self; inh sourceless→Piercing+SecA+1 |
| EX11-016 PolarBearmon (Lv.5) | payoff | &lt;Iceclad&gt;; [On Play][WD] trash any 2, then may place an opp sourceless Digimon as top/bottom security |
| EX8-023 PolarBearmon (Lv.5) | payoff | &lt;Iceclad&gt;; [On Play][WD] trash any 2, then 1 opp sourceless Digimon can't suspend or activate [WD] |
| EX7-023 Hexeblaumon (Lv.6) | finisher | Sec A. +1, &lt;Iceclad&gt;; [WD] trash any 4, then if opp sourceless return a Tamer; [Opp Turn] ≤-source Digimon can't suspend |
| EX11-017 Skadimon (Lv.6) | finisher | &lt;Iceclad&gt;,&lt;Barrier&gt;; [OP][WD][WA] OPT free-play Suzune/Lv.4↓ Ice-Snow; [All Turns] OPT on others played/digivolve trash any 3 + can't-suspend |
| EX8-028 Skadimon (Lv.6) | finisher | &lt;Iceclad&gt;,&lt;Barrier&gt;; [WD] free-play Lv.(4+X) Ice-Snow; [WD][WA] OPT **place 1 sourceless Digimon as bottom security → unsuspend** |
| BT17-077 Imperialdramon PM (Lv.7) | finisher | &lt;Blast Digivolve&gt; (free from white Lv.6); [OP][WD] trash ALL opp sources + recycle trashes; [WA] return opp sourceless → unsuspend |
| EX8-066 Suzune Kazuki (Tamer) | engine | [Start Main] +1 memory if opp has Digimon; [All Turns] when your Digimon played/digivolve & any is Ice-Snow, suspend this → trash 1 opp source |
| EX11-057 Suzune Kazuki (Tamer) | engine | [Start Main] +1; [On Play] trash 1 per your Ice-Snow Digimon; [All Turns] when effects trash opp sources, suspend this → +1 memory |
| P-228 Frozen Crown (Option) | **cost ramp** | [Main] search Ice-Snow + LIBERATOR, place in battle area; [Your Turn] when a Suzune Kazuki is played, &lt;Delay&gt; → 1 Digimon may digivolve into a Lv.6↓ LIBERATOR in hand with **cost reduced by 3** |

## Digivolution lines & cost gates

Printed evo boxes (from `cards.json` `evo_costs`, all from **blue**):

- Lv.2 → Lv.3: EX11-014 / EX8-019 Penguinmon (from Lv.2, **cost 1**).
- Lv.3 → Lv.4: EX7-020 Paledramon (cost 2); EX8-022 Frigimon, EX11-015 Frigimon, P-215 Icemon (cost 3).
- Lv.4 → Lv.5: EX7-021 CrysPaledramon (cost 3); EX11-016 / EX8-023 PolarBearmon (cost 4).
- Lv.5 → Lv.6: EX7-023 Hexeblaumon, EX11-017 / EX8-028 Skadimon (cost 4).
- Lv.6 → Lv.7: BT17-077 Imperialdramon PM (white+blue, cost 6) **or &lt;Blast Digivolve&gt; free**.

**Cost-reduction sources (the deck's tempo):**
1. **EX8-019 Penguinmon** — static `cost_reduction` at `before_pay_cost`: −1 to *any* digivolution where EX8-019 is the source and the target is an [Ice-Snow] Digimon.
2. **EX8-028 Skadimon alt evo box** — "Digivolve Lv.5 w/[Ice-Snow] trait: Cost 3" (vs the cost-4 blue path); a printed reduced-cost path, not a modifier. (EX8-022/EX8-023 carry analogous alt Ice-Snow boxes.)
3. **P-228 Frozen Crown &lt;Delay&gt;** — −3 to digivolve a Digimon into a Lv.6↓ LIBERATOR card from hand, triggered when a Suzune Kazuki is played.
4. **BT17-077 &lt;Blast Digivolve&gt;** — digivolve from a white Lv.6 for **0** (no cost paid).

## Named combos

### A. Strip-and-Iceclad (core)
- Cards: any &lt;Iceclad&gt; payoff (EX8-022 / EX11-016 / EX8-023 / EX7-021 / Hexeblaumon / Skadimon) + the opponent's sourced Digimon.
- Expected outcome: the [WD]/[On Play] clause removes N of the opponent's digivolution cards; an Iceclad attacker then wins a battle it would *lose* on DP (compare stack counts, not DP); when the opponent reaches **zero** sourced Digimon, the "no digivolution cards" inheritances/clauses (Piercing, Security A. +1, suspend-locks) come online.
- Basis: §16 &lt;Iceclad&gt; (stack-count compare); DCGO `EX8_022`/`EX8_023`/`EX7_021`.
- Rank: **highest** (the deck's whole plan).

### B. Suzune Kazuki trash engine
- Cards: EX8-066 Suzune Kazuki (+ optionally EX11-057) + any Ice-Snow Digimon play/digivolve.
- Expected outcome: each time you play or digivolve into an Ice-Snow Digimon, EX8-066 may **suspend itself to trash 1 opponent source**; EX11-057 then converts each such trash into **+1 memory** by suspending. The two Suzunes plus a climb chain several source-trashes + memory in one turn.
- Basis: DCGO `EX8_066` / `EX11_057`; trigger timing "when your Digimon are played or digivolve".
- Rank: **high**.

### C. Cost-ramp climb
- Cards: EX8-019 Penguinmon → Ice-Snow Lv.4 (e.g. EX8-022 Frigimon); and P-228 &lt;Delay&gt; / BT17-077 &lt;Blast Digivolve&gt;.
- Expected outcome: digivolving **from EX8-019** into an Ice-Snow Digimon costs **1 less**; the Frigimon's own [WD] then strips opponent sources on arrival — cheap tempo + denial in one step. Blast Digivolve closes for free.
- Basis: EX8-019 `cost_reduction`; §16 &lt;Blast Digivolve&gt;; P-228 &lt;Delay&gt; −3.
- Rank: **high** (user-requested focus: *digivolution costs* + *[WD] effects firing*).

### D. Skadimon EX8-028 place-and-unsuspend (the softlock card)
- Cards: EX8-028 Skadimon + a sourceless Digimon on either field.
- Expected outcome: [WD] free-plays a Lv.(4+opp-sourceless) Ice-Snow from hand; the [WD]/[WA] OPT clause **places 1 sourceless Digimon (own or opp) as the bottom security card → Skadimon unsuspends** (re-attack). The pay-target pick spans **both** battle areas (`SelectionKind::AnyField`).
- Basis: DCGO `EX8_028`; this is the UI-softlock fixed in this change (see below).
- Rank: medium (single-card, but the regression locus).

### E. Imperialdramon PM board-wipe finisher
- Cards: BT17-077 over a white Lv.6 (e.g. Skadimon line) + opponent sourced board.
- Expected outcome: &lt;Blast Digivolve&gt; free → [On Play]/[WD] **trash ALL** opponent digivolution cards (whole board sourceless at once) → recycle trashes to deck → Iceclad/Piercing alpha-strike.
- Rank: medium.

## Playstyle
Control/tempo. Low-to-mid memory curve; the deck wants the opponent passing
with a board it can't profitably attack into. Suzune Kazuki + EX8-066's
Start-of-Main +1 funds the climb; source-denial buys turns; Iceclad + sourceless
bonuses convert the grind into damage.

## Win conditions
Iceclad beatdown into a source-stripped board; Piercing + Security A. +N
(unlocked while the opponent has no Digimon with digivolution cards) push extra
security checks; suspend-locks (Hexeblaumon, EX8-023, EX11-017, EX8-028's
self-unsuspend re-attack) stop the opponent defending or racing back.

## Ranked interactions to test (user focus: digivolution **costs** + [WD] firing)
1. **EX8-019 cost −1** climb into an Ice-Snow Lv.4 whose own [WD] then trashes opponent sources (cost reduction + [WD] firing in one). — Combo C.
2. **Alt Ice-Snow reduced evo box** (EX8-028 cost-3 from a Lv.5 Ice-Snow vs cost-4 blue). — Combo C.
3. **Standard evo cost** charged on a normal blue climb (control: no reducer present). — baseline.
4. **Suzune Kazuki [All Turns]** fires on an Ice-Snow digivolve → suspend → trash 1 opp source (EX8-066), and EX11-057 → +1 memory on the trash. — Combo B.
5. **EX8-022 / EX7-020 [WD]** actually removes opponent sources on digivolve, and the "opp sourceless" rider (memory / Jamming) only triggers when the board reaches zero. — Combo A.
6. **EX11-015 Frigimon [WD]** free-plays Suzune Kazuki when ≤1 Tamer. — Combo B/C.
7. **EX8-028 [WD] free-play** dynamic cap + the **AnyField** place-as-security unsuspend (drives the fixed selection end-to-end). — Combo D.
8. **BT17-077 &lt;Blast Digivolve&gt;** pays 0 and [WD] trashes all opponent sources. — Combo E.

## UI softlock fix recorded in this change
EX8-028's [WD]/[WA] "place 1 Digimon as the bottom security card" pick is a
`select_any_permanent` spanning **both** battle areas. It previously installed as
`SelectionKind::Target`, which the board's field-selection helpers
(`OwnField`/`OppField` only) could not map — every target was unclickable, and
the inner pick being mandatory, no Decline showed either: a hard softlock. Fixed
by a new `SelectionKind::AnyField` (engine) that the board decodes per-id
(`encode_attack(player, index)`) to highlight/route both sides. Engine guard:
`ex8_028_pay_unsuspend_offers_sourceless_digimon_on_both_sides`. Affects all 14
`select_any_permanent` cards + 5 `select_dna_pair` cards.
