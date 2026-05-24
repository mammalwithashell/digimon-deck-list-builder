## ADDED Requirements

### Requirement: BT22-008, BT22-017, BT17-007, BT17-019 inherited EoT DNA digivolve surfaces inline at trigger fire

The four Omnimon-line inherited carriers — BT22-008 Agumon, BT22-017 Gabumon, BT17-007 Agumon (Tai-themed), and BT17-019 Gabumon (Matt-themed) — SHALL author their `[End of Your Turn]` inherited DNA digivolve clause using a triggered clause with `scope: inherited`, `optional: true`, and a body invoking `may_dna_digivolve_now`. The clause SHALL surface the DNA digivolve player choice inline at end-of-turn trigger resolution, NOT defer it via `alt_path_registration` to a subsequent turn.

#### Scenario: BT22-008 inherited prompts inline at EoT

- **WHEN** BT22-008 (or a permanent stack with BT22-008 in its digivolution cards) is on the controller's field at end of the controller's turn
- **AND** the controller has at least one other own-field Digimon eligible as a DNA digivolve partner
- **AND** the controller's hand contains at least one Digimon card eligible as the DNA digivolve target
- **THEN** the engine surfaces an accept/decline prompt for the BT22-008 inherited EoT DNA digivolve as part of the EoT trigger batch resolution
- **AND** on accept, the controller picks partner and target inline, and the merged Digimon enters the battle area as part of the same EoT batch

#### Scenario: BT22-017 inherited prompts inline at EoT

- **WHEN** BT22-017 (or a permanent stack with BT22-017 in its digivolution cards) is on the controller's field at end of the controller's turn
- **AND** the controller has at least one other own-field Digimon eligible as a DNA digivolve partner
- **AND** the controller's hand contains at least one Digimon card eligible as the DNA digivolve target
- **THEN** the same inline prompt sequence as BT22-008's scenario fires

#### Scenario: BT17-007 inherited prompts inline at EoT

- **WHEN** BT17-007 is on field at end of controller's turn AND a partner + target are eligible
- **THEN** the same inline prompt sequence fires

#### Scenario: BT17-019 inherited prompts inline at EoT

- **WHEN** BT17-019 is on field at end of controller's turn AND a partner + target are eligible
- **THEN** the same inline prompt sequence fires

### Requirement: Omnimon-line EoT chain completes on a single turn

After the controller plays MetalGarurumon (cost-reduced via a Tamer with Matt Ishida in its name), uses MG's mandatory `[On Play] [When Digivolving]` effect to digivolve their Agumon (a BT22-008 carrier) into WarGreymon, and ends their turn, the engine SHALL surface and resolve the following EoT chain in a single turn:

1. BT22-008 inherited DNA digivolve prompt — accept → pick WG as partner → pick Omnimon as target.
2. The merged Omnimon enters the battle area; its `[On Play] [When Digivolving]` effect returns opponent Digimon (with ≤ Omnimon's digivolution-card count) and may delete an opponent Digimon.
3. Omnimon's `[All Turns] [Once Per Turn]` triggers if an opponent Digimon leaves the battle area, trashing one of their Option cards in the battle area and trashing their top security card.
4. Tai & Matt's `[End of Your Turn] [Once Per Turn] 1 of your Omnimon may attack a player` trigger fires — accept → designate Omnimon to attack.
5. Omnimon attacks opponent security; the BT17-015 WG inherited `[When Attacking] [Once Per Turn]` trashes the top of opponent security; the BT17-027 MG inherited `[When Attacking] [Once Per Turn]` unsuspends Omnimon (allowing follow-up attacks before turn rotates).

The full chain SHALL complete before the turn rotates to the opponent.

#### Scenario: Single-turn Omnimon EoT chain via Agumon line

- **WHEN** a behavioral test constructs the Agumon-line scenario as described
- **AND** the controller resolves each prompt in the EoT chain in the order listed above
- **THEN** Omnimon is on the field at end of resolution with stack `[Agumon, WG, MG, Omnimon]`
- **AND** opponent security count has decreased by at least 2 (one from Omnimon's All Turns, one from WG inherited When Attacking; plus the actual attack consumption)
- **AND** Omnimon is unsuspended (MG inherited unsuspend resolved)
- **AND** the turn has not yet rotated to the opponent when the chain completes
