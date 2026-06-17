# Digimon TCG — Keyword Semantics (compact baseline)

> **Source:** Comprehensive Rules Manual `general_rule.pdf` Ver.3.6 (Last updated 2025/12/25),
> §16 "Keyword Effects" (pp.33–40), cross-checked against `glossary.pdf`.
> **Verified** by reading the PDF pages directly (not from memory or `RULES_CONTEXT.md`).
> This is the cheap baseline. For full rule text use `/digimon-rules <keyword>` (reads the cited
> PDF pages) or `/digimon-rules deep` (loads the deep digest). PDFs live base-only:
> `BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"` → `"$BASE/Digimon TCG resources/general_rule.pdf"`.
> When the manual revises (e.g. Ver.3.7), re-verify §16 and bump this stamp.

## Kind legend
- **Persistent** — a rule-altering standing effect; not "processed" as an action (§15-8-2).
- **Mandatory** — §16 states the processing is mandatory (must happen once triggered).
- **Optional** — §16 states the processing is optional ("you may" / "up to").
- **Opt-cost→Mand** — has an *optional processing condition* ("By suspending/deleting/trashing…"); paying it is optional, but if you do, the resulting effect is **mandatory** (§15-6/15-7).

## Keyword table (§16-3 … §16-40, p.33–40)

| Keyword | Kind | Type / when | Semantics | Rule § |
|---|---|---|---|---|
| `<Security A. +x / -x>` (was `<Security Attack>`) | Persistent | While attacking the player | Checks x more / fewer security cards; multiple instances don't combine into one value; -x floors at 0 (can't win checking 0) | 16-3 |
| `<Blocker>` | Persistent | Opponent's Digimon attacks | May suspend to redirect the attack to this Digimon; max 1 block per block timing even with multiple `<Blocker>` | 16-4 |
| `<Recovery +x (Deck)>` | Mandatory | On effect | Place top x deck cards face-down on top of security stack | 16-5 |
| `<Piercing>` | Mandatory | Trigger; end of attack, after battle | If it attacks & deletes the battling Digimon and survives (attack on player, opp has ≥1 security), performs the security check; **the check is mandatory**; not vs Security Digimon | 16-6 |
| `<Draw x>` | Mandatory | On effect | Draw x cards; mandatory | 16-7 |
| `<Jamming>` | Persistent | Battle vs Security Digimon | Not deleted as a result of a battle with a Security Digimon | 16-8 |
| `<Digisorption -x>` | Opt-cost→Mand | Immediate; digivolving into this card from hand | *By suspending 1 of your Digimon* (optional), reduce the digivolve cost by x (then mandatory); floors at 0; multiple can stack | 16-9 |
| `<Reboot>` | Mandatory | Opponent's unsuspend phase | Unsuspend this Digimon during the opponent's unsuspend phase; mandatory; once even with multiples | 16-10 |
| `<De-Digivolve x>` | Mandatory | On effect (targets opp Digimon) | Trash up to x from top of an opponent's Digimon (stops at no digivolution cards / Lv.3); **trashing mandatory, can't choose 0**; can't reduce Lv.3 or lower | 16-11 |
| `<Retaliation>` | Mandatory | Trigger; this deleted in battle | When deleted in battle, delete the Digimon it battled; mandatory | 16-12 |
| `<Digi-Burst X>` | Optional | On specified timing | Trash X of this Digimon's digivolution cards to activate the linked effect; optional | 16-13 |
| `<Rush>` | Persistent | — | Can attack the same turn it was played | 16-14 |
| `<Blitz>` | Optional | After digivolving | May attack even if memory is ≥1 on opponent's side; "this Digimon may attack" is optional; can't if memory is already ≥0 on opp side upon activating, or if otherwise unable to attack | 16-15 |
| `<Delay>` | Optional | Option in battle area | Trash this card to activate the linked effect; optional; not the turn it entered the battle area; no cost/color needed | 16-16 |
| `<Decoy (X)>` | Opt-cost→Mand | Immediate; another of your (X) Digimon would be deleted by opp effect | *By deleting this Digimon* (optional), prevent that deletion (then mandatory); only one of simultaneous deletions; can't if this is itself deleted | 16-17 |
| `<Armor Purge>` | Opt-cost→Mand | Immediate; this would be deleted | *By trashing the top card of this Digimon* (optional), prevent the deletion (then mandatory); needs ≥1 digivolution card; surviving modifiers carry over | 16-18 |
| `<Save>` | Optional | On effect | May place this card under 1 of your Tamers; optional | 16-19 |
| `<Material Save x>` | Optional | Immediate; this deleted | May place x of the top card's DigiXros-requirement cards from its digivolution cards under 1 of your Tamers; optional, but if processed must place the number whenever possible | 16-20 |
| `<Evade>` | Opt-cost→Mand | Immediate; this would be deleted | *By suspending this Digimon* (optional), prevent the deletion (then mandatory) | 16-21 |
| `<Raid>` | Optional | Trigger; when this attacks | May switch the attack target to the opponent's unsuspended Digimon with the highest DP; optional; attacker chooses if tied | 16-22 |
| `<Alliance>` | Opt-cost→Mand | Trigger; when this attacks | *By suspending 1 of your other Digimon* (optional), add its DP to the attacker and gain `<Security A. +1>` for the attack (then mandatory); added DP/SA persist even if the suspended Digimon leaves | 16-23 |
| `<Barrier>` | Opt-cost→Mand | Immediate; this would be deleted in battle | *By trashing the top card of your security stack* (optional), prevent that deletion (then mandatory) | 16-24 |
| `<Blast Digivolve>` | Optional | On effect | May digivolve into this card in hand without paying the cost; optional; digivolves the chosen battle-area Digimon | 16-25 |
| `<Fortitude>` | Mandatory | Trigger; this (with digivolution cards) deleted | Play this Digimon (from trash) without paying the cost; mandatory | 16-26 |
| `<Mind Link>` | Mandatory | On effect | Place a Tamer with this effect into a Tamer-less Digimon's digivolution cards; mandatory (but `[Main]`-timing instances may be timed by the player) | 16-27 |
| `<Partition (…)>` | Optional | Immediate; this + 1 of each specified card in its digivolution cards would be removed (not by your effect/battle) | May play 1 of each specified card from the digivolution cards without paying costs; optional; all-or-nothing across the specified set | 16-28 |
| `<Collision>` | Persistent | While this is attacking | All opponent's Digimon gain `<Blocker>` and the opponent is forced to block whenever possible | 16-29 |
| `<Blast DNA Digivolve (…)>` | Optional | On effect | A specified Digimon + a hand card may DNA-digivolve into this card in hand without paying cost; optional | 16-30 |
| `<Scapegoat>` | Opt-cost→Mand | Immediate; this would be deleted (not by your effect) | *By deleting 1 of your other Digimon* (optional), prevent the deletion (then mandatory) | 16-31 |
| `<Vortex>` | Optional | Trigger; end of your turn | May attack an opponent's Digimon at end of turn; optional; also lets it attack the turn it was played | 16-32 |
| `<Overclock (…)>` | Opt-cost→Mand | Trigger; end of your turn | *By deleting 1 of your Tokens / other specified Digimon* (optional), this may attack a player without suspending (then mandatory) | 16-33 |
| `<Iceclad>` | Persistent | Battle (not vs Security Digimon) | Compare number of digivolution cards instead of DP; higher count wins, tie = both lose | 16-34 |
| `<Decode (…)>` | Optional | Immediate; this would leave the battle area (not by a battle) | May play 1 specified Digimon card from its digivolution cards without paying the cost; optional | 16-35 |
| `<Fragment (X)>` | Opt-cost→Mand | Immediate; this would be deleted | *By choosing & trashing X of this Digimon's digivolution cards* (optional), it isn't deleted (then mandatory) | 16-36 |
| `<Execute>` | Optional | Trigger; end of your turn | May attack (incl. an opponent's unsuspended Digimon); at end of that attack this Digimon is deleted (pending processing); optional | 16-37 |
| `<Progress>` | Persistent | While this is attacking | This Digimon isn't affected by your opponent's effects while attacking | 16-38 |
| `<Link +x>` | Persistent | — | Adds x to this Digimon's maximum link cards; multiples add, but aren't a single combined-value effect | 16-39 |
| `<Training>` | Opt-cost→Mand | Activation; main phase (also in breeding area) | *By suspending this Digimon during the main phase* (optional), place the top deck card at the bottom of this Digimon's digivolution cards (then mandatory) | 16-40 |

## Notes
- §16-1/16-2: keyword effects are the bracketed-icon effects; the same keyword is the same effect type even with a different numeric value or named card in the icon.
- **Not §16 keywords** (defined elsewhere, listed here so they aren't mistaken for §16 keywords): `[DigiXros]` / `[Assembly]` alt-play with material (§1-3-11), DNA Digivolution (§8 / glossary "Actions"), `[Counter]` and other effect *timings* (§15-16). Look these up via their own sections, not §16.
- Optional-vs-mandatory is the single most error-prone axis for faithful implementation (rule 17 / no-approximations): expose the optional choice to the action space; never auto-resolve an `Optional` or the cost half of an `Opt-cost→Mand` keyword.
