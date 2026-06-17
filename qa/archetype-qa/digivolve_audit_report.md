# DSL Digivolution Audit — Confirmed Findings

442 DSL Digimon audited; **157 confirmed**, 88 refuted (adversarial verification).

> LLM vision findings — spot-verify against the card image before patching. evo-colour/cost = data fixes (cards.json/overrides); alt-path = YAML authoring.

## MISSING_EVO_COLOR (48)
- **AD1-002** [high]: evo_costs should contain BOTH: (1) Red Lv.8 Cost 8 [primary digivolution circle, currently missing], and (2) Red Lv.3 Cost 3 [alternate digivolution via Takuya Kanbara + 2 Hybrid, currently missing]. 
- **AD1-010** [high]: evo_costs should be [{"color": "blue", "level": 3, "cost": 2}] to match the printed digivolution circle on the card face. This represents the alternative digivolution path for Garurumon (Lv.3 with [Om
- **BT11-042** [high]: evo_costs should contain TWO entries: [Yellow Lv.4 Cost 3, Purple Lv.3 Cost 3]. Currently missing Purple Lv.3 Cost 3. Corrected array: [{"card_color": 2, "level": 4, "memory_cost": 3}, {"card_color": 
- **BT12-021** [high]: evo_costs should include both: Blue Lv.2 Cost 0 AND Green Lv.2 Cost 0. Current data is missing Green Lv.2 Cost 0.
- **BT12-057** [high]: evo_costs should be [Yellow Lv.6 Cost 6, Green Lv.6 Cost 6, Green Lv.6 Cost 6] to match the three distinct circles on the card face. The current data only contains one Green Lv.6 Cost 6 entry and is m
- **BT13-012** [high]: evo_costs should contain: [{card_color: 0 (Red), level: 3, memory_cost: 3}, {card_color: 2 (Yellow), level: 3, memory_cost: 3}]. Additionally, Red Lv.5 cost 3 visible on the card is also absent from e
- **BT13-020** [high]: evo_costs should include: [{"card_color": 0, "level": 6, "memory_cost": 5}, {"card_color": 2, "level": 5, "memory_cost": 5}]. The card image clearly shows two digivolution circles in the top-left: a R
- **BT13-040** [high]: evo_costs should include: [Yellow Lv.3 cost 4, Blue Lv.3 cost 4]. The card image clearly shows a yellow digivolution circle (Lv.3 cost 4) and a separate blue digivolution circle (bottom-right, also Lv
- **BT13-088** [high]: evo_costs should include: [{"card_color": 6, "level": 5, "memory_cost": 3}, {"card_color": 6, "level": 3, "memory_cost": 1}]. The second entry represents the alternate purple Lv.3 digivolution path fr
- **BT13-112** [high]: evo_costs should be [{"card_color": 1, "level": 4, "memory_cost": [cost from black circle]}, {"card_color": 3, "level": 4, "memory_cost": [cost from blue circle]}]. The card image shows two distinct L
- **BT19-012** [medium]: evo_costs should include: [{ card_color: 0 (Red), level: 4, memory_cost: 4 }, { card_color: 2 (Yellow), level: 4, memory_cost: 3 }]. The Yellow Lv.4 Cost 3 variant applies when digivolving a Lv.4 with
- **BT19-038** [high]: evo_costs should include both: [Yellow Lv.4 Cost 4, Green Lv.4 Cost 4]. Currently cards.json only lists Yellow Lv.4 Cost 4 (card_color: 2). Should add: {card_color: 3, level: 4, memory_cost: 4}
- **BT19-072** [high]: evo_costs should contain TWO entries: (1) card_color: 5 (Red), level: 5, memory_cost: 3 AND (2) card_color: 6 (Purple), level: 5, memory_cost: 3
- **BT20-016** [high]: evo_costs should include: [Red Lv.4 Cost 4] for standard digivolve AND dna_costs should include [Red Lv.4 + Purple Lv.4 Cost 0] for DNA digivolve. Card image shows red and purple cost circles at top-l
- **BT20-037** [high]: evo_costs should be empty or removed entirely. The card has only DNA Digivolve (Yellow Lv.6 + Green/Black Lv.6: Cost 0), not a standard Yellow Lv.6 Cost 5 digivolution path. All standard digivolution 
- **BT21-011** [medium]: evo_costs should include both: { card_color: 0 (Red), level: 2, memory_cost: 1 } AND { card_color: 2 (Yellow), level: 2, memory_cost: 0, requirements: "Xros Heart or Hero trait" }. Currently only Red 
- **BT21-021** [medium]: evo_costs should include: [{"card_color": 0, "level": 4, "memory_cost": 4}, {"card_color": 2, "level": 4, "memory_cost": 3}]. Currently has only the Red Lv.4 Cost 4 entry. The Yellow Lv.4 Cost 3 circl
- **BT21-023** [high]: evo_costs should include both: [Red Lv.4 Cost 4, Yellow Lv.4 Cost 4 (trait: Sup.)]. Current data omits the Yellow Sup. trait-based digivolution path entirely.
- **BT21-043** [high]: evo_costs should include both: Yellow Lv.2 cost 2, Yellow Lv.3 cost 2. Currently missing Yellow Lv.2 cost 2.
- **BT21-055** [high]: evo_costs should contain: [{"color": 5, "level": 3, "cost": 0}, {"color": 5, "level": 2, "cost": 0}] where color 5 represents Black
- **BT21-070** [high]: evo_costs should contain three entries: [Purple Lv.4 Cost 2, Purple Lv.3 Cost 2, Green Lv.2 Cost 2]. Current data only includes Purple Lv.3 Cost 2. The card image shows all three digivolution circles 
- **BT21-073** [high]: evo_costs should contain two entries: [Purple Lv.4 Cost 4, Green Lv.4 Cost 4 (Sup. trait-based alternative)]. The card image shows a green digivolution circle marked with "Sup" as the alternative requ
- **BT21-101** [high]: evo_costs should include both: [Red Lv.5 Cost 5, Purple/Ult. trait-based Cost 5]. Note: The card image shows the second circle as purple/violet (not white as claimed), but the alternative Ult. digivol
- **BT22-009** [high]: evo_costs should be: [Red Lv.4 cost 4, Red Lv.3 cost 2, Green Lv.2 cost 2]. The card image shows three digivolution circles: top circle is Red Lv.4 (standard play cost 4), middle circle is Red Lv.3 co
- **BT22-015** [high]: evo_costs should be [Blue Lv.6 Cost 5, Yellow Lv.6 Cost 5]. Card face clearly shows two distinct filled circles in the digivolution cost area: one blue circle labeled "Lv.6" and "Cost 5", and one yell
- **BT22-017** [high]: evo_costs should be [{"card_color": 1, "level": 3, "memory_cost": 3}, {"card_color": 1, "level": 2, "memory_cost": 0}]. The card face shows two printed digivolution circles: Blue Lv.3 Cost 3 (standard
- **BT23-013** [high]: evo_costs should include both: Red Lv.4 Cost 3 (digivolve from SaviorHuckmon or Level 5 digimon) AND Red Lv.5 Cost 5 (digivolve from Huckmon with condition). Current data lists only Red Lv.5 Cost 4, m
- **BT23-064** [high]: evo_costs should be [Purple Lv.3 Cost 2]
- **BT23-102** [medium]: evo_costs should include both entries: Yellow (color_code 2) Lv.5 Cost 5 AND Purple (color_code 6) Lv.5 Cost 0. Full corrected array: [{"card_color": 2, "level": 5, "memory_cost": 5}, {"card_color": 6
- **BT24-012** [high]: evo_costs should be [{"color": "Red", "level": 3, "cost": 2}] or equivalent representation capturing Red Lv.3 cost 2
- **BT25-037** [high]: evo_costs should include two entries: [{"card_color": 2, "level": 3, "memory_cost": 3}, {"card_color": 5, "level": 3, "memory_cost": 2}] — the card image clearly shows Yellow Lv.3 circle and Blue Lv.3
- **BT25-044** [high]: evo_costs array is incomplete. Card image shows: (1) Blue Lv.5 digivolution cost 3 (alt condition with [Angel]/[Archangel]/[TS] traits); (2) Yellow Lv.4 standard digivolution cost 4. Current cards.jso
- **BT25-047** [high]: evo_costs should be: [{"card_color": 3, "level": 2, "memory_cost": 0}] where card_color 3 = Green. The card image clearly shows a Green Lv.2 cost 0 digivolution circle in the top-left. Current JSON ha
- **BT25-059** [high]: evo_costs should include: [{"card_color": 2, "level": 4, "memory_cost": 4}, {"card_color": 3, "level": 5, "memory_cost": 4}] — currently missing Yellow Lv.4 Cost 4
- **BT25-060** [high]: evo_costs should include: [{"card_color": 3, "level": 5, "memory_cost": 4}, {"card_color": 4, "level": 4, "memory_cost": 4}]. The card face clearly shows Green Lv.5 Cost 4 and Blue Lv.4 Cost 4 in the 
- **BT9-112** [high]: evo_costs should be [Purple (color 6) Lv.6 Cost 6, Black (color 5) Lv.6 Cost 6]. Card image clearly shows two stacked digivolution circles in top-left: purple circle above, black circle below, both la
- **EX10-020** [high]: evo_costs should be [{"level": 5, "color": "green", "cost": 3}] — standard digivolve route from Level 5 Green with cost 3
- **EX10-034** [high]: evo_costs should include both entries: [{ card_color: 5 (Black), level: 5, memory_cost: 5 }, { card_color: 6 (Purple), level: 5, memory_cost: 5 }]. Currently cards.json only has the Black entry; the P
- **EX4-005** [high]: evo_costs should include: Red Lv.2 Cost 1 (already present) AND Yellow Lv.3 Cost 1 (missing). Note: Card image shows Yellow, not Blue as claimed in evidence. Current data is missing the Yellow Lv.3 Co
- **EX4-006** [high]: evo_costs should be [Red Lv.2 Cost 1, Purple Lv.3 Cost 1]. The card image clearly shows two digivolution cost circles: a red circle (Lv.2, Cost 1) and a purple circle (Lv.3, Cost 1) stacked vertically
- **EX8-022** [high]: evo_costs should include both: [{"card_color": 1, "level": 3, "memory_cost": 3}, {"card_color": 2, "level": 3, "memory_cost": 2}] to reflect the Blue Lv.3 Cost 3 and Yellow Lv.3 Cost 2 digivolve circl
- **EX8-023** [high]: evo_costs should include: [{"card_color": 1, "level": 4, "memory_cost": 4}, {"card_color": 2, "level": 4, "memory_cost": 3}] where color 2 = Yellow with Ice-Snow trait condition
- **EX8-074** [high]: evo_costs should include both: [{"card_color": 3, "level": 5, "memory_cost": 3}, {"card_color": 0, "level": 5, "memory_cost": 4}] where color 3=Green and color 0=Red
- **P-101** [high]: evo_costs should be [Cyan Lv.? cost 4, Cyan Lv.3 cost 3, Purple Lv.3 cost 3] — the third digivolution circle is definitively Purple, not Black
- **ST3-10** [high]: evo_costs should include both: Yellow Lv.5 Cost 10 AND Blue Lv.2 Cost 0. Current data lists only Yellow Lv.5 Cost 2 (which is also incorrect - should be Cost 10). The blue circle is completely missing
- **ST4-04** [high]: evo_costs should be [{'card_color': 3, 'level': 2, 'memory_cost': 0}, {'card_color': 3, 'level': 3, 'memory_cost': 0}] — the card image clearly shows two green digivolution circles in the top-left: on
- **ST5-06** [high]: evo_costs should include both: [{ card_color: 5 (Black), level: 3, memory_cost: 2 }, { card_color: 1 (Blue), level: 2, memory_cost: 2 }]. The card image clearly shows two digivolution circles in the t
- **ST5-09** [high]: evo_costs should be: [{"card_color": 4, "level": 3, "memory_cost": 3}, {"card_color": 5, "level": 4, "memory_cost": 3}]. Card image clearly shows two digivolution circles: blue Lv.3 Cost 3 and black L

## WRONG_EVO_COST (34)
- **AD1-014** [high]: evo_costs should be: [{"card_color": 1, "level": 5, "memory_cost": 4}, {"card_color": 1, "level": 5, "memory_cost": 3}] — Blue Lv.5 Cost 4 (standard) + Blue Lv.5 Cost 3 (alternative when digivolving f
- **AD1-016** [high]: evo_costs should include two entries: [Yellow Lv.5 Cost 4 (standard)] AND [Yellow Lv.5 Cost 3 (when digivolving from RizeGreymon or DATA SQUAD trait)]. The alternative Cost 3 condition is printed on t
- **BT13-112** [high]: evo_costs should be [{"card_color": 4, "level": 4, "memory_cost": 2}, {"card_color": 0, "level": 4, "memory_cost": 2}] - The card image clearly shows TWO digivolution circles, both displaying Lv.4, no
- **BT16-101** [high]: evo_costs should include: Yellow Lv.6 cost 4 (alternative digivolve condition: [Rapidmon] Cost 4). Note: claim stated Lv.5 but card shows Lv.6.
- **BT17-077** [high]: Blue Lv.6 cost 5 (card_color: 1, level: 6, memory_cost: 5)
- **BT19-035** [high]: evo_costs should be [{"card_color": 2, "level": 3, "memory_cost": 2}] to match the printed yellow Lv.3 circle showing Cost 2 and the xros_req text stating "[Digivolve] Lv.3 w/[Xros Heart] trait: Cost 
- **BT19-038** [high]: Yellow Lv.4 Cost should be 3 (not 4). The card shows a yellow circle for Lv.4 digivolution with cost 3, and the C# code confirms this via an alternate digivolution requirement: "digivolutionCost: 3" f
- **BT21-042** [high]: evo_costs should include both: [Yellow Lv.3 cost 3 (standard)] AND [Yellow Lv.3 cost 2 (alternate: w/[Agumon] in name and [Dinosaur] trait)]. Card image shows two distinct cost circles: blue circle (L
- **BT21-044** [high]: evo_costs should be [{ card_color: 2 (Yellow), level: 4, memory_cost: 3 }]. Card image shows Yellow Lv.4 circle with printed cost 3. Alt-digivolve via [GeoGreymon] also costs 3 (confirmed in BT21_044.
- **BT22-052** [high]: evo_costs should be [Green Lv.5 cost 3]; alt_paths digivolve cost should be 3. The card's printed black text states "Lv.5 w/[CS] trait: Cost 3" (authoritative), matching C# implementation line 25 (dig
- **BT23-035** [high]: Yellow Lv.5 Cost 3 (cards_json_evo_costs[0].memory_cost should be 3, not 4)
- **BT23-054** [high]: evo_costs should be: { "card_color": 5, "level": 3, "memory_cost": 3 }
- **BT24-037** [high]: evo_costs should be empty array [] — the card shows NO standard Yellow Lv.4 digivolution circle at cost 4. The only Yellow Lv.4 cost-0 path is via DNA Digivolution (correctly stored in dna_costs). The
- **BT24-040** [high]: Yellow Lv.5 Cost 3
- **BT24-041** [high]: evo_costs should be [Yellow Lv.5 cost 3]. The printed card clearly states "Digivolve Lv.5 w/[Beastkin]/[Dark Dragon]/[TS] Trait Cost 3" in the effect text, and this is corroborated by the C# code (BT2
- **BT24-046** [high]: evo_costs should be: [{ card_color: 3 (Green), level: 3, memory_cost: 2 }]. The printed card image clearly shows the Green Lv.3 digivolve circle displaying cost 2, not 3. The effect text confirms "[Di
- **BT24-051** [high]: evo_costs should be [{"card_color": 3, "level": 5, "memory_cost": 3}]
- **BT24-059** [high]: evo_costs should be [Lv.4 cost 3 with Angel or Sea Animal trait condition, or Lv.4 cost 3 with S trait]
- **BT25-008** [high]: Red Lv.2 cost should be 0 (not 1). Card image shows empty/zero-cost circle. Alternative digivolve condition: by trashing up to 2 [Iliad] or [TS] trait cards from hand, Draw 1 for each. CS code confirm
- **BT25-012** [high]: Alternative digivolution cost should be Lv.3 Cost 2 (with [TS] trait condition). The card face clearly shows "[Digivolve] Lv.3 w/[TS] trait: Cost 2" in the effect text, and the C# implementation confi
- **BT25-016** [high]: evo_costs should be [Red Lv.4 Cost 4, Red Lv.4 Cost 3] - the card image clearly shows two red digivolution circles: top circle Cost 4 (standard path), bottom circle Cost 3 (alt-path with [TS] trait co
- **BT25-018** [high]: evo_costs should include both: Red Lv.5 Cost 4 (standard) AND Red Lv.5 Cost 3 (alternative with [TS] trait). Card shows two distinct cost circles: top circle Red Lv.5/4, bottom circle Red Lv.5/3. Blac
- **BT25-025** [high]: evo_costs should be [Blue Lv.4 cost 3] for Aegiomon digivolution condition. Current data incorrectly lists cost as 4.
- **BT25-036** [high]: evo_costs should be [{ "card_color": 2, "level": 2, "memory_cost": 2 }] — Yellow Level 2 cost 2, matching the yellow Lv.2 circle on card image
- **BT25-042** [high]: evo_costs[0].memory_cost should be 3 (currently 4 in cards.json). The printed card shows a single Lv.5 Yellow digivolution with cost 3, displayed in the yellow circle. The alt-digivolve text confirms 
- **BT25-043** [high]: evo_costs should be [{"card_color": 2, "level": 5, "memory_cost": 3}] — Yellow Lv.5 Cost 3, as shown on the printed card, confirmed by C# implementation line 23 (parameter value 3), and supported by t
- **BT25-044** [medium]: evo_costs should include alternate digivolution path: Lv.5 w/[Angel]/[Archangel]/[TS] traits: Cost 3. Current cards.json lists only Yellow Lv.5 Cost 4 (the base path), missing the alt-digivolution Cos
- **BT25-053** [high]: evo_costs[0].memory_cost should be 3 (currently 4). Card image shows red Lv.4 circle with cost 3; confirmed by C# line 23 (AddSelfDigivolutionRequirementStaticEffect with cost 3 parameter) and xros_re
- **BT25-058** [high]: evo_costs should be: [{"card_color": 3, "level": 5, "memory_cost": 4}]. The Green Lv.5 digivolution cost in the image shows 4, not 5. This matches the xros_req field which correctly states "Cost 4" fo
- **BT25-070** [high]: evo_costs should include Black Lv.3 cost 2 (regular digivolution, matching the printed circle on card image)
- **EX11-044** [high]: yaml_alt_paths cost should be 3, not 4. Current YAML lists cost: 4 for Black Lv.5 digivolution; card image and JSON both show cost 3 (memory_cost: 3). The digivolution cost circle visible on the card 
- **P-215** [high]: evo_costs should be [{ "card_color": 1, "level": 3, "memory_cost": 2 }] (Blue Lv.3 Cost 2, not Cost 3)
- **ST19-11** [high]: evo_costs should be [Yellow Lv.3 cost 3] - the card image clearly shows a Lv.3 digivolve circle with cost 3, but current data records claim Lv.4 cost 3. Image is authoritative source.
- **ST19-12** [high]: evo_costs should be [Yellow Lv.3 cost 3] - data currently shows level_eq: 5 but card image clearly displays Lv.3 in the blue digivolution circle

## WRONG_EVO_COLOR (21)
- **BT12-047** [high]: evo_costs should be [Blue Lv.2 Cost 0, Yellow Lv.2 Cost 0] — not Green Lv.2
- **BT13-075** [high]: evo_costs should have card_color: 6 (Yellow) instead of 5 (Black) for Lv.5 Cost 4 — the card image clearly shows a yellow circle for this digivolution requirement
- **BT13-112** [high]: evo_costs should be [Red Lv.4 cost 4, Blue Lv.4 cost 4]. The current data incorrectly lists Red Lv.6 cost 4 (wrong level). Note: The claim's reference to "Black Lv.4" is incorrect—the card image shows
- **BT16-040** [high]: evo_costs Lv.2 should be Blue cost 1 (not Red cost 1). Card image shows Blue-filled digivolution circle at Lv.2 with cost 1. Card's primary color is Green (confirmed by card frame color and effect fil
- **BT16-082** [high]: evo_costs should be [{"card_color": 4, "level": 2, "memory_cost": 1}] — change color code from 0 (Red) to 4 (White) to match the White Lv.2 cost 1 digivolution circle shown on the card image
- **BT18-102** [high]: evo_costs should be empty array [] or removed entirely. The card has NO standard level-based digivolution cost. The ONLY digivolution method is the assembly condition represented in xros_req: "[Digivo
- **BT22-015** [high]: evo_costs should include Blue Lv.6 Cost 5 and Yellow Lv.6 Cost 5. The current entry [{"card_color": 0 (Red), "level": 6, "memory_cost": 6}] is incorrect—Red with Cost 6 does not appear on the card fac
- **BT23-054** [high]: evo_costs should be [Blue Lv.3 Cost 3] instead of [Black Lv.3 Cost 4]. The printed digivolve circle (top-left) shows a blue circle with Lv.3 and cost 4 as the visual, but the black-text effect below s
- **BT24-051** [high]: evo_costs should be [{ card_color: 2 (Blue), level: 5, memory_cost: 3 }] — the card image shows a BLUE Lv.5 circle with cost 3, not Green cost 4. Additionally, the memory_cost in JSON lists 4 but shou
- **BT25-027** [medium]: evo_costs should include: [{"card_color": 1, "level": 4, "memory_cost": 3}, {"card_color": 1, "level": 3, "memory_cost": [cost from gold circle]}]. Card shows two circles: navy Lv.4 Cost 3 and gold Lv
- **BT25-070** [high]: evo_costs[0].card_color should be 6 (Purple) instead of 5 (Black). The Lv.3 digivolution cost circle on the card image is clearly purple/violet filled, matching the card frame color and Logamon's type
- **EX10-032** [high]: evo_costs should be [Purple/Magenta Lv.4 Cost 3] — the printed digivolution circle(s) on the card image are distinctly pink/magenta in color, not black. The circle color does NOT match the declared Bl
- **EX4-038** [high]: Evolution circle color should be black (currently renders as blue). Card EX4-038 is declared color_is: black in YAML (line 134) and card_colors: [5] (black) in JSON. The printed Digivolve Cost circle 
- **P-123** [high]: evo_costs should be [{"card_color": 4, "level": 2, "memory_cost": 0}] (White Lv.2 Cost 0) instead of card_color 0 (Red). The card's declared color is White [4], and the digivolve circle requirement of
- **ST3-05** [high]: evo_costs should list: Yellow Lv.2 cost 2 (currently incorrectly listed as Yellow Lv.3 cost 2)
- **ST3-06** [high]: evo_costs should be [Purple Lv.2 cost 4] (not Yellow Lv.3). The color and level corrections in the claim are correct, but the cost in the claim is wrong—the card shows cost 4, not cost 2.
- **ST3-08** [high]: evo_costs should be [Yellow Lv.3 cost 3] — card image shows yellow digivolution circle with "Lv.3" printed inside, not Lv.4
- **ST5-03** [high]: Digivolution cost should be Blue Lv.2 Cost 0, not Black Lv.2 Cost 0. In YAML: change line 14 from "color_is: black" to "color_is: blue". In cards.json: change evo_costs card_color from 5 (Black) to 1 
- **ST5-04** [high]: evo_costs should specify Blue Lv.2 Cost 0 (not Black Lv.2 Cost 0). In JSON: change "card_color": 5 to "card_color": 1 (or appropriate Blue enum value). In YAML: change "color_is: black" to "color_is: 
- **ST5-13** [high]: evo_costs should be [{"card_color": 1, "level": 5, "memory_cost": 4}] — card_color must be 1 (Red) not 5 (Black), as the card image shows a RED Lv.5 Cost 4 digivolution circle
- **ST9-09** [high]: evo_costs should contain TWO entries: (1) {card_color: 3, level: 3, memory_cost: 2} [Green - currently present], AND (2) {card_color: 1, level: 3, memory_cost: 2} [Blue - currently MISSING]. The card 

## WRONG_ALT_PATH (21)
- **AD1-002** [high]: alt_paths.yaml_alt_paths[0] should enforce TWO conditions: (1) top card name_contains "Takuya Kanbara" AND (2) 2 or more [Hybrid] trait cards in digivolution sources (cost 3, ignore_requirements: fals
- **BT12-047** [high]: alt_paths should have TWO digivolve paths: (1) from: { level_eq: 2, color_is: blue }, cost: 0; (2) from: { level_eq: 2, color_is: red }, cost: 0. The card image clearly shows a blue Lv.2 circle and a 
- **BT13-075** [high]: yaml_alt_paths should use color_is: yellow (not color_is: black). The digivolution cost circle on the card is printed as a Yellow Lv.5 cost 4 circle, clearly visible in the top-left area of the card.
- **BT16-082** [medium]: alt_paths should change from: { level_eq: 2 } to from: { level_eq: 2, color_is: white } — the card image shows a single White circle, not an unrestricted Level 2 digivolution
- **BT20-037** [high]: No correction needed - DNA alt_path already correct: cost 0, materials Yellow Lv.6 and Green/Black Lv.6, matching printed text exactly.
- **BT20-056** [high]: evo_costs should be: [{ card_color: "purple" (not 5), level: 5, memory_cost: 3 }]. The printed evolution circle on the card image shows a PURPLE/DARK circle, not BLUE. The YAML alt_paths correctly dec
- **BT21-017** [high]: evo_costs should be corrected: alt_paths digivolve from level_eq 3 should declare cost 2 (not cost 0). The printed circles on the card show Red Lv.3 Cost 2 and Blue Lv.3 Cost 2; the YAML currently dec
- **BT21-018** [medium]: alt_paths digivolve should include level_eq: 3 alongside trait_has: "Stnd." — the printed card clearly shows a red/maroon Lv.3 cost 3 circle. Current: {from: {trait_has: "Stnd."}, cost: 3}. Corrected:
- **BT22-025** [high]: alt_paths should include: kind: digivolve, from: { level_eq: 5, trait_contains: CS }, cost: 3 (the alt-digivolve with [CS] trait requirement is missing from current YAML; the standard blue Lv.5 cost 4
- **BT22-029** [high]: evo_costs[0].level should be 0 (not 2). Card image shows yellow circle with Lv.0 cost 0 as alternative digivolution path.
- **BT22-032** [high]: alt_paths should be: kind: digivolve, from: { level_eq: 2, color_is: yellow }, cost: 2 (Light Blue Lv.2 circle with cost 2 shown on card face)
- **BT22-036** [high]: alt_paths[0].from.level_eq should be 3 (not 4). Card image shows Lv.3 circle in yellow with digivolution cost 3.
- **BT22-040** [high]: alt_paths[0].from.level_eq should be 3 (not 5). Card image shows yellow Lv.3 cost circle with cost 3. Full corrected line: from: { level_eq: 3, color_is: yellow }
- **BT22-041** [high]: evo_costs should be: [{"card_color": 2, "level": 4, "memory_cost": 4}]. The card face clearly shows a yellow digivolve cost circle marked "Lv.4" with cost "4" in the circle, not Lv.5.
- **BT24-011** [high]: alt_paths should contain only ONE entry: kind=digivolve, from={level_eq: 3, trait_has: TS}, cost: 2, ignore_requirements: true. Remove the first alt_path entry at lines 13-15 which incorrectly allows 
- **BT24-101** [high]: alt_paths[1] (trait_has: TS digivolution): cost should be 5, not 3
- **BT25-054** [medium]: No correction needed. Current YAML alt_paths are accurate: (1) Lv.4 Green cost 4; (2) Lv.4 with TS trait cost 3. Both paths match printed card image exactly.
- **EX10-033** [high]: alt_paths: Black Lv.5 cost should be 3 (not 4). Current YAML line 84 "cost: 4" should be "cost: 3" to match the card image and cards.json memory_cost value.
- **EX8-074** [high]: yaml_alt_paths should specify: { kind: digivolve, from: { level_eq: 5, color_is: 'red' }, cost: 4 }. The card face clearly displays a red-filled Lv.5 Cost 4 digivolution circle, which requires color_i
- **ST1-07** [high]: alt_paths[0].from should be { level_eq: 3, color_is: red } instead of { level_eq: 3 }. The card image clearly shows a RED digivolution cost circle for Lv.3 Cost 2, and the JSON evo_costs already speci
- **ST1-09** [high]: alt_paths should be: {kind: digivolve, from: {level_eq: 3, color_is: red}, cost: 3}. The card image shows a red-filled Lv.3 digivolution cost circle. Current YAML has level_eq: 4 (incorrect; card show

## MISSING_ALT_PATH (33)
- **AD1-004** [high]: yaml_alt_paths should include: (1) Lv.5 digimon with [Greymon] in name: Cost 3, and (2) Lv.5 digimon with [ADVENTURE] or [HERO] trait: Cost 3. C# code at lines 27-48 implements this correctly as a com
- **BT12-028** [high]: Verified correct
- **BT12-031** [high]: YAML alt_paths should include: alt_paths: [ { from: { name_is: "Imperialdramon: Dragon Mode" }, cost: 2 } ] — This named-source alternative digivolution condition is printed on the card face in the bl
- **BT16-028** [high]: yaml_alt_paths should include: alt_path for digivolve from [Paildramon]/[Dinobeemon] at cost 3. Card image shows this in black text in the effect box: "[Digivolve] [Paildramon]/[Dinobeemon]: Cost 3". 
- **BT16-040** [high]: alt_path for level_eq: 2 should include colour constraint Blue. Current YAML line 11-12 should read: `from: { level_eq: 2, colour_is: [blue] }` with `cost: 1`. The printed card shows a single blue Lv.
- **BT16-102** [high]: alt_paths should include: kind: digivolve, from: { level_eq: 5, color_count: 2, name_contains: "Magnamon" }, cost: 5. Current YAML has only the standard Yellow Lv.5 cost 4 path; the 2-color Magnamon c
- **BT17-007** [high]: alt_paths should include both: (1) the existing name-based path [from: {name_is: Koromon}, cost: 0] AND (2) a standard level-color path [level_eq: 2, color_is: red, cost: 0]. The card image shows a di
- **BT18-064** [high]: yaml_alt_paths should include: alt_path with name_contains "Sephirothmon" and cost 0. Card face prints '[Digivolve] [Sephirothmon]: Cost 0' in the effect box, and C# code at BT18_064.cs lines 26-28 co
- **BT19-075** [high]: alt_path entry missing: {name_contains: 'Millenniummon', cost: 2} — Card face prints '[Digivolve] [Millenniummon]: Cost 2' in effect box; C# implementation confirms AddSelfDigivolutionRequirementStati
- **BT20-021** [medium]: yaml_alt_paths should include DNA Digivolve entry: [DNA Digivolve] Lv.6 w/Jesmon + Lv.6 w/Gankoomon, Cost 0. The card face clearly prints this condition in the effect box, and the C# implementation at
- **BT20-102** [high]: evo_costs must include standard digivolution paths: Blue Lv.6 cost 6, Red Lv.6 cost 6 (both visible as numbered circles on the card image). These are baseline requirements, not alt-paths; they should 
- **BT22-041** [high]: alt_paths should include TWO digivolution conditions:
1) Standard: kind=digivolve, from={level_eq: 5, color_is: yellow}, cost: 4
2) Alt-digivolve: kind=digivolve, from={level_eq: 5, color_is: any, nam
- **BT23-059** [high]: alt_paths should contain TWO entries: (1) kind: digivolve, from: { name_in: ["Justimon: Accel Arm", "Justimon: Critical Arm"] }, cost: 1; (2) kind: digivolve, from: { level_eq: 5, trait_has: CS }, cos
- **BT24-018** [high]: alt_paths should contain two entries: (1) existing "kind: digivolve, from: { level_eq: 6, color_is: red }, cost: 4", and (2) NEW alternate digivolve with condition "While you have [Owen Dreadnought], 
- **BT24-062** [high]: alt_paths should have entry: { "condition": "Lv.4 w/[Machine]/[Cyborg]/[TS]", "cost": 3 }. Currently evo_costs is empty and the condition text is in xros_req field instead of being properly structured
- **BT25-025** [medium]: evo_costs should include: (1) standard path [Blue Lv.4 cost 4], (2) Aegiomon-named alt-path [any level, Aegiomon source required, cost 3]. The YAML alt_paths section is missing the Aegiomon cost-3 ent
- **BT25-042** [high]: evo_costs should be [{ color: yellow, level: 5, cost: 4 }]. The current YAML lacks the standard color-restricted digivolution path. Only the trait-based alt-path (Lv.5 with [Angel]/[Archangel]/[TS], c
- **BT25-051** [medium]: alt_paths should include TWO digivolution paths: (1) standard Green Lv.3 Cost 3 [no additional requirements]; (2) Green Lv.3 Cost 2 with trait_has: TS requirement. Current YAML only has the TS-gated c
- **BT25-053** [medium]: alt_paths should include: { kind: digivolve, from: { name_contains: "Aegiomon" }, cost: 3 } — this Aegiomon-named cost-3 alt-digivolution is printed on the card face (shown as "Digivolve: [Aegiomon] C
- **BT25-069** [high]: alt_paths should include: { level: 3, trait_requirement: TS, cost: 2 }. The card image clearly shows "[Digivolve] Lv.3 w/[TS] trait: Cost 2" and the C# code (line 18-21) implements exactly this condit
- **EX11-074** [high]: alt_paths should include a second digivolution path: Cost 6 digivolution from Lv.6 with condition requirement: While you have [Shoto Kazama], [GrandGalemon]. Current YAML only lists the standard green
- **EX4-005** [medium]: evo_costs (standard paths) should be: [Red Lv.3 Cost 0, Blue Lv.2 Cost 1, Yellow Lv.1 (cost unlabeled in image)]. These are the three colored cost circles printed on the card. The YAML currently omits
- **EX9-012** [high]: alt_paths should contain three digivolve paths:
1. kind: digivolve, from: { level_eq: 4, color_is: red }, cost: 4
2. kind: digivolve, from: { card_name_contains: "MetalGreymon" }, cost: 1
3. kind: dig
- **EX9-019** [high]: alt_paths should contain three digivolve paths, not one:
  - kind: digivolve
    from: { level_eq: 4, color_is: blue }
    cost: 4
  - kind: digivolve
    from: { name_contains: "WereGarurumon" }
    
- **ST19-07** [high]: yaml_alt_paths is missing an entry. Card image shows: Lv.5 yellow digimon Cost 2 circle in upper-left area. Current YAML only accounts for Lv.3 yellow Cost 2. Correction needed: add alt_paths entry fo
- **ST19-09** [high]: yaml_alt_paths should include: kind: digivolve, from: { level_eq: 6, color_is: yellow }, cost: 6 (in addition to the existing Lv.4 cost 3 entry). The card image shows a yellow Lv.6 circle in the upper
- **ST19-10** [medium]: yaml_alt_paths should include TWO entries:
1. Standard: kind: digivolve, from: { level_eq: 4, color_is: yellow }, cost: 4
2. Alternate (MISSING): kind: digivolve, from: { level_eq: 4, name_contains_an
- **ST19-11** [high]: alt_paths should include a second digivolution entry: from level 7 (likely blue/non-yellow color) with cost 7, in addition to the existing level 4 yellow cost 3 path. The card face shows two digivolut
- **ST19-12** [high]: alt_paths should include: - kind: digivolve, from: { level_eq: 11, color_is: yellow }, cost: 11. The yellow Lv.11 cost 11 circle is clearly visible in the upper left of the card image but missing from
- **ST5-06** [high]: alt_paths should contain: kind: digivolve, from: { level_eq: 2, color_is: blue }, cost: 2. The card image clearly shows a BLUE Lv.2 Cost 2 digivolution circle, but the current YAML entry is black Lv.3
- **ST5-09** [high]: alt_paths should include two digivolution routes: (1) black Lv.4 Cost 3 [existing], AND (2) blue Lv.3 Cost 3 [missing]. In YAML: add second alt_path entry with `from: { level_eq: 3, color_is: blue }` 
- **ST6-11** [high]: evo_costs should include two paths: [Purple Lv.4 Cost 3 (currently present), Purple Lv.3 Cost 7 (currently missing)]. In YAML alt_paths format: add a second entry with level_eq: 3, cost: 7, and color_
- **ST9-06** [high]: alt_paths must include: { kind: digivolve, from: { level_eq: 4, color_is: green }, cost: 4 } in addition to the existing Blue Lv.5 Cost 4 path. Also, cards.json evo_costs must add { card_color: 3 (Gre
