# ST-1 Gaia Red — Model

Scope: the 16 unique cards of the worldwide ST-1 *Gaia Red* starter deck
(ST1-01 … ST1-16, all Red). All implemented as DSL YAML under
`code/digimon-engine/cards/st1/` and audited `AUDITED-OK`
(`qa/qa-reports/2026-05-29-starter-decks-st1-6-faithfulness-audit.md`).
This doc models the deck as a *system* — combos, lines, win conditions — to seed
the multi-card interaction tests in `code/digimon-engine/tests/archetypes/`.

Sources cited inline: printed text (`cards/st1/<ID>.json`); shipping YAML
(`cards/st1/<ID>.yaml`); DCGO C#
(`DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_<NN>.cs`); official
`Digimon TCG resources/general_rule.pdf` §16 keyword rules.

## Card pool & roles

| Card | Role (payoff/enabler/engine/tech) | One-line function |
|---|---|---|
| ST1-01 Koromon (Lv.2 egg) | engine / inherited payoff | Inherited [Your Turn] +1000 DP **only while the carrier has 4+ digivolution cards** — the "tall stack" reward. |
| ST1-02 Biyomon (Lv.3 rookie, 3000 DP, Vaccine) | enabler | Vanilla bird-line base; no effect. |
| ST1-03 Agumon (Lv.3 rookie, 2000 DP, Vaccine) | enabler / inherited payoff | Inherited [Your Turn] +1000 DP (unconditional); the WarGreymon-line base. |
| ST1-04 Dracomon (Lv.3 rookie, 4000 DP, Data) | enabler | Vanilla dragon-line base; no effect. |
| ST1-05 Birdramon (Lv.4 champion, 5000 DP) | enabler | Vanilla; bird-line champion / digivolution source. |
| ST1-06 Coredramon (Lv.4 champion, 6000 DP, Virus) | tech | `<Blocker>`; [When Attacking] lose 2 memory. Defensive body / tempo brake. |
| ST1-07 Greymon (Lv.4 champion, 4000 DP) | engine / inherited payoff | Inherited `<Security A. +1>` — the stacking Security-Attack source. |
| ST1-08 Garudamon (Lv.5 ultimate, 7000 DP) | payoff | [When Digivolving] 1 of your Digimon gets +3000 DP for the turn (mandatory, 1 target). |
| ST1-09 MetalGreymon (Lv.5 ultimate, 7000 DP) | engine / inherited payoff | Inherited [Your Turn] when blocked, gain 3 memory (punishes the block). |
| ST1-10 Phoenixmon (Lv.6 mega, 12000 DP) | payoff | Vanilla mega top-end (bird line). |
| ST1-11 WarGreymon (Lv.6 mega, 12000 DP) | payoff (finisher body) | [Your Turn] `<Security A. +1>` for **every 2** digivolution cards — the tall-stack security multiplier. |
| ST1-12 Tai Kamiya (Tamer, cost 2) | engine (anthem) + tech | [Your Turn] **all your Digimon** +1000 DP (aura); [Security] play this Tamer free. |
| ST1-13 Shadow Wing (Option, cost 1) | tech / combat trick | [Main] 1 of your Digimon +3000 DP for turn; [Security] all your Digimon `<Security A. +1>` until end of your next turn. |
| ST1-14 Starlight Explosion (Option, cost 2) | tech (defensive) | [Main] your **Security** Digimon +7000 DP (until opp's next turn end); [Security] same +7000 for the turn. |
| ST1-15 Giga Destroyer (Option, cost 6) | removal | [Main]/[Security] delete **up to 2** opponent Digimon with 4000 DP or less. |
| ST1-16 Gaia Force (Option, cost 8) | removal (finisher) | [Main]/[Security] delete **1** opponent Digimon (no DP cap). |

## Digivolution lines

Two parallel Red lines share the Lv.2 egg ST1-01 Koromon (the only digi-egg in
the deck; cost 0). Evo costs from `evo_costs` in each card JSON / `alt_paths` in
the YAML.

- **WarGreymon line (primary payoff):**
  ST1-01 Koromon (Lv.2) → ST1-03 Agumon (Lv.3, evo cost 0 from Lv.2) →
  ST1-07 Greymon (Lv.4, evo cost 2 from Lv.3) → ST1-09 MetalGreymon (Lv.5, evo
  cost 3 from Lv.4) → ST1-11 WarGreymon (Lv.6, evo cost 4 from Lv.5).
  Colour gate: every step is Red→Red; level gates are `level_eq` of the prior
  level. This is the **tall-stack engine** — each source card in the stack
  carries an inherited payoff (Agumon +1000 DP, Greymon Sec.A+1, MetalGreymon
  on-block memory, Koromon +1000 DP at 4+ sources).
- **Bird / Garudamon line (secondary):**
  ST1-01 Koromon → ST1-02 Biyomon (Lv.3) → ST1-05 Birdramon (Lv.4) →
  ST1-08 Garudamon (Lv.5) → ST1-10 Phoenixmon (Lv.6). Mostly vanilla bodies;
  Garudamon's [When Digivolving] +3000 is the one combat trick on this line.
- **Dragon line (off-line bodies):**
  ST1-01 Koromon → ST1-04 Dracomon (Lv.3) → ST1-06 Coredramon (Lv.4). Provides
  the deck's `<Blocker>` (Coredramon); no mega.

## Named combos

### Tall-Stack Security Rush (WarGreymon checks 4)
- Cards: ST1-11 WarGreymon (top) over a full ST1-01 → ST1-03 → ST1-07 → ST1-09
  stack (4 digivolution sources), optionally amplified by ST1-13 [Security].
- Expected mechanical outcome: on the controller's turn a 5-card WarGreymon stack
  (4 sources) checks **4 security cards on one successful attack on the player**:
  base 1 + WarGreymon's own `floor(4/2)=2` + Greymon's inherited `+1` = 4. (At
  base 1 a vanilla attacker checks 1; the +3 here is the combo's whole payoff.)
  DP side-effect from the same stack: Agumon inherited +1000 and Koromon +1000
  (active at 4+ sources) push the 12000-DP WarGreymon to 14000 on the controller's
  turn before any Tamer/option buff.
- Rules/keyword basis: Security Attack +X is persistent and **multiple instances
  add separately** (`general_rule.pdf` §16-3 / RULES_CONTEXT §16-3-3); negative
  result floors at 0 (§16-3-4). Security-check count is read at the
  security-check step of a successful player attack. DCGO: `ST1_11.cs`
  (`count = DigivolutionCards.Count / 2`, owner-turn gated),
  `ST1_07.cs` (`ChangeSelfSAttackStaticEffect(1, isInheritedEffect:true)`),
  `ST1_03.cs` / `ST1_01.cs` (inherited DP). Engine: `dynamic_security_attack_aura_bonus`
  + `has_keyword(SecurityAttackPlus)` stack-walk.
- Rank: **high** (this is the deck's identity and primary win condition; spans 4-5 cards).

### Tai Anthem widens removal & combat (Tai + small bodies + Giga Destroyer)
- Cards: ST1-12 Tai Kamiya + any of your Digimon + interaction with the
  *opponent's* effective DP for ST1-15 Giga Destroyer's "4000 DP or less" gate.
- Expected mechanical outcome: Tai's [Your Turn] aura raises **every one of YOUR
  Digimon** +1000 DP simultaneously (a 3000-DP Biyomon reads 4000; a 12000
  WarGreymon reads 13000) and does **not** touch opponent Digimon. The genuine
  multi-card claim: the aura is a board-wide simultaneous swing that stacks
  additively on top of inherited DP and option buffs (Tai +1000 ∧ Agumon
  inherited +1000 ∧ Shadow Wing +3000 all add on one body). Note Giga Destroyer's
  cap reads the **opponent's** DP, which Tai does not modify — so Tai is a combat
  enabler, not a removal-window widener (refuted as a removal interaction; kept as
  an anthem-stacking interaction).
- Rules/keyword basis: DP modifiers are additive and simultaneous
  (`general_rule.pdf` §3 DP; aura is a persistent [Your Turn] effect). DCGO:
  `ST1_12.cs` `ChangeDPStaticEffect(changeValue:1000, permanentCondition = owner
  battle-area Digimon, owner-turn gated)`. Engine: declarative aura, verified by
  `tick_declarative_effects` in the existing per-card test.
- Rank: **medium** (Tai is a near-auto include and the aura is central, but the
  pure aura is already pinned per-card; the *additive-stack* claim is the new part).

### Garudamon combat trick into Gaia Force lethal (buff + remove the blocker)
- Cards: ST1-08 Garudamon (or ST1-13 Shadow Wing) + ST1-16 Gaia Force / ST1-15
  Giga Destroyer, vs an opponent blocker.
- Expected mechanical outcome: remove the opponent's only `<Blocker>` / wall with
  Gaia Force (delete 1, no DP cap) or Giga Destroyer (≤4000 DP), then the buffed
  attacker (Garudamon +3000, or any body under Tai's anthem) attacks into open
  security. The system claim a test asserts: opponent battle-area count drops by 1
  (deleted body → trash) **and** the controller's attacker now has an unblocked
  line — i.e. removal + buff compose into a security-check swing the same turn.
- Rules/keyword basis: deletion to trash (`general_rule.pdf` §6 deletion);
  `<Blocker>` redirect only fires if a blocker exists (§16-4). DCGO removal
  is a plain delete (`ST1_15.cs` / `ST1_16.cs` Delete). Engine: `delete_permanent`,
  `select_opponent_permanent`.
- Rank: **medium-low** (each piece is single-card-tested; the compose is real but
  the removal halves are already well covered in `gaia_red.rs`).

### Security-board defensive lock (Starlight Explosion + Security A. on attackers)
- Cards: ST1-14 Starlight Explosion + ST1-13 Shadow Wing [Security] + any
  attacker checking extra security.
- Expected mechanical outcome: ST1-14 pumps the controller's **security Digimon**
  +7000 DP, making them brutal to attack into; ST1-13's [Security] grants all the
  controller's field Digimon `<Security A. +1>` until end of next turn — a defensive
  trigger that converts a security check into offence next turn. The interaction
  claim: two separate security-triggered options layer a defensive DP wall and an
  offensive security-check bump that both persist into the controller's next turn.
- Rules/keyword basis: `general_rule.pdf` §16-3 (Sec.A stacking), modifier expiry
  `end_of_your_next_turn` vs `end_of_opponents_next_turn` (the audit's ST2-14
  lesson on expiry windows). DCGO `ST1_13.cs` / `ST1_14.cs`.
- Rank: **low** (security-trigger lines are situational; both halves are
  single-card-tested; cross-card persistence is the only new assertion).

## Playstyle
- **Class:** aggro / tempo with a single tall-stack engine. The deck wins by
  digivolving the Agumon→WarGreymon line tall, then converting accumulated
  `<Security A.>` into multi-card security checks while inherited/aura DP keeps
  the attacker out of trade range.
- **Tempo:** Red's signature low evo costs (0/2/3/4 up the WarGreymon line) let
  the deck climb fast; Coredramon's [When Attacking] −2 memory is the one tempo
  brake / defensive option.
- **Memory curve:** cheap rookies (cost 0 digivolves from egg), mid options
  (Shadow Wing 1, Starlight Explosion 2, Tai 2), and a heavy removal/finisher tail
  (Giga Destroyer 6, Gaia Force 8). MetalGreymon's inherited "+3 memory when
  blocked" can refund a turn's tempo if the opponent walls.

## Win conditions
1. **Tall-stack security rush:** WarGreymon (or a deep stack) checking 3-4
   security on a single connected attack, racing the opponent's security to 0.
2. **Removal-into-open-security:** Gaia Force / Giga Destroyer clear the blocker
   or threat, then a buffed body (Tai anthem, Garudamon/Shadow Wing trick) connects.
3. **Anthem beatdown:** Tai's board-wide +1000 keeps every Digimon ahead in DP
   trades, grinding security through repeated unblocked attacks.

## Ranked interactions to test
1. **Tall-Stack Security Rush** — highest value: the security-check count is an
   emergent property of *four cards in one stack* (WarGreymon's `floor(n/2)` +
   Greymon's inherited `+1` + base 1), plus simultaneous inherited DP from Koromon
   (4+-source gate) and Agumon. No per-card test asserts the *summed* check count
   on a real attack; `gaia_red.rs` only checks the component bonuses statically
   (`dynamic_security_attack_aura_bonus == 2`, `has_keyword(SecurityAttackPlus(1))`).
   The interaction test should drive a real attack-on-player and assert 4 security
   cards are checked (board diff: opponent security −4 / to trash).
2. **Tai Anthem additive-stack** — second: assert Tai's +1000 aura **adds on top
   of** inherited Agumon +1000 and a Shadow Wing +3000 on one WarGreymon-line body
   simultaneously (effective DP = base + 1000 + 1000 + 3000), and that opponent
   Digimon are untouched. This is the multi-source DP-stacking claim no single-card
   test makes; the per-card Tai test only asserts the bare +1000.
3. **Garudamon/removal compose** — third: digivolve to Garudamon (+3000 to the
   attacker), Gaia Force the opponent's blocker, then assert the board diff
   (opponent field −1 to trash) and that the buffed attacker's effective DP holds
   through the removal — the cross-card "buff survives a same-turn removal" claim.

### Candidate interactions deliberately dropped (not silently truncated)
- **Coredramon `<Blocker>` + MetalGreymon "when blocked, +3 memory"** — tempting
  as a defensive engine, but the two cards are on *different lines* and never share
  a stack; the +3-memory is already pinned per-card (`st1_09_inherited_on_block...`)
  and the blocker redirect is a single-card mechanic. No genuine cross-card system.
- **Giga Destroyer window widened by Tai** — *refuted*: Giga Destroyer's "4000 DP
  or less" reads the **opponent's** DP, which Tai's owner-only aura does not change.
  Not a real interaction (documented above so the negative is on record).
- **Starlight Explosion security-wall + Shadow Wing security Sec.A** — real but
  **low rank**: both halves are security-triggered and situational, each is
  single-card-tested, and the only new assertion is cross-turn modifier persistence
  (better covered as an expiry regression than an archetype combo). Dropped from the
  top-3 author list.
- **Bird line (Biyomon→Birdramon→Phoenixmon) bodies** — all vanilla; nothing to
  compose. Garudamon is the only bird-line card with a tested effect.
