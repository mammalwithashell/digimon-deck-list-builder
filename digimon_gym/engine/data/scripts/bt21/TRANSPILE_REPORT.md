# BT21 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 103
- Scripts with effects: 103
- Total effects: 399
- Factory effects: 154
- Activate effects: 245

## Per-Card Breakdown

```
BT21_006: 1 effects
  [factory] dp_modifier
BT21_053: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack
  [EffectTiming.WhenLinked] gain_keyword_cannot_attack
BT21_054: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, de_digivolve
  [EffectTiming.WhenLinked] delete
BT21_055: 2 effects
  [factory] change_digi_cost
  [EffectTiming.OnDigivolutionCardDiscarded] delete (inherited)
BT21_056: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.None] cost_reduction (inherited)
BT21_057: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [factory] reboot
BT21_058: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDigivolutionCardReturnToDeckBottom] delete (inherited) (1/turn)
BT21_059: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] app_fusion_condition (descriptive-tagged)
  [factory] blocker
  [EffectTiming.WhenLinked] de_digivolve (1/turn)
  [EffectTiming.WhenLinked] de_digivolve
BT21_060: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.WhenRemoveField] play_card
  [EffectTiming.OnUseAttack] no-action (inherited) (1/turn)
BT21_061: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_alliance, force_attack (1/turn)
  [factory] alliance
BT21_062: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnStartMainPhase] delete
  [EffectTiming.WhenRemoveField] no-action
BT21_087: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] play_card, add_to_hand, reveal_and_select
  [factory] security_play
BT21_098: 4 effects
  [EffectTiming.OptionSkill] delete
  [factory] delay
  [EffectTiming.OnUseAttack] no-action
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT21_003: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
BT21_031: 2 effects
  [factory] change_digi_cost
  [EffectTiming.OnEndAttack] gain_memory (inherited) (1/turn)
BT21_032: 3 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [factory] dp_modifier
BT21_033: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] jamming
BT21_034: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnTappedAnyone] draw
  [factory] jamming
BT21_035: 4 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnAttackTargetChanged] unsuspend (1/turn)
BT21_036: 4 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] unsuspend
BT21_037: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend
BT21_038: 5 effects
  [factory] alt_digivolve_req
  [factory] evade
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.None] target_lock (inherited)
BT21_039: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnUseAttack] digivolve (1/turn)
BT21_085: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] draw, gain_memory, suspend, de_digivolve
  [factory] security_play
BT21_094: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.WhenTopCardTrashed] digivolve
BT21_095: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [factory] vortex
  [EffectTiming.None] grant_skill
  [EffectTiming.SecuritySkill] play_card
BT21_005: 1 effects
  [EffectTiming.WhenLinked] draw (inherited) (1/turn)
BT21_046: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] digivolve
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEndTurn] play_card (inherited)
BT21_047: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT21_048: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
BT21_049: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend (1/turn)
BT21_050: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnUseAttack] redirect_attack (1/turn)
  [EffectTiming.OnEnterFieldAnyone] suspend (inherited) (1/turn)
BT21_051: 6 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [factory] blocker
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, bounce
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, bounce
BT21_052: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] evade
  [EffectTiming.OnEnterFieldAnyone] delete, suspend
  [EffectTiming.OnTappedAnyone] destroy_security, unsuspend (1/turn)
BT21_097: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnEndTurn] no-action
BT21_063: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [factory] save
  [factory] dp_modifier
BT21_064: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
BT21_065: 2 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
BT21_066: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] no-action
  [factory] dp_modifier
  [EffectTiming.None] no-action
BT21_067: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnUseAttack] draw, trash_from_hand (inherited) (1/turn)
BT21_068: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] mill
  [EffectTiming.OnEnterFieldAnyone] mill
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
BT21_069: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] retaliation
BT21_070: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.WhenLinked] add_to_hand
BT21_071: 5 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_memory
  [EffectTiming.OnEnterFieldAnyone] gain_memory
  [EffectTiming.WhenLinked] draw, trash_from_hand
BT21_072: 5 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [factory] dp_modifier
  [factory] dp_modifier
BT21_073: 7 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] no-action (1/turn)
  [EffectTiming.WhenLinked] force_attack (descriptive-tagged)
  [EffectTiming.WhenRemoveField] trash_from_hand (1/turn)
BT21_074: 7 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, de_digivolve (1/turn)
  [EffectTiming.OnUseAttack] trash_digivolution_cards, de_digivolve (1/turn)
  [EffectTiming.WhenLinked] delete
BT21_075: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_raid, gain_keyword_retaliation
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_raid, gain_keyword_retaliation
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT21_076: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_raid, gain_keyword_retaliation, mill
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_raid, gain_keyword_retaliation, mill
  [EffectTiming.OnUseAttack] digivolve (1/turn)
  [EffectTiming.OnDestroyedAnyone] destroy_security (inherited)
BT21_077: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, grant_skill
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, grant_skill
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT21_078: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_alliance, force_attack (1/turn)
  [factory] alliance
BT21_079: 5 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEndAttack] delete (1/turn)
  [EffectTiming.OnDestroyedAnyone] play_card
BT21_088: 3 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory
  [EffectTiming.BeforePayCost] suspend, cost_reduction
  [factory] security_play
BT21_089: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_blocker
  [factory] security_play
BT21_099: 2 effects
  [EffectTiming.OptionSkill] digivolve
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT21_100: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [factory] delay
  [EffectTiming.OnDestroyedAnyone] digivolve
  [EffectTiming.SecuritySkill] gain_memory
BT21_001: 1 effects
  [EffectTiming.OnLoseSecurity] digivolve (inherited) (1/turn)
BT21_002: 1 effects
  [EffectTiming.OnUseAttack] draw (inherited) (1/turn)
BT21_007: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [factory] dp_modifier
BT21_008: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT21_009: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenLinked] play_card (1/turn)
  [factory] raid
BT21_010: 3 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] dp_modifier
BT21_011: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [factory] save
  [factory] rush
BT21_012: 2 effects
  [EffectTiming.OnDeclaration] suspend, play_card
  [factory] dp_modifier
BT21_013: 5 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnUseAttack] digivolve
  [factory] dp_modifier
BT21_014: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_piercing
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_piercing
  [EffectTiming.OnLoseSecurity] digivolve
  [factory] dp_modifier
BT21_015: 4 effects
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] dp_modifier
BT21_016: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] raid
  [EffectTiming.None] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [factory] dp_modifier
BT21_017: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT21_018: 5 effects
  [factory] alt_digivolve_req
  [factory] raid
  [factory] rush
  [EffectTiming.WhenLinked] force_attack (descriptive-tagged) (1/turn)
  [EffectTiming.WhenLinked] force_attack (descriptive-tagged)
BT21_019: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] dp_modifier
BT21_020: 4 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [factory] security_attack_plus
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT21_021: 7 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.None] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnEndAttack] delete, play_card, cost_reduction
  [factory] rush
BT21_022: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenRemoveField] trash_digivolution_cards (inherited) (1/turn)
BT21_023: 6 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] delete
  [EffectTiming.WhenLinked] delete
BT21_024: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_security, destroy_security
  [EffectTiming.OnEnterFieldAnyone] add_to_security, destroy_security
  [factory] dp_modifier
BT21_025: 3 effects
  [factory] progress
  [EffectTiming.OnAttackTargetChanged] destroy_security (1/turn)
  [EffectTiming.OnLoseSecurity] play_card (inherited) (1/turn)
BT21_026: 5 effects
  [EffectTiming.None] cost_reduction
  [factory] blocker
  [factory] rush
  [factory] raid
  [EffectTiming.OnDestroyedAnyone] unsuspend (1/turn)
BT21_027: 8 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.None] no-action
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenRemoveField] no-action
BT21_028: 5 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnUseAttack] delete
BT21_029: 6 effects
  [factory] progress
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndAttack] no-action
  [EffectTiming.OnDestroyedAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnLoseSecurity] play_token (descriptive-tagged)
BT21_029_token: 1 effects
  [EffectTiming.OnDestroyedAnyone] destroy_security
BT21_030: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnUseAttack] no-action (1/turn)
BT21_080: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAddDigivolutionCards] draw, gain_memory, suspend
  [factory] security_play
BT21_081: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEndTurn] suspend, gain_keyword_piercing, force_attack
  [factory] security_play
BT21_082: 3 effects
  [EffectTiming.OnStartMainPhase] digivolve
  [factory] security_play
  [EffectTiming.OnLoseSecurity] play_card (inherited) (1/turn)
BT21_083: 3 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory
  [EffectTiming.OnEnterFieldAnyone] suspend, force_attack
  [factory] security_play
BT21_084: 3 effects
  [factory] set_memory_3
  [EffectTiming.WhenLinked] draw, suspend, play_card
  [factory] security_play
BT21_090: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnAddDigivolutionCards] digivolve
  [EffectTiming.SecuritySkill] play_card
BT21_091: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT21_092: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] play_card
  [EffectTiming.SecuritySkill] play_card
BT21_093: 6 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] delete
  [factory] delay
  [EffectTiming.OnLoseSecurity] digivolve
  [EffectTiming.SecuritySkill] delete
BT21_101: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] destroy_security, unsuspend (1/turn)
  [EffectTiming.None] destroy_security, unsuspend (1/turn)
  [EffectTiming.WhenLinked] destroy_security, unsuspend (1/turn)
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnUseAttack] no-action
BT21_102: 3 effects
  [EffectTiming.OnStartTurn] draw, suspend
  [EffectTiming.OnUseAttack] draw, suspend
  [EffectTiming.OnDeclaration] play_card (1/turn)
BT21_004: 1 effects
  [EffectTiming.OnTappedAnyone] draw (inherited) (1/turn)
BT21_040: 3 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] dp_modifier
BT21_041: 2 effects
  [factory] alt_digivolve_req
  [factory] security_play
BT21_042: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] digivolve (1/turn)
  [factory] dp_modifier
BT21_043: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.WhenLinked] change_dp
BT21_044: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_rush, gain_keyword_alliance, force_attack
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_rush, gain_keyword_alliance, force_attack
  [EffectTiming.OnDestroyedAnyone] add_to_security (1/turn)
  [EffectTiming.OnDestroyedAnyone] add_to_security (inherited) (1/turn)
BT21_045: 5 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] delete (1/turn)
  [EffectTiming.OnUseAttack] delete (1/turn)
  [EffectTiming.OnUseAttack] change_dp, suspend (1/turn)
BT21_086: 4 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnTappedAnyone] change_dp, gain_keyword_piercing (1/turn)
BT21_096: 2 effects
  [EffectTiming.OptionSkill] gain_keyword_rush, force_attack, attack_unsuspended
  [EffectTiming.SecuritySkill] play_card, add_to_hand
```
