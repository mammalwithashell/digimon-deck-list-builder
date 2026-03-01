# Digimon TCG Rules Reference

Sources:
- Comprehensive Rules Manual Ver.3.6 (2025/12/25) — `Digimon TCG resources/general_rule.pdf`
  - Web: https://world.digimoncard.com/rule/pdf/general_rule.pdf?20251225
- Official Rule Manual for Web Ver.5.0 — `Digimon TCG resources/manual.pdf`
  - Web: https://world.digimoncard.com/rule/pdf/manual.pdf?20250711
- Glossary — `Digimon TCG resources/glossary.pdf`
  - Web: https://world.digimoncard.com/rule/pdf/glossary.pdf?20220422

Rule numbers (e.g. 16-3) reference the Comprehensive Rules Manual sections.

---

## 1. Game Overview & Victory/Loss Conditions

### Victory Conditions (1-2-3)
- **Security knockout**: A successful attack on opponent with 0 security cards by a Digimon that can perform 1+ security checks (1-2-3-1, 11-5-1-2)
- **Deck-out**: Opponent has 0 cards in deck and can't draw during draw phase (1-2-3-2)

### Other Loss Conditions
- **Forfeit**: Either player may declare forfeit at any time; immediate loss (1-2-4). Forfeit doesn't trigger or activate effects (1-2-5)
- **Effect-induced**: Some card effects can cause a player to win or lose (1-2-6)

### Fundamental Principles (1-3)
- Card text that conflicts with rules takes priority over the rules (1-3-1)
- If requested to perform an impossible action, skip it; perform as many as possible (1-3-2)
- If an object is requested to change to a state it's already in, nothing happens (1-3-3)
- If processing would be 0 or fewer times, it can't be performed (1-3-4)
- When multiple players must make simultaneous choices, turn player chooses first (1-3-5)
- When choosing cards by a rule/effect, at least 1 must be chosen (1-3-6)
- Numerical modifications always result in integers (1-3-7)
- Multiple DP modifications: calculate total modifier first, then apply to original (1-3-8)
- Costs can't go below 0 even if reduced by effects (1-3-9)
- When shuffling, if a card to be shuffled is public, make it private first (1-3-10)

---

## 2. Game Areas (Section 3)

### Public Areas (3-1-2-1)
All card information visible to both players: **Field** (battle area + breeding area), **Trash**

### Private Areas (3-1-2-2)
Card information hidden from both players: **Deck**, **Digi-Egg Deck**, **Hand** (visible to owner only), **Security Stack**

### Area Rules
- **New Card Rule** (3-1-3-1): When a card moves from one area to another, it becomes a "new card" — a different card from what it was in the previous area. Previous effects, states, and targeting are lost.
- Card count in each area is always public information (3-1-3-2)
- When multiple cards leave an area simultaneously, the card owner chooses placement order (3-1-3-4)

### Deck (3-2)
- Private area. Players can't change card order. When moving multiple cards from deck to another area, move 1 at a time but consider them placed simultaneously (3-2-4)

### Digi-Egg Deck (3-3)
- Private area. 0-5 cards, max 4 copies of same card number

### Field (3-4)
- Public area. Divided into **breeding area** and **battle area**
- Cards placed unsuspended by default (3-4-4)

### Breeding Area (3-4-6) — CRITICAL IMMUNITY RULES
- Only 1 card can be in the breeding area (3-4-6-2)
- **Cards can't be affected by effects** unless the effect explicitly specifies/references breeding areas (3-4-6-3)
- **Effects on cards can't trigger or activate** unless the effect explicitly specifies/references breeding areas (3-4-6-4)
- **Cards can't be chosen for effects** unless the effect explicitly specifies/references breeding areas (3-4-6-5)
- Trigger conditions can't be met by cards in breeding areas (3-4-6-6)
- Activation conditions can't be met by cards in breeding areas (3-4-6-7)
- Information on cards in breeding areas can't be referenced (3-4-6-8)

### Battle Area (3-4-7)
- Any number of cards can be placed here

### Hand (3-5)
- Private area; card owner can freely view their hand

### Trash (3-6)
- Public area. Players can change card order in their own trash

### Security Stack (3-7)
- Private area. Cards face-down, spread so card count is visible. Players can't change order. If a face-up card is placed in security, it becomes public information (3-7-4)

---

## 3. Game Terminology (Section 4)

### Memory (4-1)
- Shared gauge, 0 at center, 1-10 on each side. Maximum 10; can't exceed (4-1-2-2)
- "X or less memory" = X on your side or further right (toward opponent) (4-1-2)
- "Gain X memory" = move marker X toward your side (left) (4-1-4)
- "Lose X memory" = move marker X toward opponent's side (right) (4-1-4)

### Digimon (4-2)
- Digimon cards and Digi-Egg cards placed on the field are called "Digimon"
- A Digimon gains inherited effects from its digivolution cards (4-2-4)
- A Digimon gets link DP and link effects from its link card (4-2-5, 4-2-6)
- Max 1 link card per Digimon (4-2-7)
- When a Digimon leaves the field, only the top card moves; all digivolution and link cards are trashed (4-2-8)

### Tamers (4-3)
- Tamer cards on the field are treated as Tamers
- When placing a card under a Tamer that already has stacked cards, new card goes to bottom (4-3-2)

### Security Digimon (4-4)
- A Digimon card flipped from security during a security check becomes a Security Digimon
- Security Digimon are NOT normal Digimon — they can't be targeted by effects that target "Digimon" (implied by 13-1-6, 15-1-8)
- Security Digimon aren't deleted when they lose battle — they're just trashed after (14-2-3)

### Stacked Cards (4-5)
- All cards in a stack on the field (1 card alone is not "stacked cards")
- Stacking order can't be changed (4-5-3)
- Bottom cards of a stack are part of the top card's information (4-5-7)
- A face-down card under another card has no card information and can't be referenced (4-5-9)

### Digivolution Cards (4-6)
- Cards placed under a Digimon. When referencing digi card info, use the cards treated as digi cards (4-6-2)

### Link Cards (4-7)
- Card plugged in sideways. Not a card on the field (4-7-4). Not a stacked/digi card
- If a card with a link card becomes a new card, link card is trashed (4-7-6)

### Deletion (4-13)
- Deletion processing = the card is trashed (4-13-1)
- When a trigger-type effect triggers on deletion, it triggers in the area where the card was deleted, pending activation until the card moves to trash (4-13-2)

### Trashing (4-14)
- Trashing = placing a card in the trash (4-14-1)
- **Trashing is NOT deletion** (4-14-3) — important distinction for [On Deletion] effects

### Moving (4-15)
- Moving = moving between breeding area and battle area only (4-15-1)
- Only Digimon with DP can be moved (4-15-2)
- Moving keeps display format (suspended/unsuspended) (4-15-3)
- Effects applied before the move are kept after the move (4-15-4)
- Effect status (pending activation) is NOT carried over when moved (4-15-5)

### Overflow (4-17)
- When a card with Overflow moves from field or from under a card to another area, pay the memory cost
- Overflow is immediate — even if processing for another action is ongoing (4-17-2)
- Overflow does NOT trigger when moving from another area TO the field (4-17-3)
- Overflow does NOT trigger when moving from one card to under another card (4-17-4)
- Multiple Overflow: turn player chooses and processes 1 at a time, then non-turn player (4-17-5)

### Color Requirements (4-19)
- To use an Option card, you must have a Digimon or Tamer of matching color on your field (battle area or breeding area) (4-19-2)
- Multicolor Options require all colors to be met (4-19-3)
- **Option from security: color requirements are ignored** (4-19-5)

---

## 4. Turn Structure (Section 6)

### Phase Order (6-1-2)
**Unsuspend Phase** → **Draw Phase** → **Breeding Phase** → **Main Phase**

### Turn End Conditions (6-1-4)
The turn ends when memory is at 1 or more on opponent's side AND all processing for the current phase is resolved. The turn ends with the current phase.

### Unsuspend Phase (6-2)
- Unsuspend all of turn player's Digimon and Tamers simultaneously
- [Start of Your Turn] and [Start of Opponent's Turn] effects trigger and activate BEFORE unsuspending (6-2-1-1)
- If processing at turn start causes turn end conditions to be met, the end of turn occurs before unsuspending (6-2-1-2)

### Draw Phase (6-3)
- Draw 1 card. First player skips draw on their first turn (6-3-1-1)
- A card MUST be drawn (mandatory). If deck has 0 cards, that player loses (deck-out)

### Breeding Phase (6-4)
Exclusive choice — ONE of the following (6-4-1):
1. **Hatch**: Flip top Digi-Egg deck card face-up into breeding area (only if breeding area is empty and Digi-Egg deck has cards)
2. **Move**: Move Digimon from breeding area to battle area (only if it has DP)
3. **Do nothing**: Proceed to main phase
- Moving from breeding is NOT considered "playing" — no summoning sickness, no [On Play] triggers

### Main Phase (6-5)
The turn player can perform any of these actions in any order, any number of times:
- A. Play a Digimon/Tamer card from hand
- B. Digivolve into a Digimon card from hand
- C. Use an Option card from hand
- D. Link a card in hand or battle area
- E. Attack
- F. Activate activation-type effects
- G. Pass

Actions can only be performed when there is no unresolved processing.

### Pass (6-5-1-7)
After declaring a pass, memory is immediately set to 3 on opponent's side.

### End of Turn (6-6)
- Even if end of turn timing arrives, current phase continues until all processing resolves (6-6-2)
- Once all end-of-turn processing resolves, opponent's turn begins (6-6-3)
- **Memory bounce-back**: If memory moves to 0 or more at end of turn, the end is postponed and current phase continues (6-6-4)

---

## 5. Playing Cards (Section 7)

### Playing a Card (7-1)
1. Declare and reveal the card
2. Pay the play cost
3. Place on field; card playing procedure resolved
- **Summoning sickness**: Cards can't attack the turn they were played (7-1-2-1)
- Immediate-type effects that trigger when played activate immediately after reveal (7-1-2-2)

### DigiXros (7-2)
- When playing a Digimon with DigiXros requirements, place specified cards from hand/battle area under it
- Play cost reduced by the amount specified per card placed
- Declaration made before paying cost; cards chosen simultaneously (7-2-2-3)
- DigiXros is "performed" when 1+ cards are placed under (7-2-2-9)
- Cards from battle area placed under = removed from battle area, digi cards trashed (7-2-2-7)
- Not mandatory — player can choose 0 cards (but if declared, must place at least 1) (7-2-2-12)

### Assembly (7-3)
- When playing a Digimon with Assembly requirements, place specified cards from trash under it
- Play cost reduced by specified amount
- Exact number of specified cards must be placed (7-3-2-4)
- Not mandatory (7-3-2-9)

---

## 6. Digivolution (Section 8)

### Standard Digivolution (8-1)
1. Declare and reveal Digimon card from hand
2. Choose 1 digivolution requirement on the revealed card; choose a field card that meets it
3. Pay the digivolution cost
4. Place revealed card on top of chosen card; draw 1 card
- If a card has multiple digi requirements, player chooses which one (8-1-2-1)
- Carries over display format — suspended stays suspended (8-1-2-4)
- Immediate-type effects trigger after reveal and card selection (8-1-2-5)
- Only 1 digivolution per Digimon at a time (8-1-2-6)
- A card that becomes a digi card is NOT removed from the field (8-1-2-8)

### DNA Digivolution (8-2)
- Digivolve multiple field Digimon into 1 new Digimon by placing them as digivolution cards
- Cards that become digi cards are NEW CARDS — lose previous states (8-2-2-1):
  - Top card digivolves without carrying over display format (8-2-2-1-1)
  - Digi cards become new cards (8-2-2-1-2)
  - If a card was attacking, attacker is removed (8-2-2-1-3)
  - If a card was an attack target, target is removed (8-2-2-1-4)
  - Any gained effects on digi cards end (8-2-2-1-5)
  - Digi cards CAN attack the same turn (8-2-2-1-6)
- Can only be performed by effects that specifically perform DNA digivolution (8-2-2-4)

### Burst Digivolve (8-3)
- Return a Tamer to hand, digivolve for the specified cost
- At end of turn, top card of the burst-digivolved stack is trashed (8-3-2-1)
- Only trashed if it's a Digimon card (8-3-2-3)

### App Fusion (8-4)
- A linked Digimon digivolves using a combination of 2 specified cards (link card + another)
- Link card placed on top of Digimon according to app fusion requirements

---

## 7. Using Option Cards (Section 9)

- Use = play Option card's [Main] effect. Color requirements must be met (9-1-1)
- Used Option card is "not in any area" until its [Main] effect resolves (9-1-4)
- After resolution, used Option card is trashed (unless effect placed it somewhere) (9-1-5)
- Immediate-type effects that trigger when card would be used activate after reveal (9-1-6)

---

## 8. Link (Section 10)

- A card with [Link] can be plugged sideways into a specified Digimon (10-1-1)
- From hand or battle area, meeting link requirements, paying link cost
- Link card is NOT a card on the field (4-7-4)
- Digimon gains link DP and link effects (4-2-5, 4-2-6)
- Max 1 link card per Digimon; new link replaces old (old is trashed) (4-2-7)
- When linking a battle area Digimon, that Digimon is removed → digi cards trashed
- If linked Digimon no longer meets link requirements, link card trashed on rule check (17-1-3-6)

---

## 9. Attacking (Section 11)

### Attack Flow (11-1-3)
1. **Attack Declaration** — Suspend attacker, choose target (opponent player OR opponent's suspended Digimon)
2. **Counter Timing** — Non-turn player may activate 1 [Counter] effect (11-3-2)
3. **Block Timing** — Non-turn player may block with 1 Blocker Digimon (12-1)
4. **Confirming Attack Success** — Determine outcome based on target
5. **End of Attack** — Attack ends, process end-of-attack effects

### Attack Declaration (11-2)
- Turn player suspends 1 Digimon in battle area to attack (11-2-1)
- Target chosen simultaneously: opponent player OR 1 of opponent's suspended Digimon (11-2-7-1)
- 1 Digimon, 1 attack per declaration (11-2-3)
- Can't attack with a Digimon that can't suspend (11-2-5)
- Even if attack target is removed, it remains the target (attack fails) (11-2-6)
- **Even if the attacking Digimon is removed, all attack timings still occur** (11-1-5)

### Blocking (12-1)
- 1 block per attack. Blocker suspends, target switches to blocker (12-1-1, 12-1-7-1)
- Attack target Digimon can't perform a block (12-1-5)
- Can only block if attacking Digimon is in battle area (12-1-6)

### Confirming Attack Success (11-5)
- **A. vs Player (has security)**: Attacker performs security check (11-5-1-1)
- **B. vs Player (0 security)**: Attacking player wins the game — BUT only if attacker can perform security checks (11-5-1-2)
- **C. vs Digimon**: Battle occurs (11-5-1-3)
- **D. Attack not successful**: If attack target was removed or conditions not met, nothing happens (11-5-1-4)

---

## 10. Security Checks (Section 13)

- Performed 1 at a time (13-1-2). Mandatory if to be performed (13-1-3)
- If the Digimon performing the check is removed from battle area, no more checks (13-1-4)
- A checked card is removed from security and is "not in any area" during processing (13-1-5)

### Security Check Procedure (13-1-7)
1. Reveal top security card (13-1-7-1)
2. Process triggered [Security] effects and other effects from the check (13-1-7-2)
3. If Security Digimon present: battle occurs (13-1-7-3). If not, skip
4. Checked card placed in trash (unless an effect placed it elsewhere) (13-1-7-4)
5. If attacker can perform another check, repeat (13-1-7-5)

### Security-Specific Rules
- [Security] effects activate immediately, no cost (from manual)
- **Color requirements for Option cards are IGNORED from security** (4-19-5)
- Security Digimon are NOT deleted when losing battle (14-2-3) — just trashed after
- A checked Digimon card is treated as a Security Digimon (13-1-6)

---

## 11. Battles (Section 14)

### Battle Procedure (14-2)
1. Compare DP of two battling cards (14-2-1)
   - Higher DP wins (14-2-1-1)
   - Lower DP loses (14-2-1-2)
   - Same DP = both lose (14-2-1-3)
2. Losing Digimon is immediately deleted. If both lose, both deleted (14-2-2)
3. Security Digimon are NOT deleted even when they lose (14-2-3)
4. Effects triggered by battle resolve before next action (14-2-4)
5. End of battle timing effects resolve (14-2-5)
6. Battle doesn't end until all processing resolves (14-2-6)

---

## 12. Effect Rules (Section 15) — CRITICAL FOR IMPLEMENTATION

### Effects Overview (15-1)
- An effect is the processing activated by a card that affects the game (15-1-1)
- A single effect is processed in the order shown in card text (15-1-2)
- **Prohibiting effect > enabling effect** (15-1-3)
- If an effect doesn't specify an area, it can specify/reference a card in the battle area or affect the battle area (15-1-7)
- Effects CAN specify/reference/affect Security Digimon if they say so (15-1-8)

### Effect Types (15-2)
- Digimon card/Digimon effect → Digimon effect (15-2-1-1)
- Tamer card/Tamer effect → Tamer effect (15-2-1-2)
- Option card effect → Option card effect (15-2-1-3)

### Inherited Effects (15-3)
- An inherited effect is gained from a digivolution card. Can only be activated by 1 card (the Digimon on top) (15-3-1)
- Inherited effects are treated as Digimon effects regardless of the digi card's category (15-3-2)
- If "this card" is specified in an inherited effect, it refers to the card itself (as a digi card) (15-3-3)

### Effect States (15-4)

#### Activation (15-4-1)
Activation = an effect being executed

#### Triggering (15-4-2)
- Triggering = when conditions are met for an effect to trigger
- Once trigger conditions are met, the effect WILL trigger regardless of ongoing processing (15-4-2-2)
- Triggered effects pending activation must be activated 1 at a time (15-4-2-3)
- If a card with pending activation becomes a new card, the effect can no longer activate (15-4-4-3)
- If a card with pending activation loses the effect, it can no longer activate (15-4-4-4)

#### Simultaneous Triggering (15-4-3)
- When multiple effects trigger simultaneously:
  1. Turn player chooses 1 of their triggered effects to activate (15-4-3-5-1)
  2. Repeat until all turn player effects resolved
  3. Non-turn player chooses 1 of their triggered effects to activate (15-4-3-5-2)
  4. Repeat until all non-turn player effects resolved
- When triggered by a rule check, effects trigger simultaneously with other effects at that timing (15-4-3-3)

#### Derived Triggering (15-4-5)
- A derived trigger = new trigger that occurs while simultaneous triggers are still resolving
- Derived triggers activate BEFORE pending earlier triggers (15-4-5-2)
- Non-turn player derived triggers activate before turn player's pending triggers (15-4-5-3)

#### Pending Activation (15-4-4)
- Period from trigger until activation
- Lost if: card becomes new card (15-4-4-3), card loses the effect (15-4-4-4), card no longer meets trigger conditions (15-4-4-5)

### Trigger Conditions (15-5)
- Trigger once per trigger condition, even if the event occurred multiple times simultaneously (15-5-2)
- If trigger conditions are met as soon as a card is placed where it can trigger, the effect triggers (15-5-3)

### Processing Conditions (15-6)
- Shown with "if" or "while" text. Processing can execute when conditions are met (15-6-1)
- A processing condition is for a specific process — different processes in the same effect don't need the same conditions (15-6-2)
- An effect can't activate when NONE of its processing conditions are met (15-6-3)

### Optional Processing Conditions (15-7)
- Shown with "by" text (e.g., "By trashing 1 card in the hand, gain 1 memory") (15-7-1)
- If optional content can't be executed, processing after conditions can't execute either (15-7-2)
- Can't partially perform optional processing conditions (15-7-3)
- Player CAN choose to execute optional conditions even if following content can't execute (15-7-4)
- Player CAN choose to execute optional conditions regardless of whether content after can execute (15-7-5)

### Effect Categories (15-8)

#### Persistent Effects (15-8-2)
- Constantly activated while conditions met (e.g., "[Your Turn] This Digimon gets +1000 DP")
- Deactivate as soon as conditions are no longer met (15-8-2-3)
- Multiple persistent effects with conflicting content: effects activated afterward take priority, except prohibiting effects (15-8-2-5)
- With processing conditions: constantly activated while processing conditions met; deactivate when not met (15-8-2-6)

#### Trigger-Type Effects (15-8-3)
- Always trigger when conditions met; effect activates (15-8-3-1)
- **Can't activate during processing for a rule or effect** (15-8-3-2)
- Will trigger when specific conditions on that card are met (15-8-3-3)
- Won't trigger when conditions aren't met (15-8-3-4)
- If trigger-type triggers when card is deleted, pending activation card = the top card in original area (15-8-3-5)
- If triggers at end of turn timing, remains triggered even if memory changes (15-8-3-6)
- Unspecified references made in the state when processing is being performed (15-8-3-7)
- References in same state as trigger conditions: use state when triggered (15-8-3-8)

#### Activation-Type Effects (15-8-4)
- Can be optionally activated by the player (e.g., [Main] effects)
- Can only be activated during main phase when there is no unresolved processing (15-8-4-2)
- With processing conditions: can only declare when conditions met (15-8-4-3-1)
- With optional processing conditions: can only declare when optional conditions can be performed; once declared, must be performed (15-8-4-4-1)

#### Immediate-Type Effects (15-8-5)
- Trigger as soon as conditions met, then **interrupt before the cause** (15-8-5-1)
- "When X would" or "when X would be removed" text (15-8-5-1)
- Only trigger simultaneously with OTHER immediate-type effects (15-8-5-3)
- Each immediate-type activated 1 at a time until the first cause is resolved (15-8-5-4)
- With processing conditions: always triggers; can activate if processing conditions are later met (15-8-5-5)

### Mandatory vs Optional Processing (15-9)
- **Mandatory**: Player MUST execute the content (15-9-1-2)
- **Optional**: Player CAN choose whether to execute (15-9-2-2)

### Effect Targets (15-10)
- "You"/"Your" = player of that card (15-10-1-1)
- "Opponent" = opponent player (15-10-1-2)
- "X Digimon" or "X cards" = choose that many; same target can't be chosen multiple times simultaneously (15-10-2-3)

### Individual vs Overall Processing (15-11)
- **Individual**: A target is chosen; processing affects that target (15-11-1-1). If target becomes new card, processing is lost (15-11-1-2)
- **Overall**: No target chosen; processing affects cards overall (e.g., "All of your opponent's Digimon get -5000 DP") (15-11-2-1). Continuous — affects cards added later too (15-11-2-2)

### Effects That Add/Change Information (15-12)
- "Add information" = add to a card's data (name, level, DP, etc.) (15-12-1)
- Can only add to 1 card at a time; new info overwrites previous (15-12-1-3)
- "Change information" = modify existing data (15-12-2). Can't change info a card didn't originally have (15-12-2-2)

### Gained Effects (15-13)
- Effects gained through other effects carry over state even if card is placed on top of stack or removed from stack (15-13-2)

### Effect Icons (15-14)
- **[X Per Turn]**: Can activate X times per turn per copy of card (15-14-1-3). If card becomes new card, can activate again (15-14-1-4)
- **{Hand}**: Can activate when revealing card from hand (15-14-2)
- **{Trash}**: Can trigger/activate while card is in trash (15-14-3)
- **{Breeding}**: Can trigger/activate while card is in breeding area (15-14-4)
- **{Security}**: Can trigger/activate while card is face-up in security stack (15-14-5)

### "Isn't affected by effects" (15-15-6)
- Card won't be affected by processing caused by effects (15-15-6-1)
- Can still be chosen for effects (just not affected) (15-15-6-3)
- Even if it gains an effect, it won't be considered to have that effect (15-15-6-4)

---

## 13. Effect Timings (15-16)

| Timing | Description | Rule |
|--------|-------------|------|
| [On Play] | When the action of playing a card is complete | 15-16-2 |
| [When Digivolving] | When digivolution into a card with the effect is complete | 15-16-3 |
| [On Deletion] | When the card with the effect is deleted | 15-16-4 |
| [When Attacking] | When an attack declaration is made for the card | 15-16-5 |
| [When Linking] | When the card becomes a link card | 15-16-6 |
| [Main] | Activation-type effect timing | 15-16-7 |
| [Your Turn] / [Opponent's Turn] | Scope: effects trigger/activate during specified turn | 15-16-8 |
| [All Turns] | Effects trigger/activate during both turns | 15-16-9 |
| [Security] | When a security check is performed on the card | 15-16-10 |
| [Start of Your Turn] | At your unsuspend phase, BEFORE unsuspending | 15-16-11 |
| [Start of Opponent's Turn] | At opponent's unsuspend phase, BEFORE unsuspending | 15-16-11-2 |
| [End of Your Turn] | When your turn ends | 15-16-12-1 |
| [End of Opponent's Turn] | When opponent's turn ends | 15-16-12-2 |
| [End of All Turns] | When any turn ends | 15-16-12-3 |
| [Start of Your Main Phase] | When your main phase begins | 15-16-13-1 |
| [Start of Opponent's Main Phase] | When opponent's main phase begins | 15-16-13-2 |
| [Counter] | During counter timing for opponent's turn | 15-16-14 |
| [End of Attack] | When end of attack timing arrives | 15-16-15 |
| [When Moving] | When the card with the effect is moved | 15-16-16 |

### Important Timing Notes
- [Security] effects immediately activate without pending activation; take precedence over simultaneous effects (15-16-10-2)
- [Start of Turn] effects trigger BEFORE unsuspending actions (15-16-11-1)

---

## 14. Keyword Effects (Section 16)

### Security Attack +X / Security A. +X (16-3)
- **Type**: Persistent
- **Processing**: Mandatory
- Modifies the number of security checks. Multiple instances add their values separately (not combined into one effect) (16-3-3)
- Negative result = 0 security checks (16-3-4)

### Blocker (16-4)
- **Type**: Persistent
- **Processing**: Mandatory
- Allows switching attack target to self by suspending
- Multiple Blockers = still only 1 block per attack (16-4-3)

### Recovery +X (16-5)
- Move specified number of cards from specified area (usually deck) to top of security stack face-down
- If "Deck" specified, top card of deck goes to security (16-5-3)
- Execute processing (16-5-2)

### Piercing (16-6)
- **Type**: Trigger-type
- **Processing**: Mandatory
- After battle with opponent's Digimon where opponent's Digimon is deleted: perform security check (if opponent has security and attack is on opponent player and successful)
- Processed at end of attack timing; triggers simultaneously with other end-of-battle effects (16-6-4)
- Multiple Piercing instances: only the 1st can trigger right before end of attack; 2nd can't trigger again (16-6-5)
- Piercing can't perform check if opponent has 0 security at time of processing (16-6-6)

### Draw X (16-7)
- Draw X cards from deck. Execute processing. Mandatory (16-7-2, 16-7-3)

### Jamming (16-8)
- **Type**: Persistent
- Digimon with Jamming is NOT deleted as result of battle with Security Digimon

### Digisorption -X (16-9)
- **Type**: Immediate-type
- **Processing**: Optional (the "suspend 1 Digimon" part)
- When digivolving into a card with this effect in hand, may suspend 1 of your Digimon to reduce digi cost by X
- Multiple instances overlap — can activate multiple for a single digivolution (16-9-4)
- Can suspend a Digimon to be used as the digivolution target (16-9-5)

### Reboot (16-10)
- **Type**: Persistent
- **Processing**: Mandatory
- Digimon unsuspends during OPPONENT's unsuspend phase
- Only unsuspended once even with multiple Reboot instances (16-10-3)
- Unsuspending performed at same time as turn player's unsuspend (16-10-5)

### De-Digivolve X (16-11)
- Trash up to X cards from top of chosen stack (starting from top card)
- Execute processing. Mandatory — can't choose to trash 0 (16-11-3)
- **Can't trash cards from level 3 or lower** (16-11-4)
- If multiple cards trashed in 1 De-Digivolve, all considered trashed simultaneously (16-11-5)

### Retaliation (16-12)
- **Type**: Trigger-type
- **Processing**: Mandatory
- When the Digimon with this effect is deleted in battle, delete the battled opponent's Digimon
- Multiple instances overlap (16-12-4)

### Digi-Burst X (16-13)
- Trash X digivolution cards from the Digimon to activate another effect
- Processing is optional (16-13-2)

### Rush (16-14)
- **Type**: Persistent
- Digimon can attack the same turn it was played

### Blitz (16-15)
- **Type**: Trigger-type (with processing condition)
- **Processing**: Optional ("this Digimon may attack")
- Can attack if opponent has 1 or more memory (on opponent's side)
- After using Blitz to declare an attack, the attack proceeds even if memory returns to 0+ (16-15-4)
- Can't use Blitz if memory is already at 0 or more upon activation (16-15-5)

### Delay (16-16)
- While the card with Delay is in battle area, by trashing that card, activate the specified effect
- Processing is optional (16-16-2)
- Can't be activated the same turn the card is placed in battle area (16-16-3)

### Decoy (Color) (16-17)
- **Type**: Immediate-type
- **Processing**: Optional ("by deleting this Digimon")
- When another of your Digimon specified by this effect would be deleted by opponent's effect, delete self to prevent it

### Armor Purge (16-18)
- **Type**: Immediate-type
- **Processing**: Optional ("by trashing the top card of the Digimon")
- When Digimon with this effect would be deleted, trash top card to prevent deletion
- Multiple instances overlap (16-18-4)

### Save (16-19)
- Place this card under 1 of your Tamers when a Digimon with this effect is deleted
- Processing is optional (16-19-3)

### Material Save X (16-20)
- **Type**: Immediate-type
- **Processing**: Optional (but if processed, must place the specified number of cards)
- When Digimon with this effect would be deleted, place X cards from digi cards (specified in DigiXros requirements) under 1 of your Tamers
- Multiple instances overlap (16-20-4)

### Evade (16-21)
- **Type**: Immediate-type
- **Processing**: Optional ("by suspending a Digimon with this effect")
- When Digimon would be deleted, suspend self to prevent deletion
- Multiple instances overlap (16-21-4)

### Raid (16-22)
- **Type**: Trigger-type
- **Processing**: Optional
- When attacking, may switch target to opponent's unsuspended Digimon with highest DP
- If multiple tied for highest DP, player that activated Raid chooses (16-22-5)

### Alliance (16-23)
- **Type**: Trigger-type
- **Processing**: Optional ("by suspending 1 of your other Digimon")
- When attacking, suspend another of your Digimon to add its DP to attacker AND gain Security A. +1
- DP added = value at time of suspension, doesn't change even if suspended Digimon's DP changes later (16-23-5)
- If suspended Digimon is removed during attack, added DP and Security A. +1 remain (16-23-6)
- Multiple Alliance can be activated for a single attack (16-23-4)

### Barrier (16-24)
- **Type**: Immediate-type
- **Processing**: Optional ("by trashing the top card of your security stack")
- When Digimon would be deleted in battle, trash top security card to prevent deletion
- Multiple instances: up to the number of triggered Barrier instances can be activated (trash that many security cards) (16-24-4)

### Blast Digivolve (16-25)
- Digivolve into the card with this effect during counter timing without paying cost
- Processing is optional (16-25-3)
- Digivolves a Digimon in the battle area (16-25-4)

### Fortitude (16-26)
- **Type**: Trigger-type
- **Processing**: Mandatory
- When a Digimon with digi cards and this effect is deleted, play self without paying cost
- Multiple instances overlap (16-26-4)

### Mind Link (16-27)
- Place a Tamer with this effect in the digi cards of a Digimon with no Tamer cards in its digi cards
- Execute processing. Mandatory (but if [Main], player chooses timing) (16-27-3)

### Partition (Color Lv.X & Color Lv.Y) (16-28)
- **Type**: Immediate-type
- **Processing**: Optional
- When Digimon with this effect and 1 of each specified card in digi cards would leave battle area (not by battle), may play 1 of each specified card from digi cards without cost

### Collision (16-29)
- **Type**: Persistent
- While the Digimon with this effect is attacking, all opponent's Digimon gain Blocker and opponent is forced to block when possible
- "Forced to block" affects the opponent player (16-29-4)

### Blast DNA Digivolve (16-30)
- 1 specified Digimon in battle area + 1 card from hand → DNA digivolve into card with this effect during counter timing without cost
- Processing is optional (16-30-3)
- Digimon's DNA digi requirements can't be ignored (16-30-5)

### Scapegoat (16-31)
- **Type**: Immediate-type
- **Processing**: Optional ("by deleting 1 of your other Digimon")
- When Digimon with this effect would be deleted (by your effects), delete another of your Digimon to prevent it

### Vortex (16-32)
- **Type**: Trigger-type (triggers at end of your turn)
- **Processing**: Optional
- Attack an opponent's Digimon at end of turn. Can also attack the same turn it was played

### Overclock (16-33)
- **Type**: Trigger-type (triggers at end of your turn)
- **Processing**: Optional ("by deleting 1 of your Tokens or 1 of your other Digimon specified by this effect")
- Digimon may attack a player without suspending

### Iceclad (16-34)
- **Type**: Persistent
- Compare number of digivolution cards instead of DP in battle (except vs Security Digimon)
- Higher digi card count wins; same count = both lose (16-34-4)

### Decode (Color Lv.X) (16-35)
- **Type**: Immediate-type
- **Processing**: Optional
- When Digimon with this effect would leave battle area (NOT by battle), may play 1 specified card from digi cards without cost

### Fragment (X) (16-36)
- **Type**: Immediate-type
- **Processing**: Optional (choosing and trashing digi cards); if executed, prevention is mandatory
- When Digimon would be deleted, choose and trash X digi cards to prevent deletion

### Execute (16-37)
- **Type**: Trigger-type (triggers at end of your turn)
- **Processing**: Optional
- Digimon may attack; at end of attack, Digimon is deleted. Can attack unsuspended Digimon
- Deletion at end of attack is pending processing (16-37-4)

### Progress (16-38)
- **Type**: Persistent
- "This Digimon isn't affected by your opponent's effects while attacking"
- Activates while the Digimon with this effect is attacking (16-38-3)

### Link +X (16-39)
- **Type**: Persistent
- Adds X to maximum link cards for the Digimon
- Multiple instances: max increases by each value (not combined into one effect) (16-39-3)

### Training (16-40)
- **Type**: Activation-type
- **Processing**: Optional ("by suspending this Digimon during the main phase")
- Suspend to place top card of deck at bottom of this Digimon's digi cards
- Can also activate in breeding area (16-40-1)
- "By suspending" is optional; if executed, placing deck card is mandatory (16-40-3)

---

## 15. Rule Checks (Section 17)

Rule checks are performed for certain circumstances during timings when they are possible.

### Rule Checks NOT Performed During (17-1-2)
- Rule check processing (17-1-2-1)
- Effect processing (17-1-2-2)

### Conditions That Trigger Rule Checks (17-1-3)
- **0 DP Digimon in battle area → deleted** (17-1-3-1)
- **Digimon without DP in battle area → trashed** (NOT deletion) (17-1-3-2)
- **Option card in battle area (unless placed by effect) → trashed** (17-1-3-3)
- **Tamer or Option card in breeding area → trashed** (17-1-3-4)
- **Face-down card on field (except Digi-Egg deck) → trashed** (17-1-3-5)
- **Link card not meeting requirements → trashed** (17-1-3-6)
- **Link cards exceeding limit → excess trashed** (17-1-3-7)

---

## 16. Other Rules (Section 18)

### Pending Processing (18-1)
- Processing that is pending will be processed at the predetermined timing, similar to triggered effects

### Overwrite Processing (18-2)
- "Instead" text replaces normal processing (18-2-1)
- If optional, player can choose whether to use the overwrite (18-2-2)
- Overwrite processing for immediate-type effects can't be interrupted (18-2-4)

### Infinite Loops (18-3)
- If neither player can stop the loop → game is a draw (18-3-2)
- If one player can stop it: turn player repeats a declared number of times, then non-turn player, then processing stops (18-3-3)

---

## 17. Game Preparation (Section 5)

1. Shuffle deck and Digi-Egg deck, place face-down
2. Determine first player (rock-paper-scissors)
3. Both draw 5 cards for initial hand
4. Starting with first player, each may declare a re-draw (once): return all 5, shuffle, draw 5 new
5. Place top 5 cards of deck face-down as security stack (top of deck = bottom of security)
6. Memory gauge at 0
7. First player's turn begins
