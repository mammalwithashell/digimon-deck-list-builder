# BT16 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 103
- Scripts with effects: 103
- Total effects: 363
- Factory effects: 139
- Activate effects: 224

## Per-Card Breakdown

```
BT16_005: 1 effects
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited) (1/turn)
BT16_049: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_memory (1/turn)
  [factory] dp_modifier
BT16_050: 2 effects
  [factory] dp_modifier_all
  [factory] dp_modifier_all
BT16_051: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] no-action
  [factory] dp_modifier
BT16_052: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
BT16_052_token: 2 effects
  [factory] decoy
  [factory] blocker
BT16_053: 5 effects
  [factory] alt_digivolve_req
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack
  [factory] dp_modifier
BT16_054: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_rush, gain_keyword_cannot_be_blocked
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_rush, gain_keyword_cannot_be_blocked
  [factory] dp_modifier_all
BT16_055: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_immune_dp_minus, gain_keyword_reboot, gain_keyword_blocker
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_immune_dp_minus, gain_keyword_reboot, gain_keyword_blocker
  [factory] dp_modifier
BT16_056: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_security, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] add_to_security, effect_immunity
  [EffectTiming.OnAddSecurity] destroy_security (1/turn)
BT16_057: 3 effects
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
BT16_058: 7 effects
  [factory] alt_digivolve_req
  [factory] collision
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] force_attack, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] force_attack, effect_immunity
  [factory] dp_modifier
BT16_059: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnEndAttack] destroy_security, unsuspend (inherited) (1/turn)
BT16_060: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] delete, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] de_digivolve (inherited) (1/turn)
BT16_061: 5 effects
  [factory] alt_digivolve_req
  [factory] collision
  [EffectTiming.OnAttackTargetChanged] digivolve
  [EffectTiming.OnDestroyedAnyone] play_card (inherited) (1/turn)
  [EffectTiming.OnEndBattle] play_card (inherited) (1/turn)
BT16_062: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.None] grant_skill
  [EffectTiming.None] grant_skill (inherited)
BT16_063: 4 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] put_to_security, effect_immunity
  [factory] partition
  [factory] partition
BT16_064: 4 effects
  [factory] alt_digivolve_req
  [factory] collision
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] unsuspend (1/turn)
BT16_065: 5 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] delete, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] delete, reveal_and_select
  [EffectTiming.OnEndTurn] play_card
BT16_087: 5 effects
  [factory] set_memory_3
  [factory] security_play
  [EffectTiming.OnDeclaration] mind_link
  [factory] blocker
  [EffectTiming.OnEndTurn] play_card (inherited)
BT16_088: 4 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnStartMainPhase] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend, de_digivolve, effect_immunity
BT16_096: 4 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] play_card, reveal_and_select
  [EffectTiming.SecuritySkill] add_to_hand, reveal_and_select
BT16_097: 2 effects
  [EffectTiming.OptionSkill] recovery, play_card
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT16_098: 1 effects
  [EffectTiming.OptionSkill] delete
BT16_002: 1 effects
  [factory] dp_modifier
BT16_016: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] digivolve (1/turn)
  [EffectTiming.OnEnterFieldAnyone] digivolve (1/turn)
  [EffectTiming.OnAllyAttack] trash_digivolution_cards (inherited)
BT16_017: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_memory (1/turn)
  [factory] dp_modifier
BT16_018: 5 effects
  [factory] alt_digivolve_req
  [factory] dp_modifier
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
BT16_019: 5 effects
  [factory] blocker
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnAllyAttack] trash_digivolution_cards (inherited)
BT16_020: 3 effects
  [factory] jamming
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory
BT16_021: 4 effects
  [factory] blocker
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnTappedAnyone] trash_digivolution_cards, gain_keyword_cannot_attack, gain_keyword_cannot_block, grant_cannot_block (1/turn)
BT16_022: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnAllyAttack] trash_digivolution_cards, change_security_attack
BT16_023: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] unsuspend, bounce
  [EffectTiming.OnEnterFieldAnyone] unsuspend, bounce
  [EffectTiming.OnEndAttack] destroy_security, unsuspend (inherited) (1/turn)
BT16_024: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, add_to_security
  [EffectTiming.OnEnterFieldAnyone] play_card, add_to_security
  [factory] blocker
BT16_025: 4 effects
  [factory] partition
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend_player, grant_cannot_unsuspend, effect_immunity
  [EffectTiming.OnAllyAttack] suspend, unsuspend, effect_immunity (1/turn)
BT16_026: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, gain_keyword_cannot_suspend_player
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, gain_keyword_cannot_suspend_player
  [EffectTiming.OnAllyAttack] delete
BT16_027: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEndAttack] bounce, unsuspend (1/turn)
BT16_028: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, unsuspend, gain_keyword_cannot_unsuspend, grant_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] digivolve
BT16_085: 4 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnStartMainPhase] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend, trash_digivolution_cards
BT16_092: 2 effects
  [EffectTiming.OptionSkill] play_card, gain_keyword_blocker, gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT16_004: 1 effects
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited) (1/turn)
BT16_037: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT16_038: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
BT16_039: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT16_040: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
  [EffectTiming.OnStartMainPhase] digivolve
  [EffectTiming.OnEnterFieldAnyone] digivolve
BT16_041: 5 effects
  [factory] retaliation
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
BT16_042: 3 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [factory] dp_modifier
BT16_043: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend
  [factory] dp_modifier
BT16_044: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEndAttack] destroy_security, unsuspend (inherited) (1/turn)
BT16_045: 3 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
BT16_046: 5 effects
  [factory] blast_digivolve
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] delete, suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnTappedAnyone] change_security_attack (descriptive-tagged) (1/turn)
BT16_047: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEndBattle] gain_memory, destroy_security
BT16_048: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, cost_reduction (1/turn)
  [EffectTiming.None] effect_immunity
  [EffectTiming.OnEndTurn] suspend, bounce, effect_immunity (1/turn)
BT16_095: 1 effects
  [EffectTiming.OptionSkill] suspend, effect_immunity
BT16_006: 1 effects
  [EffectTiming.OnDestroyedAnyone] gain_memory, trash_from_hand (inherited)
BT16_066: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_from_hand
  [EffectTiming.OnAllyAttack] draw, trash_from_hand (inherited) (1/turn)
BT16_067: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
BT16_068: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
BT16_069: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, effect_immunity
  [EffectTiming.OnAllyAttack] draw, trash_from_hand (inherited) (1/turn)
BT16_070: 4 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnEnterFieldAnyone] delete
BT16_071: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] digivolve
  [EffectTiming.OnEndAttack] delete, play_card (inherited)
BT16_072: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] play_card
BT16_073: 3 effects
  [factory] retaliation
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] play_card
BT16_074: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, effect_immunity
  [EffectTiming.OnEndAttack] destroy_security, unsuspend (inherited) (1/turn)
BT16_075: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_rush (inherited) (1/turn)
BT16_076: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] digivolve
  [EffectTiming.OnEndAttack] unsuspend (inherited) (1/turn)
BT16_077: 5 effects
  [EffectTiming.None] jogress_condition
  [factory] raid
  [factory] partition
  [factory] partition
  [EffectTiming.OnEnterFieldAnyone] play_card, gain_keyword_rush, force_attack
BT16_078: 3 effects
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnDestroyedAnyone] play_card (1/turn)
BT16_079: 5 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnAllyAttack] play_card (1/turn)
  [EffectTiming.OnEnterFieldAnyone] play_card (1/turn)
  [EffectTiming.OnEndTurn] delete (1/turn)
BT16_080: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.OnDestroyedAnyone] recovery
BT16_081: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnDestroyedAnyone] no-action (1/turn)
BT16_089: 4 effects
  [EffectTiming.BeforePayCost] cost_reduction, effect_immunity
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] delete, effect_immunity
  [factory] security_play
BT16_099: 4 effects
  [EffectTiming.OptionSkill] trash_from_hand, add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] play_card, cost_reduction (1/turn)
  [EffectTiming.SecuritySkill] trash_from_hand, add_to_hand, reveal_and_select
BT16_100: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.BeforePayCost] play_card, cost_reduction, destroy_security
  [EffectTiming.OptionSkill] delete, add_to_security
  [EffectTiming.SecuritySkill] change_dp
BT16_001: 1 effects
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT16_007: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
BT16_008: 5 effects
  [factory] jamming
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
BT16_009: 4 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] change_dp
BT16_010: 4 effects
  [factory] alt_digivolve_req
  [factory] retaliation
  [EffectTiming.OnEndTurn] delete, unsuspend
  [EffectTiming.OnDestroyedAnyone] play_card
BT16_011: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, add_to_hand
  [EffectTiming.OnEnterFieldAnyone] delete, add_to_hand
  [EffectTiming.OnDestroyedAnyone] destroy_security (inherited)
  [EffectTiming.OnReturnCardsToHandFromTrash] gain_keyword_rush (1/turn)
BT16_012: 6 effects
  [factory] partition
  [factory] partition
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
BT16_013: 5 effects
  [factory] blast_digivolve
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLoseSecurity] no-action (1/turn)
BT16_014: 3 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.None] grant_skill
BT16_015: 3 effects
  [factory] alt_digivolve_req
  [factory] blitz
  [EffectTiming.OnDestroyedAnyone] delete, play_card
BT16_084: 4 effects
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnStartMainPhase] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend, effect_immunity
  [factory] security_play
BT16_091: 2 effects
  [EffectTiming.OptionSkill] play_card, force_attack
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT16_082: 1 effects
  [EffectTiming.OnMove] add_to_hand (1/turn)
BT16_083: 2 effects
  [EffectTiming.OnDestroyedAnyone] play_card, bounce, effect_immunity
  [EffectTiming.OnEndTurn] delete, play_card, return_to_deck (1/turn)
BT16_090: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnDeclaration] play_card (1/turn)
  [factory] security_play
BT16_003: 1 effects
  [factory] blocker
BT16_029: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT16_030: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] digivolve (1/turn)
  [EffectTiming.OnEnterFieldAnyone] digivolve (1/turn)
BT16_031: 4 effects
  [factory] barrier
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
BT16_032: 4 effects
  [factory] collision
  [factory] armor_purge
  [EffectTiming.OnAttackTargetChanged] no-action (1/turn)
  [factory] alt_digivolve_req
BT16_033: 3 effects
  [factory] armor_purge
  [factory] alt_digivolve_req
  [EffectTiming.OnSecurityCheck] gain_memory, recovery
BT16_034: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, change_security_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, change_security_attack
  [EffectTiming.OnEndAttack] destroy_security, unsuspend (inherited) (1/turn)
BT16_035: 3 effects
  [factory] barrier
  [factory] reboot
  [EffectTiming.OnLoseSecurity] unsuspend (1/turn)
BT16_036: 6 effects
  [EffectTiming.None] jogress_condition
  [factory] barrier
  [factory] blocker
  [factory] partition
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEndTurn] destroy_security
BT16_086: 7 effects
  [factory] set_memory_3
  [factory] security_play
  [EffectTiming.OnDeclaration] mind_link
  [factory] blocker
  [factory] barrier
  [EffectTiming.OnEndTurn] play_card (inherited)
  [EffectTiming.None] also_treated_as (descriptive-tagged)
BT16_093: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] digivolve, gain_keyword_immune_dp_minus
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT16_094: 4 effects
  [EffectTiming.OptionSkill] add_to_hand
  [factory] delay
  [EffectTiming.OnDeclaration] trash_from_hand
  [EffectTiming.SecuritySkill] no-action
BT16_101: 6 effects
  [factory] armor_purge
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, force_attack, effect_immunity
  [EffectTiming.OnDestroyedAnyone] gain_memory (1/turn)
  [EffectTiming.OnEndBattle] gain_memory (1/turn)
  [factory] dp_modifier_all
BT16_102: 5 effects
  [factory] armor_purge
  [factory] blocker
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, unsuspend, effect_immunity
  [EffectTiming.OnLoseSecurity] no-action (1/turn)
```


## Cross-Validation Results

Checked 102 cards against digimoncard.io effect text.

### Forward Mismatches (API mentions X, script missing)

```
BT16-005: API has 'blocker' but script missing implementation
BT16-007: API has 'memory_gain' but script missing implementation
BT16-011: API has 'bounce' but script missing implementation
BT16-013: API has 'delete_opponent' but script missing implementation
BT16-013: API has 'dp_modification' but script missing implementation
BT16-018: API has 'destruction_immunity' but script missing implementation
BT16-019: API has 'suspend_target' but script missing implementation
BT16-021: API has 'attack_prevention' but script missing implementation
BT16-023: API has 'suspend_target' but script missing implementation
BT16-024: API has 'digivolve_into' but script missing implementation
BT16-029: API has 'dp_modification' but script missing implementation
BT16-030: API has 'dp_modification' but script missing implementation
BT16-031: API has 'bounce' but script missing implementation
BT16-031: API has 'dp_modification' but script missing implementation
BT16-036: API has 'dp_modification' but script missing implementation
BT16-038: API has 'digivolve_into' but script missing implementation
BT16-038: API has 'piercing' but script missing implementation
BT16-052: API has 'attack_prevention' but script missing implementation
BT16-052: API has 'blocker' but script missing implementation
BT16-052: API has 'play' but script missing implementation
BT16-053: API has 'attack_prevention' but script missing implementation
BT16-055: API has 'de_digivolve' but script missing implementation
BT16-057: API has 'attack_prevention' but script missing implementation
BT16-065: API has 'digivolve_into' but script missing implementation
BT16-074: API has 'security_trash' but script missing implementation
BT16-075: API has 'bounce' but script missing implementation
BT16-076: API has 'delete_opponent' but script missing implementation
BT16-080: API has 'dp_modification' but script missing implementation
BT16-080: API has 'once_per_turn' but script missing implementation
BT16-082: API has 'reveal_top' but script missing implementation
BT16-084: API has 'bounce' but script missing implementation
BT16-084: API has 'dp_modification' but script missing implementation
BT16-085: API has 'bounce' but script missing implementation
BT16-087: API has 'piercing' but script missing implementation
BT16-088: API has 'bounce' but script missing implementation
BT16-091: API has 'digivolve_into' but script missing implementation
BT16-092: API has 'destruction_immunity' but script missing implementation
BT16-092: API has 'digivolve_into' but script missing implementation
BT16-094: API has 'dp_modification' but script missing implementation
BT16-094: API has 'reveal_top' but script missing implementation
BT16-094: API has 'security_trash' but script missing implementation
BT16-095: API has 'dp_modification' but script missing implementation
BT16-096: API has 'mill' but script missing implementation
BT16-097: API has 'digivolve_into' but script missing implementation
BT16-099: API has 'mill' but script missing implementation
BT16-100: API has 'mill' but script missing implementation
```

### Reverse Mismatches (Script claims X, API doesn't mention)

```
BT16-012: script has '_is_partition' but API text doesn't mention it
BT16-025: script has '_is_partition' but API text doesn't mention it
BT16-036: script has '_is_partition' but API text doesn't mention it
BT16-063: script has '_is_partition' but API text doesn't mention it
BT16-077: script has '_is_partition' but API text doesn't mention it
```

### Timing Mismatches

```
BT16-013: has inherited effect text but no is_inherited_effect flag
BT16-014: timing 'When Attacking' -> is_on_attack not found
BT16-014: timing 'When Digivolving' -> is_when_digivolving not found
BT16-015: timing 'When Digivolving' -> is_when_digivolving not found
BT16-025: has inherited effect text but no is_inherited_effect flag
BT16-026: has inherited effect text but no is_inherited_effect flag
BT16-027: has inherited effect text but no is_inherited_effect flag
BT16-029: has inherited effect text but no is_inherited_effect flag
BT16-030: has inherited effect text but no is_inherited_effect flag
BT16-031: has inherited effect text but no is_inherited_effect flag
BT16-038: has inherited effect text but no is_inherited_effect flag
BT16-046: has inherited effect text but no is_inherited_effect flag
BT16-052: has inherited effect text but no is_inherited_effect flag
BT16-080: [Once Per Turn] in API but no set_max_count_per_turn
BT16-080: timing 'When Digivolving' -> is_when_digivolving not found
BT16-095: timing 'Security' -> is_security_effect not found
BT16-098: timing 'Security' -> is_security_effect not found
```

### Structural Warnings

```
BT16-013: API has inherited effect but script has no is_inherited_effect
BT16-025: API has inherited effect but script has no is_inherited_effect
BT16-026: API has inherited effect but script has no is_inherited_effect
BT16-027: API has inherited effect but script has no is_inherited_effect
BT16-029: API has inherited effect but script has no is_inherited_effect
BT16-030: API has inherited effect but script has no is_inherited_effect
BT16-031: API has inherited effect but script has no is_inherited_effect
BT16-038: API has inherited effect but script has no is_inherited_effect
BT16-046: API has inherited effect but script has no is_inherited_effect
BT16-052: API has inherited effect but script has no is_inherited_effect
BT16-095: API has security effect but script has no is_security_effect
BT16-098: API has security effect but script has no is_security_effect
```

