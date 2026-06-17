# Digimon TCG — Deep Rules Digest (verified)

> **Source:** Comprehensive Rules Manual `general_rule.pdf` Ver.3.6 (Last updated 2025/12/25) + `glossary.pdf`.
> Each claim cites a rule § (e.g. `(11-5-1-1)`) you can re-open in the PDF.
> **Loaded on demand via `/digimon-rules deep`** to act as a deep TCG thinking partner.
> **Supersedes** the retired `docs/RULES_CONTEXT.md`. PDFs live base-only:
> `BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"` → `"$BASE/Digimon TCG resources/general_rule.pdf"`.
> Keyword-by-keyword optional/mandatory lives in `keyword-semantics.md`; page pointers in `rules-index.json`.
> Re-verify and bump on manual revisions.

---

## 0. Fundamental principles (§1-2, §1-3) — internalize these first

- **Victory:** you win by a successful attack on the opponent while they have **0 security** with a Digimon that can perform ≥1 security check (1-2-3-1, 11-5-1-2-1); or by **deck-out** — opponent can't draw in their draw phase (1-2-3-2). Forfeit is an immediate loss and **does not trigger/activate effects** (1-2-4, 1-2-5).
- **Card text beats rules.** Any card text that conflicts with the rules takes priority (1-3-1).
- **Impossible / no-op actions:** an impossible action is skipped; do as many as possible (1-3-2). Changing to a state already held does nothing (1-3-3). Processing 0-or-fewer times can't be performed (1-3-4).
- **Turn player chooses first** on simultaneous choices (1-3-5).
- **Numbers:** modifiers always resolve to an integer (1-3-7), are applied to the original value and summed (1-3-8); a cost can never go below 0 (1-3-9). Shuffles must randomize (1-3-10).

## 1. Turn structure & phases (§6, glossary)

Turn order of phases (6-1-2): **Unsuspend → Draw → Breeding → Main**. The next phase doesn't start until all processing for the current one resolves (6-1-3).

- **Unsuspend (6-2):** turn player unsuspends all their Digimon and Tamers simultaneously (6-2-1). `[Start of Your Turn]` effects trigger/activate **before** unsuspending (6-2-1-1, 15-16-11). Cards needn't unsuspend in any order (6-2-1-3).
- **Draw (6-3):** draw 1 (6-3-1). **The first player skips their draw on turn 1 only** (6-3-1-1).
- **Breeding (6-4):** hatch a Digi-Egg (reveal top egg into the empty breeding area), move a Lv.3+ Digimon from breeding to battle area, or do nothing (6-4-1). Breeding-area Digimon can't activate effects, aren't affected by other cards' effects, and can't be referenced (glossary). Only 1 may occupy it.
- **Main (6-5):** in any order, any number of times (when no unresolved processing): play a Digimon/Tamer, digivolve, use an Option, link a card, attack, activate an activation-type effect, or **pass** (6-5-1).

## 2. Memory gauge — the turn-end engine (§1-4-2, §6-1-4, §6-6, glossary)

One shared gauge, your side and opponent's side, 0 in the center, max 10 each (1-4-2). Paying a memory cost moves the marker toward/into the opponent's side.

- **Turn end (6-1-4-1):** the turn ends once memory is **≥1 on the opponent's side** *and* all current-phase processing has resolved — then the turn ends with the current phase.
- **Postponement (6-6-4):** if memory moves back to **0 or more on your side** at the end of turn, the end is postponed and the current phase continues (your turn keeps going).
- **Pass (6-5-1-7-1):** declaring a pass immediately moves memory to the opponent's **3**, regardless of where it was.

## 3. Playing cards & alternate plays (§7, §1-3-11)

- **Play procedure (7-1-3):** declare + reveal → pay play cost → place on field. If the card can no longer be played, the reveal returns to origin (not "removal"), and **memory doesn't move** when a cost can't be paid (7-1-2-3, 9-1-8).
- A card **can't unsuspend the turn it was played** (7-1-2-1) — base reason Digimon can't attack the turn played (overridden by `<Rush>`/`<Vortex>`/etc.).
- **DigiXros (7-2):** play a Digimon with DigiXros requirements, placing the specified cards from hand/battle area under it, reducing the play cost per card placed. Declared right before paying cost (7-2-2-2); not mandatory (7-2-2-12); placing 0 = not performed (7-2-2-9); an "X-card DigiXros" needs exactly X placed (7-2-2-10).
- **Assembly (7-3):** like DigiXros but materials come **from the trash** (7-3-1); exact specified count required (7-3-2-4).
- **Declaring use (1-3-11):** you can't declare use of a card whose cost/alternate cost can't be paid (1-3-11-1); optional processing conditions used as an alternate cost must be processable (1-3-11-3).

## 4. Digivolution, costs & inherited effects (§8, §2-3-6, §15-3)

- **Standard (8-1):** stack the new Digimon on a card meeting one of its digivolution requirements, pay that requirement's digivolve cost, then **draw 1** (8-1-3-3). A card may have **multiple digivolution requirements** — the player chooses which to use (8-1-2-1, 2-3-6); this is the engine's "cost choice" prompt point. Digivolution may proceed even when a draw is impossible (no card drawn) (8-1-2-10).
- A digivolved Digimon is a **single Digimon** including its digivolution cards (8-1-2-3); it carries over suspend state (8-1-2-4). Only 1 digivolution per Digimon at a time (8-1-2-6). Ignoring requirements (alt-digivolve) still digivolves (8-1-2-2).
- **DNA digivolution (8-2):** the materials become *new* cards and don't carry prior state (8-2-2-1); only by an effect that specifically performs DNA digivolution (8-2-2-4).
- **Burst Digivolve (8-3):** return a Tamer to play; the top card of the burst-digivolved stack is trashed at end of the turn it digivolved (pending processing, 8-3-2-1).
- **App Fusion (8-4):** place a specified link card from a Digimon on top to digivolve, drawing 1 (8-4-3-3).
- **Inherited effects (15-3):** an effect gained from a digivolution card; can't be activated by just one card (15-3-1); treated as a **Digimon effect** regardless of the source card's category (15-3-2); `"this card"` in an inherited effect refers to the digivolution card itself (15-3-3).

## 5. Attacking, blocking, security checks, battles (§11–§14)

- **Attack timings (11-1-3):** Attack declaration → **Counter timing** → **Block timing** → Confirming success → End of attack. Each completes before the next begins (11-1-4). Only the turn player attacks (11-1-2); 1 Digimon = 1 attack (11-2-3).
- **Targets (11-2-7-1):** choose the opponent **player** or **1 of their suspended Digimon**. If the target Digimon is later removed, the target stays "removed" and the **attack fails** (11-2-6). Target can switch by rule/effect (e.g. `<Raid>`, `<Blocker>`) but not to an existing target (11-2-7-3).
- **Counter (11-3):** only one `[Counter]` per attack (11-3-2). **Block (11-4 / §12):** the non-turn player may use a `<Blocker>` Digimon; the target switches to the blocker; **only 1 block per attack** (12-1-2); a block can't be made by a Digimon that can't suspend (12-1-4), nor by the attack target itself (12-1-5).
- **Confirming success (11-5):** attack on player with **≥1 security** → perform a security check (11-5-1-1); on player with **0 security** → attacker **wins** unless it can't perform security checks (11-5-1-2-1); on a Digimon → a **battle** (11-5-1-3-1).
- **Security checks (13):** **mandatory** (13-1-3), one at a time (13-1-2). A checked card leaves the stack and is "in no area" (13-1-5); a checked **Digimon** becomes a **Security Digimon** (13-1-6) — not a regular Digimon, only its security effects work, and effects targeting regular Digimon can't hit it (glossary). Multi-checks (Security A. +x) flip one at a time, fully resolving each before the next (glossary).
- **Battles (14):** compare DP; lower **loses and is deleted** (14-2-1, 14-2-2); equal DP → **both lose** (14-2-1-3). **Security Digimon are not deleted when they lose** (14-2-3).

## 6. Effect rules — categories, timing, processing order (§15)

**The single most error-prone area for faithful implementation. Read this before scripting any non-trivial card.**

- **Effect categories (15-8-1):**
  - **Persistent (15-8-2):** always active, no trigger (e.g. `[Your Turn] +1000 DP`). Active as soon as conditions are met (15-8-2-2); off when no longer met (15-8-2-3). Multiple overlap; conflicting → **later-activated wins, except prohibiting effects** (15-8-2-5). Persistent-with-processing-conditions are active only while those conditions hold (15-8-2-6).
  - **Trigger-type (15-8-3):** triggers when conditions met, then activates (`[When Attacking]`, `[On Deletion]`, …). **Can't activate during rule/effect processing** (15-8-3-2) — it waits as *pending activation*. Once triggered it stays triggered even if memory changes (15-8-3-6). References during processing use the **state at trigger time** for removal/"when X is played" cases (15-8-3-8).
  - **Activation-type (15-8-4):** player-activated `[Main]` effects, optional; declarable only while processing/optional conditions can be met (15-8-4-3-1, 15-8-4-4-1).
  - **Immediate-type (15-8-5):** "when X **would**" effects that **interrupt right before their cause** (15-8-5-2) — e.g. `<Decoy>`, `<Armor Purge>`, `<Evade>`, `<Barrier>`, would-be-deleted/removed/digivolve. Only simultaneous with other immediate-type (15-8-5-3); activate one at a time until the interrupting cause resolves (15-8-5-4).
- **Processing order for simultaneous triggers (15-4-3):** triggers at the same timing are **pending activation**, resolved **1 at a time** (15-4-2-3); the **turn player activates all of theirs first**, then the non-turn player (15-4-3-5). Rule-check-induced triggers join the same timing (15-4-3-3). **Derived triggering** (a new trigger arising while simultaneous ones resolve) activates **before** the still-pending ones (15-4-5-2/3).
- **Trigger vs processing conditions:**
  - **Trigger conditions (15-5):** the "when …" that fires the effect; a single condition triggers **once** even if met multiple times simultaneously (15-5-2).
  - **Processing conditions (15-6):** "if"/"while" gates on a process; an effect can't activate if **none** of its processing conditions are met (15-6-3); different processes in one effect are gated independently (15-6-2).
  - **Optional processing conditions (15-7):** "**By** X, Y" — the player **chooses** whether to execute the cost X (15-7-4); you **can't perform only part** of the optional condition (15-7-3); you may choose to do it even if the *result* can't be executed (15-7-5).
- **Mandatory vs optional processing (15-9):** mandatory text **must** be executed — the player can't decline (15-9-1-2) — and is performed **whenever possible** (15-1-5). Optional text the player chooses (15-9-2). **(No-approximations corollary: never auto-resolve an optional choice or the cost half of a "by X, Y" — surface it to the action space; see rule 17.)**
- **Prohibition beats permission:** a prohibiting effect takes precedence over an enabling one (15-1-3).
- **Targets (15-10/15-11):** "X Digimon"/"X cards" choose individual targets (15-10-2); "all" doesn't choose — it's **overall processing** that also catches later-added matches (15-11-2-2). Individual processing locks onto its target even if it later stops meeting the condition (15-11-1-3-2); overall processing tracks the live set (15-11-2-3).
- **Add vs change information (15-12):** "treated as …" adds info, overwriting same-kind info, one of {play cost, level, DP} at a time (15-12-1-3); "change the name/…" changes info but can't add info a card never had (15-12-2-2).
- **Reveal/look ≠ leave area (15-15-3, 15-15-4 — flagged in the manual):** revealing or looking at cards does **not** by itself trigger "removed from your security stack"; **but** if revealed/looked-at cards are then **trashed/added to hand**, the area count *is* modified and those removal effects **do** trigger (15-15-3-3, 15-15-4-4). A classic interaction trap.
- **Effect icons (15-14):** `[X Per Turn]` counts each activation toward X (15-14-1); a card treated as a new card (e.g. via DNA digivolution) refreshes its `[X Per Turn]` count (15-14-1-4). `{Hand}` activates when revealed from hand (15-14-2); `{Trash}` from trash (15-14-3); `{Breeding}` from the breeding area (15-14-4); `{Security}` while face-up in the security stack during a check (15-14-5).
- **Gained effects (15-13):** carry over the effect's state at the time it was gained, even if the source card is buried/removed (15-13-2).

## 7. Common interaction gotchas (each cited)

- **`[Security]` effects jump the queue:** they activate immediately **without** pending activation, so they take activation precedence even when triggering simultaneously with other effects (15-16-10-2).
- **0-DP deletion is a rule check, not a battle:** a battle-area Digimon at 0 DP is deleted (17-1-3-1); a Digimon with no DP is "in no area" and trashed (17-1-3-2). Rule checks don't run during rule/effect processing (17-1-2-1) — a Digimon at 0 DP mid-processing isn't deleted until the current processing resolves. *(Matches engine memory `project_dp_zero_deletion`.)*
- **Pending processing** (e.g. "lose 3 memory at end of turn", `<Execute>`/`<Piercing>` tails, Burst Digivolve trash) resolves at its predetermined timing like a triggered effect (18-1-1) and joins simultaneous triggers (18-1-2).
- **`<Piercing>` ordering:** on a battle that deletes a Digimon with `[On Deletion]`, the `[On Deletion]` resolves **first**, then `<Piercing>` is processed before end of attack (16-6-4); the security check from `<Piercing>` is **mandatory** (16-6-3) and not vs Security Digimon (16-6-1).
- **`<Blocker>` cap:** even multiple `<Blocker>` instances allow only 1 block in a block timing (16-4-3, 12-1-2).
- **`<Collision>` forces blocks** and grants the opponent's Digimon `<Blocker>` while the carrier attacks (16-29-1) — "all opponent Digimon gain Blocker" affects Digimon; "forced to block" affects the opponent player (16-29-4).
- **`<Jamming>`** stops deletion only against **Security Digimon**, but its `<Security A. +x>` extra check still happens (glossary `<Jamming>`).
- **Removed-then-target:** if the attack target Digimon is removed mid-attack, the target stays removed and the attack fails — it does **not** redirect to the player (11-2-6).
- **Suspended is targetable:** only **suspended** opponent Digimon can be chosen as an attack target (11-2-7-1); unsuspended Digimon can only be hit via effects (e.g. `<Raid>`, `<Vortex>`, `<Execute>`).
