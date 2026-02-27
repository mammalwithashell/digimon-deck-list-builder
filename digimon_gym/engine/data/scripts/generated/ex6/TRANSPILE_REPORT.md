# EX6 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 74
- Scripts with effects: 74
- Total effects: 257
- Factory effects: 64
- Activate effects: 193

## Per-Card Breakdown

```
EX6_005: 1 effects
  [EffectTiming.OnStartMainPhase] gain_memory, add_to_hand (inherited)
EX6_036: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] play_token (descriptive-tagged) (inherited)
EX6_037: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] draw
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
EX6_038: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] change_dp
  [EffectTiming.OnAddDigivolutionCards] draw (1/turn)
  [factory] dp_modifier
EX6_039: 4 effects
  [EffectTiming.BeforePayCost] cost_reduction, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] play_token (descriptive-tagged) (inherited)
EX6_040: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] change_dp
  [EffectTiming.OnAddDigivolutionCards] gain_keyword_blocker, gain_keyword_reboot (1/turn)
  [factory] dp_modifier
EX6_041: 3 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve (inherited) (1/turn)
EX6_042: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] no-action
  [EffectTiming.OnDeclaration] force_attack, effect_immunity
  [EffectTiming.OnAddDigivolutionCards] gain_keyword_blocker, gain_keyword_reboot (1/turn)
  [EffectTiming.WhenPermanentWouldBeDeleted] trash_digivolution_cards (inherited) (1/turn)
EX6_043: 5 effects
  [EffectTiming.OnStartMainPhase] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
  [factory] blocker
  [factory] jamming
EX6_044: 6 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] reboot
  [EffectTiming.OnDeclaration] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.None] no-action (inherited)
EX6_002: 1 effects
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
EX6_012: 2 effects
  [factory] blocker
  [factory] jamming
EX6_013: 2 effects
  [factory] jamming
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory
EX6_014: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
EX6_015: 3 effects
  [EffectTiming.OnEnterFieldAnyone] bounce, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] bounce, effect_immunity
  [EffectTiming.OnAddDigivolutionCards] play_card (1/turn)
EX6_066: 2 effects
  [EffectTiming.OptionSkill] bounce
  [EffectTiming.SecuritySkill] bounce
EX6_004: 1 effects
  [EffectTiming.OnTappedAnyone] change_dp (inherited) (1/turn)
EX6_032: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX6_033: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX6_034: 4 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEndAttack] play_card (inherited) (1/turn)
EX6_035: 5 effects
  [factory] blast_digivolve
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
EX6_006: 3 effects
  [EffectTiming.None] cost_reduction (inherited)
  [EffectTiming.OnStartMainPhase] effect_immunity
  [EffectTiming.OnEndTurn] play_card, effect_immunity
EX6_045: 2 effects
  [EffectTiming.OnDestroyedAnyone] delete
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
EX6_046: 2 effects
  [EffectTiming.OnDestroyedAnyone] draw, trash_from_hand
  [factory] dp_modifier
EX6_047: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [factory] dp_modifier
EX6_048: 5 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, effect_immunity
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
EX6_049: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [factory] dp_modifier
EX6_050: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] gain_memory, trash_from_hand
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand (inherited) (1/turn)
EX6_051: 4 effects
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand (inherited) (1/turn)
EX6_052: 3 effects
  [factory] scapegoat
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited) (1/turn)
EX6_053: 4 effects
  [factory] retaliation
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
  [factory] scapegoat
EX6_054: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] recovery, destroy_security
  [EffectTiming.OnEnterFieldAnyone] recovery, destroy_security
  [EffectTiming.WhenRemoveField] play_card, return_to_deck
EX6_055: 4 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [factory] rush
  [factory] security_attack_plus
EX6_056: 4 effects
  [factory] rush
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, mill
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, mill
  [EffectTiming.WhenRemoveField] no-action
EX6_057: 6 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] delete, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] delete, effect_immunity
  [EffectTiming.WhenRemoveField] no-action (1/turn)
  [EffectTiming.OnDestroyedAnyone] destroy_security (1/turn)
EX6_058: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] no-action
EX6_059: 4 effects
  [factory] scapegoat
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnDiscardHand] play_card (1/turn)
EX6_060: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand, suspend
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand, suspend
  [EffectTiming.WhenRemoveField] no-action
EX6_061: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand (1/turn)
  [EffectTiming.WhenRemoveField] no-action
EX6_069: 3 effects
  [EffectTiming.OptionSkill] no-action
  [factory] delay
  [EffectTiming.OnDestroyedAnyone] play_card
EX6_070: 5 effects
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OptionSkill] delete, effect_immunity
  [factory] delay
  [EffectTiming.OnEndTurn] delete
  [EffectTiming.SecuritySkill] delete
EX6_071: 1 effects
  [EffectTiming.OptionSkill] delete, trash_from_hand
EX6_073: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnAllyAttack] destroy_security
EX6_074: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend, digivolve
  [EffectTiming.OnEndTurn] play_card (1/turn)
  [factory] security_play
EX6_001: 1 effects
  [EffectTiming.OnAddDigivolutionCards] gain_memory (inherited) (1/turn)
EX6_007: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] change_dp
  [EffectTiming.OnAddDigivolutionCards] draw (1/turn)
  [factory] dp_modifier
EX6_008: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] change_dp
  [EffectTiming.OnAddDigivolutionCards] gain_keyword_raid, gain_keyword_piercing (1/turn)
  [factory] dp_modifier
EX6_009: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] change_security_attack (descriptive-tagged)
  [EffectTiming.OnAddDigivolutionCards] gain_keyword_raid, gain_keyword_piercing (1/turn)
  [EffectTiming.OnAttackTargetChanged] destroy_security (inherited) (1/turn)
EX6_010: 5 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnDeclaration] delete
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.None] disable_effect (descriptive-tagged) (inherited)
EX6_011: 5 effects
  [EffectTiming.None] jogress_condition
  [factory] raid
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve, destroy_security, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve, destroy_security, effect_immunity
EX6_065: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] no-action
  [factory] delay
  [EffectTiming.WhenRemoveField] play_card
EX6_062: 4 effects
  [factory] partition
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] bounce
  [factory] security_attack_plus
EX6_072: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] play_card, jogress_condition
  [EffectTiming.SecuritySkill] add_to_hand
EX6_003: 1 effects
  [EffectTiming.OnAllyAttack] add_to_hand, add_to_security, destroy_security (inherited) (1/turn)
EX6_016: 2 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX6_017: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
EX6_018: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnStartMainPhase] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] digivolve, put_to_security, effect_immunity (1/turn)
EX6_019: 2 effects
  [factory] barrier
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
EX6_020: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX6_021: 3 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp, add_to_hand, add_to_security, destroy_security
  [EffectTiming.OnEnterFieldAnyone] change_dp, add_to_hand, add_to_security, destroy_security
  [factory] blocker
EX6_022: 4 effects
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] play_card, change_security_attack
  [EffectTiming.OnEnterFieldAnyone] play_card, change_security_attack
  [factory] alliance
EX6_023: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete, change_security_attack (1/turn)
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (1/turn)
  [EffectTiming.WhenRemoveField] add_to_hand
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited) (1/turn)
EX6_024: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] change_security_attack, effect_immunity (1/turn)
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (1/turn)
  [EffectTiming.WhenRemoveField] add_to_hand
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited) (1/turn)
EX6_025: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, change_security_attack (1/turn)
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (1/turn)
  [EffectTiming.WhenRemoveField] add_to_hand
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited) (1/turn)
EX6_026: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] change_dp, change_security_attack, gain_keyword_blocker (1/turn)
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (1/turn)
  [EffectTiming.WhenRemoveField] add_to_hand
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited) (1/turn)
EX6_027: 6 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] change_dp, destroy_security
  [EffectTiming.OnEnterFieldAnyone] change_dp, destroy_security
  [EffectTiming.OnLoseSecurity] force_attack, change_security_attack (descriptive-tagged) (1/turn)
  [EffectTiming.OnLoseSecurity] recovery (1/turn)
EX6_028: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] recovery
  [EffectTiming.OnEnterFieldAnyone] recovery
  [EffectTiming.OnAddSecurity] bounce (1/turn)
EX6_029: 3 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] play_card, put_to_security, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] play_card, put_to_security, effect_immunity
EX6_030: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp, play_card, destroy_security
  [EffectTiming.WhenRemoveField] destroy_security
EX6_031: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndTurn] put_to_security, effect_immunity (1/turn)
EX6_063: 4 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_barrier
  [EffectTiming.OnStartMainPhase] gain_keyword_barrier
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend
  [factory] security_play
EX6_064: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnTappedAnyone] suspend, digivolve
  [factory] security_play
EX6_067: 2 effects
  [EffectTiming.OptionSkill] unsuspend
  [EffectTiming.SecuritySkill] recovery, add_to_hand
EX6_068: 3 effects
  [EffectTiming.OptionSkill] add_to_security
  [factory] delay
  [EffectTiming.OnDestroyedAnyone] play_card, destroy_security
```


## Cross-Validation Results

Checked 74 cards against digimoncard.io effect text.

### Forward Mismatches (API mentions X, script missing)

```
EX6-006: API has 'mill' but script missing implementation
EX6-006: API has 'once_per_turn' but script missing implementation
EX6-010: API has 'piercing' but script missing implementation
EX6-023: API has 'bounce' but script missing implementation
EX6-024: API has 'bounce' but script missing implementation
EX6-025: API has 'bounce' but script missing implementation
EX6-025: API has 'reveal_top' but script missing implementation
EX6-026: API has 'bounce' but script missing implementation
EX6-031: API has 'play' but script missing implementation
EX6-035: API has 'dp_modification' but script missing implementation
EX6-036: API has 'play' but script missing implementation
EX6-039: API has 'play' but script missing implementation
EX6-043: API has 'play' but script missing implementation
EX6-054: API has 'mill' but script missing implementation
EX6-055: API has 'delete_opponent' but script missing implementation
EX6-058: API has 'delete_opponent' but script missing implementation
EX6-058: API has 'mill' but script missing implementation
EX6-062: API has 'piercing' but script missing implementation
EX6-067: API has 'suspend_target' but script missing implementation
EX6-071: API has 'security_trash' but script missing implementation
EX6-072: API has 'bounce' but script missing implementation
EX6-072: API has 'digivolve_into' but script missing implementation
EX6-073: API has 'mill' but script missing implementation
```

### Reverse Mismatches (Script claims X, API doesn't mention)

```
EX6-062: script has '_is_partition' but API text doesn't mention it
```

### Timing Mismatches

```
EX6-011: has inherited effect text but no is_inherited_effect flag
EX6-012: has inherited effect text but no is_inherited_effect flag
EX6-027: has inherited effect text but no is_inherited_effect flag
EX6-028: has inherited effect text but no is_inherited_effect flag
EX6-029: has inherited effect text but no is_inherited_effect flag
EX6-035: has inherited effect text but no is_inherited_effect flag
EX6-065: timing 'Security' -> is_security_effect not found
EX6-068: timing 'Security' -> is_security_effect not found
EX6-069: timing 'Security' -> is_security_effect not found
EX6-071: timing 'Security' -> is_security_effect not found
```

### Structural Warnings

```
EX6-011: API has inherited effect but script has no is_inherited_effect
EX6-012: API has inherited effect but script has no is_inherited_effect
EX6-027: API has inherited effect but script has no is_inherited_effect
EX6-028: API has inherited effect but script has no is_inherited_effect
EX6-029: API has inherited effect but script has no is_inherited_effect
EX6-035: API has inherited effect but script has no is_inherited_effect
EX6-065: API has security effect but script has no is_security_effect
EX6-068: API has security effect but script has no is_security_effect
EX6-069: API has security effect but script has no is_security_effect
EX6-071: API has security effect but script has no is_security_effect
```

