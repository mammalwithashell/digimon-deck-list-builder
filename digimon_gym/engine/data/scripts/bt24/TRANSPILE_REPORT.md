# BT24 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 102
- Scripts with effects: 102
- Total effects: 401
- Factory effects: 131
- Activate effects: 270

## Per-Card Breakdown

```
BT24_005: 1 effects
  [EffectTiming.OnAddDigivolutionCards] reveal_and_select (inherited) (1/turn)
BT24_052: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnMove] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT24_053: 3 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] blocker
BT24_054: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnTappedAnyone] suspend (inherited) (1/turn)
BT24_055: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnTappedAnyone] suspend (inherited) (1/turn)
BT24_056: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] delete
BT24_057: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] de_digivolve
BT24_058: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] reboot
BT24_059: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card, reveal_and_select
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
BT24_060: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] play_card, reveal_and_select
  [EffectTiming.OnAddDigivolutionCards] suspend
  [EffectTiming.WhenRemoveField] play_card (inherited) (1/turn)
BT24_061: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] de_digivolve (inherited) (1/turn)
BT24_062: 7 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.None] no-action
  [EffectTiming.OnEndAttack] no-action
  [EffectTiming.OnEndTurn] no-action
  [EffectTiming.None] target_lock (inherited)
BT24_063: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT24_064: 4 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnTappedAnyone] de_digivolve (1/turn)
BT24_065: 5 effects
  [factory] alt_digivolve_req
  [factory] overclock
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.WhenRemoveField] play_card, trash_from_hand (1/turn)
BT24_086: 7 effects
  [EffectTiming.None] no-action
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] mind_link
  [factory] reboot
  [factory] alliance
  [EffectTiming.OnEndTurn] play_card (inherited)
BT24_002: 1 effects
  [EffectTiming.OnEndTurn] unsuspend (inherited) (1/turn)
BT24_019: 3 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [factory] jamming
BT24_020: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnUnTappedAnyone] draw (inherited) (1/turn)
BT24_021: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_022: 5 effects
  [factory] alt_digivolve_req
  [factory] jamming
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnUnTappedAnyone] draw (inherited) (1/turn)
BT24_023: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] jamming
BT24_024: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand, cost_reduction (1/turn)
BT24_025: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnUnTappedAnyone] digivolve
  [EffectTiming.OnEndTurn] unsuspend (1/turn)
  [factory] jamming
BT24_026: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_027: 5 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT24_028: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnUnTappedAnyone] digivolve
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
BT24_029: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndAttack] play_card (1/turn)
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
BT24_030: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnTappedAnyone] unsuspend (1/turn)
  [EffectTiming.WhenRemoveField] suspend
BT24_090: 6 effects
  [EffectTiming.None] no-action
  [factory] blocker
  [factory] alliance
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] play_card, trash_from_hand
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT24_091: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] unsuspend
  [EffectTiming.OnAllyAttack] bounce (1/turn)
BT24_004: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
BT24_042: 3 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_043: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
BT24_044: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, add_to_hand, reveal_and_select
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT24_045: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_046: 5 effects
  [factory] alt_digivolve_req
  [factory] jamming
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
BT24_047: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT24_048: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] unsuspend (inherited) (1/turn)
BT24_049: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
BT24_050: 5 effects
  [factory] alt_digivolve_req
  [factory] evade
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand (inherited) (1/turn)
BT24_051: 8 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.None] no-action
BT24_085: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEndTurn] suspend, trash_from_hand
  [factory] security_play
BT24_094: 6 effects
  [EffectTiming.None] no-action
  [factory] alliance
  [factory] dp_modifier_all
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] play_card, trash_from_hand
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT24_095: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] suspend
  [EffectTiming.OnAllyAttack] bounce (1/turn)
BT24_006: 1 effects
  [EffectTiming.WhenLinked] draw, trash_from_hand (inherited) (1/turn)
BT24_007: 1 effects
  [EffectTiming.OnDiscardHand] play_card (inherited) (1/turn)
BT24_066: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT24_067: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenLinked] play_card, trash_from_hand (1/turn)
  [factory] retaliation
BT24_068: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_069: 5 effects
  [EffectTiming.OnMove] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] blocker
  [factory] dp_modifier
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_070: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT24_071: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
BT24_072: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] security_attack_plus
BT24_073: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_074: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
BT24_075: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] security_attack_plus
BT24_076: 4 effects
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT24_077: 6 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.WhenLinked] delete
BT24_078: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] digivolve, destroy_security
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
BT24_079: 5 effects
  [factory] alt_digivolve_req
  [factory] overclock
  [EffectTiming.None] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] no-action (1/turn)
BT24_080: 6 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEndTurn] digivolve
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
BT24_081: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDestroyedAnyone] play_card
BT24_087: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.WhenLinked] draw, suspend, play_card, trash_from_hand
  [factory] security_play
BT24_088: 3 effects
  [EffectTiming.OnStartTurn] play_card
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [factory] security_play
BT24_096: 2 effects
  [EffectTiming.OnEnterFieldAnyone] return_to_deck
  [EffectTiming.OptionSkill] no-action
BT24_097: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] delete
  [EffectTiming.OnAllyAttack] delete (1/turn)
BT24_098: 3 effects
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.SecuritySkill] play_card, trash_from_hand, add_to_hand
BT24_099: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.SecuritySkill] no-action
BT24_001: 1 effects
  [EffectTiming.OnLoseSecurity] delete (inherited) (1/turn)
BT24_008: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT24_009: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_010: 4 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] de_digivolve
  [factory] raid
BT24_011: 3 effects
  [factory] alt_digivolve_req
  [factory] raid
  [factory] raid
BT24_012: 3 effects
  [factory] blocker
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT24_013: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_014: 3 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
BT24_015: 5 effects
  [factory] blocker
  [factory] alt_digivolve_req
  [EffectTiming.SecuritySkill] play_card
  [EffectTiming.OnAttackTargetChanged] delete (1/turn)
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT24_016: 4 effects
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLoseSecurity] play_card, trash_from_hand (inherited) (1/turn)
BT24_017: 2 effects
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
BT24_018: 6 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] destroy_security, unsuspend
  [EffectTiming.OnLoseSecurity] delete (1/turn)
  [EffectTiming.WhenRemoveField] no-action (1/turn)
BT24_082: 3 effects
  [EffectTiming.OnStartMainPhase] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend
  [factory] security_play
BT24_083: 3 effects
  [EffectTiming.OnStartTurn] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] security_play
BT24_089: 2 effects
  [EffectTiming.OptionSkill] play_card, trash_from_hand
  [EffectTiming.OnTappedAnyone] digivolve
BT24_100: 2 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
BT24_102: 4 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory, suspend
  [factory] dp_modifier_all
  [EffectTiming.OnEndTurn] suspend
  [factory] security_play
BT24_003: 1 effects
  [EffectTiming.OnLoseSecurity] digivolve (inherited) (1/turn)
BT24_031: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] recovery, add_to_hand, destroy_security (inherited) (1/turn)
BT24_032: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenLinked] no-action
BT24_033: 3 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [factory] barrier
BT24_034: 6 effects
  [factory] alt_digivolve_req
  [factory] barrier
  [EffectTiming.OnMove] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] barrier
BT24_035: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] barrier
BT24_036: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] change_dp
BT24_037: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] play_card (1/turn)
  [EffectTiming.WhenRemoveField] play_card (inherited) (1/turn)
BT24_038: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] change_dp (1/turn)
  [EffectTiming.WhenLinked] change_dp
BT24_039: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.SecuritySkill] play_card
  [factory] blocker
  [factory] barrier
  [EffectTiming.OnDestroyedAnyone] recovery (inherited)
BT24_040: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] no-action (1/turn)
BT24_041: 8 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [factory] blocker
  [factory] reboot
BT24_084: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnLoseSecurity] suspend, digivolve
  [factory] security_play
BT24_092: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT24_093: 3 effects
  [EffectTiming.OptionSkill] recovery, add_to_hand, destroy_security
  [EffectTiming.OnLoseSecurity] add_to_security
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT24_101: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLoseSecurity] destroy_security (1/turn)
  [EffectTiming.WhenRemoveField] destroy_security (1/turn)
```
