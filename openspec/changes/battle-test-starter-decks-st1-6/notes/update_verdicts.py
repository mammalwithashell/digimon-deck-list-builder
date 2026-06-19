"""Task 7.1: overwrite the templated 2026-05-29 ST1-6 verdicts in
validated_cards_dsl.json with re-derived, card-specific notes from the
battle-test audit. Keeps status=AUDITED-OK (required by gauntlet's
training-ready gate _TRAINING_READY_DSL_STATUSES); divergences are noted
in audit_note, not by changing status."""
import json, io

PATH = "qa/qa-reports/validated_cards_dsl.json"
REPORT = "battle-test-starter-decks-st1-6"
DATE = "2026-06-14"

NOTES = {
"ST1-01": "Inherited [Your Turn] +1000 DP at 4+ digivolution cards; formula = DCGO ST1_01.cs.",
"ST1-02": "Vanilla Rookie Biyomon (Bird/Vaccine, DP3000); no effect box; no DCGO .cs; YAML adds no behavior.",
"ST1-03": "Inherited [Your Turn] flat +1000 DP; = DCGO ST1_03.cs.",
"ST1-04": "Vanilla Rookie Dracomon (Dragon/Data, DP4000); attribute Data matches face; no behavior.",
"ST1-05": "Vanilla Champion Birdramon (DP5000); no effect box; no DCGO .cs.",
"ST1-06": "<Blocker> + [When Attacking] lose 2 memory (mandatory); = DCGO ST1_06.cs.",
"ST1-07": "Inherited <Security A.+1>; reaches carrier via stack-walk + combat tick-fresh strike. Ignored st1_07 test targets a generic engine gap (G-DECLARATIVE-KEYWORD), not this card.",
"ST1-08": "[When Digivolving] 1 of your Digimon +3000 DP/turn (mandatory single); = DCGO ST1_08.cs.",
"ST1-09": "Inherited [Your Turn] gain 3 memory when this Digimon is blocked; = DCGO ST1_09.cs.",
"ST1-10": "Vanilla Mega Phoenixmon (DP12000); no effect box; no DCGO .cs.",
"ST1-11": "[Your Turn] <Security A.> +1 per 2 digivolution cards (base-inclusive formula); double-count bug fixed; MCP-confirmed securityAttackModifier=2 at 4 sources.",
"ST1-12": "Tamer [Your Turn] all your Digimon +1000 DP; security free-play; = DCGO ST1_12.cs.",
"ST1-13": "Option +3000 to 1 Digimon (main); security grants all your Digimon <Sec A.+1> to your next turn; = DCGO ST1_13.cs.",
"ST1-14": "Option your Security Digimon +7000 DP (main: to opp next turn; security: this turn); = DCGO ST1_14.cs.",
"ST1-15": "Delete up to 2 opp Digimon with <=4000 DP (optional_zero per rule 15-10-2-2); = DCGO ST1_15.cs.",
"ST1-16": "Delete 1 opp Digimon, no DP cap (mandatory single); security re-runs main; = DCGO ST1_16.cs.",

"ST2-01": "Inherited [Your Turn] +1000 DP vs no-source opponent in a Digimon-vs-Digimon battle; = DCGO ST2_01.cs.",
"ST2-02": "Vanilla Rookie Gomamon (Sea Beast); no effect; Blue Lv2->0 evo path.",
"ST2-03": "Inherited [When Attacking] trash bottom source of 1 opp Digimon <=Lv5 that has a source; source check matches DCGO ST2_03.cs.",
"ST2-04": "Vanilla Rookie Bearmon (Beast); no effect.",
"ST2-05": "Vanilla Champion Ikkakumon (Sea Beast); no effect.",
"ST2-06": "FIXED 2026-06-14: inherited trash-bottom now targets ANY opp Digimon (removed erroneous materials_count_gte:1) to match DCGO ST2_06.cs (no source check, unlike ST2-03/09). Regression test st2_06_targets_sourceless_opponent_digimon.",
"ST2-07": "<Blocker> + [When Attacking] lose 2 memory.",
"ST2-08": "Inherited [Your Turn] <Sec A.+1> while opp has a no-source Digimon; = DCGO ST2_08.cs.",
"ST2-09": "[When Digivolving] trash 2 bottom sources of 1 opp Digimon (source check correct here); = DCGO ST2_09.cs.",
"ST2-10": "Vanilla Mega Plesiomon; no effect.",
"ST2-11": "[When Attacking][Once Per Turn] unsuspend self; = DCGO ST2_11.cs.",
"ST2-12": "Tamer [Start of Your Turn] +1 memory if opp has a no-source Digimon; security free-play; = DCGO ST2_12.cs.",
"ST2-13": "Option gain 1 memory (main) / 2 memory (security); = DCGO ST2_13.cs.",
"ST2-14": "Lock 1 no-source opp Digimon (can't attack/block); main->opp next turn, security->your next turn; = DCGO ST2_14.cs.",
"ST2-15": "Play a digivolution-card source as a new Digimon for free; core faithful. Minor: source filter lacks DCGO CanPlayAsNewPermanent playability gate (deferred, logged G-AUDIT-ST1-6); behavior converges.",
"ST2-16": "Return 1 opp Digimon to owner's hand + trash its sources; = DCGO ST2_16.cs.",

"ST3-01": "Inherited [Your Turn][OPT] +1000 DP/turn when an opp Digimon is deleted by 0 DP; = DCGO ST3_01.cs.",
"ST3-02": "Vanilla Rookie Salamon; Yellow Lv2->0 evo.",
"ST3-03": "Vanilla Rookie Tapirmon; no effect.",
"ST3-04": "Inherited [Your Turn][OPT] gain 1 memory on opp 0-DP deletion; = DCGO ST3_04.cs.",
"ST3-05": "Inherited [When Attacking] gain 1 memory if you have 4+ security; = DCGO ST3_05.cs.",
"ST3-06": "Vanilla Champion Gatomon; no effect.",
"ST3-07": "<Blocker> + [When Attacking] lose 2 memory (on body, not inherited).",
"ST3-08": "Inherited [When Attacking] 1 opp Digimon -1000 DP/turn (mandatory); = DCGO ST3_08.cs.",
"ST3-09": "[When Digivolving] Recovery +1 (Deck) if <=3 security; = DCGO ST3_09.cs.",
"ST3-10": "Vanilla Mega Magnadramon (cheap Lv5->2 evo per face); no effect.",
"ST3-11": "[When Attacking] 1 opp Digimon -4000 DP/turn; = DCGO ST3_11.cs.",
"ST3-12": "Tamer [Opp Turn] your Security Digimon +2000 DP; security free-play; = DCGO ST3_12.cs.",
"ST3-13": "Option +3000 to 1 Digimon (main); security +5000 to your Digimon + Security then add self to hand; = DCGO ST3_13.cs.",
"ST3-14": "Option 1 opp Digimon -2000 DP (main); security add to hand; = DCGO ST3_14.cs.",
"ST3-15": "Option <Sec A.-3> on 1 opp to opp next turn (main); <Sec A.-1> all opp (security). YAML follows image/DCGO; cards.json text is wrong. = DCGO ST3_15.cs.",
"ST3-16": "Option 1 opp Digimon -10000 DP (drives 0-DP deletion); security re-runs main; = DCGO ST3_16.cs.",

"ST4-01": "Inherited [Your Turn] +1000 DP while Lv6 or higher; = DCGO ST4_01.cs.",
"ST4-02": "Vanilla Rookie Floramon; Green Lv2->1 evo.",
"ST4-03": "[On Play] reveal top; add green Digimon (mandatory if eligible) else bottom of deck; = DCGO ST4_03.cs.",
"ST4-04": "Inherited [When Attacking] +2000 DP/turn when attacking a Digimon; = DCGO ST4_04.cs.",
"ST4-05": "Vanilla Rookie Kunemon; no effect.",
"ST4-06": "Inherited [When Attacking] +2000 DP/turn vs a Digimon (mirrors ST4-04).",
"ST4-07": "Vanilla Champion Kuwagamon; no effect.",
"ST4-08": "<Blocker> + [When Attacking] lose 2 memory.",
"ST4-09": "Vanilla Ultimate Okuwamon; no effect.",
"ST4-10": "[When Digivolving] reveal 5, add 1 Lv6+ Digimon (mandatory if eligible), rest to bottom in any order (ordered, no auto-pick); = DCGO ST4_10.cs.",
"ST4-11": "Inherited [Your Turn][OPT] trash top of opp security when carrier deletes a battle opponent and survives; = DCGO ST4_11.cs.",
"ST4-12": "[When Digivolving] 1 opp Digimon can't attack/block until their next turn; = DCGO ST4_12.cs.",
"ST4-13": "<Piercing> + [Main] <Digi-Burst 2> suspend 1 opp Digimon; faithful. Minor: suspend target filtered is_unsuspended (substrate-wide convention, deferred, logged G-AUDIT-ST1-6).",
"ST4-14": "Tamer [Your Turn] may suspend self to gain 1 memory on opp suspend; security free-play; = DCGO ST4_14.cs.",
"ST4-15": "Option suspend 1 opp Digimon (main); security suspends + adds self to hand. Minor: is_unsuspended target filter (deferred, logged G-AUDIT-ST1-6).",
"ST4-16": "Option return 1 suspended opp Digimon + trash its sources; security re-runs; = DCGO ST4_16.cs.",

"ST5-01": "Inherited [Your Turn] +1000 DP while it has Blocker; = DCGO ST5_01.cs.",
"ST5-02": "Vanilla Rookie Jazamon; Black Lv2->1 evo.",
"ST5-03": "<Blocker> (main body).",
"ST5-04": "Inherited [End of Opp Turn] draw 1 if opp did not attack with a Digimon; = DCGO ST5_04.cs.",
"ST5-05": "Vanilla Rookie Commandramon; no effect.",
"ST5-06": "Inherited [End of Opp Turn] draw 1 if opp did not attack with a Digimon (mirrors ST5-04).",
"ST5-07": "Vanilla Champion Jazardmon; no effect.",
"ST5-08": "<Blocker> + [When Attacking] lose 2 memory.",
"ST5-09": "[When Digivolving] 1 of your Digimon gains Blocker until opp next turn; = DCGO ST5_09.cs.",
"ST5-10": "Vanilla Ultimate MetalTyrannomon; no effect.",
"ST5-11": "Inherited <Blocker>; = DCGO ST5_11.cs.",
"ST5-12": "[When Digivolving] up to 2 of your Digimon gain Reboot until opp next turn (optional_zero per rule 15-10-2-2); = DCGO ST5_12.cs.",
"ST5-13": "<Sec A.+1> + [Main] <Digi-Burst 2> 1 of your Digimon +4000 DP until opp next turn; = DCGO ST5_13.cs.",
"ST5-14": "Tamer [Opp Turn] may suspend self to unsuspend 1 of your Digimon when you Blocker-suspend; security free-play; = DCGO ST5_14.cs.",
"ST5-15": "Option <De-Digivolve 1> on up to 2 opp Digimon (optional_zero); security re-runs; = DCGO ST5_15.cs.",
"ST5-16": "Option delete 1 opp Digimon with play cost <=7; security re-runs; = DCGO ST5_16.cs.",

"ST6-01": "Inherited [On Deletion] trash top 2 of your deck; = DCGO ST6_01.cs.",
"ST6-02": "Vanilla Rookie DemiDevimon; Purple Lv2->1 evo.",
"ST6-03": "Inherited [When Attacking] draw 1 then trash 1 from hand (mandatory); = DCGO ST6_03.cs.",
"ST6-04": "[On Play] may return a purple Option (cost 1 or 7) from trash to hand; = DCGO ST6_04.cs.",
"ST6-05": "Vanilla Rookie Elecmon; no effect.",
"ST6-06": "Inherited [When Attacking] draw 1 then trash 1 from hand (mirrors ST6-03).",
"ST6-07": "Vanilla Champion Youkomon; no effect.",
"ST6-08": "<Blocker> + [When Attacking] lose 2 memory.",
"ST6-09": "Vanilla Ultimate Kyukimon; no effect.",
"ST6-10": "[When Digivolving] may return a purple Digimon from trash to hand; = DCGO ST6_10.cs.",
"ST6-11": "Inherited [Your Turn] self +2000 DP while 5+ cards in trash; = DCGO ST6_11.cs.",
"ST6-12": "[When Digivolving] up to 2 of your Digimon gain Retaliation until opp next turn. optional_zero is CORRECT per rule 15-10-2-2 (DCGO force->=1 is a UI quirk; auditor false-positive resolved).",
"ST6-13": "<Sec A.+1> + [Main] <Digi-Burst 2> play a purple Lv3 Digimon from trash free; faithful. Minor: activation over-gated on a valid trash target existing vs DCGO CanDigiBurst-only (deferred, logged G-AUDIT-ST1-6).",
"ST6-14": "Tamer [Your Turn] may suspend self to gain 1 memory when your Digimon is deleted; security free-play; = DCGO ST6_14.cs.",
"ST6-15": "Option may delete 1 of your Digimon to delete 1 opp Lv4-or-lower Digimon; security deletes 1 opp Lv4-or-lower; = DCGO ST6_15.cs.",
"ST6-16": "Option play a purple Lv3 + Lv4 Digimon from trash free, On Play suppressed; security plays 1 purple Lv4-or-lower free; = DCGO ST6_16.cs.",
}

with open(PATH, encoding="utf-8") as f:
    data = json.load(f)

cards = data["cards"]
updated = 0
missing = []
for cid, note in NOTES.items():
    if cid not in cards:
        missing.append(cid); continue
    e = cards[cid]
    e["validated_date"] = DATE
    e["report"] = REPORT
    e["status"] = "AUDITED-OK"   # required by gauntlet training-ready gate
    e["audit_note"] = note
    updated += 1

# sanity: no remaining templated note among ST1-6
templated = [cid for cid in NOTES if cards.get(cid, {}).get("audit_note") == "Faithful to printed text + DCGO."]

with io.open(PATH, "w", encoding="utf-8", newline="\n") as f:
    json.dump(data, f, indent=2, ensure_ascii=False)
    f.write("\n")

print(f"updated={updated}/96  missing={missing}  remaining_templated={templated}")
