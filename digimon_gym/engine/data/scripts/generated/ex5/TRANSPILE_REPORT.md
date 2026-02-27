# EX5 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 76
- Scripts with effects: 76
- Total effects: 248
- Factory effects: 70
- Activate effects: 178

## Per-Card Breakdown

```
BT15_086: 6 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] mind_link
  [factory] jamming
  [factory] blocker
  [EffectTiming.OnEndTurn] play_card (inherited)
EX5_005: 1 effects
  [EffectTiming.OnDestroyedAnyone] draw (inherited)
EX5_044: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] de_digivolve (inherited)
EX5_045: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
EX5_046: 5 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] trash_from_hand, add_to_hand
  [EffectTiming.WhenPermanentWouldBeDeleted] no-action (inherited)
EX5_047: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] digivolve
  [EffectTiming.OnDestroyedAnyone] de_digivolve (inherited)
EX5_048: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] force_attack, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] force_attack, effect_immunity
  [EffectTiming.OnAllyAttack] play_card, reveal_and_select (inherited) (1/turn)
EX5_049: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
EX5_050: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [factory] decoy
  [factory] blocker
EX5_051: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [factory] blocker
EX5_052: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.None] effect_immunity
  [factory] blocker
EX5_053: 4 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnSecurityCheck] play_card, grant_no_security_battle (1/turn)
  [EffectTiming.OnDestroyedAnyone] delete
EX5_054: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] add_to_security, redirect_attack (1/turn)
EX5_055: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] de_digivolve, bounce
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, bounce
  [EffectTiming.OnEndAttack] unsuspend (1/turn)
EX5_002: 1 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve (inherited) (1/turn)
EX5_015: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.WhenPermanentWouldBeDeleted] return_to_deck (inherited) (1/turn)
EX5_016: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] gain_memory (inherited) (1/turn)
EX5_017: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
EX5_018: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, trash_from_hand
  [EffectTiming.WhenPermanentWouldBeDeleted] return_to_deck (inherited) (1/turn)
EX5_019: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnAllyAttack] trash_digivolution_cards
  [EffectTiming.OnAllyAttack] gain_memory (inherited) (1/turn)
EX5_020: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] cost_reduction
  [factory] change_digi_cost
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [factory] dp_modifier
EX5_021: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnUseOption] gain_memory (1/turn)
  [EffectTiming.OnAllyAttack] gain_memory (inherited) (1/turn)
EX5_022: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards (1/turn)
  [EffectTiming.OnAllyAttack] gain_memory (inherited) (1/turn)
EX5_023: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, unsuspend
  [EffectTiming.OnAllyAttack] trash_from_hand, unsuspend (inherited) (1/turn)
EX5_024: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] bounce, unsuspend
  [EffectTiming.OnEnterFieldAnyone] bounce, unsuspend
  [EffectTiming.OnDestroyedAnyone] delete
EX5_025: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, effect_immunity (1/turn)
  [EffectTiming.OnAllyAttack] trash_digivolution_cards, effect_immunity (1/turn)
  [EffectTiming.OnDigivolutionCardDiscarded] unsuspend (1/turn)
EX5_026: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] grant_skill, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] delete, return_to_deck
EX5_065: 4 effects
  [EffectTiming.OnAddDigivolutionCards] gain_memory, suspend
  [EffectTiming.OnStartTurn] play_card
  [EffectTiming.OnStartTurn] play_card, bounce, effect_immunity
  [factory] security_play
EX5_067: 1 effects
  [EffectTiming.OptionSkill] play_card, effect_immunity
EX5_003: 1 effects
  [factory] dp_modifier
EX5_004: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
EX5_035: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
EX5_036: 1 effects
  [factory] dp_modifier
EX5_037: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnUseOption] gain_memory (1/turn)
EX5_038: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnEndBattle] unsuspend (1/turn)
EX5_039: 3 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] dp_modifier
EX5_040: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnTappedAnyone] draw (1/turn)
EX5_041: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend_player, grant_cannot_unsuspend, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend_player, grant_cannot_unsuspend, effect_immunity
  [EffectTiming.OnDestroyedAnyone] delete
EX5_042: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [factory] rush
EX5_043: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, cost_reduction (1/turn)
  [EffectTiming.OnDeclaration] play_card, cost_reduction (1/turn)
  [EffectTiming.OnEnterFieldAnyone] bounce (1/turn)
EX5_006: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
EX5_056: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
EX5_057: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
EX5_058: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
EX5_058_token: 1 effects
  [EffectTiming.OnDestroyedAnyone] trash_from_hand
EX5_059: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_retaliation
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
EX5_060: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card (1/turn)
EX5_061: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
EX5_062: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand, cost_reduction (1/turn)
  [EffectTiming.OnDeclaration] play_card, trash_from_hand, cost_reduction (1/turn)
  [EffectTiming.OnEnterFieldAnyone] draw
EX5_063: 4 effects
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] no-action
EX5_069: 3 effects
  [EffectTiming.OptionSkill] delete, trash_from_hand
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] play_card
EX5_001: 1 effects
  [EffectTiming.OnAddDigivolutionCards] digivolve (inherited) (1/turn)
EX5_007: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] gain_memory (inherited) (1/turn)
EX5_008: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
EX5_009: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnDestroyedAnyone] draw
  [factory] security_attack_plus
EX5_010: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnDestroyedAnyone] delete
  [factory] security_attack_plus
EX5_011: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, play_card
  [EffectTiming.OnDestroyedAnyone] gain_memory
  [factory] security_attack_plus
EX5_012: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] cost_reduction
  [factory] change_digi_cost
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] dp_modifier
EX5_013: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
  [EffectTiming.OnAllyAttack] no-action (1/turn)
  [EffectTiming.OnDestroyedAnyone] delete
EX5_014: 4 effects
  [factory] alt_digivolve_req
  [factory] blitz
  [factory] security_attack_plus
  [EffectTiming.OnLoseSecurity] delete (1/turn)
EX5_064: 4 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] suspend, digivolve
  [EffectTiming.OnDeclaration] suspend, digivolve
  [factory] security_play
EX5_066: 1 effects
  [EffectTiming.OptionSkill] delete, add_to_hand
EX5_073: 6 effects
  [EffectTiming.None] jogress_condition
  [factory] security_attack_plus
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.WhenRemoveField] trash_digivolution_cards
EX5_070: 5 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.SecuritySkill] add_to_hand
  [EffectTiming.OptionSkill] digivolve
  [EffectTiming.WhenRemoveField] add_to_hand, add_to_security (inherited)
EX5_071: 2 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
EX5_072: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] play_card
  [EffectTiming.SecuritySkill] add_to_hand
EX5_027: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] recovery, add_to_hand, destroy_security
  [EffectTiming.OnDestroyedAnyone] change_dp (inherited)
EX5_028: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX5_029: 3 effects
  [EffectTiming.OnAllyAttack] cost_reduction, destroy_security
  [EffectTiming.OnAllyAttack] cost_reduction
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX5_030: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnAllyAttack] digivolve
  [EffectTiming.OnDestroyedAnyone] change_dp (inherited)
EX5_031: 2 effects
  [EffectTiming.OnEnterFieldAnyone] destroy_security, unsuspend
  [EffectTiming.OnAllyAttack] add_to_security (inherited) (1/turn)
EX5_032: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [factory] blocker
EX5_033: 4 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, destroy_security, gain_keyword_rush (1/turn)
  [EffectTiming.OnAllyAttack] play_card, destroy_security, gain_keyword_rush (1/turn)
  [factory] barrier
  [EffectTiming.None] grant_skill
EX5_034: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnTappedAnyone] change_dp, change_security_attack (1/turn)
EX5_068: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] change_dp, suspend, force_attack
  [EffectTiming.SecuritySkill] change_dp, suspend
EX5_074: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] return_to_deck
  [EffectTiming.OnAllyAttack] return_to_deck
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.None] effect_immunity
```


## Cross-Validation Results

Checked 75 cards against digimoncard.io effect text.

### Forward Mismatches (API mentions X, script missing)

```
EX5-023: API has 'bounce' but script missing implementation
EX5-024: API has 'suspend_target' but script missing implementation
EX5-032: API has 'fortitude' but script missing implementation
EX5-035: API has 'fortitude' but script missing implementation
EX5-036: API has 'fortitude' but script missing implementation
EX5-037: API has 'piercing' but script missing implementation
EX5-038: API has 'piercing' but script missing implementation
EX5-039: API has 'fortitude' but script missing implementation
EX5-040: API has 'piercing' but script missing implementation
EX5-042: API has 'fortitude' but script missing implementation
EX5-049: API has 'fortitude' but script missing implementation
EX5-049: API has 'piercing' but script missing implementation
EX5-055: API has 'fortitude' but script missing implementation
EX5-056: API has 'draw_keyword' but script missing implementation
EX5-057: API has 'bounce' but script missing implementation
EX5-058: API has 'play' but script missing implementation
EX5-060: API has 'piercing' but script missing implementation
EX5-062: API has 'delete_opponent' but script missing implementation
EX5-063: API has 'memory_gain' but script missing implementation
EX5-065: API has 'digivolve_into' but script missing implementation
EX5-066: API has 'bounce' but script missing implementation
EX5-070: API has 'bounce' but script missing implementation
EX5-072: API has 'bounce' but script missing implementation
EX5-074: API has 'dp_modification' but script missing implementation
```

### Reverse Mismatches (Script claims X, API doesn't mention)

```
EX5-050: script has '_is_decoy' but API text doesn't mention it
```

### Timing Mismatches

```
EX5-013: has inherited effect text but no is_inherited_effect flag
EX5-014: timing 'When Digivolving' -> is_when_digivolving not found
EX5-024: has inherited effect text but no is_inherited_effect flag
EX5-037: has inherited effect text but no is_inherited_effect flag
EX5-038: has inherited effect text but no is_inherited_effect flag
EX5-040: has inherited effect text but no is_inherited_effect flag
EX5-041: has inherited effect text but no is_inherited_effect flag
EX5-049: has inherited effect text but no is_inherited_effect flag
EX5-053: has inherited effect text but no is_inherited_effect flag
EX5-060: has inherited effect text but no is_inherited_effect flag
EX5-066: timing 'Security' -> is_security_effect not found
EX5-067: timing 'Security' -> is_security_effect not found
EX5-069: timing 'Security' -> is_security_effect not found
EX5-071: timing 'Security' -> is_security_effect not found
```

### Structural Warnings

```
EX5-013: API has inherited effect but script has no is_inherited_effect
EX5-024: API has inherited effect but script has no is_inherited_effect
EX5-037: API has inherited effect but script has no is_inherited_effect
EX5-038: API has inherited effect but script has no is_inherited_effect
EX5-040: API has inherited effect but script has no is_inherited_effect
EX5-041: API has inherited effect but script has no is_inherited_effect
EX5-049: API has inherited effect but script has no is_inherited_effect
EX5-053: API has inherited effect but script has no is_inherited_effect
EX5-060: API has inherited effect but script has no is_inherited_effect
EX5-066: API has security effect but script has no is_security_effect
EX5-067: API has security effect but script has no is_security_effect
EX5-069: API has security effect but script has no is_security_effect
EX5-071: API has security effect but script has no is_security_effect
```

