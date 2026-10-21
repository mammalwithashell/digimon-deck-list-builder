# Aegis conformance scenario index

Source: `apps/api/src/engine/conformance/*.test.ts` @ `ec0cd22f0`. One line per `it(...)` title; parametrized `it.each(...)` tests are listed once, by their template title, and marked `[each]` (each expands to several runtime cases). Generated; do not edit by hand.

Total: 689 entries across 85 files (647 `it`, 42 `it.each` [each]; each `[each]` entry expands to several runtime cases).

## _kb.meta.test.ts (4)

- _kb.meta.test.ts: every cited id exists in the rules index
- _kb.meta.test.ts: every not-testable id exists in the rules index
- _kb.meta.test.ts: no id is both cited and not-testable
- _kb.meta.test.ts: reports coverage (proxy only — not a pass/fail gate; see README)

## activation-cost-and-borrowed-gates.test.ts (2: 1 it, 1 it.each)

- activation-cost-and-borrowed-gates.test.ts: fires the cost threshold from printed use cost when payment-only reduction reaches zero (Q5462)
- activation-cost-and-borrowed-gates.test.ts: [each] %s: blocks a borrowed When Digivolving effect at the lender (Q5543)

## activation-cost-borrowed-effects.test.ts (2)

- activation-cost-borrowed-effects.test.ts: lets the player decline BT23-045's borrowed optional placement cost
- activation-cost-borrowed-effects.test.ts: lets the player decline the borrowed cost from hand

## activation-cost-compound-assignment.test.ts (2)

- activation-cost-compound-assignment.test.ts: assigns distinct physical trash cards to both placement components
- activation-cost-compound-assignment.test.ts: does not declare an impossible activation-type compound payment

## activation-cost-targetless-payload.test.ts (1)

- activation-cost-targetless-payload.test.ts: pays BT19-086's four-Device compound cost when no Cyberdramon exists

## activation-processing-costs.test.ts (6: 3 it, 3 it.each)

- activation-processing-costs.test.ts: [each] Option use with no evolution destination keeps its processing choice optional: accept=%s
- activation-processing-costs.test.ts: 15-7-5: payable placement may be declared without a subsequent evolution card
- activation-processing-costs.test.ts: 15-8-4-4-1: declaring a payable condition commits its payment before optional evolution
- activation-processing-costs.test.ts: [each] Henry pays his declared condition independently of evolution availability: destination=%s
- activation-processing-costs.test.ts: [each] $tamer publicly orders its complete unpaid placement batch: $permutation
- activation-processing-costs.test.ts: BT15-091 rejects forged or duplicate ordered loose payments atomically

## ch01-game-overview.test.ts (23)

- ch01-game-overview.test.ts: 1-1-1: a match seats exactly two players, seat 0 and seat 1
- ch01-game-overview.test.ts: 1-2-1/1-2-2: the game is not over until a player wins or loses; declareLoss ends it
- ch01-game-overview.test.ts: 1-2-5: a forfeit (surrender) is a real loss cause distinct from a rule/effect loss
- ch01-game-overview.test.ts: 1-2-3-1: an attack with >=1 security check landing on 0 security cards wins the game
- ch01-game-overview.test.ts: 1-2-3-2: a player with an empty deck who must draw loses (deck-out)
- ch01-game-overview.test.ts: 1-3-1: card text overrides the rules — <Rush> lets a just-played Digimon attack immediately
- ch01-game-overview.test.ts: 1-3-3: requesting a card be changed to the state it's already in is a no-op
- ch01-game-overview.test.ts: 1-3-7: a numerical value modified by a rule/effect is always truncated to an integer
- ch01-game-overview.test.ts: 1-3-9: a cost that would become negative floors at 0, never goes negative
- ch01-game-overview.test.ts: 1-4-2/1-3-10 cross-check: memory gauge is clamped to the documented [-10, 10] range
- ch01-game-overview.test.ts: 1-3-11-3: with the conditions met, EX8-074 is declared at 0 memory and both Digimon are suspended to pay 7
- ch01-game-overview.test.ts: 1-3-11-3: the declaration is refused when the optional processing conditions can't be met
- ch01-game-overview.test.ts: 1-3-11-3 control: when the printed cost is payable, the reduction isn't needed and declining it pays 11
- ch01-game-overview.test.ts: 1-3-11-3: after declaring at 0 memory the suspension is mandatory, so no 'you may' prompt opens
- ch01-game-overview.test.ts: 1-3-11-4: a DigiXros is declared when its alternate cost is payable only through an immediate effect that widens the source areas
- ch01-game-overview.test.ts: 1-3-11-4: the DigiXros declaration is refused when the widening effect's processing can't be performed
- ch01-game-overview.test.ts: 1-4-1-2-1: a legal main deck must be exactly 50 cards, not more or fewer
- ch01-game-overview.test.ts: 1-4-1-2-2/1-4-1-3-2: at most 4 copies of a card with the same card number
- ch01-game-overview.test.ts: 1-4-1-3-1: the egg deck may have 0 to 5 cards
- ch01-game-overview.test.ts: 1-4-2-2: the gauge is shared and clamped to 10 on both sides, never exceeding it
- ch01-game-overview.test.ts: 1-4-2-3: 0 is the center — the left/right split is symmetric per-player perspective
- ch01-game-overview.test.ts: 1-4-4-1/1-4-4-2: token cards are non-deck cards used by effects
- ch01-game-overview.test.ts: 1-4-4-5 (structural corollary): a token can be played even without a play cost (Diaboromon has 14)

## ch02-card-information.test.ts (34)

- ch02-card-information.test.ts: 2-2-3..2-2-6: the four card categories are distinct and drive real predicates
- ch02-card-information.test.ts: 2-3-1-4: an ACE card's stored name never literally contains 'ACE'
- ch02-card-information.test.ts: 2-3-1-2: bracket-name filtering (nameOrTrait) matches only that exact card name
- ch02-card-information.test.ts: matches printed 'in any trait' wording by trait substring only
- ch02-card-information.test.ts: 2-3-2-1/2-3-2-2: multiple traits (form/attribute/type) are separate, independently matchable
- ch02-card-information.test.ts: 2-3-3-3: a card that grants Blocker does not itself possess persistent Blocker
- ch02-card-information.test.ts: 2-3-3-4: a conditional Blitz mention does not count as an unconditional keyword declaration
- ch02-card-information.test.ts: 2-3-3-1/2-3-11-1: upper text (effect) is a field distinct from inherited/security text
- ch02-card-information.test.ts: 2-3-4-4-1: an 'also treated as having [Greymon]' card has [Greymon] in its name but is not named [Greymon]
- ch02-card-information.test.ts: 2-3-4-4-1: a 'from [Greymon]' digivolve route is open from a real Greymon, not from the aliased Gaiomon
- ch02-card-information.test.ts: 2-3-4-4-1: BT9-012's identical 'from [Greymon]' route also refuses the aliased Gaiomon
- ch02-card-information.test.ts: 2-3-4-5-1: RB1-004 is also treated as card number P-009, so both printings share one copy budget
- ch02-card-information.test.ts: 2-3-4-6-1: an 'up to 50 copies' card is legal at 50 copies while a normal card stays capped at 4
- ch02-card-information.test.ts: 2-3-5-2: a digivolution cost is a color+level+memory requirement engine code reads
- ch02-card-information.test.ts: 2-3-5-2: every printed digivolution cost names a real card color
- ch02-card-information.test.ts: implemented: dnaDigivolveInto merges 2 battle-area materials into a new permanent carrying both as its digivolution stack
- ch02-card-information.test.ts: implemented: playing EX10-058 via DigiXros reduces its cost per material and places both materials under it
- ch02-card-information.test.ts: implemented: the alternate Cost-0 digivolve is accepted and applied even though the normal cost (5) is unaffordable
- ch02-card-information.test.ts: NOW MET: the returned [Marcus Damon] alternate cost should leave the battle area and land in hand
- ch02-card-information.test.ts: implemented: appFusionCostFor computes a real, printed fusion cost
- ch02-card-information.test.ts: exposes BT26 three-name App Fusion recipes and accepts every distinct ordered pair
- ch02-card-information.test.ts: 2-3-10: a printed Assembly requirement is captured structurally and read by the engine subsystem
- ch02-card-information.test.ts: 2-3-11-4: Link requirement/cost/DP are distinct, independently-populated fields
- ch02-card-information.test.ts: 2-3-11-5: Option information (DUAL card lower text) is a distinct field from the main effect
- ch02-card-information.test.ts: 2-4-3-1/2-4-3-2: a multicolor card matches EITHER of its colors, and can't be treated as lacking one
- ch02-card-information.test.ts: 2-5-2: current (modified) DP is referenced over the printed original once modified
- ch02-card-information.test.ts: 2-6-1: play cost is the memory the engine actually deducts when the card is played
- ch02-card-information.test.ts: 2-7-1: an Option card's playCost field IS its use cost — there is no separate field
- ch02-card-information.test.ts: 2-9-1: the printed level is the value level references compare against
- ch02-card-information.test.ts: 2-9-2: cards with no printed level ('Lv.-') are treated as having no level
- ch02-card-information.test.ts: 2-10-1: an Overflow ACE card costs its controller its printed overflow memory on leave
- ch02-card-information.test.ts: a DUAL card is playable as its Option side (an explicit useAs: "option"), applying its printed Option effect
- ch02-card-information.test.ts: 2-12-1: matching card name AND card number => the same card (copy-limit groups by cardId)
- ch02-card-information.test.ts: 2-13-1: illustration/illustrator/copyright/rarity/block-icon are display-only, not referenced by filters

## ch03-game-areas.test.ts (20)

- ch03-game-areas.test.ts: 3-1-1: the six named areas each have a real, distinct zone on PlayerState
- ch03-game-areas.test.ts: 3-1-2-2-1: an opponent's deck/hand/security (private areas) are withheld from your view
- ch03-game-areas.test.ts: 3-1-2-1-1: your OWN private zones ARE visible to you (you may always look at your own areas)
- ch03-game-areas.test.ts: 3-1-3-2: the number of cards in a private area is public information (mirrored *Count fields)
- ch03-game-areas.test.ts: 3-1-3-1-2: a card that changes area is a NEW card — Permanent-level state has no analogue off-field
- ch03-game-areas.test.ts: 3-2-2: deck cards are placed face-down (private) — 3-2-3: no client intent can reorder it
- ch03-game-areas.test.ts: 3-3-2: the egg deck is private and face-down, structurally identical in kind to the deck
- ch03-game-areas.test.ts: 3-4-2/3-4-3: the field is public, and a card entering it is placed UNSUSPENDED
- ch03-game-areas.test.ts: 3-4-4: the field is divided into distinct breeding and battle areas
- ch03-game-areas.test.ts: 3-4-7-2/3-4-7-5: only 1 card fits in breeding, and a breeding permanent can't be chosen as a target
- ch03-game-areas.test.ts: 3-4-7-4: a card's [Main] ability can't be activated while it's in the breeding area
- ch03-game-areas.test.ts: 3-4-7-6: a breeding-area digivolution doesn't meet a battle-area Tamer's trigger condition
- ch03-game-areas.test.ts: control: the same digivolution on the BATTLE area does meet it
- ch03-game-areas.test.ts: 3-4-7-8: an [Armor Form] Digimon in the breeding area doesn't waive the color requirement
- ch03-game-areas.test.ts: control: the same [Armor Form] Digimon on the battle area does waive it
- ch03-game-areas.test.ts: 3-4-8-2: any number of cards can be placed in the battle area
- ch03-game-areas.test.ts: 3-5-3: the hand is private, but its owner can always look at it
- ch03-game-areas.test.ts: 3-6-3: the trash is public and face-up — visible in BOTH players' views
- ch03-game-areas.test.ts: 3-7-2: the security stack is private and face-down until individually revealed
- ch03-game-areas.test.ts: 3-7-4: a face-up security card becomes public information (revealed to the opponent's view)

## ch04-basic-terminology.test.ts (56: 55 it, 1 it.each)

- ch04-basic-terminology.test.ts: 4-1-1/4-1-4: paying a cost moves the gauge toward the opponent; 'gain X' moves it back toward you
- ch04-basic-terminology.test.ts: 4-1-2/4-1-3: 'or less/more memory' reads from each side's OWN perspective of the shared gauge
- ch04-basic-terminology.test.ts: [each] 4-2-2: playing $card (cost $playCost) at $memory memory moves the gauge exactly $playCost toward the opponent
- ch04-basic-terminology.test.ts: 4-2-2: a cost larger than the gauge can move is not paid in part
- ch04-basic-terminology.test.ts: 4-2-3-1: with an [Iliad] Tamer, BT24-041 (play cost 12) pays exactly the alternate cost of 7
- ch04-basic-terminology.test.ts: 4-2-3-1: the alternate cost of 7 is what decides payability, so the play is legal at -3 memory and lands on -10
- ch04-basic-terminology.test.ts: 4-2-3-1 control: without an [Iliad] Tamer the cost isn't altered and the printed 12 is paid
- ch04-basic-terminology.test.ts: 4-3-1: a Digi-Egg card or Digimon card placed on the field is treated as a Digimon
- ch04-basic-terminology.test.ts: NOW MET: a Digimon with a Link card should get the printed link DP bonus added to its DP
- ch04-basic-terminology.test.ts: 4-4-1: a Tamer card placed on the field is treated as a Tamer, not a Digimon
- ch04-basic-terminology.test.ts: 4-5-1: a Digimon card flipped from the security stack becomes face-up and is treated as a Digimon
- ch04-basic-terminology.test.ts: playing a DUAL card lets the player declare which side (Digimon or Option) it's used as
- ch04-basic-terminology.test.ts: rejects playing a DUAL card as a Digimon without firing its Option side (CR 7-1-1)
- ch04-basic-terminology.test.ts: all 6 DUAL cards have at least one OnUseOption-reachable Option-side effect registered
- ch04-basic-terminology.test.ts: 4-6-5-1/4-6-5-2: a DUAL card's Digimon information (dp/level/evoCosts) is populated and referenceable as a Digimon
- ch04-basic-terminology.test.ts: 4-6-6-1/4-6-6-2: a DUAL card's Option information is a DISTINCT field from its Digimon effect text
- ch04-basic-terminology.test.ts: 4-7-1/4-7-2: a permanent's stack is empty for a single (un-digivolved) card, and gains entries only once digivolved
- ch04-basic-terminology.test.ts: 4-7-3: the stacking order can't be changed by moving the permanent — the array identity/order is stable across other mutations
- ch04-basic-terminology.test.ts: 4-7-8: deleting a permanent trashes its stacked (digivolution) cards at the same time
- ch04-basic-terminology.test.ts: 4-16-1: deleting a battle-area Digimon trashes its Digi-Egg digivolution card
- ch04-basic-terminology.test.ts: 4-7-10: a face-down digivolution card is hidden from the opponent, but its own controller may see it
- ch04-basic-terminology.test.ts: 4-8-1: a digivolution card is a card placed under a Digimon (the demoted former top card after a digivolve)
- ch04-basic-terminology.test.ts: 4-9-2/4-9-4: a link card is a SEPARATE array from the digivolution stack, and is not itself an on-field permanent
- ch04-basic-terminology.test.ts: 4-9-5: linking to a Digimon already at its link limit (1) trashes the existing link card
- ch04-basic-terminology.test.ts: 4-10-1: a linked card is any card that HAS a link card — the predicate is exactly linked.length > 0
- ch04-basic-terminology.test.ts: 4-11-2/4-11-3: 'owner' is the card's controlling player, and 'opponent' is always the other seat
- ch04-basic-terminology.test.ts: 4-12-1: state.turnSeat identifies the turn player; the other seat is the non-turn player
- ch04-basic-terminology.test.ts: 4-13-1: a card's orientation (isSuspended) starts unsuspended, and suspend()/unsuspend() flip it
- ch04-basic-terminology.test.ts: 4-14-1/4-14-2: drawing moves a card from a player's OWN deck to their OWN hand
- ch04-basic-terminology.test.ts: 4-15-1: deletion processing always trashes the card — there is no other destination
- ch04-basic-terminology.test.ts: 4-16-1/4-16-3: trashing places a card in the trash directly — it is NOT deletion processing and doesn't remove a permanent
- ch04-basic-terminology.test.ts: 4-17-1/4-17-3: moving from breeding to battle keeps the card's orientation (isSuspended) as-is
- ch04-basic-terminology.test.ts: 4-17-2: only a Digimon WITH DP can be moved from breeding — a freshly hatched Lv.2 Digi-Egg (DP 0) can't
- ch04-basic-terminology.test.ts: 4-18-2/4-18-3: hatching is rejected with an empty egg deck, and rejected when breeding is already occupied
- ch04-basic-terminology.test.ts: 4-18-1: a legal hatch flips the top Digi-Egg face-up into the (empty) breeding area
- ch04-basic-terminology.test.ts: 4-19-1: an <Overflow> ACE card leaving the field costs its OWNER its printed overflow amount
- ch04-basic-terminology.test.ts: 4-19-5-1/4-19-5-2: simultaneous Overflow is charged turn-player-first, then the non-turn player — order changes the clamped result
- ch04-basic-terminology.test.ts: 4-20-1/4-20-2: after using a DUAL card's Option side, a Digimon may digivolve into it for free INSTEAD OF the pending trash
- ch04-basic-terminology.test.ts: declining Arts Digivolve falls through to the normal trash (it's 'may', not mandatory)
- ch04-basic-terminology.test.ts: no permanent meets the digivolution requirement => no decision is raised at all, straight to trash
- ch04-basic-terminology.test.ts: 4-21-1: a played token is a real permanent carrying full card information (name, DP, color)
- ch04-basic-terminology.test.ts: NOW MET: a token removed from the field should be removed from the game, not placed in the trash
- ch04-basic-terminology.test.ts: 4-22-2 (structural): the color-requirement gate is real and driven by CardDefinition.optionColorRequirements
- ch04-basic-terminology.test.ts: 4-22-2: an ordinary Option card's color requirement (from its printed `colors`) is enforced when no matching color is on the field
- ch04-basic-terminology.test.ts: 4-22-2: an Option permanent in the battle area is not a Digimon/Tamer color source
- ch04-basic-terminology.test.ts: 4-22-2 (positive control): the same mono-color Option plays once a matching-color source is on the field
- ch04-basic-terminology.test.ts: 4-22-3: a multicolor Option needs EVERY listed color represented, not just one
- ch04-basic-terminology.test.ts: 4-22-4: a single multicolor Digimon/Tamer can meet the requirement for more than 1 color at once
- ch04-basic-terminology.test.ts: 4-22-2 waiver seam: a color-waiver static on a HAND-resident card now fires (was silently inert)
- ch04-basic-terminology.test.ts: 4-22-2 waiver seam is BOUNDED: an ordinary on-field-only Static effect (not a pure color waiver) still requires on-field presence
- ch04-basic-terminology.test.ts: 4-23-1: a keyword filter matches only a card whose printed text carries that keyword icon/term
- ch04-basic-terminology.test.ts: 4-24-1: "XX/YY" means "XX OR YY" — a kind filter listing both kinds matches EITHER
- ch04-basic-terminology.test.ts: 4-27-1/4-27-2: "for each X" scales an effect's count by how many matching things exist at activation
- ch04-basic-terminology.test.ts: 4-27-4: a single trigger fires exactly once even though its 'each' text references a count of multiple things
- ch04-basic-terminology.test.ts: 4-29-1: 'they also gain <Rush>' targets exactly the red or blue [TS] Digimon that gain <Alliance>
- ch04-basic-terminology.test.ts: 4-29-1: the 'also' <Rush> shares the [Your Turn] duration and is absent on the opponent's turn

## ch05-game-preparation.test.ts (4)

- ch05-game-preparation.test.ts: 5-1-2: token cards are never part of a legal decklist — validateDecklist rejects a synthetic TOKEN- id
- ch05-game-preparation.test.ts: 5-1-1: the deck-and-egg-deck items required for a game are validated before setup runs (a legal decklist passes)
- ch05-game-preparation.test.ts: 5-2-1-1/5-2-1-2/5-2-1-4/5-2-1-7/5-2-1-8: runSetup shuffles, deals a 5-card opening hand, zeroes memory, and sets the declared first player's turn
- ch05-game-preparation.test.ts: 5-2-1-5: a mulligan redraw returns the whole hand to the deck, reshuffles, and deals a fresh 5-card hand

## ch06-game-procedures.test.ts (20)

- ch06-game-procedures.test.ts: 6-1-2: a turn proceeds through Active, Draw, Breeding, and Main in that fixed order
- ch06-game-procedures.test.ts: 6-1-3: the next phase does not begin until the current phase's hook has resolved (Active's unsuspend runs before Draw fires)
- ch06-game-procedures.test.ts: 6-1-4-1: the turn ends once the opponent's memory reaches the 1-or-more default threshold
- ch06-game-procedures.test.ts: 6-2-1: the Active phase unsuspends the turn player's permanents at the start of the turn
- ch06-game-procedures.test.ts: 6-2-1-1: a start-of-turn rule/effect window (OnStartTurn) fires BEFORE the unsuspending processing
- ch06-game-procedures.test.ts: 6-3-1-1: the first player skips the draw on their own first turn, but draws normally afterward
- ch06-game-procedures.test.ts: 6-3-1: an empty deck on the draw step is a deck-out loss for the turn player, not a draw of 0
- ch06-game-procedures.test.ts: 6-4-1: the turn player gets exactly ONE breeding action — a second hatch/move attempt in the same window is rejected
- ch06-game-procedures.test.ts: 6-4-1-3: 'do nothing' (skip) also closes the window, structurally identical to taking an action
- ch06-game-procedures.test.ts: 6-4-1: with no legal breeding action available, the window auto-resolves with no client round trip
- ch06-game-procedures.test.ts: 6-5-1-1-1 / 6-5-1-2-1: playing a Digimon card and digivolving are both legal Main-phase verbs
- ch06-game-procedures.test.ts: 6-5-1-7-1: declaring a pass immediately moves memory to 3 in the INCOMING turn player's favor
- ch06-game-procedures.test.ts: 6-5-1-4: a Main-phase 'link a card from hand' verb exists as a player-facing intent
- ch06-game-procedures.test.ts: 6-5-1-4/§10-1-3: linkCard is rejected outside the Main phase
- ch06-game-procedures.test.ts: 6-5-1-4/§10-1-3: linkCard is rejected for a seat that is not the turn player
- ch06-game-procedures.test.ts: §10-1-3-1: linkCard is rejected when the chosen Digimon doesn't meet the link requirement
- ch06-game-procedures.test.ts: §10-1-3-2: linkCard is rejected when the printed link cost isn't affordable
- ch06-game-procedures.test.ts: 6-6-3: once end-of-turn processing resolves, the non-turn player's turn begins
- ch06-game-procedures.test.ts: 6-6-4: an OnEndTurn effect that moves memory back to 0 or more postpones the end of the turn and continues the Main phase
- ch06-game-procedures.test.ts: 6-6-4: a turn whose gauge never crossed is not postponed (the rule needs a MOVE, not a value)

## ch07-playing-a-card.test.ts (21: 20 it, 1 it.each)

- ch07-playing-a-card.test.ts: 7-1-1: a card is played from hand into the battle area as a Main-phase action; illegal off-phase
- ch07-playing-a-card.test.ts: 7-1-2-1: a Digimon can't attack the same turn it was played
- ch07-playing-a-card.test.ts: 7-1-2-4: an unaffordable play returns the card unchanged and doesn't move memory
- ch07-playing-a-card.test.ts: 7-2-1: the play cost is reduced by the requirement's value for each material placed
- ch07-playing-a-card.test.ts: 7-2-2-1/6: chosen hand materials are revealed and consumed atomically by the declaration
- ch07-playing-a-card.test.ts: [each] 7-2-2-8: printed material order holds when %s starts on the battle area
- ch07-playing-a-card.test.ts: 7-2-2-4: a declared DigiXros can't place 0 materials
- ch07-playing-a-card.test.ts: 7-2-2-7: a battle-area material's OWN digivolution cards are trashed, not carried under the 
- ch07-playing-a-card.test.ts: 7-2-2-10: an 'if DigiXrosing with 2 cards' clause fires only when 2+ materials were actually placed
- ch07-playing-a-card.test.ts: 7-2-3-3/7-2-3-4: the reduced cost is paid, materials are placed under it, and the card resolves on the field
- ch07-playing-a-card.test.ts: 7-3-1: playing a Digimon by Assembly places the exact trash materials under it and reduces the play cost
- ch07-playing-a-card.test.ts: 7-3-2-7/7-3-2-9: Assembly isn't mandatory — the same card plays normally at full cost with no declaration
- ch07-playing-a-card.test.ts: a card with no Assembly requirement can't be played by Assembly
- ch07-playing-a-card.test.ts: 7-3-2-4: the exact material count must be placed — fewer or more is rejected
- ch07-playing-a-card.test.ts: 7-3-1: materials come from the TRASH only — a hand/battle-area card is not a legal Assembly material
- ch07-playing-a-card.test.ts: 7-3-1: a trash card that doesn't match the recipe's name/trait slot is rejected
- ch07-playing-a-card.test.ts: 7-3-1: a trash card matching the trait but failing the printed level bound is rejected
- ch07-playing-a-card.test.ts: 7-3-2-6: the player's declared order becomes the physical stacking order for a same-slot recipe
- ch07-playing-a-card.test.ts: 7-3-2-6/differentNames: a repeated-name Assembly recipe rejects duplicate materials
- ch07-playing-a-card.test.ts: 7-3-3-2/7-3-3-3: the reduced cost is paid after other modifiers, and materials are placed before the play resolves
- ch07-playing-a-card.test.ts: 7-3-3-3: an unaffordable Assembly play (even reduced) is rejected before any state changes

## ch08-digivolution.test.ts (20: 19 it, 1 it.each)

- ch08-digivolution.test.ts: 8-1-1: a hand Digimon stacks onto an existing field permanent, paying its evolve cost, as a Main-phase action
- ch08-digivolution.test.ts: 8-1-2-3/8-1-2-4: the digivolved Digimon is ONE Digimon (prior top becomes a digivolution card), carrying over suspended orientation
- ch08-digivolution.test.ts: 8-1-2-6: only 1 digivolution can be performed at a time — a second digivolve is rejected while one is mid-resolution
- ch08-digivolution.test.ts: 8-1-2-7: an unaffordable digivolve returns the revealed card unchanged and doesn't move memory
- ch08-digivolution.test.ts: 3-4-5: the base permanent's own permanentId is preserved — digivolving is not removal from the field
- ch08-digivolution.test.ts: 8-1-2-8: digivolution still succeeds with an empty deck (no draw, no crash)
- ch08-digivolution.test.ts: 8-2-1: DNA digivolution consumes 2+ field materials into ONE new Digimon (not a same-permanent stack)
- ch08-digivolution.test.ts: 8-2-2-1-1: the DNA-digivolved Digimon ALWAYS enters unsuspended — a suspended material's orientation is NOT carried over
- ch08-digivolution.test.ts: 8-2-2-1-2: a material's own linked card is trashed immediately when it becomes a digivolution card
- ch08-digivolution.test.ts: 8-2-2-1-6: [X Per Turn] uses spent by the materials' cards, inherited ones included, are reset
- ch08-digivolution.test.ts: 8-2-2-4: DNA digivolution can't be performed by the standard digivolve verb (a mismatched single base is illegal)
- ch08-digivolution.test.ts: 8-2-3-2: the DNA digivolution cost is paid off the shared memory gauge
- ch08-digivolution.test.ts: NOW MET: the DNA digivolution procedure should draw the player 1 card
- ch08-digivolution.test.ts: 8-3-1/8-3-3-4: burst digivolve stacks the card and resolves via its OWN (non-printed) requirement
- ch08-digivolution.test.ts: 8-3-2-1: trashes the Burst top card and promotes the prior top at end of turn
- ch08-digivolution.test.ts: 8-4-1: App Fusion places a linked card's specified fusion-target on top of a battle-area Digimon
- ch08-digivolution.test.ts: 8-4-2-1: the fusion-target's printed app-fusion cost is charged
- ch08-digivolution.test.ts: 8-4-3-3: the App Fusion procedure draws the player 1 card
- ch08-digivolution.test.ts: [each] 8-1-3-1..3: choosing the $label pays that requirement's cost, stacks the card, and draws 1
- ch08-digivolution.test.ts: 8-1-3-1: the chosen field card must meet the CHOSEN requirement — no fallback to another path

## ch09-using-cards.test.ts (7)

- ch09-using-cards.test.ts: 9-1-1: using an Option is gated by its color requirement, exactly like playing a Digimon
- ch09-using-cards.test.ts: 9-1-4: a used Option is treated as 'in no area' while its [Main] effect resolves
- ch09-using-cards.test.ts: 9-1-4: an Option whose [Main] effect throws still ends up in trash with resolvingOption cleared
- ch09-using-cards.test.ts: 9-1-5: a used Option (no ＜Delay＞) is trashed as pending processing once its [Main] effect resolves
- ch09-using-cards.test.ts: 9-1-7: cards are used 1 at a time — a second use is rejected while one is mid-resolution
- ch09-using-cards.test.ts: 9-1-8: an unaffordable Option use returns the card unchanged and doesn't move memory
- ch09-using-cards.test.ts: 9-1-9-2/9-1-9-3: the use cost is paid BEFORE the [Main] effect activates

## ch10-link.test.ts (11)

- ch10-link.test.ts: 10-1-1/10-1-3-2: linking a hand card to a battle-area Digimon is a Main-phase action that pays the printed link cost
- ch10-link.test.ts: 10-1-1 (bare case): a card ALREADY on the battle area can be linked to another Digimon, not just a hand card
- ch10-link.test.ts: 10-1-1 + INFERENCE: a battle-area source's own digivolution stack and own link card are 
- ch10-link.test.ts: control: an unrelated removeLooseInstance caller (trash) still CANNOT consume a permanent's own top card
- ch10-link.test.ts: EX11-027's own [On Play] 'link this Digimon to 1 of your other Digimon' actually links (proves the real card, not just the primitive)
- ch10-link.test.ts: 10-1-2-2: a card plugged in for a link becomes a link card — tracked in `linked`, distinct from the digivolution stack, and flips face-up
- ch10-link.test.ts: 10-1-2-3: an immediate-type effect tied to linking fires only AFTER the card is linked and the Digimon is chosen
- ch10-link.test.ts: NOW MET: when a Digimon already holding a link card receives another, the new one is plugged in at the BOTTOM
- ch10-link.test.ts: the player-facing linkCard verb and a card effect's Link action converge on the SAME outcome at the boundary
- ch10-link.test.ts: <Link +1> raises the limit for BOTH paths — 2 cards land and stay (no trim) once headroom covers them
- ch10-link.test.ts: BT21-009 carries a real, structured link requirement (printed 'Cost 1')

## ch11-attacking.test.ts (23)

- ch11-attacking.test.ts: 11-1-2: only the turn player can attack — a non-turn-player attack declaration is rejected
- ch11-attacking.test.ts: 11-1-3/11-1-4: the attack timings resolve strictly in order — declaration, then the block window, then the block itself
- ch11-attacking.test.ts: 11-1-5: once an attack is declared, the later timings still occur even if the attacker leaves the field mid-resolution
- ch11-attacking.test.ts: 11-2-1: declaring an attack suspends the attacking Digimon
- ch11-attacking.test.ts: 11-2-3: one Digimon attacks per declaration; a suspended attacker cannot declare again
- ch11-attacking.test.ts: 11-2-4: a new attack declaration can't be made while another attack is mid-resolution
- ch11-attacking.test.ts: 11-2-5: an attack declaration can't be made using a Digimon that's already suspended
- ch11-attacking.test.ts: 11-2-6: even if the attack target is removed mid-attack, that target remains the attack target and the attack simply fails
- ch11-attacking.test.ts: 11-2-7-1: the legal attack targets are the opponent player, or one of the opponent's SUSPENDED Digimon
- ch11-attacking.test.ts: 11-2-7-2: the attack target may be switched to another target mid-attack (a rule/effect redirect)
- ch11-attacking.test.ts: 11-2-7-4: if the attacking Digimon isn't in the battle area, its target can't be changed and it fails on any target
- ch11-attacking.test.ts: 11-3-1/11-1-3: the counter timing window opens between attack declaration and the block window
- ch11-attacking.test.ts: 11-3-1: with nothing activatable, no response window is announced and the attack continues
- ch11-attacking.test.ts: 11-3-2: the per-attack cap is 1 — activating closes the window, and a second activation this attack is refused
- ch11-attacking.test.ts: 11-3-1: a real [Counter] card activates through the window via the respondCounter verb (EX12-033)
- ch11-attacking.test.ts: 11-4-1: the block timing is when the non-turn player may use a ＜Blocker＞ Digimon to block
- ch11-attacking.test.ts: 11-5-1-1: a successful player-directed attack against 1+ security triggers a security check
- ch11-attacking.test.ts: 11-5-1-2: a successful player-directed attack against EMPTY security wins the game for the attacker
- ch11-attacking.test.ts: 11-5-1-3: a successful attack ON A DIGIMON is a battle between the two Digimon
- ch11-attacking.test.ts: 11-5-1-4: an unsuccessful attack has no security check or battle, then reaches end-of-attack timing
- ch11-attacking.test.ts: 11-6-1: every attack — successful or not — reaches the end-of-attack timing exactly once
- ch11-attacking.test.ts: 11-6-2: the attack-in-progress flag drops only once end-of-attack processing has fully resolved
- ch11-attacking.test.ts: rejects a Main-phase-only verb outside the Main phase

## ch12-blocking.test.ts (6)

- ch12-blocking.test.ts: 12-1-1: a block switches the attack target onto the blocking Digimon
- ch12-blocking.test.ts: 12-1-2/12-1-3: a block can only be performed once per attack — a second declareBlock against the same window is rejected
- ch12-blocking.test.ts: 12-1-4: a suspended Digimon (one that can't suspend again) can't block
- ch12-blocking.test.ts: 12-1-6: a block can only be performed if the attacking Digimon is (still) in the battle area
- ch12-blocking.test.ts: 12-1-7-1: declaring a block suspends the chosen blocking Digimon
- ch12-blocking.test.ts: declining the block window is legal, and leaves the attack directed at its original target

## ch13-security-checks.test.ts (13)

- ch13-security-checks.test.ts: 13-1-1/13-1-4: a security check on the opponent's security stack is mandatory whenever it applies
- ch13-security-checks.test.ts: 13-1-2: exactly 1 security check is performed per attack (Strike 1), regardless of remaining security
- ch13-security-checks.test.ts: 13-1-2 (modified Strike): a raised check count (＜Security Attack +N＞-style) checks that many cards in ONE security check
- ch13-security-checks.test.ts: 13-1-3: a security check is performed one card at a time — each is fully resolved before the next begins
- ch13-security-checks.test.ts: 13-1-3: re-evaluates the live check count after each resolved card
- ch13-security-checks.test.ts: 13-1-5: if the checking Digimon leaves play mid-check, no further security checks are performed
- ch13-security-checks.test.ts: 13-1-6: a checked card leaves the security stack immediately — before any battle against it resolves
- ch13-security-checks.test.ts: 13-1-6: a pre-existing face-up checked card is removed before its check resolves
- ch13-security-checks.test.ts: 13-1-7/13-1-8-3-1: a checked Digimon card battles the attacker as a Security Digimon
- ch13-security-checks.test.ts: 15-16-10-2: the [Security] effect activates before the effects the check triggered
- ch13-security-checks.test.ts: 13-1-8-3-2: with no Security Digimon (a non-Digimon check), the flow proceeds straight to the next step without a battle
- ch13-security-checks.test.ts: 13-1-8-4: a revealed card is trashed unless an effect already relocated it to an area
- ch13-security-checks.test.ts: 13-1-8-5: if the checking Digimon can perform another security check, it does — the Strike loop continues

## ch14-battles.test.ts (5)

- ch14-battles.test.ts: 14-1/14-2-1-1/14-2-1-2: a battle compares DP — the higher-DP card wins, the lower-DP card loses and is deleted
- ch14-battles.test.ts: 14-2-1-3/14-2-2: equal DP is a tie — BOTH cards lose and are deleted AT THE SAME TIME (a single combatResolved carries both ids)
- ch14-battles.test.ts: 14-2-3: a Security Digimon that loses a battle is NOT deleted the way a battle-area permanent is — it's simply trashed as a loose card
- ch14-battles.test.ts: 14-2-4: an effect triggered by the battle (OnDestroyedAnyone) is fully resolved before the next action (end-of-attack) begins
- ch14-battles.test.ts: NOW MET: a card's own '[When this Digimon wins/ends a battle]' clause filed at EffectTiming.OnEndBattle should actually fire when a battle ends

## ch15-01-effect-basics.test.ts (14: 13 it, 1 it.each)

- ch15-01-effect-basics.test.ts: 15-1-5: a mandatory (non-optional) effect resolves without asking — no 'use this effect?' prompt
- ch15-01-effect-basics.test.ts: 15-1-7/15-2-1-1: an unqualified target spec reaches the battle area, and its source is classified as a Digimon effect
- ch15-01-effect-basics.test.ts: 15-3-2: an inherited effect gained from a digivolution card is still classified as a Digimon effect
- ch15-01-effect-basics.test.ts: 15-9-1/15-9-2: a mandatory clause always runs; an optional clause with abortOnDecline stops the rest when declined
- ch15-01-effect-basics.test.ts: 15-12-1-1/15-12-1-4/15-12-1-5: treating a Tamer as a Digimon with DP adds BOTH the kind and DP it didn't have
- ch15-01-effect-basics.test.ts: 15-13-2: a gained (conferred) stack effect survives being placed as a digivolution card under its grantor
- ch15-01-effect-basics.test.ts: 15-15-2-1/15-15-2-2: a target "gains" a keyword and is thereafter affected by it
- ch15-01-effect-basics.test.ts: (structural) the DP change-information ledger applies 'most recently applied wins' — the same rule §15-12-1-3 states for added information
- ch15-01-effect-basics.test.ts: 15-1-8: an effect that references Security Digimon changes the DP a Security Digimon battles with
- ch15-01-effect-basics.test.ts: [each] 15-1-9: returning a $label to the hand meets the 'return to hand' processing condition although it never reaches the hand
- ch15-01-effect-basics.test.ts: 15-1-10: a 'card added to the security stack' trigger fires for a Digimon card but not for a Digi-Egg or token
- ch15-01-effect-basics.test.ts: 15-15-6-1: the replacing Digimon stands in for a bracketed requirement ([Greymon] of BT10-024)
- ch15-01-effect-basics.test.ts: 15-15-6-2 control: BT19-065 accepts a material that meets its printed requirement
- ch15-01-effect-basics.test.ts: 15-15-6-2: the replacing Digimon can't stand in for a 'different card numbers' requirement (BT19-065)

## ch15-02-timing-and-resolution.test.ts (24)

- ch15-02-timing-and-resolution.test.ts: 15-4-1-1: activation is the effect being EXECUTED — the use ledger records a use only once resolve() has run
- ch15-02-timing-and-resolution.test.ts: 15-4-2-3: pending-activation effects resolve strictly one at a time, never concurrently
- ch15-02-timing-and-resolution.test.ts: 15-4-3-5-1/2: ALL of the turn player's simultaneously-triggered effects resolve before ANY of the non-turn player's
- ch15-02-timing-and-resolution.test.ts: orderTurnPlayerFirst is stable within each side (doesn't reorder same-seat effects)
- ch15-02-timing-and-resolution.test.ts: 15-4-4-5: an effect that no longer meets its trigger conditions before activating can no longer be activated
- ch15-02-timing-and-resolution.test.ts: 15-4-4-3: a trigger parked between windows lapses when its source becomes a digivolution card
- ch15-02-timing-and-resolution.test.ts: NOW MET: a derived trigger for the non-turn player should activate BEFORE the turn player's still-pending effects
- ch15-02-timing-and-resolution.test.ts: Q6723: a same-controller derived trigger cuts ahead without joining the older effect's ordering group
- ch15-02-timing-and-resolution.test.ts: 15-5-2: a triggering from 1 trigger condition triggers only once, even if it occurred multiple times at the same moment
- ch15-02-timing-and-resolution.test.ts: 15-6-3: an effect can't be activated when none of its processing conditions are met
- ch15-02-timing-and-resolution.test.ts: 15-6-3 (negative control): the same effect activates once its processing condition is met
- ch15-02-timing-and-resolution.test.ts: 15-7-5: a player may pay an executable optional processing condition even when the payload can't resolve
- ch15-02-timing-and-resolution.test.ts: 15-7-1/2: refusing a payable processing condition preserves the hand and skips the whole payload
- ch15-02-timing-and-resolution.test.ts: 15-7-2: an unpayable clause cost skips every action of the clause, not just the first
- ch15-02-timing-and-resolution.test.ts: the opponent owns the choice between two deletion targets after the source pays
- ch15-02-timing-and-resolution.test.ts: 15-7-1: a payable clause cost is paid exactly once and the whole clause then resolves
- ch15-02-timing-and-resolution.test.ts: 15-7-2: declining the opponent's optional inside the clause does not refund the paid cost
- ch15-02-timing-and-resolution.test.ts: 15-8-3-1: BT1-070's [On Play] is a trigger-type effect — it triggers unconditionally once its condition (being played) is met
- ch15-02-timing-and-resolution.test.ts: 15-8-3-4: a trigger-type effect does NOT trigger when its condition is never met
- ch15-02-timing-and-resolution.test.ts: 15-8-4-1/15-8-4-2: BT15-009's [Main][Once Per Turn] is activation-type — it needs a player DECLARATION during the main phase, unlike a trigger-type effect
- ch15-02-timing-and-resolution.test.ts: 15-8-4-3-1: a player can't DECLARE an activation-type effect whose cost can't be paid
- ch15-02-timing-and-resolution.test.ts: 15-8-4-3-1 (negative control): the same effect activates once its cost is payable
- ch15-02-timing-and-resolution.test.ts: 15-7-5: a payable By condition remains declarable with no legal target
- ch15-02-timing-and-resolution.test.ts: 15-8-4-3-1 (boundary): a cost exactly equal to maxCostFor(seat) is payable, one more is not

## ch15-03-targeting-and-selection.test.ts (11)

- ch15-03-targeting-and-selection.test.ts: 15-10-1-3-1: "both players" affects BOTH players' own zones, not just the activating player's
- ch15-03-targeting-and-selection.test.ts: 15-10-2-1: an exact-count target ("1 Digimon") is individual processing — a decision names the SPECIFIC card affected
- ch15-03-targeting-and-selection.test.ts: 15-11-1-3-1/15-11-1-3-2: an individually-chosen target keeps the effect even after it no longer meets the CHOOSING condition
- ch15-03-targeting-and-selection.test.ts: 15-11-2-1/15-11-2-3-1: overall processing with 'their Digimon with X' restricts EVERY matching Digimon, not one chosen at random
- ch15-03-targeting-and-selection.test.ts: (structural) a target COUNT of "all" IS the real overall-processing path when the IR uses it correctly
- ch15-03-targeting-and-selection.test.ts: 15-15-3-1/15-15-3-2 (+ ch03 comprehensive-0280's batch-move ordering): revealing doesn't change the deck's count; only the FINAL placement does
- ch15-03-targeting-and-selection.test.ts: 15-15-3-6/15-15-3-9-2: the activating player controls the return order and searches only their own deck when no player is named
- ch15-03-targeting-and-selection.test.ts: NOW MET: an immune permanent should still be a legal, CHOOSABLE candidate — just unaffected once chosen
- ch15-03-targeting-and-selection.test.ts: offers the choice even when the immune permanent is the ONLY candidate
- ch15-03-targeting-and-selection.test.ts: BT21-010's own condition reads permanentCount+distinctNames against a real filter — 3 SAME-named [Hero] Tamers do NOT satisfy 'different names'
- ch15-03-targeting-and-selection.test.ts: BT23-055's placedInBattleAreaByEffect filter matches an on-field Option permanent and rejects a Digimon permanent

## ch15-04-continuous-and-static.test.ts (20)

- ch15-04-continuous-and-static.test.ts: 15-8-2-2/15-8-2-3: a persistent DP boost is active on the owner's turn and inactive on the opponent's — the rules' OWN worked example
- ch15-04-continuous-and-static.test.ts: 15-14-1-2/15-14-1-3/15-14-1-5-1: a [Once Per Turn] use is tracked PER CARD COPY and resets at the turn boundary
- ch15-04-continuous-and-static.test.ts: 15-14-1-2: an event that fails the clause's own trigger condition spends no [Once Per Turn] use
- ch15-04-continuous-and-static.test.ts: 15-14-2-1: BT9-042's {Hand}[Main] effect is declarable while the card sits face-up in hand
- ch15-04-continuous-and-static.test.ts: NOW MET: onAddHand's default base guard should NOT require the source to be on the battle area
- ch15-04-continuous-and-static.test.ts: NOW MET: a {Trash}-resident effect should activate while its card sits in the trash, not require the battle area
- ch15-04-continuous-and-static.test.ts: 15-14-4-1: a {Breeding}-flagged effect activates while its card is in the breeding area and NOT on the battle area
- ch15-04-continuous-and-static.test.ts: 15-14-5-1: a {Security} effect activates while its card sits face-up in the security stack, with no on-field base guard
- ch15-04-continuous-and-static.test.ts: 15-16-1/15-16-2: [On Play] fires exactly at the point a card's play completes — BT1-070's own worked example
- ch15-04-continuous-and-static.test.ts: 15-16-3-1: [When Digivolving] triggers at the point a digivolve into that card completes
- ch15-04-continuous-and-static.test.ts: 15-16-4-1: [On Deletion] triggers exactly when the card with the effect is deleted — real card BT1-035 'Gain 2 memory.'
- ch15-04-continuous-and-static.test.ts: 15-16-5-1: [When Attacking] triggers at the point an attack is declared — BT9-042's INHERITED clause fires on the higher Digimon's attack
- ch15-04-continuous-and-static.test.ts: 15-16-7-1: [Main] is exactly the activation-type-effect window — BT15-009 again, cited for its own timing icon this time
- ch15-04-continuous-and-static.test.ts: 15-16-8-1: ST1-12's [Your Turn] DP boost is gated to its OWNER's turn specifically (not just 'any turn')
- ch15-04-continuous-and-static.test.ts: 15-16-9-1: an [All Turns] effect triggers/activates on BOTH players' turns, unlike [Your Turn]
- ch15-04-continuous-and-static.test.ts: 15-16-10-2: a triggered [Security] effect activates immediately, without joining the pending-activation queue
- ch15-04-continuous-and-static.test.ts: 15-16-13-1: [Start of Your Main Phase] fires at the OnStartMainPhase window — BT22-007's {Breeding} clause, gated to the OWNER's turn
- ch15-04-continuous-and-static.test.ts: NOW MET: a compiled '[When Moving]' effect should fire at the real OnMove window
- ch15-04-continuous-and-static.test.ts: 15-8-5-1/15-8-5-2: an immediate-type 'would be deleted' reaction interrupts BEFORE the deletion and can prevent it — real ＜Barrier＞ card BT13-041
- ch15-04-continuous-and-static.test.ts: BT22-007's selfDigivolutionCountAtLeast condition gates its play-3 clause on 10+ digivolution cards, driven through the REAL interpreter

## ch16a-security-blocker-draw.test.ts (10)

- ch16a-security-blocker-draw.test.ts: 16-4-3: 2 stacked <Security A. +1> grants check 2 cards total, NOT a combined <Security A. +2>
- ch16a-security-blocker-draw.test.ts: a printed <Security A. +1> attacker (AD1-005) registers a real +1 grant that strikeFor turns into 2 checks
- ch16a-security-blocker-draw.test.ts: 16-4-4: a heavily negative modifier floors the check count at 0, never negative
- ch16a-security-blocker-draw.test.ts: 16-5-1: a printed-<Blocker> Digimon (AD1-005) may block; a plain Digimon may not
- ch16a-security-blocker-draw.test.ts: 16-6-1: places the specified number of cards from the deck face down on top of the security stack
- ch16a-security-blocker-draw.test.ts: 16-7-1/16-7-3: a Piercing attacker that deletes the defender in battle performs a mandatory security check before end of attack
- ch16a-security-blocker-draw.test.ts: NEGATIVE CONTROL: without Piercing, deleting the defender in a permanent-target battle triggers NO security check
- ch16a-security-blocker-draw.test.ts: 16-8-1/16-8-3: <Draw 1> mandatorily draws 1 card from the player's own deck
- ch16a-security-blocker-draw.test.ts: 16-9-1: a printed-<Jamming> attacker isn't deleted by losing a battle against a Security Digimon
- ch16a-security-blocker-draw.test.ts: NEGATIVE CONTROL: without Jamming, the identical losing battle against a Security Digimon deletes the attacker

## ch16b-digivolve-and-battle-keywords.test.ts (21: 14 it, 7 it.each)

- ch16b-digivolve-and-battle-keywords.test.ts: 16-10-1: suspending 1 Digimon reduces the digivolution cost by the printed amount
- ch16b-digivolve-and-battle-keywords.test.ts: 16-11-1: an opponent's <Reboot> permanent unsuspends during the turn player's own unsuspend phase
- ch16b-digivolve-and-battle-keywords.test.ts: NEGATIVE CONTROL: without Reboot, an opponent's suspended permanent stays suspended through the turn player's unsuspend phase
- ch16b-digivolve-and-battle-keywords.test.ts: a Digimon deleted in battle with <Retaliation> also deletes the opponent's battled Digimon
- ch16b-digivolve-and-battle-keywords.test.ts: 16-14-1: trashing the specified number of digivolution cards activates the attached effect
- ch16b-digivolve-and-battle-keywords.test.ts: [each] %s publicly attacks the turn played while the neutral control cannot
- ch16b-digivolve-and-battle-keywords.test.ts: loses printed Rush after public digivolution without resetting its fresh-play restriction
- ch16b-digivolve-and-battle-keywords.test.ts: [each] $source inherited Rush matches $destination and resolves $security security
- ch16b-digivolve-and-battle-keywords.test.ts: [each] $source continuously grants Rush to a new eligible recipient until its source is removed
- ch16b-digivolve-and-battle-keywords.test.ts: [each] $source inherited trigger distinguishes plays, enforces frequency and resets next turn
- ch16b-digivolve-and-battle-keywords.test.ts: EX3-049 grants Rush only to the newly played D-Brigade subject
- ch16b-digivolve-and-battle-keywords.test.ts: P-098 offers only blue Rush recipients after a yellow effect play
- ch16b-digivolve-and-battle-keywords.test.ts: [each] %s chooses one Rush recipient from a public simultaneous Dragon play
- ch16b-digivolve-and-battle-keywords.test.ts: [each] %s keeps two inherited Rush occurrences independent on one host
- ch16b-digivolve-and-battle-keywords.test.ts: [each] %s binds Rush only to the Dragon in a public mixed play
- ch16b-digivolve-and-battle-keywords.test.ts: 16-16-1/16-16-5: offers the attack only once the opponent has 1 or more memory
- ch16b-digivolve-and-battle-keywords.test.ts: 16-17-1/16-17-3: a <Delay> [Main] ability is unusable the turn the card enters the battle area
- ch16b-digivolve-and-battle-keywords.test.ts: becomes activatable (and mandatorily trashes the card) once it's been on the field since a prior turn
- ch16b-digivolve-and-battle-keywords.test.ts: a <Decoy>-protected Digimon stays deletable: the protection is paid by deleting the Decoy holder
- ch16b-digivolve-and-battle-keywords.test.ts: NOW MET: a printed-<Armor Purge> Digimon that loses a battle should be spared by trashing its own top card
- ch16b-digivolve-and-battle-keywords.test.ts: names the saved Digimon as the source of its battle <Armor Purge> prompt

## ch16c-deletion-and-advanced-keywords.test.ts (40)

- ch16c-deletion-and-advanced-keywords.test.ts: 16-20-1: on deletion, the card may be placed under 1 of the controller's Tamers instead of trashed
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: on deletion, <Material Save N> should place N DigiXros-requirement digivolution cards under a Tamer
- ch16c-deletion-and-advanced-keywords.test.ts: 16-22-1/2: the prompt opens while the Digimon would be deleted, and suspending prevents the deletion
- ch16c-deletion-and-advanced-keywords.test.ts: 16-22-3: suspending is optional — declining leaves the deletion in place
- ch16c-deletion-and-advanced-keywords.test.ts: 16-22-3: an already-suspended Digimon can't meet the condition, so no prompt opens and it is deleted
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: a printed-<Raid> attacker should be able to switch its attack onto the opponent's highest-DP unsuspended Digimon
- ch16c-deletion-and-advanced-keywords.test.ts: 16-24-1: suspending another Digimon adds its DP to the attacker (attack-only, no block-time variant)
- ch16c-deletion-and-advanced-keywords.test.ts: 16-25-1: trashing the top card of the controller's security stack prevents a battle deletion
- ch16c-deletion-and-advanced-keywords.test.ts: 16-26-1: a card printing <Blast Digivolve> digivolves from hand for free, without paying the cost
- ch16c-deletion-and-advanced-keywords.test.ts: 16-26-1 control: a Digimon WITHOUT <Blast Digivolve> is still rejected on insufficient memory
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: a <Fortitude> Digimon with digivolution cards, on deletion, should replay itself for free
- ch16c-deletion-and-advanced-keywords.test.ts: 16-28-1: places the Tamer with this effect into the digivolution cards of a Tamer-less Digimon
- ch16c-deletion-and-advanced-keywords.test.ts: 16-29-1: an opponent-effect removal replays one of each specified digivolution card for free
- ch16c-deletion-and-advanced-keywords.test.ts: 16-30-1: hasCollision reads the printed keyword the same way the block-forcing consume seam does
- ch16c-deletion-and-advanced-keywords.test.ts: 16-31-1: Counter Blast DNA consumes one field Digimon and one hand card without paying memory
- ch16c-deletion-and-advanced-keywords.test.ts: 16-31-1 control: a plain (non-<Blast>) card DNA digivolves normally when its printed requirement matches
- ch16c-deletion-and-advanced-keywords.test.ts: 16-31-1 control: a plain (non-<Blast>) card's DNA digivolve is rejected on insufficient memory
- ch16c-deletion-and-advanced-keywords.test.ts: 16-31-1 control: dnaDigivolve materials that don't match the printed requirement are rejected
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: deleting another of the controller's Digimon should prevent a <Scapegoat> Digimon's non-owner-effect deletion
- ch16c-deletion-and-advanced-keywords.test.ts: 16-33-1: <Vortex> does not bypass the normal suspended-target requirement
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: 16-33-1's 'also allows attacking the same turn the Digimon was played' relax summoning sickness for a Vortex attack
- ch16c-deletion-and-advanced-keywords.test.ts: 16-34-1/2: at the end of the turn, deleting a Token makes it attack the player without suspending
- ch16c-deletion-and-advanced-keywords.test.ts: 16-34-3: deleting is optional — declining it means no attack
- ch16c-deletion-and-advanced-keywords.test.ts: 16-35-1: printed <Iceclad> is read the same way the DP-vs-digivolution-count swap consumes it
- ch16c-deletion-and-advanced-keywords.test.ts: 16-36-1/2: an effect deletion plays the specified card from the digivolution cards before they are trashed, for free
- ch16c-deletion-and-advanced-keywords.test.ts: 16-36-1: returning the Digimon to the hand is also a leave other than by battle
- ch16c-deletion-and-advanced-keywords.test.ts: 16-36-1/2: a battle deletion does not fire it
- ch16c-deletion-and-advanced-keywords.test.ts: 16-36-3: playing the card is optional — declining trashes it with the stack
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: trashing the specified number of digivolution cards should prevent a <Fragment> Digimon's deletion
- ch16c-deletion-and-advanced-keywords.test.ts: 16-38-1: at end of turn the Digimon may attack, and is deleted at the end of that attack
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: a <Progress> Digimon should be immune to the opponent's effects while attacking
- ch16c-deletion-and-advanced-keywords.test.ts: 16-40-1: adds the specified number to the maximum link cards a Digimon may carry
- ch16c-deletion-and-advanced-keywords.test.ts: a Digimon exceeding its link max sheds only the excess linked cards on the field-legality sweep
- ch16c-deletion-and-advanced-keywords.test.ts: 16-41-1: suspending this Digimon during the main phase places the deck's top card under it as a digivolution card
- ch16c-deletion-and-advanced-keywords.test.ts: BT25-093 compiles a real WaiveColorRequirement action (not an inert keyword marker)
- ch16c-deletion-and-advanced-keywords.test.ts: baseline (unaffected): BT25-093's own color requirement is still enforced when unmet and no Use Req. condition holds
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: once a [TS] trait card is in play, BT25-093 plays despite having no Red source
- ch16c-deletion-and-advanced-keywords.test.ts: every card printing ＜Use Req.＞ compiles a WaiveColorRequirement waiver node
- ch16c-deletion-and-advanced-keywords.test.ts: NOW MET: on deletion, an <Ascension> card should be placeable atop the controller's security stack instead of trashed
- ch16c-deletion-and-advanced-keywords.test.ts: a sibling [On Deletion] effect still resolves when another Digimon deleted in the same batch ascends

## ch17-rule-checks.test.ts (5)

- ch17-rule-checks.test.ts: the ruleProcessing re-entrancy latch suppresses a real, concrete violation while a pass is in flight
- ch17-rule-checks.test.ts: BT3-101's own worked-example shape ('-3000 DP AND <Security Attack -1>' in one effect) proves 
- ch17-rule-checks.test.ts: NOW MET: an Option-kind permanent sitting in the battle area (not placed there by an effect) should be trashed by the rule-check sweep
- ch17-rule-checks.test.ts: NOW MET: a linked card whose printed <Link> requirement the host no longer (or never did) satisfy should be trashed by the rule-check sweep
- ch17-rule-checks.test.ts: spares a harness-seeded battle-area Option by default and sweeps one seeded with placedByEffect: false

## ch18-other-information.test.ts (7)

- ch18-other-information.test.ts: BT1-090 ('[Main] Gain 2 memory. At end of turn, lose 2 memory.') is the rule's OWN worked example: 
- ch18-other-information.test.ts: NOW MET: with a [Kiriha Aonuma] in play and a [MetalGreymon] in the trash, BT10-019's printed 'instead' clause should let the player return it to hand INSTEAD of the default reveal-top-4 processing
- ch18-other-information.test.ts: BT10-084's 'trash this Digimon's digivolution cards instead' runs straight from the replaced trash, with nothing resolving in between
- ch18-other-information.test.ts: NOW MET: when the rule-check fixpoint can't converge, the match should end in a draw (§18-3-2), not throw an engine error
- ch18-other-information.test.ts: §18-3-3's turn-player/non-turn-player 'declare a repeat count' stop procedure has no protocol representation
- ch18-other-information.test.ts: §18-3-3: a loop one of the players CAN stop is stopped, not resolved as a §18-3-2 draw
- ch18-other-information.test.ts: §18-3-3-3: after the stop, the same infinite loop is not performed again in that window

## digivolution-card-placement.test.ts (3: 0 it, 3 it.each)

- digivolution-card-placement.test.ts: [each] $card public attack $mode reports only actual bottom deck payment
- digivolution-card-placement.test.ts: [each] $card still offers and pays the processing condition without a target (15-7-5)
- digivolution-card-placement.test.ts: [each] $card actual turn with egg=$hasEgg retains face-up bottom placement and full processing

## duration-remaining-fronts.test.ts (2)

- duration-remaining-fronts.test.ts: keeps an opponent-turn grant anchored to its granting seat
- duration-remaining-fronts.test.ts: keeps separate endpoints for grants installed before and after turnover

## effect-play-digixros.test.ts (3)

- effect-play-digixros.test.ts: plays the card normally when no legal material exists
- effect-play-digixros.test.ts: plays the card without materials when the player declines DigiXros
- effect-play-digixros.test.ts: offers DigiXros for a card that <Partition> plays

## effect-play-route-matrix.test.ts (23)

- effect-play-route-matrix.test.ts: hand (manual): BT26-014 places its TB material from trash
- effect-play-route-matrix.test.ts: effect from hand: EX12-072 Metal Empire's Security effect plays EX12-064 Megadramon with Assembly
- effect-play-route-matrix.test.ts: effect from trash: BT26-067 Wizardmon plays BT26-073 Aegiochusmon: Dark with Assembly
- effect-play-route-matrix.test.ts: effect from security: ST10-06 Mastemon plays EX12-064 Megadramon with Assembly
- effect-play-route-matrix.test.ts: effect from deck reveal: EX12-058 HiAndromon reveals and plays EX12-064 Megadramon with Assembly
- effect-play-route-matrix.test.ts: hand (manual): BT10-077 MadLeomon places a Bagra Army card from hand
- effect-play-route-matrix.test.ts: effect from hand: BT21-021 OmniShoutmon's End of Attack plays BT19-014 with DigiXros (Q4727)
- effect-play-route-matrix.test.ts: effect from trash: BT10-084 Tactimon plays BT10-077 MadLeomon with DigiXros
- effect-play-route-matrix.test.ts: effect from security: ST10-06 Mastemon plays BT10-077 MadLeomon with DigiXros
- effect-play-route-matrix.test.ts: effect from deck reveal: BT10-105's Security effect plays BT10-061 with DigiXros
- effect-play-route-matrix.test.ts: hand (manual): EX13-045 Examon DNA digivolves over two level 5 Digimon treated as level 6
- effect-play-route-matrix.test.ts: effect from hand: EX3-020 Wingdramon's End of Your Turn DNA digivolves into EX3-074 Examon
- effect-play-route-matrix.test.ts: effect from trash: BT17-101 DNA digivolves two Digimon when a level 6 with Pulsemon in text is played
- effect-play-route-matrix.test.ts: hand (manual): BT23-016 with BT23-039 linked app fuses into BT23-021
- effect-play-route-matrix.test.ts: effect from hand: BT21-084 Haru Shinkai app fuses the linked Digimon into BT21-073
- effect-play-route-matrix.test.ts: effect from trash: BT24-087 Rei Katsura app fuses the linked Digimon into BT24-038 in trash
- effect-play-route-matrix.test.ts: hand (manual): BT13-018 ShineGreymon burst digivolves into BT13-020 by returning Marcus Damon
- effect-play-route-matrix.test.ts: hand (manual): blue BT10-024 MetalGreymon digivolves into EX4-051 only through 'from [MetalGreymon]: 3'
- effect-play-route-matrix.test.ts: effect from hand: BT19-077 Calumon digivolves blue MetalGreymon into EX4-051 for the alternate cost minus 2
- effect-play-route-matrix.test.ts: effect from trash: BT13-085 Crowmon digivolves into BT26-082 Ravemon for its [Crowmon] cost 3
- effect-play-route-matrix.test.ts: effect from security: BT16-024 MagnaAngemon digivolves into EX6-028 Seraphimon for its [MagnaAngemon] cost 3 minus 2
- effect-play-route-matrix.test.ts: effect from deck reveal: BT24-060 Hisyaryumon may digivolve into BT20-080 only through its [SEEKERS] requirement
- effect-play-route-matrix.test.ts: EX5-069 Biting Crush's Delay plays EX5-063 Leviamon from trash when the opponent's effect plays a Digimon

## glossary.test.ts (19)

- glossary.test.ts: a Tamer permanent is not a legal attack target — Tamers 'can't be targeted by an attacking Digimon'
- glossary.test.ts: Breeding Area: 'Only 1 Digimon can be in the breeding area at a time'
- glossary.test.ts: Security Stack: 'A player loses the game if they are attacked with zero cards remaining in their security stack'
- glossary.test.ts: On Play: 'Does not apply to digivolving Digimon' — a digivolve does NOT fire the base's [On Play]
- glossary.test.ts: On Deletion: 'Triggers when...deleted when its DP is reduced to 0' — a rule-check deletion (not just combat) fires [On Deletion]
- glossary.test.ts: Suspended: 'Suspended Digimon can be targeted for attacks' — an unsuspended Digimon can't be
- glossary.test.ts: Draw Phase: 'The player who goes first does not draw during their initial turn' AND 
- glossary.test.ts: Once Per Turn vs Twice Per Turn are independently tracked, and per (instance, effectKey) not globally
- glossary.test.ts: DP: 'the Digimon with the lower number loses and is deleted' in battle
- glossary.test.ts: In-Training: 'Corresponds to Level 2' — a hatched Digi-Egg carries level 2
- glossary.test.ts: Play Cost: playing a Tamer deducts EXACTLY its printed play cost from memory
- glossary.test.ts: Cost: playing an Option deducts EXACTLY its printed cost from memory
- glossary.test.ts: Pass: 'the memory counter is moved to the opponent's number 3 space regardless of where it was before passing'
- glossary.test.ts: cross-references existing deep coverage rather than re-deriving it
- glossary.test.ts: 'If your opponent has zero security cards and you attack with a Digimon that checks zero cards, you can't win the game'
- glossary.test.ts: <Jamming> spares the attacker from a Security Digimon loss, but a SEPARATE <Security A. +1> check still runs afterward
- glossary.test.ts: 'unsuspended during not only your unsuspend phase, but your opponent's unsuspend phase AS WELL' — Reboot doesn't replace normal self-unsuspend
- glossary.test.ts: 'if the Digimon is suspended... <Blitz> won't enable it to attack'
- glossary.test.ts: <Delay>: "It's not necessary to pay an Option card's memory cost...when activating its <Delay> effect"

## interaction-pending-trigger-matrix.test.ts (4)

- interaction-pending-trigger-matrix.test.ts: keeps BT12-002's spent inherited When Attacking use through ordinary digivolution
- interaction-pending-trigger-matrix.test.ts: rechecks the target pool when an earlier PrinceMamemon deletion removes its only target
- interaction-pending-trigger-matrix.test.ts: retains two BT24-086 when-played triggers from Nail Bone's sequential plays
- interaction-pending-trigger-matrix.test.ts: keeps the Progress attacker selectable for Damemon's paid deletion but unaffected

## interaction-rule-check-triggers.test.ts (9)

- interaction-rule-check-triggers.test.ts: offers a rule-check deletion beside a played Digimon's [On Play] at one checkpoint
- interaction-rule-check-triggers.test.ts: orders rule-check Ascension with [Start of Your Turn] before its security placement
- interaction-rule-check-triggers.test.ts: drops pending [On Deletion] when rule-check Ascension is chosen first
- interaction-rule-check-triggers.test.ts: drops an inherited [On Deletion] when its host ascends before activation
- interaction-rule-check-triggers.test.ts: drops a surviving rule-check watcher if an earlier same-batch effect removes its source
- interaction-rule-check-triggers.test.ts: offers zero-DP [On Deletion] and [Start of Your Turn] in one order choice
- interaction-rule-check-triggers.test.ts: seat 1 orders its start-turn and rule-check effects before seat 0's rule-check effect
- interaction-rule-check-triggers.test.ts: keeps a rule check caused by a chosen deletion effect in a later derived batch
- interaction-rule-check-triggers.test.ts: keeps a rule-check deletion watcher with printed start-turn and deletion effects

## interaction-trigger-timing.test.ts (10: 8 it, 2 it.each)

- interaction-trigger-timing.test.ts: resolves the turn player's EX13-028 On Deletion before the opponent's after a mutual battle deletion
- interaction-trigger-timing.test.ts: lets seat 0 order simultaneous deletion effects and decline play before seat 1 chooses
- interaction-trigger-timing.test.ts: lets seat 1 order simultaneous deletion effects and decline play before seat 0 chooses
- interaction-trigger-timing.test.ts: drops a pending battle deletion watcher when an earlier On Deletion removes its surviving source
- interaction-trigger-timing.test.ts: [each] offers both in one order request and resolves them as chosen (%s)
- interaction-trigger-timing.test.ts: resolves EX6 Shoutmon's Alliance and When Attacking before MetalEtemon and the inherited EX5 Etemon
- interaction-trigger-timing.test.ts: resolves BT13-021's When Attacking before BT11 Analogman's redirect on an effect-driven attack
- interaction-trigger-timing.test.ts: [each] holds %s's On Play until EX13 Alphamon's End of Turn effect ends, then resolves it inside the attack
- interaction-trigger-timing.test.ts: lets BT12-002's inherited When Attacking draw again after Paildramon DNA digivolves in the same turn
- interaction-trigger-timing.test.ts: lets BT13-021 use its Once Per Turn When Attacking again after it leaves and is played again

## kb-citation-drift.test.ts (4)

- kb-citation-drift.test.ts: accepts unchanged reviewed text and records the pin and calling file
- kb-citation-drift.test.ts: rejects changed text under a retained ID before recording coverage
- kb-citation-drift.test.ts: rejects a missing ID even when a fingerprint is supplied
- kb-citation-drift.test.ts: preserves legacy citations without claiming a reviewed fingerprint

## keyword-alliance-consent.test.ts (2)

- keyword-alliance-consent.test.ts: lets the controller choose one exact ally and applies its DP to the battle
- keyword-alliance-consent.test.ts: allows public refusal while preserving every eligible ally and completing the attack

## keyword-ascension-lifecycle.test.ts (6: 4 it, 2 it.each)

- keyword-ascension-lifecycle.test.ts: [each] $card preserves exact card identity and source destinations
- keyword-ascension-lifecycle.test.ts: [each] Execute preserves the chosen reaction order (Ascension first: %s)
- keyword-ascension-lifecycle.test.ts: publicly refuses optional Ascension and leaves the deleted card in trash
- keyword-ascension-lifecycle.test.ts: orders public Ascension before a pending On Deletion effect and preserves source identity
- keyword-ascension-lifecycle.test.ts: uses a granted Ascension from a public keyword grant and moves that exact card
- keyword-ascension-lifecycle.test.ts: expires granted Ascension before the next player's public battle

## keyword-blast-digivolve-consent.test.ts (5: 4 it, 1 it.each)

- keyword-blast-digivolve-consent.test.ts: [each] does not waive a base grant's condition or turn restriction for %s
- keyword-blast-digivolve-consent.test.ts: does not offer or accept a generic Blast Digivolve onto Yuuko
- keyword-blast-digivolve-consent.test.ts: passes the real Counter window without consuming an ACE or base when the defender declines
- keyword-blast-digivolve-consent.test.ts: accepts AD1-005 over the selected real red level-5 base and waives its cost
- keyword-blast-digivolve-consent.test.ts: does not expose AD1-005 when no real red level-5 host satisfies its printed requirement

## keyword-blast-dna-consent.test.ts (2)

- keyword-blast-dna-consent.test.ts: chooses an exact field/hand recipe and resolves the DNA stack and attack
- keyword-blast-dna-consent.test.ts: passes the Counter and preserves both DNA materials while the attack resolves

## keyword-blocker-lifecycle.test.ts (3: 2 it, 1 it.each)

- keyword-blocker-lifecycle.test.ts: publicly plays BT19-064, blocks on the opponent turn, then expires it
- keyword-blocker-lifecycle.test.ts: [each] rejects %s from a public block window
- keyword-blocker-lifecycle.test.ts: accepts an untapped printed AD1-005 block and resolves against that exact blocker

## keyword-blocker-source-changes.test.ts (2)

- keyword-blocker-source-changes.test.ts: uses an inherited Blocker through a public opponent-turn block window
- keyword-blocker-source-changes.test.ts: loses inherited Blocker when a public legal evolution changes the host

## keyword-de-digivolve-parameters.test.ts (5)

- keyword-de-digivolve-parameters.test.ts: publicly plays BT19-063 and trashes exactly one top source
- keyword-de-digivolve-parameters.test.ts: publicly plays BT19-026 and trashes exactly two top sources
- keyword-de-digivolve-parameters.test.ts: does not trash a level 3-only stack or an empty stack
- keyword-de-digivolve-parameters.test.ts: publicly plays P-240 and trashes three top cards without removing the level 3 card
- keyword-de-digivolve-parameters.test.ts: uses the public number choice to declare one of a printed De-Digivolve 2

## keyword-decoy-parameters.test.ts (5)

- keyword-decoy-parameters.test.ts: protects a specified Deva through public Gaia Force and trashes the Decoy holder
- keyword-decoy-parameters.test.ts: does not protect a non-Deva target with the printed trait parameter
- keyword-decoy-parameters.test.ts: leaves both cards' deletion outcomes to the refusal branch
- keyword-decoy-parameters.test.ts: protects a black target with BT6-059's public color parameter
- keyword-decoy-parameters.test.ts: rejects a non-black target for BT6-059 without spending the Decoy

## keyword-decoy-source-continuity.test.ts (2)

- keyword-decoy-source-continuity.test.ts: removes a P-045 inherited Decoy grant when its Armageddemon source leaves
- keyword-decoy-source-continuity.test.ts: keeps the continuous Decoy grant while its source remains in play

## keyword-delay-boundaries.test.ts (1)

- keyword-delay-boundaries.test.ts: refuses the entry-turn Delay and activates it after a natural owner-turn boundary

## keyword-detach-lifecycle.test.ts (10: 7 it, 3 it.each)

- keyword-detach-lifecycle.test.ts: [each] %s may trash a specified link to prevent deletion by the opponent's Option
- keyword-detach-lifecycle.test.ts: [each] prevents opponent %s after paying one eligible link
- keyword-detach-lifecycle.test.ts: only the threatened Digimon's controller can decline Detach
- keyword-detach-lifecycle.test.ts: does not offer payment using a link without the specified trait
- keyword-detach-lifecycle.test.ts: does not save a Digimon from the zero-DP rule after an opponent Option reduces its DP
- keyword-detach-lifecycle.test.ts: fires an allied EX10-062 link-trash watcher when Detach pays its link cost
- keyword-detach-lifecycle.test.ts: fires an allied EX10-062 watcher while Gaia Force is prevented by Detach
- keyword-detach-lifecycle.test.ts: does not prevent the owner's own deletion cost
- keyword-detach-lifecycle.test.ts: pays Detach when an attack into an equal-DP security Digimon would delete the holder
- keyword-detach-lifecycle.test.ts: [each] does not offer the processing choice with unpayable links $links (accept=$accept)

## keyword-digi-burst-parameters.test.ts (5)

- keyword-digi-burst-parameters.test.ts: trashes exactly one source for BT4-072's public Main activation
- keyword-digi-burst-parameters.test.ts: trashes exactly two sources for ST4-13 and resolves its public target effect
- keyword-digi-burst-parameters.test.ts: trashes exactly three sources for BT4-049
- keyword-digi-burst-parameters.test.ts: trashes exactly four sources for BT4-062 after public digivolution
- keyword-digi-burst-parameters.test.ts: does not declare BT4-062's Digi-Burst when only three sources exist

## keyword-digisorption-consent.test.ts (3)

- keyword-digisorption-consent.test.ts: accepts BT3-056 -3 and selects the exact payer among two public candidates
- keyword-digisorption-consent.test.ts: refuses BT3-056 -3 before payment and pays the full printed cost
- keyword-digisorption-consent.test.ts: accepts BT5-058 -2 through a public evolution and charges the reduced cost

## keyword-engage-consent.test.ts (1)

- keyword-engage-consent.test.ts: declines the natural End of Your Turn attack without changing board or security

## keyword-execute-consent.test.ts (2)

- keyword-execute-consent.test.ts: declines the natural End of Your Turn Execute attack
- keyword-execute-consent.test.ts: accepts Execute and chooses an unsuspended opposing Digimon

## keyword-fragment-parameters.test.ts (4)

- keyword-fragment-parameters.test.ts: trashes exactly three selected physical sources and prevents deletion
- keyword-fragment-parameters.test.ts: does not partially pay Fragment when fewer than three sources exist
- keyword-fragment-parameters.test.ts: allows public refusal with three eligible sources and then deletes the host normally
- keyword-fragment-parameters.test.ts: uses EX12-060 Fragment 2 through a public battle and preserves its host

## keyword-guard-lifecycle.test.ts (18: 14 it, 4 it.each)

- keyword-guard-lifecycle.test.ts: [each] %s pays its own printed Guard to save another Digimon from public Gaia Force
- keyword-guard-lifecycle.test.ts: [each] prevents opposing %s relocation by paying Guard, destination %s
- keyword-guard-lifecycle.test.ts: [each] face-up security grants Guard to the actual ME holder, faceUp: %s
- keyword-guard-lifecycle.test.ts: drops a face-up EX12-072 Guard grant after public security exchange
- keyword-guard-lifecycle.test.ts: re-enters Guard after EX12-072 is publicly replayed from hand
- keyword-guard-lifecycle.test.ts: one paid grantor saves both targets of public Iron-Fisted Onslaught after its aura disappears
- keyword-guard-lifecycle.test.ts: offers each eligible Guard holder in order until one pays for the shared leave event
- keyword-guard-lifecycle.test.ts: allows a Guard holder targeted by the same simultaneous deletion to pay with the other holder
- keyword-guard-lifecycle.test.ts: does not prevent a leave when Guard's self-deletion is stopped by Evade
- keyword-guard-lifecycle.test.ts: a Paishu produced by public evolution pays Guard without granting the parent Guard
- keyword-guard-lifecycle.test.ts: does not use Guard when an opposing DP reduction deletes another Digimon by rule
- keyword-guard-lifecycle.test.ts: does not use Guard against a public battle deletion
- keyword-guard-lifecycle.test.ts: does not use Guard to prevent the controller's public Heat Viper payment
- keyword-guard-lifecycle.test.ts: a lone security-granted Guard holder cannot pay to save itself
- keyword-guard-lifecycle.test.ts: only the actual granted holder's controller can answer its Guard decision
- keyword-guard-lifecycle.test.ts: [each] lets the controller choose %s when both keyword reactions apply
- keyword-guard-lifecycle.test.ts: keeps a paid Guard stack's inherited OnDeletion and then resolves sibling Detach
- keyword-guard-lifecycle.test.ts: falls back to Guard when Detach has only a non-Seven-Code linked card

## keyword-iceclad-security-boundary.test.ts (1)

- keyword-iceclad-security-boundary.test.ts: uses DP against a Security Digimon despite the Iceclad attacker's larger stack

## keyword-jamming-lifecycle.test.ts (4)

- keyword-jamming-lifecycle.test.ts: does not protect a Jamming attacker from a security option's deletion effect
- keyword-jamming-lifecycle.test.ts: keeps a printed Jamming attacker after losing a stronger security battle
- keyword-jamming-lifecycle.test.ts: deletes an ordinary attacker after losing the same stronger security battle
- keyword-jamming-lifecycle.test.ts: inherits Jamming from AD1-010 and survives the public security battle

## keyword-link-parameters.test.ts (5)

- keyword-link-parameters.test.ts: applies printed Link +6 capacity through a public third link
- keyword-link-parameters.test.ts: publicly pays the printed Link cost 2 and moves the exact card from hand
- keyword-link-parameters.test.ts: publicly links an Appmon card, pays its printed cost, and preserves its instance
- keyword-link-parameters.test.ts: refuses a public Link intent when the controller cannot pay its printed cost
- keyword-link-parameters.test.ts: uses printed Link Max +1 through a second public link intent

## keyword-material-save-parameters.test.ts (6)

- keyword-material-save-parameters.test.ts: accepts Material Save 1 and saves exactly one eligible source
- keyword-material-save-parameters.test.ts: accepts Material Save 4 and saves four of six eligible sources
- keyword-material-save-parameters.test.ts: accepts Material Save 2 and places the exact eligible materials under the Tamer
- keyword-material-save-parameters.test.ts: refuses Material Save when no Tamer exists and trashes the host and materials
- keyword-material-save-parameters.test.ts: honors an explicit public refusal when an eligible Tamer is available
- keyword-material-save-parameters.test.ts: accepts the printed Material Save 3 maximum and preserves all selected source identities

## keyword-mind-link-boundaries.test.ts (1)

- keyword-mind-link-boundaries.test.ts: chooses one eligible Digimon and excludes a target that already has a Tamer

## keyword-piercing-lifecycle.test.ts (5)

- keyword-piercing-lifecycle.test.ts: performs exactly one security check when native Piercing wins and deletes in battle
- keyword-piercing-lifecycle.test.ts: does not check security for a winning attacker without Piercing
- keyword-piercing-lifecycle.test.ts: does not check security when native Piercing loses the battle
- keyword-piercing-lifecycle.test.ts: does not check security when equal native Piercing attackers tie and both leave play
- keyword-piercing-lifecycle.test.ts: uses a public evolution grant to give an ally Piercing, then checks exactly one security

## keyword-raid-consent.test.ts (1)

- keyword-raid-consent.test.ts: chooses one of two equal highest-DP unsuspended targets

## keyword-reboot-lifecycle.test.ts (3)

- keyword-reboot-lifecycle.test.ts: unsuspends a native BT4-070 holder during the opponent's turn
- keyword-reboot-lifecycle.test.ts: unsuspends a native BT5-069 holder while a plain Digimon stays suspended
- keyword-reboot-lifecycle.test.ts: unsuspends a BT19-020 holder through its inherited Reboot keyword

## keyword-recovery-parameters.test.ts (5)

- keyword-recovery-parameters.test.ts: publicly evolves BT7-038 and places the exact deck card face down on security
- keyword-recovery-parameters.test.ts: publicly evolves BT9-040 with its exact Angewomon source and recovers to the five-security limit
- keyword-recovery-parameters.test.ts: does not draw or change security when the Recovery deck area is empty
- keyword-recovery-parameters.test.ts: publicly plays BT2-039 and places exactly two named deck instances on security in deck order
- keyword-recovery-parameters.test.ts: does not recover from BT9-040 when its public evolution begins above five security

## keyword-retaliation-lifecycle.test.ts (4)

- keyword-retaliation-lifecycle.test.ts: deletes the winning opponent Digimon when native Retaliation loses a battle
- keyword-retaliation-lifecycle.test.ts: applies inherited Retaliation from BT19-067 and preserves source identity
- keyword-retaliation-lifecycle.test.ts: does not trigger Retaliation from a plain battle loss
- keyword-retaliation-lifecycle.test.ts: does not trigger Retaliation when an opponent effect deletes the holder

## keyword-rina-trigger-ordering.test.ts (1)

- keyword-rina-trigger-ordering.test.ts: orders two ready Rinas while excluding a spent Rina, then resolves both declines

## keyword-scapegoat-lifecycle.test.ts (5)

- keyword-scapegoat-lifecycle.test.ts: uses granted Scapegoat to sacrifice the selected ally and preserve the holder
- keyword-scapegoat-lifecycle.test.ts: allows the controller to refuse Scapegoat, sending the holder to trash
- keyword-scapegoat-lifecycle.test.ts: uses native Scapegoat from EX11-023 to preserve the host and trash the chosen ally
- keyword-scapegoat-lifecycle.test.ts: does not protect against its controller's own public delete-own effect
- keyword-scapegoat-lifecycle.test.ts: uses Scapegoat against a public opponent Gaia Force deletion and preserves the selected holder

## keyword-security-attack-accumulation.test.ts (2: 1 it, 1 it.each)

- keyword-security-attack-accumulation.test.ts: combines WarGreymon's public +1 with ST1-07's inherited +1 for three exact checks
- keyword-security-attack-accumulation.test.ts: [each] publicly applies the current -2 provider to one complete security attack with %s Security cards

## keyword-security-attack-lifecycle.test.ts (2)

- keyword-security-attack-lifecycle.test.ts: publicly grants BT26-017 Security Attack +1 and reveals two cards
- keyword-security-attack-lifecycle.test.ts: publicly applies BT19-035 Security Attack -1, then expires it

## keyword-succession-lifecycle.test.ts (13: 4 it, 9 it.each)

- keyword-succession-lifecycle.test.ts: [each] $card publishes Succession after legal evolution over $base
- keyword-succession-lifecycle.test.ts: [each] publicly removes the current $upper provider and exposes the lower $lower source for $card
- keyword-succession-lifecycle.test.ts: publicly removes BT3-056, then the exposed BT25-059 reaction fires on the next natural turn
- keyword-succession-lifecycle.test.ts: publicly loses a live copied redirector when its granted host returns to hand
- keyword-succession-lifecycle.test.ts: [each] public Chronomon attack resolves the gained effect, acceptance %s
- keyword-succession-lifecycle.test.ts: [each] public Digisorption payment with %s redirect
- keyword-succession-lifecycle.test.ts: [each] redirect is used only once after two public evolutions, copied %s
- keyword-succession-lifecycle.test.ts: uses two independent copied Digisorption redirectors for two public payments
- keyword-succession-lifecycle.test.ts: resets a copied BT3-056 redirector across a real next turn
- keyword-succession-lifecycle.test.ts: [each] supplemental failed suspension preserves redirect usage, copied %s
- keyword-succession-lifecycle.test.ts: [each] public paid suspension resolves Ceresmon's reaction, copied %s
- keyword-succession-lifecycle.test.ts: [each] public Bagramon placement copies $lender without copying Succession again
- keyword-succession-lifecycle.test.ts: [each] public Giromon placement hides $0 from Succession

## keyword-training-boundaries.test.ts (4)

- keyword-training-boundaries.test.ts: places the exact face-down deck instance during a natural main phase
- keyword-training-boundaries.test.ts: rejects a known Training activation key when the mandatory placement has no deck card
- keyword-training-boundaries.test.ts: does not advertise Training from a suspended source or accept it from the opponent
- keyword-training-boundaries.test.ts: can repeat Training after a natural full turn boundary

## keyword-unblockable-consumer.test.ts (5)

- keyword-unblockable-consumer.test.ts: skips an otherwise eligible blocker and reaches security
- keyword-unblockable-consumer.test.ts: allows a plain Agumon attack to be blocked by the same printed blocker
- keyword-unblockable-consumer.test.ts: grants Unblockable to a matching Knightmon recipient through the public attack path
- keyword-unblockable-consumer.test.ts: keeps a nonmatching recipient blockable on the same granted board
- keyword-unblockable-consumer.test.ts: expires EX4-042's matching-name grant at the natural opponent turn boundary

## keyword-use-req-field.test.ts (2)

- keyword-use-req-field.test.ts: waives BT26-080's Purple Option requirement for a TS Digimon in breeding
- keyword-use-req-field.test.ts: rejects the same Purple Option when only a non-TS green card is in breeding

## king-drasil-batch-placement-events.test.ts (1)

- king-drasil-batch-placement-events.test.ts: emits public placement events for the egg and Royal Knight tops

## king-drasil-joint-placement.test.ts (2)

- king-drasil-joint-placement.test.ts: places the revealed egg first, then all Royal Knights beneath existing cards
- king-drasil-joint-placement.test.ts: honors the activating player's explicit Royal Knight order choice

## manual-01-cards-decks-and-early-procedures.test.ts (2)

- manual-01-cards-decks-and-early-procedures.test.ts: NOW MET: a Digimon card should not be able to digivolve onto a token permanent
- manual-01-cards-decks-and-early-procedures.test.ts: moving a Digimon out of the breeding area does NOT fire its [On Play] effect, and it CAN 

## manual-02-attacks-ace-and-special-digivolution.test.ts (1)

- manual-02-attacks-ace-and-special-digivolution.test.ts: the manual's own worked example (4 [Starmons]/etc placed under Shoutmon X4, DigiXros -2 

## manual-03-effect-rules-and-keywords.test.ts (1)

- manual-03-effect-rules-and-keywords.test.ts: a Digimon standing at 0 DP after effects finish resolving is deleted at the next rule-check 

## mother-eater-placement.test.ts (2: 1 it, 1 it.each)

- mother-eater-placement.test.ts: [each] $name
- mother-eater-placement.test.ts: with an empty Digi-Egg deck still offers the 10-card-stack clause and plays three sources

## placement-remaining-fronts.test.ts (2)

- placement-remaining-fronts.test.ts: installs BT7-056 after singular relocation before the add event
- placement-remaining-fronts.test.ts: fires an acquired BT7-056 watcher once for a batch relocation

## scoped-placement-consumers.test.ts (5)

- scoped-placement-consumers.test.ts: recomputes a face-up egg source before the placement event is dispatched
- scoped-placement-consumers.test.ts: keeps a hidden deck source from activating its inherited effect
- scoped-placement-consumers.test.ts: recomputes before a top digivolution-card placement event
- scoped-placement-consumers.test.ts: recomputes before an ordinary face-up placement event
- scoped-placement-consumers.test.ts: refreshes an existing source-count aura before a hidden deck placement event

## stack-metadata-filtering.test.ts (1)

- stack-metadata-filtering.test.ts: does not trigger a stack-name watcher from a hidden Gammamon source

## stack-visible-source-priority.test.ts (4)

- stack-visible-source-priority.test.ts: copies the visible Bacchusmon watcher while ignoring a lower hidden copy
- stack-visible-source-priority.test.ts: does not expose a Bacchusmon watcher when every matching source is face down
- stack-visible-source-priority.test.ts: filters a hidden highest matching source before selecting the visible provider
- stack-visible-source-priority.test.ts: proves the public Giromon-to-Succession producer chain

## tamer-suspend-cost-consent.test.ts (3)

- tamer-suspend-cost-consent.test.ts: asks Rina's controller and, when accepted, suspends Rina and replays Aero's public effect
- tamer-suspend-cost-consent.test.ts: refusal leaves Rina and the legal opponent target unchanged
- tamer-suspend-cost-consent.test.ts: does not offer the unpayable triggered clause when Rina is already suspended

## trigger-ordering-source-departure.test.ts (11)

- trigger-ordering-source-departure.test.ts: runs a derived On Play after its deferred On Deletion body and before the older sibling
- trigger-ordering-source-departure.test.ts: offers simultaneous optional deletion triggers to the owning controllers in turn-player order
- trigger-ordering-source-departure.test.ts: offers two real trash triggers together, then drops only the source that departed
- trigger-ordering-source-departure.test.ts: keeps the public deletion priority turn-player-first with valid optional payloads
- trigger-ordering-source-departure.test.ts: resolves an inherited OnDeletion after its host leaves in battle
- trigger-ordering-source-departure.test.ts: prioritizes a public derived deletion trigger over an older pending trigger
- trigger-ordering-source-departure.test.ts: drops an enemy Bacchusmon watcher when its source is deleted before activation
- trigger-ordering-source-departure.test.ts: shows the pending enemy Bacchusmon watcher when a smaller Agumon survives
- trigger-ordering-source-departure.test.ts: preserves inherited OnDeletion through a nested Yolei deletion window
- trigger-ordering-source-departure.test.ts: drops a copied Chronomon effect when its source becomes the new top card
- trigger-ordering-source-departure.test.ts: resolves the same copied Chronomon effect while its source remains buried
