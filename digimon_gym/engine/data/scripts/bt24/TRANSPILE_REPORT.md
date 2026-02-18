# BT24 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 102
- Scripts with effects: 102
- Total effects: 419
- Factory effects: 148
- Activate effects: 271

## Per-Card Breakdown

```
BT24_005: 1 effects
  [EffectTiming.OnAddDigivolutionCards] reveal_and_select (inherited) (1/turn)
BT24_052: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnMove] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
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
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_return_to_deck, gain_keyword_cannot_return_to_hand
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_return_to_deck, gain_keyword_cannot_return_to_hand
  [EffectTiming.WhenLinked] delete
BT24_057: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack
  [EffectTiming.OnDestroyedAnyone] gain_keyword_cannot_attack
  [EffectTiming.OnDestroyedAnyone] de_digivolve
BT24_058: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] reboot
BT24_059: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnDestroyedAnyone] play_card, reveal_and_select
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
BT24_060: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] play_card, reveal_and_select
  [EffectTiming.OnAddDigivolutionCards] suspend, force_attack
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
  [EffectTiming.OnEndAttack] play_card
  [EffectTiming.OnEndTurn] play_card
  [EffectTiming.None] target_lock (inherited)
BT24_063: 5 effects
  [factory] alt_digivolve_req
  [factory] collision
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [factory] collision
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
  [EffectTiming.WhenRemoveField] play_card (1/turn)
BT24_086: 7 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
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
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnUnTappedAnyone] draw (inherited) (1/turn)
BT24_023: 7 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] jamming
BT24_024: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnAllyAttack] play_card, cost_reduction (1/turn)
BT24_025: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnUnTappedAnyone] digivolve
  [EffectTiming.OnEndTurn] unsuspend (1/turn)
  [factory] jamming
BT24_026: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, gain_keyword_jamming, gain_keyword_blocker
  [EffectTiming.OnAllyAttack] trash_from_hand, gain_keyword_jamming, gain_keyword_blocker
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_027: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT24_028: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, gain_keyword_cannot_be_deleted_by_battle
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
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [factory] blocker
  [factory] alliance
  [EffectTiming.None] grant_skill
  [EffectTiming.OptionSkill] play_card, cost_reduction
  [EffectTiming.SecuritySkill] play_card
BT24_091: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] unsuspend, bounce
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
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnAllyAttack] trash_from_hand, suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_046: 5 effects
  [factory] alt_digivolve_req
  [factory] jamming
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
BT24_047: 3 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, unsuspend, force_attack
  [EffectTiming.OnEnterFieldAnyone] suspend, unsuspend, force_attack
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT24_048: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnDestroyedAnyone] unsuspend (inherited) (1/turn)
BT24_049: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce, suspend
  [EffectTiming.OnEnterFieldAnyone] bounce, suspend
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
BT24_050: 5 effects
  [factory] alt_digivolve_req
  [factory] evade
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
BT24_051: 9 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend, force_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend, force_attack
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnAllyAttack] unsuspend
  [factory] rush
  [EffectTiming.None] grant_skill
BT24_085: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEndTurn] suspend, force_attack
  [factory] security_play
BT24_094: 6 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [factory] alliance
  [factory] dp_modifier_all
  [EffectTiming.None] grant_skill
  [EffectTiming.OptionSkill] play_card, cost_reduction
  [EffectTiming.SecuritySkill] play_card
BT24_095: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] suspend, gain_keyword_cannot_unsuspend
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
  [EffectTiming.WhenLinked] play_card (1/turn)
  [factory] retaliation
BT24_068: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] mill (inherited) (1/turn)
BT24_069: 5 effects
  [EffectTiming.OnMove] trash_from_hand, mill
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, mill
  [factory] blocker
  [factory] dp_modifier
  [EffectTiming.OnAllyAttack] mill (inherited) (1/turn)
BT24_070: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT24_071: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card
BT24_072: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, gain_keyword_blocker, gain_keyword_retaliation
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, gain_keyword_blocker, gain_keyword_retaliation
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] security_attack_plus
BT24_073: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card, mill
  [EffectTiming.OnDestroyedAnyone] play_card, mill
  [EffectTiming.OnAllyAttack] mill (inherited) (1/turn)
BT24_074: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
BT24_075: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [factory] security_attack_plus
BT24_076: 4 effects
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
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
  [EffectTiming.None] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] no-action (1/turn)
BT24_080: 6 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEndTurn] digivolve
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] delete
BT24_081: 7 effects
  [factory] execute
  [factory] rush
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnAllyAttack] delete, trash_from_hand
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
  [EffectTiming.OptionSkill] mill
BT24_097: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] delete
  [EffectTiming.OnAllyAttack] delete (1/turn)
BT24_098: 4 effects
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT24_099: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [factory] delay
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
BT24_011: 4 effects
  [factory] alt_digivolve_req
  [factory] rush
  [factory] raid
  [factory] raid
BT24_012: 3 effects
  [factory] blocker
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT24_013: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnAllyAttack] delete, trash_from_hand
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_014: 5 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
  [factory] decode
BT24_015: 5 effects
  [factory] blocker
  [factory] alt_digivolve_req
  [EffectTiming.SecuritySkill] play_card
  [EffectTiming.OnAttackTargetChanged] delete (1/turn)
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT24_016: 4 effects
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnAllyAttack] add_to_security, destroy_security
  [EffectTiming.OnEnterFieldAnyone] add_to_security, destroy_security
  [EffectTiming.OnLoseSecurity] play_card (inherited) (1/turn)
BT24_017: 3 effects
  [factory] raid
  [factory] progress
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete, play_token
BT24_018: 7 effects
  [factory] alt_digivolve_req
  [factory] progress
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] destroy_security, unsuspend
  [EffectTiming.OnLoseSecurity] delete (1/turn)
  [EffectTiming.WhenRemoveField] no-action (1/turn)
BT24_082: 3 effects
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend, force_attack
  [factory] security_play
BT24_083: 3 effects
  [EffectTiming.OnStartTurn] play_card
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] security_play
BT24_089: 3 effects
  [EffectTiming.OptionSkill] play_card
  [factory] delay
  [EffectTiming.OnTappedAnyone] digivolve
BT24_100: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
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
  [EffectTiming.OnMove] play_card, add_to_hand, destroy_security
  [EffectTiming.OnEnterFieldAnyone] play_card, add_to_hand, destroy_security
  [EffectTiming.OnEnterFieldAnyone] play_card, add_to_hand, destroy_security
  [factory] barrier
BT24_035: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, play_card
  [EffectTiming.OnEnterFieldAnyone] change_dp, play_card
  [factory] barrier
BT24_036: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnDestroyedAnyone] change_dp
  [EffectTiming.OnDestroyedAnyone] change_dp
BT24_037: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] change_dp, force_attack, change_security_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, force_attack, change_security_attack
  [EffectTiming.WhenRemoveField] play_card (1/turn)
  [EffectTiming.WhenRemoveField] play_card (inherited) (1/turn)
BT24_038: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] change_dp (1/turn)
  [EffectTiming.WhenLinked] change_dp
BT24_039: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.SecuritySkill] play_card
  [factory] blocker
  [factory] barrier
  [EffectTiming.OnDestroyedAnyone] recovery (inherited)
BT24_040: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, gain_keyword_cannot_suspend_player, disable_effect
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, gain_keyword_cannot_suspend_player, disable_effect
  [EffectTiming.WhenRemoveField] put_to_security (descriptive-tagged) (1/turn)
BT24_041: 8 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] play_card, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] play_card, de_digivolve
  [EffectTiming.OnDestroyedAnyone] play_card, de_digivolve
  [factory] blocker
  [factory] reboot
BT24_084: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnLoseSecurity] suspend, digivolve
  [factory] security_play
BT24_092: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT24_093: 4 effects
  [EffectTiming.OptionSkill] recovery, add_to_hand, destroy_security
  [factory] delay
  [EffectTiming.OnLoseSecurity] add_to_security
  [EffectTiming.SecuritySkill] play_card
BT24_101: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, recovery, destroy_security
  [EffectTiming.OnEnterFieldAnyone] change_dp, recovery, destroy_security
  [EffectTiming.OnLoseSecurity] destroy_security (1/turn)
  [EffectTiming.WhenRemoveField] destroy_security (1/turn)
```
