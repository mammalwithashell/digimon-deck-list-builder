# BT15 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 102
- Scripts with effects: 102
- Total effects: 299
- Factory effects: 86
- Activate effects: 213

## Per-Card Breakdown

```
BT15_005: 1 effects
  [EffectTiming.OnUnTappedAnyone] draw (inherited) (1/turn)
BT15_055: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] reboot
BT15_056: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] effect_immunity
  [EffectTiming.OnTappedAnyone] suspend (inherited) (1/turn)
BT15_057: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT15_058: 5 effects
  [factory] blocker
  [factory] alt_digivolve_req
  [EffectTiming.OnTappedAnyone] suspend (inherited) (1/turn)
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend, grant_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend, grant_cannot_unsuspend
BT15_059: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [factory] reboot
BT15_060: 5 effects
  [factory] blocker
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnAllyAttack] de_digivolve (inherited) (1/turn)
  [EffectTiming.None] no-action
BT15_061: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [factory] reboot
BT15_062: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] play_card
  [factory] reboot
BT15_063: 4 effects
  [factory] blocker
  [factory] alt_digivolve_req
  [EffectTiming.OnTappedAnyone] unsuspend (inherited) (1/turn)
  [EffectTiming.OnTappedAnyone] digivolve
BT15_064: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] de_digivolve (inherited) (1/turn)
  [EffectTiming.OnAllyAttack] delete (1/turn)
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT15_065: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] restrict_attack, gain_keyword_cannot_attack_player, grant_cannot_attack
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, de_digivolve
  [factory] security_attack_plus
BT15_066: 4 effects
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnAllyAttack] de_digivolve
  [EffectTiming.OnEndTurn] delete, play_card
  [factory] reboot
BT15_067: 4 effects
  [factory] blocker
  [factory] alt_digivolve_req
  [EffectTiming.OnTappedAnyone] play_card (1/turn)
  [EffectTiming.OnEnterFieldAnyone] bounce (1/turn)
BT15_086: 6 effects
  [EffectTiming.OnStartMainPhase] gain_memory, trash_from_hand
  [factory] security_play
  [EffectTiming.OnDeclaration] mind_link
  [factory] jamming
  [factory] blocker
  [EffectTiming.OnEndTurn] play_card (inherited)
BT15_087: 6 effects
  [factory] set_memory_3
  [factory] security_play
  [EffectTiming.OnDeclaration] mind_link
  [factory] reboot
  [factory] alliance
  [EffectTiming.OnEndTurn] play_card (inherited)
BT15_096: 4 effects
  [EffectTiming.OptionSkill] trash_from_hand, add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.SecuritySkill] trash_from_hand, add_to_hand, reveal_and_select
BT15_097: 2 effects
  [EffectTiming.OptionSkill] delete, trash_from_hand
  [factory] security_play
BT15_002: 1 effects
  [EffectTiming.OnAddHand] change_dp (inherited) (1/turn)
BT15_019: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_digivolution_cards
BT15_020: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] draw, gain_keyword_blocker
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT15_021: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] gain_keyword_cannot_attack (inherited) (1/turn)
BT15_022: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack
  [factory] jamming
BT15_023: 1 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_digivolution_cards
BT15_024: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, play_card, cost_reduction
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT15_025: 2 effects
  [factory] rush
  [factory] jamming
BT15_026: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnAddHand] effect_immunity (1/turn)
BT15_027: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] play_card
  [factory] blocker
BT15_028: 1 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_digivolution_cards
BT15_029: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
BT15_030: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, bounce
  [EffectTiming.OnDestroyedAnyone] trash_digivolution_cards, bounce, effect_immunity
BT15_031: 3 effects
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAllyAttack] bounce
  [EffectTiming.OnEndTurn] delete, play_card
BT15_032: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce (1/turn)
  [EffectTiming.OnAllyAttack] bounce (1/turn)
  [EffectTiming.OnAllyAttack] gain_memory
BT15_083: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAddHand] gain_memory, suspend
BT15_090: 2 effects
  [EffectTiming.OptionSkill] bounce
  [factory] security_play
BT15_091: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] digivolve
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT15_101: 5 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] evade
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnTappedAnyone] unsuspend (1/turn)
BT15_004: 1 effects
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged) (inherited)
BT15_043: 2 effects
  [EffectTiming.OnStartMainPhase] change_dp, suspend, effect_immunity
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT15_044: 1 effects
  [EffectTiming.OnDestroyedAnyone] gain_keyword_cannot_unsuspend
BT15_045: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT15_046: 1 effects
  [EffectTiming.OnTappedAnyone] draw (1/turn)
BT15_047: 2 effects
  [EffectTiming.None] effect_immunity
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT15_048: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_unsuspend, suspend
  [factory] dp_modifier
BT15_049: 4 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] change_dp, redirect_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, redirect_attack
  [EffectTiming.None] effect_immunity
BT15_050: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] play_card
BT15_051: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_memory
  [factory] dp_modifier
BT15_052: 3 effects
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAllyAttack] bounce
  [EffectTiming.OnEndTurn] delete, play_card
BT15_053: 3 effects
  [EffectTiming.OnStartMainPhase] suspend, gain_keyword_piercing
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_piercing
  [EffectTiming.None] effect_immunity
BT15_054: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend (1/turn)
  [EffectTiming.OnMove] suspend (1/turn)
BT15_085: 2 effects
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] suspend, redirect_attack
BT15_094: 2 effects
  [EffectTiming.OptionSkill] change_dp, suspend
  [factory] security_play
BT15_095: 3 effects
  [EffectTiming.OptionSkill] suspend
  [EffectTiming.OptionSkill] destroy_security, add_temp_effect, effect_immunity
  [EffectTiming.SecuritySkill] add_to_hand, suspend
BT15_006: 1 effects
  [EffectTiming.OnDestroyedAnyone] draw, trash_from_hand (inherited)
BT15_068: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] add_temp_effect, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT15_069: 1 effects
  [EffectTiming.OnDestroyedAnyone] draw, gain_memory
BT15_070: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [factory] retaliation
BT15_071: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] draw, delete, trash_from_hand
  [EffectTiming.OnEndAttack] gain_memory (inherited) (1/turn)
BT15_072: 2 effects
  [factory] blocker
  [EffectTiming.WhenRemoveField] no-action (1/turn)
BT15_073: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] draw, trash_from_hand
  [factory] retaliation
BT15_074: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT15_075: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, change_dp, trash_from_hand
  [EffectTiming.OnAllyAttack] draw, change_dp, trash_from_hand
  [EffectTiming.OnEndAttack] gain_memory (inherited) (1/turn)
BT15_076: 4 effects
  [factory] blast_digivolve
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
BT15_077: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] play_card
  [factory] retaliation
BT15_078: 3 effects
  [EffectTiming.OnEnterFieldAnyone] grant_skill, effect_immunity (1/turn)
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] play_card, redirect_attack
BT15_079: 4 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnEndTurn] delete, play_card
  [factory] retaliation
BT15_080: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] delete
BT15_081: 4 effects
  [factory] security_attack_plus
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] delete
BT15_098: 3 effects
  [EffectTiming.OptionSkill] play_card
  [factory] delay
  [EffectTiming.OnDestroyedAnyone] play_card
BT15_099: 2 effects
  [EffectTiming.OptionSkill] draw, delete, trash_from_hand
  [factory] security_play
BT15_100: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, return_to_deck
  [EffectTiming.OptionSkill] delete, trash_from_hand
  [factory] security_play
BT15_001: 1 effects
  [EffectTiming.OnDestroyedAnyone] add_to_hand (inherited)
BT15_007: 2 effects
  [EffectTiming.OnStartMainPhase] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT15_008: 1 effects
  [EffectTiming.OnAllyAttack] draw (1/turn)
BT15_009: 1 effects
  [EffectTiming.OnDeclaration] delete (1/turn)
BT15_010: 1 effects
  [EffectTiming.OnAllyAttack] delete (1/turn)
BT15_011: 3 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
BT15_012: 7 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] save
  [factory] material_save
  [EffectTiming.OnStartTurn] gain_memory
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.None] no-action
BT15_013: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT15_014: 4 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] delete (1/turn)
BT15_015: 1 effects
  [EffectTiming.OnDeclaration] force_attack (descriptive-tagged) (1/turn)
BT15_016: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, gain_keyword_cannot_attack
  [EffectTiming.OnEnterFieldAnyone] delete, gain_keyword_cannot_attack
  [EffectTiming.OnDestroyedAnyone] bounce (inherited)
BT15_017: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, destroy_security
  [EffectTiming.OnDestroyedAnyone] delete, destroy_security
  [EffectTiming.OnEnterFieldAnyone] play_card
BT15_018: 2 effects
  [EffectTiming.OnEndTurn] delete (1/turn)
  [EffectTiming.OnEndTurn] delete (1/turn)
BT15_082: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnReturnCardsToHandFromTrash] play_card
  [factory] security_play
BT15_088: 2 effects
  [EffectTiming.OptionSkill] play_card, add_to_hand
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT15_089: 2 effects
  [EffectTiming.OptionSkill] delete
  [factory] security_play
BT15_102: 3 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEndTurn] mill (1/turn)
BT15_003: 1 effects
  [EffectTiming.OnAllyAttack] gain_memory (inherited) (1/turn)
BT15_033: 1 effects
  [factory] barrier
BT15_034: 2 effects
  [EffectTiming.OnLoseSecurity] change_dp (inherited) (1/turn)
  [EffectTiming.OnStartMainPhase] add_to_hand, add_to_security, destroy_security
BT15_035: 4 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, change_security_attack
  [EffectTiming.OnDestroyedAnyone] trash_from_hand, change_security_attack
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited)
BT15_036: 3 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
BT15_037: 4 effects
  [factory] barrier
  [factory] barrier
  [EffectTiming.OnLoseSecurity] gain_memory (1/turn)
  [EffectTiming.OnDiscardSecurity] play_card
BT15_038: 4 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnLoseSecurity] recovery (1/turn)
BT15_039: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] add_temp_effect, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] add_temp_effect, effect_immunity
  [EffectTiming.None] grant_skill
  [EffectTiming.None] grant_skill (inherited)
BT15_040: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
BT15_041: 3 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnDestroyedAnyone] change_dp
  [EffectTiming.OnEndTurn] delete, play_card
BT15_042: 3 effects
  [EffectTiming.OnLoseSecurity] add_to_security (1/turn)
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
BT15_084: 4 effects
  [EffectTiming.OnDiscardSecurity] change_security_attack (descriptive-tagged)
  [factory] set_memory_3
  [EffectTiming.OnLoseSecurity] suspend, change_security_attack (1/turn)
  [factory] security_play
BT15_092: 3 effects
  [EffectTiming.OnDiscardSecurity] no-action
  [EffectTiming.OptionSkill] play_card, add_to_security, destroy_security
  [EffectTiming.SecuritySkill] no-action
BT15_093: 2 effects
  [EffectTiming.OptionSkill] change_dp, destroy_security
  [factory] security_play
```


## Cross-Validation Results

Checked 102 cards against digimoncard.io effect text.

### Forward Mismatches (API mentions X, script missing)

```
BT15-001: API has 'bounce' but script missing implementation
BT15-007: API has 'mill' but script missing implementation
BT15-013: API has 'bounce' but script missing implementation
BT15-014: API has 'blocker' but script missing implementation
BT15-029: API has 'bounce' but script missing implementation
BT15-030: API has 'mill' but script missing implementation
BT15-031: API has 'blocker' but script missing implementation
BT15-031: API has 'digivolve_into' but script missing implementation
BT15-040: API has 'dp_modification' but script missing implementation
BT15-050: API has 'piercing' but script missing implementation
BT15-051: API has 'draw_keyword' but script missing implementation
BT15-052: API has 'digivolve_into' but script missing implementation
BT15-052: API has 'piercing' but script missing implementation
BT15-061: API has 'attack_prevention' but script missing implementation
BT15-061: API has 'destruction_immunity' but script missing implementation
BT15-063: API has 'suspend_target' but script missing implementation
BT15-066: API has 'digivolve_into' but script missing implementation
BT15-074: API has 'attack_prevention' but script missing implementation
BT15-078: API has 'piercing' but script missing implementation
BT15-079: API has 'digivolve_into' but script missing implementation
BT15-082: API has 'security_trash' but script missing implementation
BT15-088: API has 'bounce' but script missing implementation
BT15-089: API has 'security_trash' but script missing implementation
BT15-092: API has 'dp_modification' but script missing implementation
```

### Reverse Mismatches (Script claims X, API doesn't mention)

```
BT15-012: script has '_is_material_save' but API text doesn't mention it
BT15-012: script has '_is_save' but API text doesn't mention it
```

### Timing Mismatches

```
BT15-014: has inherited effect text but no is_inherited_effect flag
BT15-026: has inherited effect text but no is_inherited_effect flag
BT15-031: has inherited effect text but no is_inherited_effect flag
BT15-038: has inherited effect text but no is_inherited_effect flag
BT15-049: has inherited effect text but no is_inherited_effect flag
BT15-050: has inherited effect text but no is_inherited_effect flag
BT15-052: has inherited effect text but no is_inherited_effect flag
BT15-076: has inherited effect text but no is_inherited_effect flag
BT15-078: has inherited effect text but no is_inherited_effect flag
BT15-083: timing 'Security' -> is_security_effect not found
BT15-085: timing 'Security' -> is_security_effect not found
BT15-098: timing 'Security' -> is_security_effect not found
```

### Structural Warnings

```
BT15-014: API has inherited effect but script has no is_inherited_effect
BT15-026: API has inherited effect but script has no is_inherited_effect
BT15-031: API has inherited effect but script has no is_inherited_effect
BT15-038: API has inherited effect but script has no is_inherited_effect
BT15-049: API has inherited effect but script has no is_inherited_effect
BT15-050: API has inherited effect but script has no is_inherited_effect
BT15-052: API has inherited effect but script has no is_inherited_effect
BT15-076: API has inherited effect but script has no is_inherited_effect
BT15-078: API has inherited effect but script has no is_inherited_effect
BT15-083: API has security effect but script has no is_security_effect
BT15-085: API has security effect but script has no is_security_effect
BT15-098: API has security effect but script has no is_security_effect
```

