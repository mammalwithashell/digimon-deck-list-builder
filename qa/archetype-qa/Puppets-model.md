# Puppets — Model

> Archetype-model artifact produced by `/archetype-interaction-test-author`
> (Phases 0-3). System-level model of the **Puppets** / [Puppet]+[LIBERATOR]
> deck: how it ramps, recurs, and closes. Sources cited inline (DCGO C# path /
> `general_rule.pdf` rule via `docs/RULES_CONTEXT.md` index). All 62 pool cards
> are IMPLEMENTED in the Rust DSL per `qa/qa-reports/validated_cards_dsl.json`
> (status `IMPLEMENTED`); the resolve-deck `script_status: missing` reflects the
> *Python* registry only and is not authoritative here.
>
> Canonical name: **Puppets**. Pool resolved over 34 decklists
> (`python code/tools/resolve_deck.py "Puppets" --json`).

## Card pool & roles

Puppets is a Yellow (splash Purple/Black) midrange/combo deck built on the
**[Puppet] trait + [LIBERATOR] trait** package. Its core loop is: cheap [Puppet]
rookies -> free-digivolve up the line by **deleting your own Tokens/[Puppet]
Digimon** -> recur the deleted bodies from trash -> close with **<Overclock>**
extra attacks. Tamers (Arisa/Mirai) and Option cards are the ramp/consistency
engine that fuels the digivolve chains.

| Card | Role | One-line function |
|------|------|-------------------|
| BT22-098 Unique Emblem: Fable Waltz (**Option**) | enabler/engine | `[Main]` free-play Shoemon/Arisa from **hand or trash**, then self-places; Delay: a [Puppet] digivolves into a [Puppet]+[LIBERATOR] hand card, cost -3 |
| P-229 Unique Emblem: Narrative Ronde (**Option**) | enabler/engine | `[Main]` reveal-3, add 1 [Puppet] + 1 [LIBERATOR] to hand; Delay (on Mirai play): a Digimon digivolves into a Lv<=6 [LIBERATOR] hand card, cost -3 |
| EX7-074 Vortex Resonance (**Option**) | enabler | `[Main]` reveal-3, add 1 [LIBERATOR] to hand; then a Digimon may digivolve into a hand Digimon, cost -4; color-bypass while you have a LIBERATOR Digimon/Tamer |
| LM-029 Yellow Scramble (**Option**) | enabler/recursion | `[Main]` a yellow Digimon digivolves into a yellow hand card, cost -3; Delay: return a yellow Digimon from trash to deck top, then (if boardless) free-play a small yellow from trash |
| P-105 Physical Training / LM-054 Treadmill Training (**Option**) | enabler | `[Main]` reveal + add; Delay: digivolve into a yellow (or yellow/black) hand card for cost, reduced by 2 |
| P-037 / LM-035 / LM-037 Memory Boost! (**Option**) | enabler | reveal-4/3 add 1 yellow/purple Digimon; Delay: gain 2 memory |
| BT4-104 Blinding Ray (**Option**) | tech | `[Main]` trash top security, gain 2 memory (security-cost tempo) |
| EX9-067 / EX11-061 Mirai Kinosaki (Tamer) | enabler | reveal/ramp + digivolve-trigger plays; the Delay trigger for Narrative Ronde |
| BT22-088 / EX7-063 / ST19-14 / P-136 Arisa Kinosaki (Tamer) | engine | on Token/[Puppet] play or deletion: draw / free-play a Lv3 [Puppet] / suspend for value; the Fable Waltz Delay trigger |
| EX9-032 / EX11-022 Karakurumon (Lv5) | engine | `[OnPlay/WhenDigivolving]` delete 1 Token/other [Puppet] -> free-digivolve into a [Puppet] hand card; <Scapegoat> |
| EX7-024 / ST19-03 / EX11-019 Shoemon (Lv3) | enabler | cost -1 to digivolve into [Puppet]; On Deletion play a Familiar Token / Lv3 [Puppet]; <Barrier> inherited |
| P-165 / EX7-025 ShoeShoemon (Lv4) | engine | `[OnPlay/WhenDigivolving]` play Familiar Token(s); On Deletion play a Lv3 [Puppet] |
| EX7-027 / BT22-036 / ST19-11 Chaperomon (Lv5) | payoff | <Overclock>; WhenDigivolving play a Lv3 [Puppet] / -DP; inherited "doesn't leave" by deleting a [Puppet] |
| EX7-030 / EX11-024 / BT22-040 / ST19-12 Cendrillmon (Lv6) | payoff | <Overclock>; WhenDigivolving play Familiar Token(s) + a [Puppet] hand card; -DP swings |
| EX9-033 / EX11-023 Kaguyamon (Lv6) | payoff/engine | grants <Alliance>+<Blocker> to all Tokens/[Puppet]; on other-deletion delete opp lowest level; EoT free-play a Lv<=4 [Puppet] from trash |
| BT22-042 Nyabootmon (Lv7) | payoff | <Overclock>; WhenDigivolving free-play Lv<=4 [Puppet] + -3000 DP per own Digimon; on other-deletion re-fire its WhenDigivolving |
| BT22-002 / ST19-01 Kyaromon, BT15-003 Nyaromon (DigiEgg) | engine | inherited draw on [Puppet]/Token deletion (Kyaromon) / when-attacking security-trash ramp (Nyaromon) |

(The named combos below reference these pieces; the full per-card roster is the
resolve-deck output. Tech/off-archetype splashes - BT9-112 DeathXmon, EX4-074
ShineGreymon: Ruin Mode, EX6-011 RagnaLoardmon, BT9-033 Pillomon, EX8-030
Tapirmon, BT5-033 Cutemon, BT13-101 Miki & Megumi - are present but not the
archetype's combo core.)

## Digivolution lines

- **Kyaromon/Nyaromon (Lv2 egg) -> Shoemon (Lv3 [Puppet]) -> ShoeShoemon (Lv4) -> Chaperomon (Lv5) -> Cendrillmon (Lv6) -> Nyabootmon (Lv7)** - the mono-Yellow [Puppet] line; each evo step is cheap (Shoemon cost -1, Option cost -3/-4).
- **Hanimon (Lv3) -> Kokeshimon (Lv4) -> Karakurumon (Lv5) -> Kaguyamon (Lv6)** - the Yellow/Purple [Puppet]+[LIBERATOR] line; Karakurumon free-digivolves by deleting a Token/[Puppet].
- **Token sub-engine:** Familiar Token (Yellow/3000 DP, `[On Deletion]` give an opponent Digimon -3000 DP) is generated by ShoeShoemon/Cendrillmon and spent as <Overclock> / free-digivolve / Scapegoat fuel - its deletion drives Kyaromon draw and Arisa/Kaguyamon triggers.

## Named combos

### Fable Waltz -> Arisa engine, then Delay cost-reduced digivolve  *(Option - top rank)*

- **Cards:** BT22-098 (Unique Emblem: Fable Waltz), an Arisa Kinosaki Tamer (e.g. EX7-063 / BT22-088), a [Puppet]+[LIBERATOR] Digimon in hand (e.g. EX11-022 Karakurumon / EX9-033 Kaguyamon), a [Puppet] base on board.
- **Expected mechanical outcome:** `[Main]` of Fable Waltz plays 1 Shoemon **or** Arisa Kinosaki from hand **or trash** without paying cost (union-zone pick), then Fable Waltz places itself in the battle area as a Delay Option (board: +1 permanent for the played card, +1 Option permanent; trash/hand -1; **0 memory paid** for the played card). On a later turn, when your Arisa Kinosaki suspends, trashing Fable Waltz lets 1 of your [Puppet] Digimon digivolve into a [Puppet]+[LIBERATOR] hand card with the **digivolution cost reduced by 3** (target stack +1 card; hand -1; memory paid = printed evo cost - 3, floored at 0). **Unhappy path:** if no Arisa is on board (or it does not suspend), the Delay cannot be declared and the Option stays inert.
- **Rules/keyword basis:** `<Delay>` (`general_rule.pdf` 16-16); union-zone free-play (no cost paid). DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/BT22/Yellow/BT22_098.cs` (`OptionSkill` union-zone play + `PlaceDelayOptionCards`; `OnTappedAnyone` Delay gated on `IsArisaKinosaki` suspend, `DigivolveIntoHandOrTrashCard` reduceCost 3).
- **Rank:** highest - a deck-frequency 16 Option that both ramps the board and is the cheapest route into the [Puppet]+[LIBERATOR] top end; the Arisa-suspend -> Delay-digivolve chain spans Option + Tamer + Digimon.

### Karakurumon: delete a Token to free-digivolve into a Puppet hand card  *(engine - high rank)*

- **Cards:** EX9-032 / EX11-022 (Karakurumon), 1 Familiar Token or other [Puppet] Digimon on board, a [Puppet] Digimon card in hand.
- **Expected mechanical outcome:** on Karakurumon's `[OnPlay/WhenDigivolving]`, by **deleting 1 of your Tokens or other [Puppet] Digimon** (board -1 permanent), Karakurumon digivolves into a [Puppet] Digimon card from hand for **cost 0, ignoring digivolution requirements** (Karakurumon's stack gains the hand card on top; hand -1; memory paid 0). The deleted Token's `[On Deletion]` (-3000 DP to an opponent Digimon) and Kyaromon's inherited draw both fire off the same deletion. **Unhappy path:** if you control no other Token/[Puppet] the effect is uncastable (no deletion cost available) and Karakurumon plays normally.
- **Rules/keyword basis:** "by [deleting]" cost paid before reward (`general_rule.pdf` cost-then-effect); ignore-digivolution-requirement free digivolve. DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/EX9/Yellow/EX9_032.cs`.
- **Rank:** high - the central free-tempo engine of the Yellow/Purple line; chains a deletion (Token), a recursion enabler, and a digivolve in one action.

### Narrative Ronde -> Mirai-triggered Delay digivolve  *(Option - high rank)*

- **Cards:** P-229 (Unique Emblem: Narrative Ronde), a Mirai Kinosaki Tamer (EX9-067 / EX11-061), a Lv<=6 [LIBERATOR] Digimon in hand, a Digimon base.
- **Expected mechanical outcome:** `[Main]` reveals top 3, adds 1 [Puppet] Digimon card **and** 1 [LIBERATOR] card to hand (hand +2; deck -2, rest bottomed), then self-places. When a Mirai Kinosaki is **played**, trashing Narrative Ronde lets 1 of your Digimon digivolve into a **Lv<=6 [LIBERATOR]** hand card with cost **reduced by 3** (target stack +1; hand -1; memory paid = evo cost - 3). **Unhappy path:** Delay requires a Mirai *play* event after the placing turn; without it the Option is inert.
- **Rules/keyword basis:** `<Delay>` (16-16); reveal-and-add-to-hand. DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/P/Yellow/P_229.cs` (`OptionSkill` reveal/add-2 + `PlaceDelayOptionCards`; `OnEnterFieldAnyone` Delay gated on a Mirai Kinosaki play, `DigivolveIntoHandOrTrashCard` reduceCost 3, Lv<=6 LIBERATOR `CardCondition`).
- **Rank:** high - deck-frequency 18 Option; the only digging-+-Delay-digivolve package keyed to the Mirai play trigger.

### Vortex Resonance: dig a LIBERATOR, then digivolve into hand for -4  *(Option - high rank)*

- **Cards:** EX7-074 (Vortex Resonance), a Digimon base, a Digimon card in hand, a [LIBERATOR] Digimon/Tamer already on board (color-bypass enabler).
- **Expected mechanical outcome:** `[Main]` reveals top 3, adds 1 [LIBERATOR] card to hand (rest bottomed), then 1 of your Digimon **may** digivolve into a hand Digimon with cost **reduced by 4** (target stack +1; hand -1; memory paid = evo cost - 4, floored at 0). While you control a [LIBERATOR] Digimon or Tamer, the card ignores its Green/Yellow color requirement (playable off an all-yellow board). **Unhappy path:** the digivolve sub-step is optional - if you decline the base pick, only the reveal/add resolves; with no LIBERATOR board the color-bypass is off.
- **Rules/keyword basis:** IgnoreColorRequirement flood-gate; reveal-add; cost-reduced digivolve. DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/EX7/Green/EX7_074.cs` (`None` IgnoreColorConditionClass scoped to this card; `OptionSkill` `SimplifiedRevealDeckTopCardsAndSelect` + `DigivolveIntoHandOrTrashCard` reduceCost 4).
- **Rank:** high - the biggest single-card digivolution discount (-4); a clean Option-enabled tempo line into the Lv5/6 payoffs.

### Overclock + Token loop: extra attack by deleting a Token  *(payoff - high rank)*

- **Cards:** a <Overclock> payoff (EX7-027/BT22-036 Chaperomon, EX7-030/EX11-024 Cendrillmon, BT22-042 Nyabootmon), a Familiar Token or other [Puppet] Digimon, and (synergy) Kyaromon egg + Arisa for the deletion payoff.
- **Expected mechanical outcome:** at end of your turn, by **deleting 1 of your Tokens or other [Puppet] Digimon** (board -1), the Overclock Digimon **attacks a player without suspending** (an extra, unsuspended attack - i.e. it can have already attacked this turn and still attack again). The deleted Familiar Token's `[On Deletion]` gives an opponent Digimon -3000 DP; Kyaromon inherited `<Draw 1>` and Arisa/Kaguyamon deletion-triggers also fire. **Unhappy path:** with no spare Token/[Puppet] to delete, Overclock cannot pay and no extra attack occurs.
- **Rules/keyword basis:** `<Overclock>` (`general_rule.pdf` 16-33) - end-of-turn, pay by deleting a Token/[Puppet], attack a player without suspending; `[On Deletion]` Token trigger; Kyaromon inherited draw on [Puppet]/Token deletion. DCGO C#: `$BASE_DCGO/.../BT22/Yellow/BT22_042.cs` (Overclock grant + cost filter), Token registry `code/digimon-engine/src/cards/tokens/familiar.rs`.
- **Rank:** high - the deck's primary closing line; converts the board into player-face damage and feeds the deletion-payoff sub-engine.

## Playstyle

- **Class:** midrange/combo. Tempo comes from cost-reduced/free digivolution (Options -3/-4, Shoemon -1, Karakurumon free) and from converting Tokens into value (draw, -DP, extra attacks).
- **Memory curve:** ramp with Tamers (Arisa "set to 3", Mirai gain 1) and Memory Boost / Blinding Ray; build a [Puppet] board; recur deleted bodies from trash (Kaguyamon EoT, Yellow Scramble, Shoemon/ShoeShoemon On-Deletion plays).
- **Resilience:** <Scapegoat> (Karakurumon/Kaguyamon), <Barrier> (Shoemon), Chaperomon/Karakurumon inherited "doesn't leave by deleting a [Puppet]", and Hanimon/Kokeshimon attack-enders make the board sticky.

## Win conditions

- **Overclock beatdown:** chain extra attacks from Chaperomon/Cendrillmon/Nyabootmon by deleting Tokens, while -DP swings (Cendrillmon, Nyabootmon, Familiar Tokens) clear blockers.
- **Security pressure:** Shoemon variants give opponent Security Digimon -3000 DP / <Security A. -1>; Nyabootmon/Cendrillmon stack <Security A. +1> via <Alliance> (Kaguyamon grants it board-wide) for multi-checks.
- **Deletion grind:** Kaguyamon (delete opp lowest level on other-deletion) + Nyabootmon (re-fire WhenDigivolving on other-deletion) attrition the opponent board while Kyaromon/Arisa refill the hand.

## Ranked interactions to test

1. **Fable Waltz -> Arisa engine + Delay cost-reduced digivolve** - Option that ramps and is the cheapest path to the [Puppet]+[LIBERATOR] top end; spans Option + Tamer + Digimon, with a clear board diff (free play, self-place, -3 digivolve). *(highest - Option-centric)*
2. **Karakurumon: delete-a-Token free-digivolve into a Puppet hand card** - the central free-tempo engine; deletion-cost + ignore-requirements + chained Token/Kyaromon triggers in one action.
3. **Narrative Ronde -> Mirai-triggered Delay digivolve** - Option dig-2 + Delay -3 digivolve keyed to the Mirai play trigger.
4. **Vortex Resonance: dig a LIBERATOR + -4 digivolve** - biggest single-card digivolution discount; color-bypass off a LIBERATOR board.
5. **Overclock + Token loop: extra attack by deleting a Token** - the closing line; Token `[On Deletion]` -DP + Kyaromon draw fire off the Overclock cost.

> **Capped at 5 (Phase 3).** Logged but dropped below the cap:
> - *Yellow Scramble (LM-029) Delay trash-recursion* - `[Main]` -3 digivolve + Delay return-to-deck/free-play; strong but overlaps combos 1/4 on the "Option cost-reduced digivolve" axis (rank 6).
> - *Shoemon/ShoeShoemon On-Deletion -> Familiar Token / Lv3 [Puppet] chain* - recursion glue; better as a per-card test than a system combo (rank 7).
> - *Kaguyamon other-deletion -> delete opp lowest level + EoT trash recursion* - payoff attrition; overlaps the Overclock/deletion sub-engine (rank 8).
> - **Note (not authored / faithfulness watch):** EX9-067 Mirai Kinosaki's `[Your Turn]` "digivolve-into-[Puppet] -> return Tamer, play Arisa/[Puppet] with play cost -3" clause is **absent** from `code/digimon-engine/cards/ex9/EX9-067.yaml` (only `[On Play]` reveal + `[Security]` are present). Any combo resting on that clause is **blocked on the missing effect** and was not ranked into the cap; route to `qa/archetype-qa/engine-gaps.md` if confirmed during Phase 4/6.
