# Digimon TCG — Keyword Semantics (compact baseline)

> **Source:** Comprehensive Rules Manual `general_rule.pdf` **Ver. 4.3 (Last updated 2026/09/18)**,
> §16 "Keyword Effects" (pp.35–42), cross-checked against `glossary.pdf`.
> **Verified** against the official text (worktree-readable copy: `docs/digimon-rules/cr-4.3.txt`,
> a `pdftotext -raw` extraction of the PDF; not from memory and not from the retired `RULES_CONTEXT.md`).
> This is the cheap baseline. For full rule text use `/digimon-rules <keyword>` (reads the cited
> PDF pages) or `/digimon-rules deep` (loads the deep digest). PDFs live base-only:
> `BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"` → `"$BASE/Digimon TCG resources/general_rule.pdf"`.
> **Numbering changed in 4.0:** a new §16-3 ("specified cards") shifted every keyword up by one
> (3.6's 16-6 Piercing is now 16-7, 16-40 Training is now 16-41). The 3.6→4.3 delta, including the
> §15-7-4 reversal, is in `changelog-3.6-to-4.3.md`. When the manual revises again, run
> `python code/tools/rules/check_rules_freshness.py`, re-verify §16 and bump this stamp.

Decoy clarification: [official Rule Q&A](https://world.digimoncard.com/rule/),
Keyword Effects → Decoy, Q228 (Mar. 28, 2024; retrieved Oct. 12, 2026), permits
Decoy when its carrier and the protected Digimon are threatened by the same
simultaneous deletion. The glossary's already-deleted-carrier restriction
does not exclude a carrier that is still in play awaiting that deletion.

## Kind legend
- **Persistent** — a rule-altering standing effect; not "processed" as an action (§15-8-2).
- **Mandatory** — §16 states the processing is mandatory (must happen once triggered).
- **Optional** — §16 states the processing is optional ("you may" / "up to").
- **Opt-cost→Mand** — has an *optional processing condition* ("By suspending/deleting/trashing…"); paying it is optional, but if you do, the resulting effect is **mandatory** (§15-6/15-7). **4.3 §15-7-4:** the cost can only be chosen when its contents can actually be executed (an unpayable cost is not offered); §15-7-5: it *can* be chosen even if the result after it can't be executed.

## Keyword table (§16-4 … §16-47, pp.35–42)

| Keyword | Kind | Type / when | Semantics | Rule § |
|---|---|---|---|---|
| `<Security A. +x / -x>` (was `<Security Attack>`) | Persistent | While attacking the player | Checks x more / fewer security cards; multiple instances don't combine into one value; -x floors at 0 (can't win checking 0) | 16-4 |
| `<Blocker>` | Persistent | Opponent's Digimon attacks | May suspend to redirect the attack to this Digimon; max 1 block per block timing even with multiple `<Blocker>` | 16-5 |
| `<Recovery +x (Deck)>` | Mandatory | On effect | Place top x cards of the specified area face-down on top of security stack (printed as `<Recovery +1>`; 3.6's "if Deck is specified" sub-rule was dropped) | 16-6 |
| `<Piercing>` | Mandatory | Trigger; pending processing resolved immediately before end-of-attack timing | "When this Digimon deletes your opponent's Digimon in battle while attacking, it checks security before the attack ends"; **the check is mandatory**; triggers simultaneously with battle triggers but is processed after them; only 1 security check per attack even if `<Piercing>` activates multiple times (`<Security A. +x>` modifies the count inside that one check); no check if no Digimon remains to perform it | 16-7 |
| `<Draw x>` | Mandatory | On effect | Draw x cards; mandatory | 16-8 |
| `<Jamming>` | Persistent | Battle vs Security Digimon | Not deleted as a result of a battle with a Security Digimon | 16-9 |
| `<Digisorption -x>` | Opt-cost→Mand | Immediate; digivolving into this card from hand | *By suspending 1 of your Digimon* (optional), reduce the digivolve cost by x (then mandatory); floors at 0; multiple can stack | 16-10 |
| `<Reboot>` | Mandatory | Opponent's unsuspend phase | Unsuspend this Digimon during the opponent's unsuspend phase; mandatory; once even with multiples | 16-11 |
| `<De-Digivolve x>` | Mandatory | On effect (targets opp Digimon) | The player **declares a number from 1 up to x** (4.3, §16-12-1; an action-space decision, `DeDigivolveCount`), then that many cards are trashed from the top of an opponent's Digimon (stops at no digivolution cards / Lv.3); **trashing mandatory, can't choose 0**; can't reduce Lv.3 or lower | 16-12 |
| `<Retaliation>` | Mandatory | Trigger; this deleted in battle | When deleted in battle, delete the Digimon it battled; mandatory | 16-13 |
| `<Digi-Burst X>` | Optional | On specified timing | Trash X of this Digimon's digivolution cards to activate the linked effect; optional | 16-14 |
| `<Rush>` | Persistent | — | Can attack the same turn it was played | 16-15 |
| `<Blitz>` | Optional | After digivolving | May attack even if memory is ≥1 on opponent's side; "this Digimon may attack" is optional; can't if memory is already ≥0 on opp side upon activating, or if otherwise unable to attack | 16-16 |
| `<Delay>` | Optional | Option in battle area | Trash this card to activate the linked effect; optional; not the turn it entered the battle area; no cost/color needed | 16-17 |
| `<Decoy (X)>` | Opt-cost→Mand | Immediate; another of your specified Digimon would be deleted by opp effect | *By deleting this Digimon* (optional), prevent that deletion (then mandatory); only one of simultaneous deletions; can't if this is itself deleted | 16-18 |
| `<Armor Purge>` | Opt-cost→Mand | Immediate; this would be deleted | *By trashing the top card of this Digimon* (optional), prevent the deletion (then mandatory); needs ≥1 digivolution card; surviving modifiers carry over | 16-19 |
| `<Save>` | Optional | On effect | May place this card under 1 of your Tamers; optional | 16-20 |
| `<Material Save x>` | Optional | Immediate; this deleted | May place x of the top card's DigiXros-requirement cards from its digivolution cards under 1 of your Tamers; optional, but if processed must place the number whenever possible | 16-21 |
| `<Evade>` | Opt-cost→Mand | Immediate; this would be deleted | *By suspending this Digimon* (optional), prevent the deletion (then mandatory) | 16-22 |
| `<Raid>` | Optional | Trigger; when this attacks | May switch the attack target to the opponent's unsuspended Digimon with the highest DP; optional; attacker chooses if tied | 16-23 |
| `<Alliance>` | Opt-cost→Mand | Trigger; when this attacks | *By suspending 1 of your other Digimon* (optional), add its DP to the attacker and gain `<Security A. +1>` for the attack (then mandatory); added DP/SA persist even if the suspended Digimon leaves | 16-24 |
| `<Barrier>` | Opt-cost→Mand | Immediate; this would be deleted in battle | *By trashing the top card of your security stack* (optional), prevent that deletion (then mandatory) | 16-25 |
| `<Blast Digivolve>` | Optional | On effect | May digivolve into this card in hand without paying the cost; optional; digivolves the chosen battle-area Digimon | 16-26 |
| `<Fortitude>` | Mandatory | Trigger; this (with digivolution cards) deleted | Play this Digimon (from trash) without paying the cost; mandatory | 16-27 |
| `<Mind Link>` | Mandatory | On effect | Place a Tamer with this effect into a Tamer-less Digimon's digivolution cards; mandatory (but `[Main]`-timing instances may be timed by the player) | 16-28 |
| `<Partition (…)>` | Optional | Immediate; this + 1 of each specified card in its digivolution cards would be removed (not by your effect/battle) | May play 1 of each specified card from the digivolution cards without paying costs; optional; all-or-nothing across the specified set ("specified cards" is now the generic §16-3) | 16-29 |
| `<Collision>` | Persistent | While this is attacking | All opponent's Digimon gain `<Blocker>` and the opponent is forced to block whenever possible | 16-30 |
| `<Blast DNA Digivolve (…)>` | Optional | On effect | A specified Digimon + a hand card may DNA-digivolve into this card in hand without paying cost; optional | 16-31 |
| `<Scapegoat>` | Opt-cost→Mand | Immediate; this would be deleted (not by your effect) | *By deleting 1 of your other Digimon* (optional), prevent the deletion (then mandatory) | 16-32 |
| `<Vortex>` | Optional | Trigger; end of your turn | May attack an opponent's Digimon at end of turn; optional; also lets it attack the turn it was played | 16-33 |
| `<Overclock (…)>` | Opt-cost→Mand | Trigger; end of your turn | *By deleting 1 of your Tokens / other specified Digimon* (optional), this Digimon attacks a player without suspending (then mandatory) | 16-34 |
| `<Iceclad>` | Persistent | Battle (not vs Security Digimon) | Compare number of digivolution cards instead of DP; higher count wins, tie = both lose | 16-35 |
| `<Decode (…)>` | Optional | Immediate; this would leave the battle area (not by a battle) | May play 1 specified Digimon card from its digivolution cards without paying the cost; optional | 16-36 |
| `<Fragment (X)>` | Opt-cost→Mand | Immediate; this would be deleted | *By choosing & trashing X of this Digimon's digivolution cards* (optional), it isn't deleted (then mandatory) | 16-37 |
| `<Execute>` | Optional | Trigger; end of your turn | May attack (incl. an opponent's unsuspended Digimon); at end of that attack this Digimon is deleted (pending processing); optional | 16-38 |
| `<Progress>` | Persistent | While this is attacking | This Digimon isn't affected by your opponent's effects while attacking | 16-39 |
| `<Link +x>` | Persistent | — | Adds x to this Digimon's maximum link cards; multiples add, but aren't a single combined-value effect | 16-40 |
| `<Training>` | Opt-cost→Mand | Activation; main phase (also in breeding area) | *By suspending this Digimon during the main phase* (optional), place the top deck card at the bottom of this Digimon's digivolution cards (then mandatory) | 16-41 |
| `<Use Req.>` | Persistent | — (new in 4.0) | Ignore the colour requirements of Option use with the specified Digimon and/or Tamers on the field | 16-42 |
| `<Ascension>` | Optional | Trigger; this deleted (new in 4.0) | May place this card on top of the security stack; optional | 16-43 |
| `<Engage>` | Optional | Trigger; end of your turn (new in 4.1) | This Digimon may attack at the end of your turn; optional | 16-44 |
| `<Guard>` | Opt-cost→Mand | Immediate; another of your Digimon would leave the battle area by an opponent's effect (new in 4.1) | *By deleting this Digimon* (optional), the other Digimon doesn't leave (then mandatory); §16-45-3 words the whole processing as optional | 16-45 |
| `<Detach (…)>` | Opt-cost→Mand | Immediate; this would leave the battle area other than by your effects (new in 4.2) | *By trashing 1 of its specified link cards* (optional), it doesn't leave (then mandatory) | 16-46 |
| `<Succession (…)>` | Persistent | — (new in 4.2) | This Digimon gains all effects other than `<Succession>` on its topmost specified digivolution card | 16-47 |

## Notes
- §16-1/16-2: keyword effects are the bracketed-icon effects; the same keyword is the same effect type even with a different numeric value or named card in the icon. **§16-3 (new in 4.0):** "specified cards" for any angle-bracket keyword are the cards meeting the parenthesised conditions.
- **Multiple instances (changed in 4.0):** the 17 per-keyword "if 1 Digimon has multiple instances of `<X>`, they overlap when they trigger" rules (3.6 §16-12-4, 16-17-4, 16-18-4, 16-20-4, 16-21-4, 16-22-4, 16-23-4, 16-24-4, 16-26-4, 16-28-4, 16-31-4, 16-32-4, 16-33-4, 16-35-4, 16-36-4, 16-37-5) were **deleted**; only the explicit value-stacking rules remain (`<Security A.>` 16-4-3, `<Blocker>` 16-5-3, `<Reboot>` 16-11-3, `<Link +>` 16-40-3, `<Piercing>` 16-7-5). Multiplicity now falls back to the generic trigger rules (§15-5-2: one trigger condition triggers once per event; §15-4-3 simultaneous triggers).
- **Not §16 keywords** (defined elsewhere, listed here so they aren't mistaken for §16 keywords): `<Overflow>` (ACE rule, §4-19), **Arts Digivolve** (DUAL-card rule, §4-20; §2-11), **DUAL cards** (§4-6), `[DigiXros]` / `[Assembly]` alt-play with material (§7-2 / §7-3; declaring use §1-3-11), DNA Digivolution (§8-2), Burst Digivolve (§8-3), App Fusion (§8-4), `[Counter]` and other effect *timings* (§15-16). Look these up via their own sections, not §16.
- Optional-vs-mandatory is the single most error-prone axis for faithful implementation (rule 17 / no-approximations): expose the optional choice to the action space; never auto-resolve an `Optional` or the cost half of an `Opt-cost→Mand` keyword — and per 4.3 §15-7-4, never *offer* a cost whose contents can't be executed.
