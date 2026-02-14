# BT23 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 103
- Scripts with effects: 103
- Total effects: 413
- Factory effects: 150
- Activate effects: 263

## Per-Card Breakdown

```
BT23_003: 1 effects
  [EffectTiming.OnEnterFieldAnyone] no-action (inherited) (1/turn)
BT23_049: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] draw, gain_memory, trash_from_hand
  [factory] dp_modifier
BT23_050: 4 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT23_051: 4 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [factory] blocker
  [EffectTiming.OnTappedAnyone] delete (1/turn)
BT23_052: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] no-action
  [factory] security_play
BT23_053: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [factory] dp_modifier
BT23_054: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT23_055: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] no-action (1/turn)
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT23_056: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAttackTargetChanged] de_digivolve (inherited) (1/turn)
BT23_057: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT23_057_token: 3 effects
  [factory] reboot
  [factory] blocker
  [factory] alliance
BT23_058: 5 effects
  [factory] alt_digivolve_req
  [factory] reboot
  [factory] blocker
  [EffectTiming.WhenRemoveField] suspend
  [EffectTiming.OnTappedAnyone] delete (1/turn)
BT23_059: 7 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDestroyedAnyone] unsuspend (1/turn)
BT23_060: 6 effects
  [factory] alt_digivolve_req
  [factory] reboot
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT23_085: 4 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnTappedAnyone] suspend, trash_from_hand
  [factory] security_play
BT23_086: 4 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, add_to_security, destroy_security
  [EffectTiming.OnEndTurn] suspend
  [factory] security_play
BT23_096: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.SecuritySkill] no-action
BT23_001: 1 effects
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT23_016: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenLinked] play_card, trash_from_hand (1/turn)
  [EffectTiming.WhenLinked] no-action
BT23_017: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand (inherited) (1/turn)
  [EffectTiming.OnAllyAttack] delete
BT23_018: 4 effects
  [factory] alt_digivolve_req
  [factory] jamming
  [EffectTiming.OnDeclaration] play_card, trash_from_hand (1/turn)
  [factory] dp_modifier
BT23_019: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] blocker
BT23_020: 3 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnTappedAnyone] no-action (1/turn)
BT23_021: 6 effects
  [EffectTiming.None] no-action
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.WhenLinked] no-action
  [EffectTiming.WhenLinked] no-action
BT23_022: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.WhenLinked] unsuspend (1/turn)
  [factory] security_attack_plus
BT23_023: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.WhenRemoveField] no-action (inherited)
BT23_024: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [factory] evade
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.WhenLinked] unsuspend (1/turn)
BT23_025: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] add_to_security
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [factory] security_play
BT23_079: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.WhenLinked] change_dp, suspend, play_card, trash_from_hand
  [factory] security_play
BT23_080: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.WhenPermanentWouldBeDeleted] no-action
  [factory] security_play
BT23_092: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.SecuritySkill] no-action
BT23_093: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] draw
  [EffectTiming.OnTappedAnyone] trash_from_hand
  [EffectTiming.SecuritySkill] no-action
BT23_002: 1 effects
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT23_037: 4 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand (inherited) (1/turn)
  [EffectTiming.OnAllyAttack] delete
BT23_038: 4 effects
  [factory] alt_digivolve_req
  [factory] dp_modifier_all
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT23_039: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenLinked] suspend
BT23_040: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] digivolve
  [factory] dp_modifier_all
BT23_041: 3 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnTappedAnyone] change_dp (1/turn)
BT23_042: 4 effects
  [factory] alt_digivolve_req
  [factory] dp_modifier_all
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [factory] dp_modifier
BT23_043: 4 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.WhenRemoveField] no-action (1/turn)
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT23_044: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
BT23_045: 5 effects
  [factory] blast_digivolve
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnTappedAnyone] unsuspend
BT23_046: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT23_047: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLoseSecurity] delete (1/turn)
BT23_083: 2 effects
  [EffectTiming.OnAddSecurity] draw, gain_memory, suspend
  [factory] security_play
BT23_084: 4 effects
  [EffectTiming.OnEndTurn] suspend, play_card, trash_from_hand
  [factory] security_play
  [factory] alliance
  [EffectTiming.OnEndTurn] no-action (inherited)
BT23_095: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.SecuritySkill] no-action
BT23_101: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT23_004: 1 effects
  [EffectTiming.OnDestroyedAnyone] no-action (inherited)
BT23_061: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
BT23_062: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] gain_memory, trash_from_hand
  [EffectTiming.OnAllyAttack] digivolve (inherited) (1/turn)
BT23_063: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] digivolve
  [EffectTiming.OnAllyAttack] digivolve (inherited) (1/turn)
BT23_064: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
BT23_065: 3 effects
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action (inherited)
BT23_066: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT23_067: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT23_068: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] delete (1/turn)
BT23_069: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
BT23_070: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndAttack] digivolve
BT23_071: 4 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnDestroyedAnyone] play_card
BT23_087: 3 effects
  [EffectTiming.OnStartMainPhase] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] security_play
BT23_088: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory, trash_from_hand
  [EffectTiming.OnEndTurn] digivolve
  [factory] security_play
BT23_097: 2 effects
  [EffectTiming.OnEnterFieldAnyone] return_to_deck
  [EffectTiming.OptionSkill] delete
BT23_098: 2 effects
  [EffectTiming.OptionSkill] play_card, trash_from_hand
  [EffectTiming.OnTappedAnyone] digivolve
BT23_005: 2 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [factory] dp_modifier
BT23_006: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT23_007: 2 effects
  [factory] alt_digivolve_req
  [factory] security_play
BT23_008: 4 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnDeclaration] play_card, trash_from_hand (1/turn)
  [factory] dp_modifier
BT23_009: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenLinked] change_dp (1/turn)
  [EffectTiming.OnEndTurn] no-action (1/turn)
BT23_010: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [factory] raid
  [factory] blocker
  [factory] blocker
BT23_011: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card, trash_from_hand (inherited)
BT23_012: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action (inherited)
BT23_013: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
BT23_014: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
BT23_015: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDestroyedAnyone] add_to_security
BT23_048: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand (inherited) (1/turn)
  [EffectTiming.OnAllyAttack] delete
BT23_078: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [factory] security_play
BT23_091: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.SecuritySkill] no-action
BT23_072: 3 effects
  [EffectTiming.OnDeclaration] draw
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnStartMainPhase] play_card (inherited)
BT23_073: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenRemoveField] no-action (1/turn)
  [EffectTiming.None] cost_reduction
BT23_074: 5 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
BT23_075: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.WhenRemoveField] play_card, trash_from_hand
  [EffectTiming.OnEndTurn] delete (1/turn)
BT23_076: 2 effects
  [EffectTiming.OnEnterFieldAnyone] recovery, add_to_hand, destroy_security
  [EffectTiming.OnTappedAnyone] digivolve
BT23_077: 4 effects
  [EffectTiming.None] no-action
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnTappedAnyone] de_digivolve
BT23_089: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.WhenRemoveField] suspend, trash_digivolution_cards
  [factory] security_play
BT23_090: 4 effects
  [factory] set_memory_3
  [factory] dp_modifier_all
  [EffectTiming.OnEndTurn] suspend, play_card, trash_from_hand
  [factory] security_play
BT23_099: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] draw
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT23_100: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnDeclaration] play_card, trash_from_hand
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT23_026: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnTappedAnyone] change_dp (inherited) (1/turn)
BT23_027: 5 effects
  [factory] alt_digivolve_req
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] barrier
BT23_028: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] no-action
BT23_029: 4 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
  [EffectTiming.OnTappedAnyone] change_dp (inherited) (1/turn)
BT23_030: 4 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnDeclaration] play_card, trash_from_hand (1/turn)
  [factory] alliance
BT23_031: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] alliance
BT23_032: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.WhenRemoveField] no-action (inherited)
BT23_033: 6 effects
  [factory] alt_digivolve_req
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] recovery (1/turn)
  [EffectTiming.WhenLinked] no-action
BT23_034: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDestroyedAnyone] add_to_security
BT23_035: 5 effects
  [factory] alt_digivolve_req
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLoseSecurity] recovery (1/turn)
BT23_036: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndTurn] no-action (1/turn)
BT23_081: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.OnTappedAnyone] change_dp, suspend
  [factory] security_play
BT23_082: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [factory] security_play
BT23_094: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.SecuritySkill] no-action
BT23_102: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.OnLoseSecurity] no-action (1/turn)
```


## Cross-Validation Issues

Checked 2 cards against digimoncard.io effect text.

```
BT23-001: API text has 'draw_keyword' but script missing corresponding implementation
BT23-005: API text has 'digivolve_into' but script missing corresponding implementation
```
