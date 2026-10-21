# Comprehensive Rules Manual — changelog 3.6 → 4.3 (engine-relevant)

> **What this is.** The project's canonical rules source moved from Comprehensive Rules Manual
> **Ver. 3.6 (2025/12/25)** to **Ver. 4.3 (2026/09/18)** on 2026-10-15 (Phase 0 of the engine
> rules-conformance campaign, `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md`).
> Five revisions landed in between (3.7, 4.0, 4.1, 4.2, 4.3). This file is the renumber-aware diff
> of the two texts (`pdftotext -raw` of each PDF; units are `N-N-N. text`, paired by normalised text,
> then by fuzzy match within a chapter) with a hand-written **engine relevance** column for every
> semantic change. Use it to (a) translate any pre-4.0 rule number you meet in code, docs, memories
> or chat, and (b) audit every engine site that encodes a rule whose meaning changed.
>
> **Headline counts** (1070 units in 3.6, 1119 in 4.3): 533 unchanged · 349 renumbered-only ·
> 101 reworded (most trivially — "display format"→"orientation", page-number artefacts) · 87 removed ·
> 136 added. Bandai's own per-version list is the **Update History** at the end of the manual
> (`cr-4.3.txt`, "Update History" → entries `(2026/03/13) Ver.3.7` … `(2026/09/18) Ver. 4.3`).
>
> **Conventions.** "3.6 §x → 4.3 §y" is the renumber map. ⚠ marks a rule whose *meaning* changed.
> **REVERSED** marks a rule whose 4.3 text says the opposite of 3.6. The engine-relevance column
> says what to check; **nothing in this file changes engine behaviour** — behaviour fixes are the
> conformance campaign's Phases 2–3, one PR each.

---

## 1. Renumber map (3.6 → 4.3) — mechanical, no meaning change unless marked ⚠

### 1.1 §16 Keyword Effects — every keyword shifted **+1** (4.0 inserted §16-3 "specified cards")

| Keyword | 3.6 § | 4.3 § |   | Keyword | 3.6 § | 4.3 § |
|---|---|---|---|---|---|---|
| `<Security A.>` | 16-3 | **16-4** |   | `<Barrier>` | 16-24 | **16-25** |
| `<Blocker>` | 16-4 | **16-5** |   | `<Blast Digivolve>` | 16-25 | **16-26** |
| `<Recovery>` | 16-5 | **16-6** |   | `<Fortitude>` | 16-26 | **16-27** |
| `<Piercing>` ⚠ | 16-6 | **16-7** |   | `<Mind Link>` | 16-27 | **16-28** |
| `<Draw>` | 16-7 | **16-8** |   | `<Partition>` | 16-28 | **16-29** |
| `<Jamming>` | 16-8 | **16-9** |   | `<Collision>` | 16-29 | **16-30** |
| `<Digisorption>` | 16-9 | **16-10** |   | `<Blast DNA Digivolve>` | 16-30 | **16-31** |
| `<Reboot>` | 16-10 | **16-11** |   | `<Scapegoat>` | 16-31 | **16-32** |
| `<De-Digivolve>` | 16-11 | **16-12** |   | `<Vortex>` | 16-32 | **16-33** |
| `<Retaliation>` | 16-12 | **16-13** |   | `<Overclock>` | 16-33 | **16-34** |
| `<Digi-Burst>` | 16-13 | **16-14** |   | `<Iceclad>` | 16-34 | **16-35** |
| `<Rush>` | 16-14 | **16-15** |   | `<Decode>` | 16-35 | **16-36** |
| `<Blitz>` | 16-15 | **16-16** |   | `<Fragment>` | 16-36 | **16-37** |
| `<Delay>` | 16-16 | **16-17** |   | `<Execute>` | 16-37 | **16-38** |
| `<Decoy>` | 16-17 | **16-18** |   | `<Progress>` | 16-38 | **16-39** |
| `<Armor Purge>` | 16-18 | **16-19** |   | `<Link +>` | 16-39 | **16-40** |
| `<Save>` | 16-19 | **16-20** |   | `<Training>` | 16-40 | **16-41** |
| `<Material Save>` | 16-20 | **16-21** |   | `<Use Req.>` (new 4.0) | — | **16-42** |
| `<Evade>` | 16-21 | **16-22** |   | `<Ascension>` (new 4.0) | — | **16-43** |
| `<Raid>` | 16-22 | **16-23** |   | `<Engage>` (new 4.1) | — | **16-44** |
| `<Alliance>` | 16-23 | **16-24** |   | `<Guard>` (new 4.1) | — | **16-45** |
|  |  |  |   | `<Detach>` (new 4.2) | — | **16-46** |
|  |  |  |   | `<Succession>` (new 4.2) | — | **16-47** |

Sub-rules move with their parent (3.6 `16-16-2` "<Delay> is optional" → 4.3 `16-17-2`), **except**
the deleted "-4 overlap" rules (§3.4 below), 3.6 `16-28-5` (Partition's "specified cards") → generic
**16-3**, 3.6 `16-28-6` → **16-29-4**, and `<Piercing>`'s sub-rules (rewritten, §2).

### 1.2 Other chapters

| 3.6 § | 4.3 § | Why |
|---|---|---|
| 1-4-6 "Dice" | 1-4-6 "Counter Markers (Dice)" | retitled |
| 2-3-4 Inherited Effects, 2-3-11 Lower Text, 2-3-12 Link | **2-3-11 Lower Text** (2-3-11-2 Inherited, 2-3-11-3 Security, 2-3-11-4 Link, 2-3-11-5 Option information) | 4.0 merged the lower-text sections; 2-3-4 is now **"(Rule)"** (rewritten 4.1) |
| 2-3-5 (Rule) … 2-3-10 App Fusion req. | shifted −1: 2-3-6 Digivolution requirements → **2-3-5**, 2-3-10 → **2-3-9**, 2-3-11 Assembly → **2-3-10** | merge above |
| 2-11 Card Number, 2-12 Other Information | **2-12**, **2-13** | 2-11 Arts Digivolve inserted |
| 3-4-3 "players can't change stack order" | — (deleted) | |
| 3-4-4 … 3-4-6-x (field / breeding area) | **3-4-6**, **3-4-7-x** | 3-4-4 / 3-4-5 inserted (permanent identity, §2) |
| 4-1-1 Memory definition | 4-1-1 (shortened) + **4-2 Costs** (new 4.2: 4-2-1 cost, 4-2-2 payment is exact, 4-2-3 alternate costs) | |
| 4-2 Digimon … 4-4 Security Digimon | **4-3 … 4-5** | 4-2 Costs inserted |
| 4-2-2 "Digi-Egg on field is a Digimon" | merged into **4-3-1** | |
| 4-2-7 link limit, 4-2-8 only top card moves | **4-9-5**, **4-7-8** (and 17-1-3-2-5 for excess links) | moved to Link cards / Stacked cards |
| 4-5 Stacked Cards … 4-10 Turn Player | **4-7 … 4-12** | 4-6 DUAL Cards inserted (4.0) |
| 4-11 Display Format | **4-13 Card Orientation** | retitled; "display format" → "orientation" everywhere |
| 4-12 Draw … 4-16 Hatching | **4-14 … 4-18** | |
| 4-13-2 / 4-14-2 (self-deletion / self-trash trigger area) | **4-15-2** / **4-16-2** (reworded: "becomes pending activation after that card is placed into the trash") | |
| 4-17 Overflow, 4-18 Tokens, 4-19 Color Requirements | **4-19**, **4-21**, **4-22** | 4-20 Arts Digivolve inserted |
| 4-18-5 token removed from game | **4-21-5** ⚠ now adds "the token itself is considered to be placed in that area" | |
| 4-20 "With XX in its text", 4-21 Slashes, 4-22 "different XX", 4-23 "With/have X cards" | **4-23**, **4-24 "XX/YY"**, **4-25**, **4-26** | |
| 15-15-7 (each/every) | **4-27 "Each" or "Every"** (moved in 3.7; 4-27-2/-3 for-each stacks, 4-27-4 one trigger per event) | |
| — | **4-28 "Option Card in the Battle Area"** (4.0), **4-29 "Also"** (4.3) | new |
| 8-1-2-3 single Digimon incl. sources, 8-1-2-8 digivolving ≠ leaving | merged into **3-4-4 / 3-4-5** | |
| 8-1-2-4 … 8-1-2-10 | **8-1-2-3 … 8-1-2-8** | |
| 13-1-2 … 13-1-7-4 | **13-1-3 … 13-1-8-4** | **13-1-2** "only 1 security check per attack" inserted |
| 15-3-3 "this card" in inherited text | **15-3-4** | 15-3-3 inserted (gained inherited effects carry over) |
| 15-14-1-4 new card refreshes OPT | deleted; replaced by **15-14-1-5** reset list (15-14-1-5-1 turn change, 15-14-1-5-2 new card) | 3.6 15-14-1-5 → **15-14-1-4** |
| 15-15-3 Revealing / 15-15-4 Searching–Looking at | **15-15-3 Effects That Reveal Cards** (15-15-3-8 "Reveal", 15-15-3-9 "Search/Look at") | merged 4.0 |
| 15-15-6 unaffected-by-effects, 15-15-7 each/every, 15-15-8 …, 15-15-9 other effects, 15-15-10 multi-target | **15-15-5**, (moved to 4-27), **15-15-6 …**, **15-15-7 "activate another effect"** (rewritten), 15-15-10 **deleted** | |
| 17-1-3-1 … 17-1-3-7 flat list | **17-1-3-1 Deletion Targets** (17-1-3-1-1 0-DP Digimon) + **17-1-3-2 Trash Targets** (17-1-3-2-1 … 17-1-3-2-7) | rewritten 4.0, §2 |

---

## 2. Semantic changes — each with engine relevance

Ordered by chapter. "Check" means: find the engine/test sites (grep the cited 4.3 number and the
retired 3.6 number) and confirm behaviour against the quoted 4.3 text; a confirmed divergence
becomes a Phase-3 TDD fix.

| Rule (4.3) | Ver. | Change | Engine relevance |
|---|---|---|---|
| **§15-7-4** | 4.3 | **REVERSED.** 3.6: "A player can choose whether or not to execute the content of optional processing conditions, regardless of whether or not the content of the conditions can be executed … if executing them isn't possible, none of the processing after the conditions can be executed." 4.3: "**If executing an optional processing condition's contents isn't possible, then the player can't choose to execute them.**" | The "may decline the cost" fact engine code attributes to 15-7-4 now lives in **15-7-1 + 15-9-2-2**; 15-7-4 itself now *forbids offering an unpayable cost*. Our open gap "unpayable optional costs still park a prompt" is definitively a bug. **111 code/test sites cite 15-7-4 and are marked `[4.3 REVERSED — see changelog-3.6-to-4.3.md]`** (§4). Matches owner trigger-timing ruling 6 (2026-10-10). |
| §15-1-2 | 4.3 | Adds: "if that single effect has optional processing conditions that affect it, **decide whether or not to perform them before processing the effect**." | Prompt timing: the "by X" decision precedes the effect body, not mid-body. Check cost-prompt placement in `effect_queue` / DSL `cost:` lowering. |
| §15-3-3 | 4.3 | New: gained inherited effects carry over, with their states, when another card is stacked on top. (3.6's 15-3-3 → 15-3-4.) | Inherited-effect identity across digivolution; gained-effect lifecycle on stack. |
| §15-13-3 | 4.3 | New: a *gained* `[On Deletion]` keeps its state (pending activation) after the card is in the trash. | Gained-effect lifecycle across trash — check `OnDeletion` for modifier-granted effects. |
| §15-15-7-2 | 4.3 | Borrowed effects: if several other effects could activate, choose one, process it, then choose the next, repeating the specified number of times. | "Activate 1 of the effects below" / "activate all" lowering. |
| §15-15-7-3 | 4.3 | "activate as this card's effect" attributes the activation to the activating card. | Effect attribution / source for borrowed effects (OPT counting, "your effect" checks). |
| §15-15-7-4 | 4.2/4.3 | A **mandatory** invoking effect does **not** make the invoked effect's optional processing conditions mandatory. | Borrowed-effect costs stay optional — a prompt must still appear. |
| §15-15-3-9-2 / -9-3 | 4.3 | Search/look-at: only the acting player sees them, **except hand searches are revealed to both**; follow-up processing happens **after** the cards are returned. | Reveal visibility (infoset / observation) and ordering of post-search steps. |
| §15-12-1-3 / §15-12-2-3 ⚠ | 4.1 | Added/changed information from a **persistent** effect always overwrites and reverts when the persistent effect is lost. | Modifier layering: persistent info-change must beat a later one-shot info-change. |
| §15-14-1-5 (+ -5-1, -5-2) | 4.0 | OPT use count resets on turn change **and** when the card becomes a new card (replaces 3.6 15-14-1-4). | Verify DNA / leave-and-return reset of `[X Per Turn]`. |
| §15-4-3-5-1 / -5-2 | 4.1 | Reworded: "The **turn player chooses** 1 of their effects … then the non-turn player chooses". | Confirms the chooser is the owner of the batch (owner ruling 2). |
| §15-4-1-2 | 3.7 | New: an activated effect resolves in full even if its card becomes a new card mid-processing. | Don't abort a resolving effect when its source is deleted/digivolved mid-body. |
| §15-8-5-4 | 4.0 | "one at a time" → "**once** until the cause that first interrupted … is resolved". | Each immediate-type effect fires at most once per interrupted cause (replacement window bookkeeping). |
| §15-15-3 (whole) | 4.0 | Reveal / search / look-at merged: a revealed card can't be referenced as being in its area (15-15-3-1); trashing/adding revealed cards *does* change the area count (15-15-3-2); the reveal is one process until all cards are placed (15-15-3-4); draws during a deck reveal come from the unrevealed cards (15-15-3-5). | Reveal-zone modelling; "removed from security" triggers; draw-during-reveal ordering. 3.6's 15-15-3-3 → 15-15-3-2, 15-15-4-4 → 15-15-3-2. |
| §15-15-10 (deleted) | 4.0 | Multi-target condition-inheritance rules removed. | Target-filter parsing shouldn't rely on them; card text now carries conditions explicitly. |
| §15-16-14-1 | 4.0 | `[Counter]` redefined as a timing for **special activation-type effects** (was "triggered upon the counter timing"). | `[Counter]` is player-activated at counter timing, not auto-triggered; §11-3-1: only the *attacked* non-turn player. |
| §1-3-11 (whole) | 4.1/4.2 | Declaring **utilization** (play/digivolve/use/link): declarable only if cost or alternate cost payable (1-3-11-1); processing conditions must be met now (1-3-11-2); with optional conditions as alternate cost, declarable only if processable and **the player must perform as much of the processing as needed to pay** (1-3-11-3); DigiXros/Assembly likewise incl. declaration-triggered immediate effects (1-3-11-4). | Declare-then-pay flows (judge-quiz cluster C); legality masks for cost-reducing "by" effects; forced material placement when paying needs it. |
| §4-2 Costs | 4.2 | New: a cost is paid with memory, **exactly** the stated value (4-2-2); a changed cost is an *alternate cost* (4-2-3-1). | Cost-payment primitive; "pay X cost" semantics; no over/under-payment. |
| §4-6 DUAL Cards | 4.0 | New card kind: Digimon-or-Option; declare which information is used (4-6-2); on the field it is a Digimon and not an "Option card in the battle area" (4-6-3); Digimon info = Digimon effects, Option info = Option effects (4-6-5-3, 4-6-6-3). | DUAL handling (`project_dual_card_production_data`); effect-type attribution per face. |
| §4-20 Arts Digivolve (+ §2-11) | 4.0 | New rule: instead of the post-use trash (pending processing) one of your cards on the field **may** digivolve into the DUAL card without paying the cost; it is **overwrite processing** of the trash. | Engine models this as `Keyword::ArtsDigivolve` (extension row in the matrix); verify the replace-the-trash timing vs 9-1-5. |
| §4-19 Overflow | 4.0 | `<Overflow>` processing is **immediate even mid-processing** (4-19-2); never on entering the field / going under a card (4-19-3/-4); simultaneous instances turn-player-first (4-19-5). | Memory movement during another action's processing; ordering of multiple Overflows. |
| §4-21-5 Tokens | 4.0 | A token leaving the field is removed from the game **but is considered to have been placed in that area**. | "When a card is trashed/placed in …" triggers for tokens. |
| §4-27-4 | 3.7 | One trigger condition met several times at once triggers **once**, but "each/every" text may reference the count. | Matches owner ruling 4; batch-event triggers. |
| §4-27-2 / -3 | 3.7 | For-each stack targeting from top/bottom; "process A and B for each X" = all A then all B. | DSL lowering of "for each" with stacked targets / two processes. |
| §4-28-1 | 4.0 | "Option card in the battle area" = only an Option **placed there by an effect**. | `<Delay>` cards vs DUAL-on-field vs rule-check trash (17-1-3-2-2). |
| §4-29-1 "Also" | 4.3 | "They also gain X" shares the preceding processing's **duration and targets**. | DSL lowering of "also"; target reuse. |
| §2-3-3-3 / -4 | 4.3 | "A card with `<Blocker>`" references only cards that **always** possess it (not granted); "keyword effect that executes processing" excludes conditional (if/while/by) forms. | Predicates like `has_keyword` / "with `<X>` in its text" must look at printed, unconditional keywords. |
| §2-3-4 (Rule) | 4.1 | Enumerates `(Rule)` grants: trait (2-3-4-2), name also treated as (2-3-4-3), has [XX] in name (2-3-4-4, not "the name [XX]"), card number also treated as (2-3-4-5), deck-copy limit (2-3-4-6). | Matches the Rule-granted trait reconciliation; name-alias semantics (`reference_name_aliases_and_dcgo_name_exclusions`). |
| §2-9-2 | 4.0 | Cards with **no level displayed** (not only "Lv.−") have no level. | Tamers/Options "no level" predicates. |
| §3-1-3-1-2 / -3 | 4.0 | A new card carries over **no** statuses/states; a card that becomes a bottom card is **not** a new card. | New-card identity (`event_permanent_identity` tests). |
| §3-4-4 / §3-4-5 ⚠ | 4.1 | Each field card is a separate Digimon/Tamer; stacking (or exposing a different card beneath) means the permanent **changed into** the new card — *not* a newly placed one (absorbs 3.6 8-1-2-3 / 8-1-2-8). | Permanent identity across digivolve / de-digivolve; "when a Digimon is played" must not fire. |
| §3-4-7-3 … -8 | 4.0 | Breeding-area cards can't be affected, triggered, chosen, meet activation conditions, or **be referenced** (3-4-7-8) except by effects naming the breeding area. | Aegis wave-6 lead (`any_permanent` vs breeding) — confirmed drift area. |
| §6-2-1 | 4.0 | Unsuspend "all of their **cards** on the field" (was Digimon and Tamers). | Options in the battle area (Delay) and DUALs unsuspend too. |
| §7-1-2-1 ⚠ | 4.1 | 3.6 "a card can't **unsuspend** the turn it was played" → 4.3 "a Digimon or Tamer, etc. can't **attack** on the turn it was placed on the field". 3.6's 7-1-2-2 (would-be-played immediates) → 7-1-2-2 retained, **7-1-2-3** likewise. | Attack legality source; "placed on the field" (not only "played") — e.g. moved from breeding, placed by effect. |
| §7-2-2-7 | 4.0 | DigiXros material from the battle area **leaves the field at that timing** (was "is removed … therefore its cards are trashed"). | Leave-field triggers for DigiXros materials. |
| §7-2-2-13 (deleted) | 4.1 | "DigiXros can be performed when an effect plays the card" removed. | Effect-played DigiXros now governed only by 7-2 generally — check. |
| §8-1-1, §8-2-1, §8-3-1, §8-4-1 | 4.1 | Procedures restated as "reveal … then stack/return"; digivolve target is "their card on the field" (Tamers digivolving per card text). | Digivolve target legality incl. non-Digimon cards. |
| §8-1-3-1, §8-3-3-1, §8-4-3-1 | 4.1 | Target chosen is "one of their **cards on the field** / in the battle area". | Same. |
| §8-2-2-1 (+ -1-1 … -1-7) ⚠ | 4.0 | DNA result carries over none of the materials' states: unsuspended (-1-1), all stacked cards new (-1-2, link cards trashed first), attacker/target status lost (-1-3/-4), applied effects end (-1-5), **OPT resets (-1-6)**, **digivolving the turn it was played removes "played this turn" so it can attack (-1-7)**. | DNA orientation (design F3), new-card state, attack legality after same-turn digivolve. |
| §8-3-2-1 | 4.1 | Burst trash at end of turn skipped if no stacked cards or the card isn't a Digimon (absorbs 3.6 8-3-2-2/-3). | Burst Digivolve end-of-turn trash. |
| §8-4-1 / §8-4-3-3 | 4.0 | App Fusion: choose a pair of **linked** cards from two specified cards; the link card is stacked on top. | App-fusion material selection (`project_appmon_link_substrate`). |
| §9-1-5 ⚠ | 4.0 | Used Option is trashed **immediately** as pending processing once its first `[Main]` resolves; "other effects and rule checks don't interrupt" the gap. | Option lifecycle: no trigger window between `[Main]` resolution and the trash. |
| §10-1-2-1 | 4.3 | "a linked card is not a stacked card" (plug-in at the bottom). | Link-card vs digivolution-card distinction (already modelled). |
| §11-3-1 | 4.0 | Counter timing is when "the `[Counter]` effect of a non-turn player **who is being attacked** can activate". | `[Counter]` only for the defending player. |
| §11-4-1 | 4.0 | Block only by "a non-turn player **who is being attacked**". | Blocker legality in multi-player-ish edge cases; no change for 2-player. |
| §11-5-1-4 / -4-1 | 4.3 | Unsuccessful attack: "no other actions are taken and **the end of attack timing occurs**" (3.6: "the attack ends without anything happening"). | `[End of Attack]` / end-of-attack pending processing must fire on fizzled attacks. |
| §13-1-1 / §13-1-2 ⚠ | 3.7/4.0 | Security check = "perform a check on the opponent's security stack"; **only 1 security check per attack**, `<Security A. +N>` modifies the card count inside that one check. | Security loop: one check, N cards; `<Piercing>` cannot add a second check (16-7-5). |
| §13-1-5 | 4.0 | Checking Digimon "no longer present in the battle area" → no more checks (was "removed from the area"). | Mid-check departure of the attacker. |
| §14-2-6 (deleted) | 4.0 | "battle won't end until all processing has been resolved" removed. | End-of-battle timing now governed by 11-1-4 / general processing rules. |
| §16-7 `<Piercing>` ⚠ | 3.7/4.0 | Rewritten: 16-7-1 reads "When this Digimon deletes your opponent's Digimon in battle while attacking, it checks security before the attack ends"; 16-7-4 pending processing "**immediately before the transition to the end of attack timing**"; **16-7-5** only 1 check per attack even if `<Piercing>` activates multiple times (`<Security A.>` modifies the count); 16-7-6 no check if no Digimon is left to perform it. 3.6's 16-6-5/16-6-6 (multi-instance / 0-security wording) deleted. | `fire_piercing_or_finish` and the Piercing tests cite the old 16-6-x; verify the single-check rule and the "no Digimon left" case; 0-security now follows 11-5-1-1. |
| §16-3 (new) + 3.6 §16-28-5 | 4.0 | Generic "specified cards" definition for every angle-bracket keyword (was Partition-only). | `<Decoy (X)>`, `<Partition (…)>`, `<Decode (…)>`, `<Overclock (…)>`, `<Detach (…)>`, `<Succession (…)>` share one parenthetical-condition resolver. |
| §16-xx-4 ×17 (deleted) ⚠ | 4.0 | The per-keyword "if 1 Digimon has multiple instances of `<X>`, they overlap when they trigger" rules are gone: 3.6 16-12-4 Retaliation, 16-17-4 Decoy, 16-18-4 Armor Purge, 16-20-4 Material Save, 16-21-4 Evade, 16-22-4 Raid, **16-23-4 Alliance** ("multiple instances can be activated for a single attack"), **16-24-4 Barrier** ("up to the number of triggered instances can be activated"), 16-26-4 Fortitude, 16-28-4 Partition, 16-31-4 Scapegoat, 16-32-4 Vortex, 16-33-4 Overclock, 16-35-4 Decode, 16-36-4 Fragment, 16-37-5 Execute. Remaining multiplicity rules: `<Security A.>` 16-4-3, `<Blocker>` 16-5-3, `<Reboot>` 16-11-3, `<Link +>` 16-40-3, `<Piercing>` 16-7-5. | Keyword multiplicity may now differ (Alliance ×2 / Barrier ×2 no longer explicitly stack); falls back to 15-5-2 (one trigger per condition per event) + 15-8-5-4 (each immediate once per cause). Engine sites citing 16-12-4 / 16-24-4 are annotated `[3.6 numbering; deleted in 4.3]`. |
| §16-4 `<Security A.>` | 4.0 | "Updated 16-3" in the history: definition wording; multiplicity (16-4-3) and ≥0 floor (16-4-4) unchanged. | None. |
| §16-6 `<Recovery>` | 4.0 | Printed as `<Recovery +1>`; 3.6 16-5-3 ("if Deck is specified …") deleted. | Recovery source parsing — the area is in the icon text. |
| §16-18-1 `<Decoy>` | 4.0 | "another of your **specified** Digimon" (uses the generic 16-3). | None beyond 16-3. |
| §16-19 `<Armor Purge>` | 4.0 | 16-19-1/-2 updated wording (immediate-type, trash the top card). | None. |
| §16-34 `<Overclock>` | 4.0 | "the Digimon with this effect **attacks** a player without suspending" (was "may attack") after the optional cost. | Once the deletion cost is paid the attack is mandatory (Opt-cost→Mand, unchanged kind). |
| §16-35 `<Iceclad>` | 4.0 | 16-35-1 / -3 reworded ("effect that changes the rules of a battle"). | None. |
| §16-37-1 `<Fragment>` | 4.0 | "choosing and trashing the specified number" wording. | None. |
| §16-42 `<Use Req.>` | 4.0/4.1 | Persistent: ignore Option colour requirements with the specified Digimon/Tamers on the field (16-42-3 updated 4.1). | **No engine `Keyword` variant yet**; colour-requirement mask (`4-22`) needs a hook. |
| §16-43 `<Ascension>` | 4.0 | Trigger-type on this Digimon's deletion; **may** place this card on top of the security stack (optional). | Implemented as `Keyword::Ascension`; never verified against this text — check optionality and the "card" (top card only) placement. |
| §16-44 `<Engage>` | 4.1 | Trigger at End of Your Turn; this Digimon **may** attack (optional). | Implemented; verify it uses normal attack legality and is optional. |
| §16-45 `<Guard>` | 4.1 | Immediate-type: when another of your Digimon would leave the battle area **by an opponent's effect**, by deleting this Digimon it doesn't leave; 16-45-3 "processing is optional". | Implemented; verify the opponent's-effect cause filter and that the cost prompt is offered only when payable (15-7-4). |
| §16-46 `<Detach>` | 4.2 | Immediate-type (16-46-2 says "trigger-type" but points at 15-8-5): when this would leave other than by your effects, by trashing 1 of its specified link cards it doesn't leave; cost optional, then mandatory. | **No engine `Keyword` variant** (design notes tests exist); needs the link-card trash cost + cause filter. |
| §16-47 `<Succession>` | 4.2 | Persistent: gains all effects other than `<Succession>` on its topmost specified digivolution card. | **No engine `Keyword` variant**; needs a gained-effect aura over the topmost matching source. |
| §17-1-3 (whole) ⚠ | 4.0 | Rule-check targets enumerated and processed **simultaneously**: delete 0-DP battle-area Digimon (17-1-3-1-1); trash DP-less Digimon (17-1-3-2-1), Options in the battle area not placed by an effect (-2-2), non-Digimon in the breeding area not placed by an effect (-2-3), face-down field cards (-2-4), link cards over the limit — only the excess (-2-5), link cards not meeting requirements (-2-6), linked cards of a category not allowed by the `<Link>` note (-2-7). Updated again in 3.7 (17-1-3-7 → -2-5 wording). | `run_state_based_rules_check` covers only 17-1-3-1-1 and 17-1-3-2-1; the player chooses which excess link cards go (Q6370). |
| §18-2-2 | 4.3 | "when the player has [Kiriha]" → "**while**" (persistent condition wording in the example). | None. |

---

## 3. Housekeeping changes (no engine impact)

- Terminology: "display format" → "orientation" (1-4-4-3, 4-7-6, 4-13, 4-17-3, 8-1-2-3, 8-2-2-1-1); "Dice" → "Counter markers" (1-4-6); "object" → "card" (1-3-3); 1-3-11 "Use" → "Utilization"; 2-6-1 / 2-7-1 play/use cost "shown using a Play/Use icon"; 6-5-1-2-x "1 of your" → "one of a player's".
- 1-4-4-4: token cards must now have their card information written on them (tournament hygiene); 1-4-4-6 deleted.
- 2-3-5-2 (deck-building (Rule)) folded into 2-3-4-6.
- 3-5-3 hand "freely" → "always" look.
- 13-1-8-3-1 security battle text unchanged; 11-5-1-1-1 refers to §13 instead of 13-1.
- Page-number artefacts of `pdftotext -raw` ("can't be27") account for most "reworded" pairs with similarity > 0.98.

## 3.4 Deleted in 4.3 (3.6 numbers; nothing to re-point to)

1-3-11-1…4 (rewritten, same numbers), 1-4-4-6, 2-3-3-3 (replaced by new text), 2-3-5-2, 3-4-3, 4-2-7 (→4-9-5), 4-2-8 (→4-7-8), 4-13-2/4-14-2 (→4-15-2/4-16-2), 4-18-5 (→4-21-5), 7-2-2-13, 8-1-2-3/8-1-2-8 (→3-4-4/3-4-5), 8-2-2-1-5/-6 (rewritten), 8-3-2-2/-3 (→8-3-2-1), 9-1-5 (rewritten), 11-3-1 (rewritten), 11-5-1-4 (rewritten + -4-1), 14-2-6, 15-1-2 (rewritten), 15-14-1-4 (→15-14-1-5-2), 15-15-3-2/-4, 15-15-4-x (→15-15-3-x), 15-15-7-2/-3 (→4-27-2), 15-15-9-2 (→15-15-7-2), 15-15-10-x, 15-16-14-1 (rewritten), 16-5-3, 16-6-1/-5/-6 (Piercing rewrite), and the 17 keyword "-4 overlap" rules listed in §2.

---

## 4. Engine/test sites still encoding a reversed or deleted rule (for the conformance campaign)

Mechanical re-pointing (2026-10-15) translated every citation in `code/digimon-engine/src`,
`code/digimon-engine/tests` and `code/digimon-dsl/src` whose section number moved (651 lines in
191 files — §16 keywords +1, §4/§13/§17 shifts, `Ver.3.6` stamps → `Ver.4.3`, 3.6 page numbers →
4.3 pages). **Engine behaviour was not changed.** Two classes were *not* re-pointed and are marked
in place instead:

- **`15-7-4` — REVERSED** (111 sites, marker `[4.3 REVERSED — see changelog-3.6-to-4.3.md]`).
  Every one of these sites uses 15-7-4 to mean "the player may decline the optional cost". That
  fact is still true in 4.3 — but its home is now **§15-7-1 / §15-9-2-2**, and 4.3's §15-7-4 says
  the opposite thing: an unpayable cost **cannot be chosen**. The campaign should (a) re-cite
  15-7-1 where the comment only means "declinable", and (b) treat any site that *offers* a cost
  prompt without first checking payability as a confirmed divergence from 15-7-4.
- **Deleted overlap rules** (marker `[3.6 numbering; deleted in 4.3 — see changelog-3.6-to-4.3.md]`):
  `16-12-4` Retaliation multi-instance (`src/cards/keyword_effects.rs`, `src/game/mod.rs`),
  `16-24-4` Barrier multi-instance (`src/replacement_window.rs`, `src/cards/keyword_effects.rs`,
  `tests/cancelled_immediate_window.rs`), `16-6-6` Piercing-vs-0-security
  (`src/combat/mod.rs`, `tests/combat/piercing_security.rs`). Their behaviour now derives from
  15-5-2 / 15-8-5-4 / 11-5-1-1 — re-derive and re-cite.
- Not re-pointed (not rule citations): DCGO line ranges (`Partition.cs:9-23`), arithmetic in
  assertion messages (`12-5=7`), task/phase ranges (`Tasks 2-5`), and `§1-7-3` / `§6-2-2` in
  `tests/rules_faq/main_phase.rs`, which never existed in either version (pre-existing typos;
  the intended rules are 17-1-3-1-1 and 1-4-2-2).
- `tests/rules_faq/generated_conformance.rs` is generated from `qa/rules/faq_conformance_cases.json`
  and carries no rule numbers; it was skipped.

### 4.1 `15-7-4` REVERSED sites (file → line numbers at the time of marking)

| File | Lines |
|---|---|
| `code/digimon-engine/src/runners/replay.rs` | 755 |
| `code/digimon-engine/tests/archetypes/richard_sampson_data_squad_ex13.rs` | 283 |
| `code/digimon-engine/tests/archetypes/witchelny_ex13.rs` | 219 |
| `code/digimon-engine/tests/cards_behavioral/bt11/bt11_089.rs` | 272, 281, 445, 463, 516, 544 |
| `code/digimon-engine/tests/cards_behavioral/bt18/bt18_087.rs` | 148, 157, 162, 605 |
| `code/digimon-engine/tests/cards_behavioral/bt19/bt19_101.rs` | 39, 330, 342, 353, 395, 452, 476, 494, 518, 694, 730, 761 |
| `code/digimon-engine/tests/cards_behavioral/bt23/bt23_035.rs` | 24, 37, 129, 134, 360, 366, 397 |
| `code/digimon-engine/tests/cards_behavioral/bt23/bt23_064.rs` | 51, 83, 148 |
| `code/digimon-engine/tests/cards_behavioral/bt24/bt24_017.rs` | 37, 136, 153, 789 |
| `code/digimon-engine/tests/cards_behavioral/bt4/bt4_097.rs` | 66, 73, 193, 232 |
| `code/digimon-engine/tests/cards_behavioral/ex10/ex10_003.rs` | 62, 126 |
| `code/digimon-engine/tests/cards_behavioral/ex10/ex10_028.rs` | 100, 286, 324, 359, 547, 627, 704, 742, 798, 1137, 1140 |
| `code/digimon-engine/tests/cards_behavioral/ex10/ex10_032.rs` | 132, 489, 525, 541, 571, 584, 613, 639, 672, 704, 746, 786, 833, 881, 920, 978, 1149, 1152 |
| `code/digimon-engine/tests/cards_behavioral/ex13/ex13_030.rs` | 20, 295 |
| `code/digimon-engine/tests/cards_behavioral/ex13/ex13_059.rs` | 408 |
| `code/digimon-engine/tests/cards_behavioral/ex8/ex8_070.rs` | 21, 120, 129, 229, 257, 287, 324, 445, 488, 527, 568, 609, 651, 703, 732, 770, 777, 851, 913 |
| `code/digimon-engine/tests/cards_behavioral/ex9/ex9_027.rs` | 25, 32, 103, 135, 142, 179, 186 |
| `code/digimon-engine/tests/cards_behavioral/lm/lm_020.rs` | 16, 205 |
| `code/digimon-engine/tests/cards_behavioral/p/p_130.rs` | 165, 176, 571 |
| `code/digimon-engine/tests/decoy_partition_processing.rs` | 2, 27 |
| `code/digimon-engine/tests/keyword_phase_f/guard.rs` | 128 (explicitly asserts the 3.6 reading: "impossible payment does not suppress optional attempt") |

