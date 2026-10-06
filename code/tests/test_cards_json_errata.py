"""Data guard: data/cards.json carries Bandai's card errata.

Bandai lists its errata at https://world.digimoncard.com/rule/errata_card/ : "The Card Errata
'After' text is applied to all game formats and takes precedence over the original wording of
the card." Production builds CardData from data/cards.json, whose digimoncard.io source serves
many errata'd cards in their original wording, and the official DB mirror
(data/card_official.json) is itself pre-errata for several (ST10-06, EX2-028, EX3-028, LM-013,
P-060, ...), so neither can be trusted to carry an errata. ERRATA records every errata on that
page (93 entries, 2020-11-27 to 2026-05-29, read 2026-10-05; the corrections are in
code/tools/archive/apply_card_errata_2026_10.py).

Each text errata names a cards.json field. That field must contain the After text, and no text
field of the card may contain the Before text. The comparison ignores what cards.json renders
differently from the page: keyword brackets (<Rush>, ⟨Rush⟩, ＜Rush＞), the reminder text
cards.json appends to a keyword (unless the errata text itself carries one), case, commas and
full stops, curly quotes, bullets, the "…" around a quoted fragment, and spacing (line breaks,
"[On Play][When Digivolving]"); words still have to match whole. The page's section labels
("Effect", "Inherited Effect", an Option face's name) are given as the field instead.

Where cards.json holds Bandai's card-list wording of the same post-errata text, or the page
misprints the After text, a sixth element gives the wording cards.json must hold instead, with
the reason; the After text stays beside it as the record. A new errata is one more entry.
"""
import json
import os
import re

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.abspath(os.path.join(_HERE, "..", ".."))

NAME = "card_name_eng"
EFFECT = "effect_description_eng"
# Tamers' and Options' [Security] text sits here too (the API's convention).
INHERITED = "inherited_effect_description_eng"
XROS = "xros_req"
OPTION = "dual.option.effect_text"  # a DUAL card's Option face

_TEXT_FIELDS = (EFFECT, INHERITED, "security_effect_description_eng", XROS,
                "dual.digimon.effect_text", "dual.digimon.inherited_text", OPTION,
                "dual.option.security_text")

# (date, card, field, After text, Before text[, what cards.json holds instead])
ERRATA = [
    ("2026-05-29", "EX4-072", EFFECT,
     "[Main] Choose 1 of your [Gallantmon], [Sakuyamon] or [MegaGargomon]. From your hand, "
     "ignoring digivolution requirements and without paying the cost, it may digivolve into a "
     "level 6 Digimon card with a different name that includes the chosen Digimon's name.",
     "[Main] Choose 1 of your level 6 Digimon. Ignoring digivolution requirements and without "
     "paying the cost, it may digivolve into a level 6 Digimon card in your hand with a "
     "different name that includes the chosen Digimon's name."),
    ("2026-05-15", "BT25-057", OPTION,
     "[Main] 1 of your Digimon gains <Rush>, <Security A. +1> and +5000 DP for the turn. Then, "
     "it may attack.",
     "[Main] 1 of your Digimon gains <Rush>, <Security A. +1> and +5000 DP until your "
     "opponent's turn ends. Then, it may attack."),
    ("2025-10-17", "BT23-078", NAME,
     "Gorou Matayoshi",
     "Goro Matayoshi"),
    ("2025-08-01", "ST10-06", EFFECT,
     "[When Digivolving] Place 1 yellow or purple Digimon card from your trash on top of your "
     "security stack face down. When DNA digivolving, search your security stack, and you may "
     "play 1 level 5 or lower Digimon card among it without paying its cost.\n"
     "Then, shuffle your security stack.",
     "[When Digivolving] Place 1 yellow or purple Digimon card from your trash on top of your "
     "security stack face down. When DNA digivolving, you may search your security stack for 1 "
     "level 5 or lower Digimon card and play it without paying its memory cost. Then, shuffle "
     "your security stack."),
    ("2025-08-01", "EX2-028", EFFECT,
     "[End of Attack] You may place this Digimon as 1 of your other Digimon's bottom "
     "digivolution card.",
     "[End of Attack] You may place this Digimon under 1 of your Digimon as its bottom "
     "digivolution card."),
    ("2025-06-03", "EX9-021", EFFECT,
     "[When Digivolving] If DNA digivolving, your opponent's effects don't affect this Digimon "
     "for the turn. Then, delete all of their Digimon with the highest level.",
     "[When Digivolving] If DNA digivolving, your opponent's Digimon's effects don't affect "
     "this Digimon for the turn. Then, delete all of their Digimon with the highest level."),
    ("2025-04-25", "EX3-069", EFFECT,
     "at the next end of your opponent’s turn",
     "at the end of your opponent’s turn"),
    ("2025-04-25", "EX4-058", EFFECT,
     "at the next end of your opponent’s turn",
     "at the end of your opponent’s turn"),
    ("2025-04-25", "EX4-063", EFFECT,
     "at the next end of your opponent’s turn",
     "at the end of your opponent’s turn"),
    ("2025-04-25", "EX4-071", EFFECT,
     "at the next end of your opponent’s turn",
     "at the end of your opponent’s turn"),
    ("2025-04-25", "BT13-089", EFFECT,
     "at the next end of your opponent’s turn",
     "at the end of your opponent’s turn"),
    ("2025-04-25", "LM-013", EFFECT,
     "at the next end of your opponent’s turn",
     "at the end of your opponent’s turn"),
    ("2025-04-18", "BT20-095", EFFECT,
     "[All Turns] When any of your [Chronicle] trait Digimon are deleted, ⟨Delay⟩.\n"
     "By moving your level 3 or higher Digimon from the breeding area to the battle area…",
     "[All Turns] When any of your [Chronicle] trait Digimon are deleted, ⟨Delay⟩.\n"
     "By moving your level 3 or higher [Chronicle] trait Digimon from the breeding area to the "
     "battle area…"),
    ("2025-04-18", "ST21-04", EFFECT,
     "[On Play] [When Digivolving] From 1 of your opponent's Digimon, trash any 1 digivolution "
     "card for every 2 colors your Tamers have. Then, return 1 of their Digimon with 1 or fewer "
     "digivolution cards to the hand.",
     "[On Play] [When Digivolving] For every 2 colors your Tamers have, trash any 1 of your "
     "opponent's Digimon's digivolution cards. Then, return 1 of their Digimon with 1 or fewer "
     "digivolution cards to the hand."),
    ("2025-03-07", "BT20-098", EFFECT,
     "[Main] By returning 9 levels' total worth of Digimon cards from your opponent's trash to "
     "the bottom of the deck, you may play 1 [Ghost] trait Digimon card of each returned card's "
     "level from your trash without paying the costs.",
     "[Main] By returning up to 9 levels' total worth of Digimon cards from your opponent's "
     "trash to the bottom of the deck, you may play 1 [Ghost] trait Digimon card of each "
     "returned card's level from your trash without paying the costs."),
    ("2025-02-21", "BT19-091", EFFECT,
     "While you have a level 5 [WarGrowlmon], [Taomon] or [Rapidmon], you may ignore this "
     "card's color requirements.",
     "“While you have a level 5 [WarGrowlmon], [Taomon] or [Rapidmon], you may ignore this "
     "card's color requirements."),
    ("2025-02-21", "BT20-077", EFFECT,
     "[On Play] [When Digivolving] Trash cards in your hand until it has 4 left. Then, play 1 "
     "8000 DP or lower Digimon card from your trash without paying the cost. For each card this "
     "effect trashed, remove 2000 from this effect's DP maximum.",
     "[On Play] [When Digivolving] Trash cards in your hand until it has 4 left. Then, play 1 "
     "8000 DP or lower Digimon card from your trash. For each card this effect trashed, remove "
     "2000 from this effect's DP maximum."),
    ("2024-11-08", "BT18-099", EFFECT,
     "[All Turns] When attack targets change, ⟨Delay⟩.\n"
     "・1 of your Digimon gains ⟨Piercing⟩ and ⟨Security A. +1⟩ until the end of your turn.",
     "[All Turns] When attack targets change, ⟨Delay⟩.\n"
     "・1 of your Digimon gains ⟨Piercing⟩ and ⟨Security A. +1⟩ the end of your turn."),
    ("2024-09-13", "ST19-08", EFFECT,
     "⟨Overclock ([Puppet] Trait)⟩ (At the end of your turn, by deleting 1 of your Tokens or "
     "other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)",
     "⟨Overclock ([Puppet] Trait)⟩ (At the end of your turn, by deleting 1 of your Tokens or "
     "other [Puppet] trait Digimon, this Digimon may attack a player without suspending.)"),
    ("2024-09-13", "ST19-12", EFFECT,
     "⟨Overclock ([Puppet] Trait)⟩ (At the end of your turn, by deleting 1 of your Tokens or "
     "other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)",
     "⟨Overclock ([Puppet] Trait)⟩ (At the end of your turn, by deleting 1 of your Tokens or "
     "other [Puppet] trait Digimon, this Digimon may attack a player without suspending.)"),
    ("2024-09-13", "EX7-027", EFFECT,
     "⟨Overclock ([Puppet] Trait)⟩ (At the end of your turn, by deleting 1 of your Tokens or "
     "other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)",
     "⟨Overclock ([Puppet] Trait)⟩ (At the end of your turn, by deleting 1 of your Tokens or "
     "other [Puppet] trait Digimon, this Digimon may attack a player without suspending.)"),
    ("2024-09-13", "EX7-030", EFFECT,
     "⟨Overclock ([Puppet] Trait)⟩ (At the end of your turn, by deleting 1 of your Tokens or "
     "other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)",
     "⟨Overclock ([Puppet] Trait)⟩ (At the end of your turn, by deleting 1 of your Tokens or "
     "other [Puppet] trait Digimon, this Digimon may attack a player without suspending.)"),
    ("2024-07-05", "P-123", EFFECT,
     "[Your Turn] [Once Per Turn] When one of your Digimon moves from the breeding area to the "
     "battle area, you may hatch in your breeding area. Then, gain 1 memory.",
     "[Your Turn] [Once Per Turn] When one of your Digimon moves from the breeding area to the "
     "battle area. You may hatch in the breeding area. Then, gain 1 memory."),
    ("2024-05-31", "BT16-060", EFFECT,
     "[On Play] [When Digivolving] Reveal the top 3 cards of your deck. For each [D-Brigade] or "
     "[DigiPolice] trait card among them, reduce the play costs of all of your opponent's "
     "Digimon by 1 for the turn.",
     "[On Play] [When Digivolving] Reveal the top 3 cards of your deck. For each [D-Brigade] or "
     "[DigiPolice] trait card among them, reduce the play costs of all of your opponent's "
     "Digimon by 1 until the end of their turn."),
    ("2024-05-17", "BT16-077", EFFECT,
     "⟨Raid⟩ ⟨Partition (purple Lv.4 & red Lv.4)⟩\n"
     "[When Digivolving] If DNA digivolving, you may play 1 level 5 or lower Digimon card with "
     "the [Free] trait from your trash without paying the cost. Then, 1 of your Digimon may "
     "gain ⟨Rush⟩ for the turn and attack a player.",
     "⟨Raid⟩ ⟨Partition (purple Lv.4 & red Lv.4)⟩\n"
     "[When Digivolving] If DNA digivolving, you may play 1 level 5 or lower Digimon card with "
     "the [Free] trait from your trash without paying the cost.Then, 1 of your Digimon gains "
     "⟨Rush⟩ for the turn and may attack a player."),
    ("2024-05-17", "BT16-091", EFFECT,
     "[Main] You may play 1 [Aquilamon] or [Gatomon] from your hand without paying the cost. "
     "Then, 2 of your Digimon may DNA digivolve into a Digimon card in your hand. The Digimon "
     "this effect DNA digivolved may gain ⟨Security A. +1⟩ for the turn and attack a player.",
     "[Main] You may play 1 [Aquilamon] or [Gatomon] from your hand without paying the cost. "
     "Then, 2 of your Digimon may DNA digivolve into a Digimon card in your hand. The Digimon "
     "this effect DNA digivolved gains ⟨Security A. +1⟩ for the turn and may attack a player."),
    ("2024-05-17", "P-115", EFFECT,
     "[On Deletion] You may play 1 Tamer card with [Nene Amano]/[Yuu Amano] in its name from "
     "your hand or trash without paying the cost. Then, ⟨Save⟩.",
     "[On Deletion] You may play 1 Tamer card with [Amano] in its name from your hand or trash "
     "without paying the cost. Then, ⟨Save⟩.",
     # Bandai's card list wording
     "[On Deletion] You may play 1 Tamer card with [Nene Amano] or [Yuu Amano] in its name from "
     "your hand or trash without paying the cost. Then, <Save>"),
    ("2024-05-10", "BT4-031", NAME,
     "MarineChimairamon",
     "MarinChimairamon"),
    ("2024-03-08", "EX4-063", EFFECT,
     "[Your Turn] When one of your Digimon with [Terriermon] or [Lopmon] in its digivolution "
     "cards would digivolve, by suspending this Tamer, reduce the digivolution cost by 1.",
     "[Your Turn] When one of your Digimon with [Terriermon] or [Lopmon] in its name would "
     "digivolve, by suspending this Tamer, reduce the digivolution cost by 1."),
    ("2024-02-09", "BT12-037", EFFECT,
     "[On Play][When Digivolving] Reveal the top 3 cards of your deck. You may play 1 [Airu "
     "Suzaki], [Ren Tobari], or [Ryoma Mogami] card among them without paying its cost. Place "
     "the remaining cards at the bottom of your deck in any order. [On Deletion] ＜Save＞.Then, "
     "place 1 Digimon card with ＜Save＞ in its text from your trash under 1 of your Tamers.",
     "[On Play][When Digivolving] Reveal the top 3 cards of your deck. You may play 1 [Airu "
     "Suzaki], [Ren Tobari], or [Ryouma Mogami] card among them without paying its cost. Place "
     "the remaining cards at the bottom of your deck in any order.[On Deletion] ＜Save＞.Then, "
     "place 1 Digimon card with ＜Save＞ in its text from your trash under 1 of your Tamers."),
    ("2024-02-09", "BT12-051", EFFECT,
     "[On Play][When Digivolving] You may play 1 [Airu Suzaki], [Ren Tobari], or [Ryoma Mogami] "
     "card from your hand without paying its cost.[On Deletion] ＜Save＞.\n"
     "Then, place 1 Digimon card with ＜Save＞ in its text from your trash under 1 of your "
     "Tamers.",
     "[On Play][When Digivolving] You may play 1 [Airu Suzaki], [Ren Tobari], or [Ryouma "
     "Mogami] card from your hand without paying its cost.[On Deletion] ＜Save＞.\n"
     "Then, place 1 Digimon card with ＜Save＞ in its text from your trash under 1 of your "
     "Tamers."),
    ("2024-01-19", "BT9-071", EFFECT,
     "[On Play] Reveal the top 3 cards of your deck. Among them, add 1 card with [Undead] or "
     "[Dark Animal] in one of its traits to the hand and trash 1 such card. Return the rest to "
     "the bottom of the deck.",
     "[On Play] Reveal the top 3 cards of your deck. Trash 1 of them, and add 1 card with "
     "[Undead] or [Dark Animal] in its traits among them to your hand. Place the rest at the "
     "bottom of your deck in any order."),
    ("2023-12-22", "BT14-023", EFFECT,
     "[When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon. [When "
     "Attacking] (Once Per Turn) Until the end of your opponent's turn, 1 of your opponent's "
     "Digimon with as many or fewer digivolution cards as this digimon can't attack.",
     "[When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon. [When "
     "Attacking] (Once Per Turn) Until the end of your opponent's turn, 1 of your opponent's "
     "Digimon with fewer digivolution cards than this digimon can't attack.",
     # the page prints (Once Per Turn); the card, [Once Per Turn]
     "[When Digivolving] Trash any 2 digivolution cards from your opponent's Digimon. [When "
     "Attacking] [Once Per Turn] Until the end of your opponent's turn, 1 of your opponent's "
     "Digimon with as many or fewer digivolution cards as this Digimon can't attack."),
    ("2023-12-22", "BT14-023", INHERITED,
     "[When Attacking][Once Per Turn] Until the end of your opponent's turn, 1 of your "
     "opponent's Digimon with as many or fewer digivolution cards as this digimon can't attack.",
     "[When Attacking][Once Per Turn] Until the end of your opponent's turn, 1 of your "
     "opponent's Digimon with fewer digivolution cards than this digimon can't attack."),
    ("2023-12-22", "BT14-029", EFFECT,
     "[When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon.\n"
     "[When Attacking][Once Per Turn] If your opponent has no Digimon with as many or more "
     "digivolution cards as this Digimon, unsuspend this Digimon.",
     "[When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon.\n"
     "[When Attacking][Once Per Turn] If your opponent has no Digimon with more digivolution "
     "cards than this Digimon, unsuspend this Digimon."),
    ("2023-12-22", "BT14-091", EFFECT,
     "[Main] Trash any 2 digivolution cards from your opponent's Digimon. Then if you have a "
     "Tamer with [Joe Kido] in its name, choose 1 of your Digimon. If your opponent has no "
     "Digimon with as many or more digivolution cards as the chosen Digimon, unsuspend it.",
     "[Main] Trash any 2 digivolution cards from your opponent's Digimon. Then, if you have a "
     "Tamer with [Joe Kido] in its name, choose 1 of your Digimon. If your opponent has no "
     "Digimon with more digivolution cards than the chosen Digimon, unsuspend it."),
    ("2023-12-15", "BT14-002", INHERITED,
     "[Your Turn] While your opponent has no Digimon with as many or more digivolution cards as "
     "this Digimon, this Digimon gains <Jamming>.",
     "[Your Turn] While your opponent has no Digimon with more digivolution cards than this "
     "Digimon, this Digimon gains <Jamming>."),
    ("2023-09-29", "ST16-11", INHERITED,
     "[When Attacking][Once Per Turn] By trashing 1 card in your hand, delete 1 of your "
     "opponent's level 4 or lower Digimon.",
     "[When Attacking][Once Per Turn] By trashing 1 card in your hand, unsuspend this Digimon"),
    ("2023-04-14", "BT12-094", EFFECT,
     "Start of Your Main Phase] By placing 1 Digimon card with ＜Save＞ in its text from your "
     "hand under this Tamer, gain 1 memory.",
     "Start of Your Main Phase] By placing 1 Digimon card with ＜Save＞ in its text from your "
     "hand under this Tamer, 1 of your Digimon gets +2000 DP for the turn."),
    ("2023-04-14", "P-071", EFFECT,
     "[Security] At the end of the battle, you may play 1 purple level 3 Digimon card from your "
     "trash without paying its memory cost. Then, add this card to its owner's hand.",
     "[Security] At the end of the battle, you may play 1 purple level 3 Digimon card from your "
     "trash without paying its memory cost."),
    ("2023-02-17", "BT11-009", EFFECT,
     "＜Material Save 1＞ (When this Digimon would be deleted, you may place 1 card in this "
     "Digimon's DigiXros requirements from this Digimon's digivolution cards under 1 of your "
     "Tamers.)",
     "＜Material Save 1＞ (When this Digimon would be deleted, you may place 2 cards in this "
     "Digimon's DigiXros requirements from this Digimon's digivolution cards under 1 of your "
     "Tamers.)"),
    ("2022-11-11", "EX3-001", INHERITED,
     "[All Turns] [Once Per Turn] When this Digimon with [Dramon] or [Examon] in its name "
     "becomes unsuspended, this Digimon gets +1000 for the turn.",
     "[All Turns][Once Per Turn] When a Digimon with [Dramon] or [Examon] in its name becomes "
     "unsuspended, this Digimon gets +1000 DP for the turn.",
     # the page drops DP; the post-errata card prints it
     "[All Turns] [Once Per Turn] When this Digimon with [Dramon] or [Examon] in its name "
     "becomes unsuspended, this Digimon gets +1000 DP for the turn."),
    ("2022-11-11", "EX3-003", EFFECT,
     "[When Attacking] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Dragon], "
     "[saur] or [Ceratopsian] in one of its traits among them to your hand. Place the rest at "
     "the bottom of your deck in any order.",
     "[When Attacking] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Dragon] in "
     "its traits among them to your hand. Place the rest at the bottom of your deck in any "
     "order."),
    ("2022-11-11", "EX3-008", EFFECT,
     "・You may digivolve 1 of your other Digimon into a level 4 purple Digimon card with the "
     "[Free] trait from your trash for the cost.\n"
     "・You may DNA digivolve this Digimon and one of your other Digimon may DNA digivolve into "
     "a Digimon card in your hand for the cost.",
     "・You may digivolve 1 of other Digimon into a level 4 purple Digimon card with [Free] in "
     "its traits from your trash for its play cost.\n"
     "・You may DNA digivolve this Digimon and one of your other Digimon in play into a Digimon "
     "card in your hand for its DNA digivolve cost.",
     # the After text garbles the second bullet; Bandai's card list wording
     "[When Digivolving] Activate 1 of the effects below.\n"
     "・1 of your other Digimon may digivolve into a level 4 purple Digimon card with [Free] in "
     "its traits from your trash for the cost.\n"
     "・This Digimon and one of your other Digimon may DNA digivolve into a Digimon card in your "
     "hand for the cost."),
    ("2022-11-11", "EX3-014", EFFECT,
     "[On Play] Delete 1 of your opponent's Digimon with 3000 or less. For each card with "
     "[Dragon], [saur] or [Ceratopsian] in one of its traits in this Digimon's digivolution "
     "cards, add 2000 DP to the maximum DP you can choose with this effect.",
     "[On Play] Delete 1 of your opponent's Digimon with 3000 DP or less. For each card with "
     "[Dragon] in its traits in this Digimon's digivolution cards, add 2000 DP to the maximum "
     "DP you can choose with this effect.",
     # the page's "3000 or less" and "add 2000 DP to the maximum DP" misprint the card
     "[On Play] Delete 1 of your opponent's Digimon with 3000 DP or less. For each card with "
     "[Dragon], [saur], or [Ceratopsian] in one of its traits in this Digimon's digivolution "
     "cards, add 2000 to the maximum DP you can choose with this effect."),
    ("2022-11-11", "EX3-014", XROS,
     "[DigiXros -2] 5 Digimon cards w/different names + [Dragon], [saur], or [Ceratopsian] in "
     "one of its traits\n"
     "On play, place from hand/battle area under this card. Reduce play cost per card.",
     "[DigiXros -2] 5 Digimon cards w/different names + [Dragon] traits\n"
     "On play, place from hand/battle area under this card. Reduce play cost per card.",
     # Bandai's card list wording
     "5 Digimon cards w/different names and [Dragon], [saur], or [Ceratopsian] in one of their "
     "traits"),
    ("2022-11-11", "EX3-022", EFFECT,
     "[When Attacking] You may play 1 blue level 3 Digimon card from 1 of your blue Digimon's "
     "digivolution cards without paying its memory cost.",
     "[When Attacking] You may play 1 blue level 3 Digimon card from one of your blue Digimon's "
     "digivolution cards without paying its memory cost.",
     # Bandai's card list wording
     "[When Attacking] You may play 1 blue level 3 Digimon card from 1 of your blue Digimon's "
     "digivolution cards without paying the cost."),
    ("2022-11-11", "EX3-022", INHERITED,
     "[When Attacking] [Once Per Turn] You may play 1 blue level 3 Digimon card from 1 of your "
     "blue Digimon's digivolution cards without paying its memory cost.",
     "[When Attacking][Once Per Turn] You may play a blue level 3 Digimon card from 1 of your "
     "blue Digimon without paying its memory cost.",
     # Bandai's card list wording
     "[When Attacking] [Once Per Turn] You may play a blue level 3 Digimon card from 1 of your "
     "blue Digimon's digivolution cards without paying the cost."),
    ("2022-11-11", "EX3-023", EFFECT,
     "[When Digivolving] You may play 1 blue level 3 Digimon card or 1 level 4 or lower Digimon "
     "card with [Aqua] or [Sea Animal] in one of its traits from one of your blue Digimon's "
     "digivolution cards without paying its memory cost. Then, you may place 1 blue Digimon "
     "card from your hand under this Digimon as its bottom digivolution card.",
     "[When Digivolving] You may play 1 blue level 3 Digimon card or 1 level 4 or lower Digimon "
     "card with [Aquatic] in its traits from one of your blue Digimon's digivolution cards "
     "without paying its memory cost. Then, you may place 1 blue Digimon card from your hand "
     "under this Digimon as its bottom digivolution card."),
    ("2022-11-11", "EX3-023", INHERITED,
     "[All Turns] [Once Per Turn] When you play a Digimon from digivolution cards, you may "
     "return 1 of your opponent's Digimon of the same level to the bottom of its owner's deck.",
     "[All Turns][Once Per Turn] When you play a Digimon from digivolution cards, return 1 of "
     "your opponent's Digimon of the same level to the bottom of its owner's deck."),
    ("2022-11-11", "EX3-024", EFFECT,
     "[Start of Opponent's Main Phase] By suspending 1 of your Digimon with [Dramon] or "
     "[Examon] in its name, your opponent attacks with 1 of their Digimon.",
     "[Start of Opponent's Main Phase] You may suspend 1 of your Digimon with [Dramon] or "
     "[Examon] in its name to force 1 of your opponent’s Digimon to attack."),
    ("2022-11-11", "EX3-024", INHERITED,
     "[Start of Opponent's Main Phase] By suspending 1 of your Digimon with [Dramon] or "
     "[Examon] in its name, your opponent attacks with 1 of their Digimon.",
     "[Start of Opponent's Main Phase] You may suspend 1 of your Digimon with [Dramon] or "
     "[Examon] in its name to force 1 of your opponent’s Digimon to attack."),
    ("2022-11-11", "EX3-025", EFFECT,
     "[On Deletion] If you don't have a [Trial of the Four Great Dragons] in play, you may "
     "place 1 [Trial of the Four Great Dragons] from your hand in your battle area.",
     "[On Deletion] If you don't have a [Trial of the Four Great Dragons] in play, place 1 "
     "[Trial of the Four Great Dragons] from your hand in your battle area."),
    ("2022-11-11", "EX3-026", EFFECT,
     "[When Digivolving] You may play 1 blue level 3 Digimon card or 1 Digimon card with "
     "[Seadramon] in its name or [Aqua] or [Sea Animal] in one of its traits from one of your "
     "blue Digimon's digivolution cards without paying its memory cost.",
     "[When Digivolving] You may play 1 blue level 3 Digimon card or 1 Digimon card with "
     "[Seadramon] in its name or [Aquatic] in its traits from one of your blue Digimon's "
     "digivolution cards without paying its memory cost."),
    ("2022-11-11", "EX3-028", EFFECT,
     "[On Play] Reveal the top 4 cards of your deck. Add 1 yellow card with [Angel], [Cherub], "
     "[Throne], [Authority], [Seraph] or [Virtue], other than [Three Great Angels], in one of "
     "its traits and 1 card with the [Four Great Dragons] trait among them to your hand. Place "
     "the rest at the bottom of your deck in any order.",
     "[On Play] Reveal the top 4 cards of your deck. Add 1 yellow card with [Angel] in its "
     "traits and 1 card with the [Four Great Dragons] in its traits among them to your hand. "
     "Place the rest at the bottom of your deck in any order."),
    ("2022-11-11", "EX3-030", EFFECT,
     "[On Play] Reveal the top 4 cards of your deck. Add 1 yellow card with [Angel], [Cherub], "
     "[Throne], [Authority], [Seraph] or [Virtue], other than [Three Great Angels], in one of "
     "its traits and 1 card with the [Four Great Dragons] trait among them to your hand. Place "
     "the rest at the bottom of your deck in any order.",
     "[On Play] Reveal the top 4 cards of your deck. Add 1 yellow card with [Angel] in its "
     "traits and 1 card with the [Four Great Dragons] in its traits among them to your hand. "
     "Place the rest at the bottom of your deck in any order."),
    ("2022-11-11", "EX3-030", INHERITED,
     "[Your Turn] [Once Per Turn] When you play a Digimon with the [Four Great Dragons] trait, "
     "1 of those Digimon gains ＜Rush＞ for the turn. (This Digimon may attack the turn it was "
     "played.)",
     "[Your Turn][Once Per Turn] When you play a Digimon with [Four Great Dragons] in its "
     "traits, it gains ＜Rush＞ for the turn. (This Digimon may attack the turn it was played.)",
     # cards.json's own <Rush> reminder text follows
     "[Your Turn] [Once Per Turn] When you play a Digimon with the [Four Great Dragons] trait, "
     "1 of those Digimon gains <Rush> for the turn."),
    ("2022-11-11", "EX3-031", INHERITED,
     "[Your Turn] [Once Per Turn] When you play a Digimon with the [Four Great Dragons] trait, "
     "1 of those Digimon gains <Rush> for the turn. (This Digimon may attack the turn it was "
     "played.)",
     "[Your Turn][Once Per Turn] When you play a Digimon with [Four Great Dragons] in its "
     "traits, it gains gains <Rush> for the turn. (This Digimon may attack the turn it was "
     "played.)",
     # cards.json's own <Rush> reminder text follows
     "[Your Turn] [Once Per Turn] When you play a Digimon with the [Four Great Dragons] trait, "
     "1 of those Digimon gains <Rush> for the turn."),
    ("2022-11-11", "EX3-033", EFFECT,
     "[When Digivolving] If you don't have a [Trial of the Four Great Dragons] in play, you may "
     "place 1 [Trial of the Four Great Dragons] from your hand in your battle area.",
     "[When Digivolving] If you don't have a [Trial of the Four Great Dragons] in play, place 1 "
     "[Trial of the Four Great Dragons] from your hand in your battle area."),
    ("2022-11-11", "EX3-034", EFFECT,
     "[When Digivolving] If you don't have a [Trial of the Four Great Dragons] in play, you may "
     "place 1 [Trial of the Four Great Dragons] from your hand in your battle area.",
     "[When Digivolving] If you don't have a [Trial of the Four Great Dragons] in play, place 1 "
     "[Trial of the Four Great Dragons] from your hand in your battle area."),
    ("2022-11-11", "EX3-035", EFFECT,
     "[When Digivolving] You may return 1 card with the [Four Great Dragons] trait from your "
     "trash to your hand.\n"
     "[When Attacking] 1 of your opponent's Digimon gets -6000 for the turn. Then, by returning "
     "1 [Magnadramon], 1 [Azulongmon], and 1 [Megidramon] from your trash to the bottom of "
     "your deck in any order, trash the top 2 cards of your opponent's security stack.",
     "[When Digivolving] Return 1 card with [Four Great Dragons] in its trait from your trash "
     "to your hand.\n"
     "[When Attacking] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, you may "
     "return 1 [Magnadramon], 1 [Azulongmon], and 1 [Megidramon] from your trash to the bottom "
     "of your deck in any order to trash the top 2 cards of your opponent's security stack."),
    ("2022-11-11", "EX3-036", EFFECT,
     "[On Deletion] If you don't have a [Trial of the Four Great Dragons] in play, you may "
     "place 1 [Trial of the Four Great Dragons] from your hand in your battle area.",
     "[On Deletion] If you don't have a [Trial of the Four Great Dragons] in play, place 1 "
     "[Trial of the Four Great Dragons] from your hand in your battle area."),
    ("2022-11-11", "EX3-045", EFFECT,
     "[When Digivolving] You may suspend 1 Digimon.\n"
     "[All Turns] [Once Per Turn] When an opponent's Digimon becomes suspended, for each other "
     "suspended Digimon with [Vegetation], [Plant], or [Fairy] in one of their traits you have "
     "in play, gain 1 memory.\n"
     "[End of Your Turn] [Once Per turn] If you have 2 or more suspended Digimon with "
     "[Vegetation], [Plant], or [Fairy] in one of their traits, return 1 of your opponent's "
     "suspended Digimon to the bottom of its owner's deck.",
     "[When Digivolving] You may suspend 1 of you or your opponent’s Digimon.\n"
     "[All Turns][Once Per Turn] When an opponent's Digimon becomes suspended, for each other "
     "suspended Digimon with [Vegetation], [Plant],or [Fairy] in its traits you have in play, "
     "gain 1 memory.\n"
     "[End of Your Turn][Once Per turn] If you have 2 or more suspended Digimon with "
     "[Vegetation], [Plant], or [Fairy] in their traits, place 1 of your opponent's suspended "
     "Digimon at the bottom of its owner's deck."),
    ("2022-11-11", "EX3-055", EFFECT,
     "[On Play] Reveal the top 3 cards of your deck. Add 1 purple or red card with the [Free] "
     "trait or 1 card with [Imperialdramon] in its name among them to your hand, and trash 1 "
     "such card among them. Place the rest at the bottom of your deck in any order.",
     "[On Play] Reveal the top 3 cards of your deck. Add 1 purple or red card with the [Free] "
     "trait or 1 card with [Imperialdramon] in its name among them to your hand, and trash 1 "
     "card among them. Place the rest at the bottom of your deck in any order."),
    ("2022-11-11", "EX3-057", EFFECT,
     "[When Digivolving] Delete 1 of your opponent's Digimon with 3000 DP or less. If no "
     "Digimon is deleted by this effect, trash the top 2 cards of both players' decks",
     "[On Deletion] Delete 1 of your opponent‘s Digimon with 3000 DP or less. If no Digimon was "
     "deleted by this effect, both players trash the top 2 cards of their decks."),
    ("2022-11-11", "EX3-058", EFFECT,
     "[When Digivolving] Activate 1 of the effects below.\n"
     "・You may digivolve 1 of your other Digimon into a level 4 red Digimon card with the "
     "[Free] trait from your trash for the cost.\n"
     "・You may DNA digivolve this Digimon and one of your other Digimon may DNA digivolve into "
     "a Digimon card in your hand for the cost.",
     "[When Digivolving] Activate 1 of the effects below.\n"
     "・You may digivolve 1 of your other Digimon into a red level 4 Digimon card with [Free] in "
     "its traits from your trash for its digivolution cost.\n"
     "・You may DNA digivolve this Digimon and one of your other Digimon into a Digimon card "
     "from your hand for its DNA digivolve cost.",
     # the After text garbles the second bullet; Bandai's card list wording
     "[When Digivolving] Activate 1 of the effects below.\n"
     "・1 of your other Digimon may digivolve into a red level 4 Digimon card with [Free] in its "
     "traits from your trash for the cost.\n"
     "・This Digimon and one of your other Digimon may DNA digivolve into a Digimon card in your "
     "hand for the cost."),
    ("2022-11-11", "EX3-063", EFFECT,
     "[When Digivolving] If DNA digivolving, your opponent chooses 1 of their Digimon. Delete "
     "all of their other Digimon. Then, ＜Blitz＞.",
     "[When Digivolving] When DNA digivolving, your opponent chooses 1 of their Digimon and "
     "deletes the rest. Then, ＜Blitz＞."),
    ("2022-11-11", "EX3-064", EFFECT,
     "[On Deletion] If you don't have a [Trial of the Four Great Dragons] in play, you may "
     "place 1 [Trial of the Four Great Dragons] from your hand in your battle area.",
     "[On Deletion] If you don't have a [Trial of the Four Great Dragons] in play, place 1 "
     "[Trial of the Four Great Dragons] from your hand in your battle area.",
     # Bandai's card list wording
     "[On Deletion] If you don't have [Trial of the Four Great Dragons] in play, you may place "
     "1 [Trial of the Four Great Dragons] from your hand in your battle area."),
    ("2022-11-11", "EX3-068", EFFECT,
     "[Main] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, you may return 1 "
     "card with the [Four Great Dragons] trait from your trash to your hand.",
     "[Main] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, return 1 card with "
     "the [Four Great Dragons] in its traits from your trash to your hand."),
    ("2022-10-28", "BT10-004", INHERITED,
     "[Your Turn][Once Per Turn] When an effect suspends a Digimon, this Digimon gets +1000 DP "
     "for the turn.",
     "[Your Turn]When an effect suspends a Digimon, this Digimon gets +1000 DP for the turn."),
    ("2022-10-28", "BT10-093", EFFECT,
     "[Your turn] [Once per turn] When you would play 1 level 4 or higher Digimon card with "
     "[Bagra Army] in its traits, by placing up to 3 purple Digimon cards from under your "
     "Tamers in the digivolution cards of the Digimon card played, reduce the memory cost of "
     "that Digimon by 2 for each card placed.",
     "[Your turn] [Once per turn] When you would play a level 4 or higher Digimon card with "
     "[Bagra Army] in its traits, by placing up to 3 purple Digimon cards from under your "
     "Tamers in the digivolution cards of the Digimon card played, reduce the memory cost of "
     "that Digimon by 2 for each card placed."),
    ("2022-10-28", "BT10-096", INHERITED,
     "[Security] Reveal the top 3 cards of your deck. You may add 1 Digimon card with [Xros "
     "Heart] in its traits among them to your hand and play 1 [Taiki Kudo] among them without "
     "paying its memory cost. Place the rest at the bottom of your deck in any order.",
     "[Security] Reveal the top 3 cards of your deck. Add 1 card with [Xros Heart] in its "
     "traits among them to your hand and play 1 [Taiki Kudo] among them without paying its "
     "memory cost. Place the rest at the bottom of your deck in any order."),
    ("2022-10-28", "BT10-097", EFFECT,
     "[Main] Reveal the top 6 cards of your deck. You may add 2 cards with [Blue Flare] in "
     "their traits among them to your hand, and play 1 [Kiriha Aonuma] among them without "
     "paying its memory cost. Place the rest at the bottom of your deck in any order. Then, "
     "place this card in your Battle Area.",
     "[Main] Reveal the top 6 cards of your deck. Add 2 cards with [Blue Flare] in their traits "
     "among them to your hand and you may play 1 [ Christopher Aonuma] among them without "
     "paying its memory cost. Place the rest at the bottom of your deck in any order. Then, "
     "place this card in your Battle Area."),
    ("2022-10-14", "BT10-086", EFFECT,
     "When a Digimon with [X Antibody] in its digivolution cards would digivolve into this "
     "card, reduce the digivolution cost by 2.",
     "When a Digimon with [X Antibody] in its traits would digivolve cards would digivolve into "
     "this card, reduce the digivolution cost by 2."),
    ("2022-10-14", "BT10-101", NAME,
     "Lónkhē Adistakto",
     "Lóankhē Adistakto"),
    ("2022-10-14", "BT10-107", INHERITED,
     "[Security] You may play 1 [Yuu Amano] from your hand or trash without paying its play "
     "cost. Then, add this card to its owner's hand.",
     "[Security] You may play 1 black Tamer card from your hand or trash without paying its "
     "play cost. Then, add this card to its owner's hand."),
    ("2022-09-05", "BT9-067", EFFECT,
     "[On Play][When Digivolving] Place 1 [Raijinmon], 1 [Fujinmon], and 1 [Suijinmon] from "
     "your trash under this Digimon in any order as its bottom digivolution cards. Gain 1 "
     "memory for each card placed.",
     "[On Play][When Digivolving] Place 1 [Raijinmon], 1 [Fuijinmon], and 1 [Suijinmon] from "
     "your trash under this Digimon in any order as its bottom digivolution cards. Gain 1 "
     "memory for each card placed."),
    ("2022-09-05", "BT8-097", EFFECT,
     "[Main] Your opponent can't play Digimon by effects until the end of their turn. Delete "
     "all of your opponent's Digimon with 6000 DP or less.",
     "[Main] Delete all of your opponent's Digimon with 6000 DP or less. Your opponent can't "
     "play Digimon by effects until the end of their next turn."),
    ("2022-08-05", "BT1-042", NAME,
     "LoaderLeomon",
     "LoaderLiomon"),
    ("2022-07-01", "EX2-055", EFFECT,
     "When you would play this Digimon, you may trash 7 or more digivolution cards from the "
     "bottom of 1 of your [Mother D-Reaper]s to set this Digimon's play cost to 0.",
     "When you would play this Digimon, you may trash 7 or more digivolution cards from the "
     "bottom of 1 of your [Mother D-Reaper]s to reduce this Digimon's play cost to 0."),
    ("2022-06-24", "EX2-053", EFFECT,
     "[On Play][When Attacking][Once Per Turn] If one of your [Mother D-Reaper]s has 5 or more "
     "digivolution cards, reveal the top 3 cards of your deck. You may play 1 card with "
     "[D-Reaper] in its traits and a play cost of 10 or less among them without paying its "
     "memory cost. Place the remaining cards at the top of your deck in any order.",
     "[On Play][When Attacking][Once Per Turn] If one of your [Mother D-Reaper]s has 5 or more "
     "digivolution cards, reveal the top 3 cards of your deck. You may play 1 card with "
     "[D-Reaper] in its traits and a play cost of 10 or less among them without paying its "
     "memory cost. Place the remaining cards at the bottom of your deck in any order."),
    ("2022-05-27", "BT8-109", EFFECT,
     "[Main] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, you may play 1 "
     "purple or yellow Digimon card with 6000 DP or less from your trash without paying its "
     "memory cost.",
     "[Main] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, play 1 purple or "
     "yellow Digimon card with 6000 DP or less from your trash without paying its memory cost."),
    ("2022-05-27", "BT8-110", INHERITED,
     "[Security] You may play 1 level 3 Digimon card with [Free] in its traits from your hand "
     "or trash without paying its memory cost.",
     "[Security] Play 1 level 3 Digimon card with [Free] in its traits from your hand or trash "
     "without paying its memory cost."),
    ("2022-05-20", "BT8-069", EFFECT,
     "[Your Turn][Once Per Turn] When one of your effects places a digivolution card under one "
     "of your Digimon, this Digimon gets +2000 DP and can't be deleted by your opponent's "
     "effects until the end of your opponent's next turn.",
     "[Your Turn][Once Per Turn] When one of your effects places a digivolution card under this "
     "Digimon, this Digimon gets +2000 DP and can't be deleted by your opponent's effects "
     "until the end of your opponent's next turn."),
    ("2022-05-20", "BT8-070", EFFECT,
     "[All Turns][Once Per Turn] When an opponent's Digimon is deleted, you may unsuspend this "
     "Digimon.",
     "[All Turns][Once Per Turn] When an opponent's Digimon is deleted, unsuspend this Digimon."),
    ("2022-04-22", "P-063", NAME,
     "Ruli Tsukiyono",
     "Ruri Tsukiyono"),
    ("2022-04-22", "P-060", INHERITED,
     "[When Attacking][Once Per Turn] If you have a [Ruli Tsukiyono] in play, gain 1 memory.",
     "[When Attacking][Once Per Turn] If you have a [Ruri Tsukiyono] in play, gain 1 memory."),
    ("2021-12-10", "EX1-053", EFFECT,
     "[Opponent's Turn] For each Digimon card with [Etemon] in its name in your trash, this "
     "Digimon gets +1000 DP.",
     "[Your Turn] For each Digimon card with [Etemon] in its name in your trash, this Digimon "
     "gets +1000 DP."),
    ("2021-11-26", "EX1-073", EFFECT,
     "[On Play] You may place up to 5 level 5 red and/or black cards with [Cyborg] in their "
     "traits and different card numbers from your hand and/or trash in this Digimon's "
     "digivolution cards to gain 1 memory for each card placed.",
     "[On Play] You may place up to 5 red and/or black cards with [Cyborg] in their traits and "
     "different card numbers from your hand and/or trash in this Digimon's digivolution cards "
     "to gain 1 memory for each card placed."),
    ("2021-11-12", "P-045", INHERITED,
     "[All Turns] All of your other Digimon with the same name as this Digimon gain ＜Decoy "
     "(Black/White)＞. (When one of your other black or white Digimon would be deleted by an "
     "opponent's effect, you may delete this Digimon to prevent that deletion.)",
     "[All Turns] All of your Digimon with the same name as this Digimon gain ＜Decoy "
     "(Black/White)＞. (When one of your other black or white Digimon would be deleted, you may "
     "delete this Digimon to prevent it.)",
     # cards.json's current <Decoy> reminder text
     "[All Turns] All of your other Digimon with the same name as this Digimon gain <Decoy "
     "(Black/White)> (When your other Black or White Digimon would be deleted by an opponent's "
     "effect,"),
    ("2021-09-17", "BT4-037", EFFECT,
     "[On Play] You may trash the top card of your security stack to have 1 of your opponent's "
     "Digimon get -2000 DP for the turn.",
     "[On Play] You may trash the top card of your security stack to have 1 of your opponent's "
     "Digimon get -2000 DP."),
    ("2021-09-03", "BT6-059", EFFECT,
     "＜Decoy (Black)＞ (When one of your other black Digimon would be deleted by an opponent's "
     "effect, you may delete this Digimon to prevent that deletion.)",
     "＜Decoy (Black)＞. (When one of your other black Digimon would be deleted, you may delete "
     "this Digimon to prevent it.)",
     # cards.json's current <Decoy> reminder text
     "<Decoy (Black)> (When your other Black Digimon would be deleted by an opponent's effect,"),
    ("2021-09-03", "BT6-064", EFFECT,
     "＜Decoy (Black)＞ (When one of your other black Digimon would be deleted by an opponent's "
     "effect, you may delete this Digimon to prevent that deletion.)",
     "＜Decoy (Black)＞. (When one of your other black Digimon would be deleted, you may delete "
     "this Digimon to prevent it.)",
     # cards.json's current <Decoy> reminder text
     "<Decoy (Black)> (When your other Black Digimon would be deleted by an opponent's effect,"),
    ("2021-05-21", "P-029", INHERITED,
     "[Your Turn] When digivolving this Digimon into an [AncientGreymon] in your hand, reduce "
     "its digivolution cost by 2.",
     "[Your Turn] When digivolving one of your Digimon into an [AncientGreymon] in your hand, "
     "reduce its digivolution cost by 2."),
    ("2021-05-21", "P-030", INHERITED,
     "[Your Turn] When digivolving this Digimon into an [AncientGarurumon] in your hand, reduce "
     "its digivolution cost by 2.",
     "[Your Turn] When digivolving one of your Digimon into an [AncientGarurumon] in your hand, "
     "reduce its digivolution cost by 2."),
    ("2021-05-14", "BT2-097", EFFECT,
     "[Main] 3 of your opponent’s level 3 Digimon get -4000 DP for the turn.",
     "[Main] Up to 3 of your opponent’s level 3 Digimon get -4000 DP for the turn."),
    ("2021-04-23", "BT3-092", EFFECT,
     "[All Turns] When another Digimon is deleted, gain 1 memory for each Digimon deleted.",
     "[All Turns] When another Digimon is deleted, gain 1 memory"),
    ("2021-02-12", "BT3-097", EFFECT,
     "[Main] 1 of your Digimon gains \"This Digimon doesn't activate the [Security] effects of "
     "any Option cards it checks\" for the turn.",
     "[Main] 1 of your Digimon gains “This Digimon doesn‘t activate the [Security] effects of "
     "any cards it checks\" for the turn."),
    ("2020-11-27", "P-012", EFFECT,
     "[Main] If you have a Digimon with [Veedramon] in its name, you may suspend this Tamer to "
     "activate one of the following effects:\n"
     "・Trigger ＜Draw 1＞. (Draw 1 card from your deck.)\n"
     "・1 of your Digimon gets +1000 DP for the turn.",
     "[Main] If you have a Digimon with [Veedramon] in its name, you may suspend this Tamer to "
     "activate one of the following effects:\n"
     "・Activate ＜Draw 1＞. (Draw 1 card from your deck.)\n"
     "・This Digimon gets +1000 DP for the turn."),
]

# Errata whose correction arrives with another change. Each must still be missing here, so the
# merge that brings the correction also drops the card from this set.
PENDING = {
    # The 2026-10 names and traits pass rewrites both cards' text, errata included
    # (code/tools/archive/fix_card_names_drift_2026_10.py).
    "EX4-058", "EX4-072",
}

# Errata to printed data other than text. EX11-025's emblem art (2026-02-06) and P-123's
# "Digivolve" label (2024-07-05) change no data.
PRINTED = {
    # 2025-04-18: a yellow segment joins the red cost ring.
    "BT21-023": {"card_colors": [0, 2]},
    # 2022-11-11: digivolves from green or blue level 5 for 5 memory, not 3.
    "BT3-111": {"evo_costs": [(3, 5, 5), (1, 5, 5)]},
    # 2021-05-28: "Attribute: Unknown Type: Unknown", not Unidentified.
    "BT4-041": {"attribute_eng": ["Unknown"], "type_eng": ["Unknown"]},
}

_GLYPHS = str.maketrans({"⟨": "<", "⟩": ">", "＜": "<", "＞": ">", "‘": "'", "’": "'", "“": '"',
                         "”": '"', ".": " ", ",": " ", "…": " ", "・": " "})
_SPACE = re.compile(r"\s+")
_HUGGED = re.compile(r" ?([\[\]<>()]) ?")
# A keyword's reminder text, once _norm has run: "<rush>(this digimon can attack ...)".
_REMINDER = re.compile(r">\((?:[^()]|\([^()]*\))*\)")


def _norm(text):
    """Words in lower case and single spaces, none around brackets, no commas or full stops."""
    text = _SPACE.sub(" ", (text or "").translate(_GLYPHS)).strip().casefold()
    return _HUGGED.sub(r"\1", text)


def reads_as(field_text, text):
    """True if `text` (the errata page's wording) reads in `field_text` (a cards.json field)."""
    want, have = _norm(text), _norm(field_text)
    if ">(" not in want:
        have = _REMINDER.sub(">", have)
    # Where `text` starts or ends mid-sentence, on a word, that word must match whole.
    lead = r"(?<![\w'])" if re.match(r"[\w']", want) else ""
    tail = r"(?![\w'])" if re.search(r"[\w']$", want) else ""
    return re.search(lead + re.escape(want) + tail, have) is not None


def _field(card, path):
    value = card
    for key in path.split("."):
        value = value.get(key) if isinstance(value, dict) else None
    return value


def errata_violations(cards, errata=ERRATA):
    """Every text errata cards.json does not carry, as strings. Empty means clean."""
    out = []
    for date, cid, field, after, before, *instead in errata:
        card = cards.get(cid)
        if card is None:
            out.append(f"{cid}: missing from cards.json")
            continue
        want = instead[0] if instead else after
        if field == NAME:
            if card.get(NAME) != want:
                out.append(f"{cid}: named {card.get(NAME)!r}; the {date} errata names it {want!r}")
            continue
        if not reads_as(_field(card, field), want):
            out.append(f"{cid}: {field} lacks the {date} errata: {want!r}")
        if _norm(before) not in _norm(after) and _norm(before) not in _norm(want):
            stale = [f for f in _TEXT_FIELDS if reads_as(_field(card, f), before)]
            if stale:
                out.append(f"{cid}: {', '.join(stale)} still holds the text before {date}: {before!r}")
    return out


def printed_violations(cards, printed=PRINTED):
    """Every PRINTED errata cards.json does not carry, as strings."""
    out = []
    for cid, fields in printed.items():
        card = cards.get(cid) or {}
        for field, want in fields.items():
            have = card.get(field)
            if field == "evo_costs":
                have = sorted((d["card_color"], d["level"], d["memory_cost"]) for d in have or [])
                want = sorted(want)
            elif field == "card_colors":
                have, want = sorted(have or []), sorted(want)
            if have != want:
                out.append(f"{cid}: {field} {have!r}; the errata gives {want!r}")
    return out


def _cards():
    with open(os.path.join(_ROOT, "data", "cards.json"), encoding="utf-8") as f:
        return json.load(f)


def test_cards_json_carries_every_errata():
    cards = _cards()
    violations = [v for v in errata_violations(cards) + printed_violations(cards)
                  if v.split(":")[0] not in PENDING]
    assert not violations, f"{len(violations)} errata missing from cards.json:\n  " + "\n  ".join(violations)


def test_pending_errata_are_still_missing():
    # A pending card that now carries its errata leaves PENDING, so the guard covers it again.
    cards = _cards()
    landed = [cid for cid in sorted(PENDING)
              if not errata_violations(cards, [e for e in ERRATA if e[1] == cid])]
    assert not landed, f"these now carry their errata; drop them from PENDING: {landed}"


def test_the_record_covers_the_errata_page():
    # A vacuous pass is the failure to fear. The page's 93 entries name 96 cards: the 2025-04-25
    # notice names six, and EX4-063 and P-123 each have two.
    assert len({e[1] for e in ERRATA} | set(PRINTED) | {"EX11-025"}) == 96


def test_brackets_reminder_text_and_spacing_are_not_differences():
    field = ("[On Play] [When Digivolving] 1 of your Digimon gains ＜Rush＞ (This Digimon can attack"
             " the turn it comes into play.) for the turn.\r\n[Main] ＜Draw 1＞.")
    assert reads_as(field, "[On Play][When Digivolving] 1 of your Digimon gains ⟨Rush⟩ for the turn")
    assert not reads_as(field, "1 of your Digimon gains <Rush> until the end of your turn.")
    # Words match whole: "at the end" is not in "that the end".
    assert not reads_as("Delete it so that the end of your opponent's turn ...",
                        "at the end of your opponent’s turn")


def test_an_errata_to_reminder_text_compares_the_reminder():
    before = ("＜Overclock ([Puppet] Trait)＞ (At the end of your turn, by deleting 1 of your Tokens or"
              " other [Puppet] trait Digimon, this Digimon may attack a player without suspending.)")
    after = before.replace("may attack", "attacks")
    assert reads_as(after, after.replace("＜", "⟨").replace("＞", "⟩"))
    assert not reads_as(before, after)


def test_pre_errata_text_anywhere_on_the_card_is_reported():
    errata = [("2026-05-15", "X-1", OPTION, "gains <Rush> for the turn.",
               "gains <Rush> until your opponent's turn ends.")]
    card = {"dual": {"option": {"effect_text": "[Main] 1 of your Digimon gains ＜Rush＞ for the turn."}},
            INHERITED: "[Main] 1 of your Digimon gains ＜Rush＞ until your opponent's turn ends."}
    violations = errata_violations({"X-1": card}, errata)
    assert len(violations) == 1 and violations[0].startswith(f"X-1: {INHERITED} still holds"), violations
    card[INHERITED] = ""
    assert errata_violations({"X-1": card}, errata) == []


def test_card_list_wording_stands_in_for_the_after_text():
    errata = [("2024-05-17", "X-1", EFFECT, "with [Nene Amano]/[Yuu Amano] in its name",
               "with [Amano] in its name", "with [Nene Amano] or [Yuu Amano] in its name")]
    assert errata_violations({"X-1": {EFFECT: "1 Tamer card with [Nene Amano] or [Yuu Amano] in its name"}},
                             errata) == []
    assert errata_violations({"X-1": {EFFECT: "1 Tamer card with [Amano] in its name"}}, errata)
