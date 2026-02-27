# ST12 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 12
- Scripts with effects: 12
- Total effects: 28
- Factory effects: 11
- Activate effects: 17

## Per-Card Breakdown

```
ST12_11: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] de_digivolve (1/turn)
ST12_16: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] delete
  [factory] security_play
ST12_01: 1 effects
  [factory] dp_modifier
ST12_04: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory (1/turn)
  [factory] dp_modifier
ST12_06: 1 effects
  [factory] dp_modifier
ST12_08: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_temp_effect, attack_unsuspended
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
ST12_09: 2 effects
  [factory] blocker
  [factory] security_attack_plus
ST12_10: 3 effects
  [factory] blitz
  [EffectTiming.OnAllyAttack] play_card
  [EffectTiming.OnEnterFieldAnyone] change_dp (1/turn)
ST12_14: 2 effects
  [EffectTiming.OptionSkill] gain_memory, change_dp, gain_keyword_piercing
  [EffectTiming.SecuritySkill] gain_memory, add_to_hand
ST12_15: 5 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] cost_reduction
  [EffectTiming.OnDeclaration] cost_reduction
  [EffectTiming.SecuritySkill] add_to_hand, reveal_and_select
ST12_12: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [factory] decoy
ST12_13: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] reboot
  [factory] reboot_non_self
```


## Cross-Validation Results

Checked 12 cards against digimoncard.io effect text.

### Reverse Mismatches (Script claims X, API doesn't mention)

```
ST12-12: script has '_is_decoy' but API text doesn't mention it
```

### Timing Mismatches

```
ST12-10: timing 'When Digivolving' -> is_when_digivolving not found
ST12-13: has inherited effect text but no is_inherited_effect flag
```

### Structural Warnings

```
ST12-13: API has inherited effect but script has no is_inherited_effect
```

