# EX10 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 74
- Scripts with effects: 74
- Total effects: 325
- Factory effects: 89
- Activate effects: 236

## Per-Card Breakdown

```
EX10_002: 1 effects
  [EffectTiming.OnAttackTargetChanged] draw (inherited) (1/turn)
EX10_003: 1 effects
  [EffectTiming.OnAllyAttack] trash_digivolution_cards (inherited) (1/turn)
EX10_024: 3 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnAllyAttack] de_digivolve
EX10_025: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDigivolutionCardDiscarded] delete (inherited)
EX10_026: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [factory] save
  [factory] blocker
EX10_027: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [factory] save
  [factory] retaliation
EX10_028: 3 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_digivolution_cards, gain_keyword_reboot, gain_keyword_blocker
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_digivolution_cards, gain_keyword_reboot, gain_keyword_blocker
  [EffectTiming.OnDigivolutionCardDiscarded] delete (inherited)
EX10_029: 4 effects
  [factory] security_play
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.WhenLinked] no-action
EX10_030: 6 effects
  [factory] alt_digivolve_req
  [factory] collision
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLinkCardDiscarded] change_dp (1/turn)
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
EX10_031: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.WhenPermanentWouldBeDeleted] play_card
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
EX10_032: 5 effects
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_digivolution_cards, gain_keyword_piercing
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_digivolution_cards, gain_keyword_piercing
  [EffectTiming.OnAllyAttack] change_dp, trash_digivolution_cards, gain_keyword_piercing
  [EffectTiming.OnDigivolutionCardDiscarded] de_digivolve (inherited)
EX10_033: 5 effects
  [factory] fragment
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
  [EffectTiming.OnAllyAttack] no-action (1/turn)
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnAllyAttack] trash_digivolution_cards
EX10_034: 9 effects
  [EffectTiming.None] no-action
  [factory] collision
  [factory] fragment
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.OnAllyAttack] change_dp, trash_digivolution_cards (1/turn)
EX10_035: 6 effects
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnDeclaration] delete
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnAllyAttack] de_digivolve
  [EffectTiming.OnDestroyedAnyone] add_to_security
  [EffectTiming.SecuritySkill] play_card
EX10_036: 6 effects
  [factory] alt_digivolve_req
  [factory] fragment
  [EffectTiming.OnEnterFieldAnyone] unsuspend (1/turn)
  [EffectTiming.OnAllyAttack] unsuspend (1/turn)
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards, destroy_security
  [EffectTiming.OnAllyAttack] delete, trash_digivolution_cards, destroy_security
EX10_062: 4 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnLinkCardDiscarded] draw, suspend
  [EffectTiming.OnEndTurn] play_card (1/turn)
  [factory] security_play
EX10_063: 3 effects
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory, suspend
  [factory] security_play
EX10_069: 3 effects
  [EffectTiming.OptionSkill] play_card
  [factory] delay
  [EffectTiming.OnTappedAnyone] digivolve
EX10_070: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] draw
  [factory] delay
  [EffectTiming.OnLinkCardDiscarded] no-action
  [EffectTiming.SecuritySkill] no-action
EX10_073: 6 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndTurn] no-action
  [EffectTiming.OnLinkCardDiscarded] delete (1/turn)
EX10_012: 6 effects
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnDeclaration] delete
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDestroyedAnyone] add_to_security
  [EffectTiming.SecuritySkill] play_card
EX10_001: 1 effects
  [EffectTiming.OnLinkCardDiscarded] gain_memory (inherited) (1/turn)
EX10_015: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [factory] save
  [EffectTiming.OnStartMainPhase] draw, trash_from_hand, suspend
EX10_016: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenLinked] suspend (1/turn)
  [EffectTiming.OnAllyAttack] suspend
EX10_017: 5 effects
  [factory] alt_digivolve_req
  [factory] jamming
  [factory] retaliation
  [EffectTiming.WhenLinked] play_card (1/turn)
  [EffectTiming.OnTappedAnyone] draw, gain_memory
EX10_018: 4 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
EX10_019: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] suspend, gain_keyword_cannot_unsuspend_player (1/turn)
  [EffectTiming.OnTappedAnyone] destroy_security
EX10_020: 6 effects
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnDeclaration] delete
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAllyAttack] bounce
  [EffectTiming.OnDestroyedAnyone] add_to_security
  [EffectTiming.SecuritySkill] play_card
EX10_021: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, effect_immunity
  [EffectTiming.OnTappedAnyone] trash_from_hand, suspend (1/turn)
EX10_022: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] change_dp, suspend, gain_keyword_piercing, change_security_attack
  [EffectTiming.OnStartMainPhase] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEndTurn] no-action (inherited)
EX10_023: 6 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
EX10_004: 1 effects
  [EffectTiming.OnMove] draw, gain_memory, trash_from_hand (inherited) (1/turn)
EX10_005: 1 effects
  [EffectTiming.OnDiscardLibrary] draw (inherited) (1/turn)
EX10_037: 3 effects
  [EffectTiming.OnDiscardLibrary] delete
  [EffectTiming.OnStartMainPhase] mill
  [factory] dp_modifier
EX10_038: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] add_to_hand
EX10_039: 3 effects
  [EffectTiming.OnStartMainPhase] no-action
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] draw (inherited)
EX10_040: 2 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
EX10_041: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDiscardLibrary] change_security_attack (descriptive-tagged)
  [EffectTiming.OnDiscardSecurity] change_security_attack (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] mill
  [EffectTiming.OnEnterFieldAnyone] mill
  [factory] barrier
EX10_042: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] mill
  [EffectTiming.OnEnterFieldAnyone] mill
  [EffectTiming.OnAddDigivolutionCards] digivolve (1/turn)
  [factory] raid
EX10_043: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnLinkCardDiscarded] gain_memory (1/turn)
  [EffectTiming.OnAllyAttack] delete
EX10_044: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDigivolutionCardDiscarded] draw (inherited)
EX10_045: 9 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [factory] collision
  [factory] rush
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, gain_keyword_blocker, gain_keyword_retaliation
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, gain_keyword_blocker, gain_keyword_retaliation
  [EffectTiming.OnAllyAttack] trash_digivolution_cards, gain_keyword_blocker, gain_keyword_retaliation
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] draw (inherited)
EX10_046: 3 effects
  [EffectTiming.OnStartMainPhase] add_to_hand, mill
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, mill
  [EffectTiming.OnAllyAttack] mill (inherited) (1/turn)
EX10_047: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] play_card
EX10_048: 5 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, gain_keyword_retaliation
  [EffectTiming.OnDestroyedAnyone] gain_keyword_blocker, gain_keyword_retaliation
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
EX10_049: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete, mill
  [EffectTiming.OnDestroyedAnyone] delete, mill
  [EffectTiming.OnAllyAttack] mill (inherited) (1/turn)
EX10_050: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_reboot, gain_keyword_blocker, mill
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_reboot, gain_keyword_blocker, mill
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] dp_modifier
EX10_051: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, de_digivolve
  [EffectTiming.OnDestroyedAnyone] play_card
EX10_052: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnAllyAttack] trash_from_hand
  [EffectTiming.WhenRemoveField] no-action (1/turn)
EX10_053: 7 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] rush
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged) (1/turn)
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited) (1/turn)
EX10_054: 4 effects
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnDestroyedAnyone] delete
EX10_055: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenRemoveField] trash_digivolution_cards (1/turn)
EX10_056: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, destroy_security (1/turn)
  [EffectTiming.OnAddDigivolutionCards] trash_digivolution_cards, destroy_security (1/turn)
EX10_057: 6 effects
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnDeclaration] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnDestroyedAnyone] add_to_security
  [EffectTiming.SecuritySkill] play_card
EX10_058: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
EX10_059: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.None] grant_skill
EX10_060: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] destroy_security, unsuspend
  [EffectTiming.OnAllyAttack] destroy_security, unsuspend
EX10_064: 5 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnStartMainPhase] draw
  [EffectTiming.BeforePayCost] suspend
  [factory] security_play
EX10_065: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] gain_memory, gain_keyword_rush
  [factory] security_play
EX10_066: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEndTurn] digivolve
  [factory] security_play
EX10_067: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_alliance
  [factory] security_play
EX10_071: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] change_dp, gain_keyword_raid, gain_keyword_piercing, gain_keyword_blocker
  [EffectTiming.OnEndTurn] destroy_security, return_to_deck, force_attack
  [EffectTiming.SecuritySkill] play_card
EX10_074: 7 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
EX10_006: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] add_to_hand
  [factory] dp_modifier
EX10_007: 5 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [factory] dp_modifier
EX10_008: 5 effects
  [factory] alt_digivolve_req
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAttackTargetChanged] destroy_security (inherited) (1/turn)
EX10_009: 4 effects
  [EffectTiming.OnEnterFieldAnyone] mill
  [EffectTiming.OnDestroyedAnyone] mill
  [EffectTiming.OnAllyAttack] play_card
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged)
EX10_010: 7 effects
  [factory] blast_digivolve
  [factory] raid
  [factory] blocker
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.None] effect_immunity
EX10_011: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnDestroyedAnyone] bounce, destroy_security (1/turn)
EX10_061: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction, destroy_security
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] delete
EX10_068: 4 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnStartMainPhase] no-action
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, return_to_deck
  [factory] security_play
EX10_072: 7 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] draw
  [factory] delay
  [EffectTiming.OnEndTurn] play_card
  [EffectTiming.OnEndTurn] delete
  [EffectTiming.SecuritySkill] play_card
  [EffectTiming.SecuritySkill] delete, add_to_hand
EX10_013: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndTurn] digivolve
  [factory] blocker
EX10_014: 5 effects
  [factory] security_play
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
  [EffectTiming.OnAllyAttack] change_dp
```
