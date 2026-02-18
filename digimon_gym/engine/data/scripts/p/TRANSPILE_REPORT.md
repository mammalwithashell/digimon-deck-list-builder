# P Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 227
- Scripts with effects: 227
- Total effects: 639
- Factory effects: 188
- Activate effects: 451

## Per-Card Breakdown

```
P_015: 1 effects
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
P_016: 1 effects
  [factory] security_attack_plus
P_026: 1 effects
  [EffectTiming.OnDeclaration] unsuspend
P_033: 2 effects
  [EffectTiming.None] grant_skill
  [factory] security_attack_plus
P_039: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
P_045: 2 effects
  [factory] decoy
  [EffectTiming.None] grant_skill (inherited)
P_070: 2 effects
  [EffectTiming.SecuritySkill] no-action
  [EffectTiming.SecuritySkill] play_card, add_to_hand
P_076: 2 effects
  [factory] change_digi_cost
  [EffectTiming.OnAllyAttack] delete (inherited)
P_078: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw
P_094: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
P_101: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnAllyAttack] delete, trash_from_hand (inherited)
P_107: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] digivolve
P_114: 4 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnAllyAttack] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] delete (1/turn)
P_121: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] play_card (inherited)
P_128: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] play_card, digivolve
  [factory] security_play
P_141: 6 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] collision
  [EffectTiming.OnTappedAnyone] unsuspend (1/turn)
  [EffectTiming.OnTappedAnyone] unsuspend (inherited) (1/turn)
P_144: 4 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnAttackTargetChanged] unsuspend (1/turn)
  [factory] dp_modifier_all
P_154: 2 effects
  [EffectTiming.WhenRemoveField] no-action
  [factory] blocker
P_157: 1 effects
  [EffectTiming.OnDestroyedAnyone] draw (inherited)
P_159: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] change_dp, gain_keyword_reboot, gain_keyword_blocker
  [EffectTiming.OptionSkill] change_dp, gain_keyword_reboot, gain_keyword_blocker
  [EffectTiming.SecuritySkill] add_to_hand, de_digivolve
P_162: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_immune_dp_minus
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_immune_dp_minus
  [factory] blocker
P_167: 3 effects
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, add_to_hand, reveal_and_select
  [EffectTiming.OnStartMainPhase] trash_digivolution_cards, add_to_hand, reveal_and_select
  [EffectTiming.OnDigivolutionCardDiscarded] de_digivolve (inherited)
P_169: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDigivolutionCardDiscarded] suspend
  [factory] security_play
P_174: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnDestroyedAnyone] delete, de_digivolve
P_176: 1 effects
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
P_179: 5 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
P_183: 6 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] blocker
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.OnAttackTargetChanged] destroy_security (1/turn)
P_184: 4 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [factory] collision
  [EffectTiming.OnEnterFieldAnyone] change_dp, unsuspend
P_203: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, gain_keyword_piercing, change_security_attack
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, gain_keyword_piercing, change_security_attack
  [EffectTiming.OnAllyAttack] de_digivolve, gain_keyword_piercing, change_security_attack
  [EffectTiming.OnDestroyedAnyone] gain_keyword_cannot_attack (1/turn)
P_204: 3 effects
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [factory] delay
  [EffectTiming.OnAllyAttack] digivolve
P_211: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack
  [factory] security_play
P_216: 6 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] delete
  [factory] blocker
P_224: 4 effects
  [EffectTiming.OnStartMainPhase] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDeclaration] suspend, play_card, cost_reduction
  [factory] security_play
P_231: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] digivolve
P_003: 1 effects
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
P_007: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited)
P_008: 2 effects
  [EffectTiming.OnAllyAttack] unsuspend (1/turn)
  [factory] security_attack_plus
P_011: 2 effects
  [EffectTiming.OnAllyAttack] change_dp, mill
  [EffectTiming.OnAllyAttack] draw, return_to_deck (inherited)
P_012: 2 effects
  [EffectTiming.OnDeclaration] draw, change_dp, suspend
  [factory] security_play
P_022: 2 effects
  [EffectTiming.OptionSkill] play_card, return_to_deck
  [EffectTiming.SecuritySkill] add_to_hand
P_030: 3 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] change_digi_cost
P_036: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
P_042: 1 effects
  [EffectTiming.OnEnterFieldAnyone] reveal_and_select
P_047: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp, mill
  [EffectTiming.OnAllyAttack] change_dp, return_to_deck (inherited)
P_048: 2 effects
  [EffectTiming.OnEnterFieldAnyone] unsuspend, return_to_deck
  [EffectTiming.OnReturnCardsToLibraryFromTrash] gain_memory (1/turn)
P_051: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw
P_052: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack
  [EffectTiming.OnAllyAttack] bounce (1/turn)
P_061: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
P_064: 2 effects
  [EffectTiming.OnAllyAttack] suspend, gain_keyword_jamming
  [factory] security_play
P_067: 2 effects
  [EffectTiming.SecuritySkill] no-action
  [EffectTiming.SecuritySkill] draw, add_to_hand
P_073: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.WhenPermanentWouldBeDeleted] trash_digivolution_cards (inherited)
P_086: 1 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
P_089: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, trash_digivolution_cards
  [EffectTiming.OnAllyAttack] return_to_deck (1/turn)
P_092: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] digivolve (inherited)
P_098: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_rush (inherited) (1/turn)
P_104: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] digivolve
P_109: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] suspend, unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, unsuspend
  [EffectTiming.OnTappedAnyone] play_card (1/turn)
P_117: 2 effects
  [EffectTiming.BeforePayCost] cost_reduction (1/turn)
  [EffectTiming.OnAllyAttack] draw (inherited)
P_124: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] play_card, digivolve
  [factory] security_play
P_138: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnUnTappedAnyone] gain_memory (inherited) (1/turn)
P_148: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
P_161: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.SecuritySkill] add_to_hand, bounce
P_164: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnEndAttack] draw (inherited) (1/turn)
P_168: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAddDigivolutionCards] suspend, digivolve
  [factory] security_play
P_171: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
P_188: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
P_190: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.WhenLinked] draw
P_196: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] digivolve
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
P_214: 5 effects
  [factory] alt_digivolve_req
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] trash_digivolution_cards (inherited)
P_215: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnMove] gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck
  [factory] blocker
P_228: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] digivolve
P_021: 2 effects
  [EffectTiming.OptionSkill] bounce, play_card
  [EffectTiming.SecuritySkill] add_to_hand
P_025: 1 effects
  [EffectTiming.OnDeclaration] change_security_attack (descriptive-tagged)
P_032: 1 effects
  [EffectTiming.OnDigivolutionCardDiscarded] gain_keyword_jamming (inherited)
P_038: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
P_044: 1 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
P_055: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEndBattle] gain_memory
P_056: 2 effects
  [EffectTiming.BeforePayCost] cost_reduction, suspend
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, gain_keyword_cannot_block
P_057: 2 effects
  [factory] dp_modifier
  [factory] dp_modifier
P_060: 1 effects
  [EffectTiming.OnAllyAttack] gain_memory (inherited) (1/turn)
P_063: 2 effects
  [EffectTiming.OnAllyAttack] change_dp, suspend
  [factory] security_play
P_069: 2 effects
  [EffectTiming.SecuritySkill] no-action
  [EffectTiming.SecuritySkill] add_to_hand, suspend
P_075: 2 effects
  [EffectTiming.BeforePayCost] grant_skill
  [EffectTiming.BeforePayCost] no-action
P_082: 1 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
P_083: 1 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_unsuspend
P_090: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEndBattle] unsuspend (1/turn)
P_093: 2 effects
  [EffectTiming.OnTappedAnyone] suspend
  [EffectTiming.None] cost_reduction (inherited)
P_100: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_unsuspend
  [factory] dp_modifier
P_106: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] digivolve
P_112: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] digivolve (inherited) (1/turn)
P_113: 4 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEndBattle] destroy_security (1/turn)
P_118: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] play_card (inherited)
P_125: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] play_card, digivolve
  [factory] security_play
P_131: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] dp_modifier
P_132: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend
  [factory] dp_modifier
P_133: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend (1/turn)
  [factory] security_play
P_140: 4 effects
  [factory] alt_digivolve_req
  [factory] evade
  [EffectTiming.None] effect_immunity
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
P_143: 1 effects
  [EffectTiming.OnEndTurn] no-action (1/turn)
P_150: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnTappedAnyone] suspend (inherited) (1/turn)
P_163: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnDestroyedAnyone] suspend
P_166: 3 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve, suspend
  [EffectTiming.OnEnterFieldAnyone] digivolve, suspend
  [factory] dp_modifier
P_173: 5 effects
  [factory] alt_digivolve_req
  [factory] collision
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnDestroyedAnyone] unsuspend (1/turn)
P_181: 3 effects
  [EffectTiming.BeforePayCost] cost_reduction (1/turn)
  [EffectTiming.OptionSkill] add_to_hand, add_to_security, destroy_security
  [EffectTiming.SecuritySkill] play_card
P_185: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] dp_modifier
  [EffectTiming.OnEndTurn] unsuspend (1/turn)
P_200: 3 effects
  [EffectTiming.OnStartMainPhase] suspend
  [EffectTiming.BeforePayCost] suspend, cost_reduction
  [factory] security_play
P_201: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] trash_from_hand, suspend (inherited) (1/turn)
P_202: 3 effects
  [factory] alt_digivolve_req
  [factory] training
  [EffectTiming.BeforePayCost] cost_reduction (1/turn)
P_208: 5 effects
  [factory] alt_digivolve_req
  [factory] execute
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] bounce (1/turn)
P_222: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnTappedAnyone] delete (1/turn)
P_230: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] digivolve
P_017: 1 effects
  [EffectTiming.OnEnterFieldAnyone] mill
P_020: 1 effects
  [EffectTiming.OnDestroyedAnyone] play_card
P_027: 1 effects
  [EffectTiming.OnDeclaration] no-action
P_034: 1 effects
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
P_040: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
P_046: 1 effects
  [EffectTiming.OnUseOption] gain_memory (inherited) (1/turn)
P_071: 2 effects
  [EffectTiming.SecuritySkill] no-action
  [EffectTiming.SecuritySkill] play_card, add_to_hand
P_077: 2 effects
  [EffectTiming.OnDiscardLibrary] gain_memory
  [EffectTiming.OnAllyAttack] no-action (inherited)
P_080: 1 effects
  [EffectTiming.OnEnterFieldAnyone] delete
P_085: 1 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
P_096: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.SecuritySkill] add_to_hand
P_102: 4 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
P_108: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] digivolve
P_115: 2 effects
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] security_attack_plus
P_142: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, force_attack
  [EffectTiming.OnDestroyedAnyone] trash_from_hand (inherited)
P_145: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] play_card
P_149: 1 effects
  [EffectTiming.OnAllyAttack] delete, trash_from_hand (inherited) (1/turn)
P_177: 1 effects
  [EffectTiming.OnDestroyedAnyone] add_to_hand (inherited)
P_187: 4 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] recovery, destroy_security, put_to_security
  [EffectTiming.OnEnterFieldAnyone] play_card, destroy_security (1/turn)
  [EffectTiming.OnAllyAttack] play_card, destroy_security (1/turn)
P_192: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [factory] retaliation
P_193: 3 effects
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [factory] delay
  [EffectTiming.OnEndTurn] play_card
P_198: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] digivolve
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
P_205: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] no-action
  [factory] delay
  [EffectTiming.OnDeclaration] play_card
  [EffectTiming.SecuritySkill] no-action
P_209: 5 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnDiscardHand] play_card (1/turn)
P_212: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] security_play
P_219: 2 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.OptionSkill] delete, play_card, gain_keyword_rush, gain_keyword_blocker
P_223: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnUseOption] no-action (1/turn)
P_232: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] digivolve
P_001: 1 effects
  [EffectTiming.OnEnterFieldAnyone] delete
P_002: 1 effects
  [EffectTiming.OnEndBattle] draw (inherited)
P_009: 1 effects
  [factory] dp_modifier
P_010: 1 effects
  [factory] security_attack_plus
P_024: 2 effects
  [EffectTiming.OptionSkill] draw
  [EffectTiming.SecuritySkill] add_to_hand
P_029: 3 effects
  [EffectTiming.OnAllyAttack] digivolve
  [EffectTiming.OnAllyAttack] delete
  [factory] change_digi_cost
P_035: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
P_041: 1 effects
  [EffectTiming.OnAllyAttack] draw
P_049: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnBlockAnyone] destroy_security (1/turn)
P_050: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
P_058: 1 effects
  [EffectTiming.None] attack_unsuspended
P_059: 1 effects
  [factory] dp_modifier
P_062: 2 effects
  [EffectTiming.OnAllyAttack] suspend, change_security_attack
  [factory] security_play
P_065: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete (inherited)
P_066: 2 effects
  [EffectTiming.SecuritySkill] no-action
  [EffectTiming.SecuritySkill] draw, add_to_hand
P_072: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] delete
P_079: 1 effects
  [EffectTiming.OnEnterFieldAnyone] delete
P_088: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnAllyAttack] delete
P_091: 3 effects
  [factory] raid
  [factory] retaliation
  [EffectTiming.OnDestroyedAnyone] add_to_hand (inherited)
P_097: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory, reveal_and_select
  [factory] raid
P_103: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] digivolve
P_110: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
P_119: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] play_card (inherited)
P_126: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] play_card, digivolve
  [factory] security_play
P_137: 4 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [factory] raid
  [EffectTiming.OnAttackTargetChanged] add_to_hand, destroy_security (1/turn)
P_152: 4 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] change_dp, delete
  [EffectTiming.None] no-action
P_155: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] draw
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
  [EffectTiming.SecuritySkill] delete, add_to_hand
P_160: 3 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnAllyAttack] digivolve
P_170: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction, return_to_deck
  [EffectTiming.None] cost_reduction
  [factory] raid
  [factory] retaliation
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] play_card
P_178: 4 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnAllyAttack] delete
P_180: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OnDigivolutionCardDiscarded] delete (inherited)
  [EffectTiming.OptionSkill] destroy_security
  [EffectTiming.SecuritySkill] delete
P_182: 4 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete
P_186: 6 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] rush
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] recovery
  [EffectTiming.OnEnterFieldAnyone] recovery
P_189: 3 effects
  [EffectTiming.SecuritySkill] play_card
  [factory] progress
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
P_199: 3 effects
  [EffectTiming.OnStartMainPhase] change_dp
  [EffectTiming.None] cost_reduction
  [factory] security_play
P_210: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [factory] security_play
P_213: 5 effects
  [factory] alt_digivolve_req
  [factory] raid
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_rush, force_attack
  [factory] decode
P_217: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenLinked] gain_memory, suspend
  [factory] security_play
P_220: 8 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.None] no-action
  [factory] reboot
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnDestroyedAnyone] play_card
P_227: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] digivolve
P_116: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
P_123: 1 effects
  [EffectTiming.OnMove] gain_memory (1/turn)
P_130: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnMove] gain_memory, suspend
  [factory] security_play
P_151: 2 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] play_card, add_to_hand, reveal_and_select
P_156: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] play_card
  [EffectTiming.SecuritySkill] play_card, add_to_hand
P_158: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDeclaration] play_card
  [factory] security_play
P_175: 2 effects
  [EffectTiming.OnStartTurn] suspend, digivolve
  [EffectTiming.OnEnterFieldAnyone] suspend, digivolve
P_206: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.SecuritySkill] play_card, add_to_hand
P_225: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] draw
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
P_005: 1 effects
  [EffectTiming.OnEnterFieldAnyone] recovery
P_006: 1 effects
  [factory] dp_modifier
P_023: 2 effects
  [EffectTiming.OptionSkill] put_to_security (descriptive-tagged)
  [EffectTiming.SecuritySkill] add_to_hand
P_028: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory
P_031: 2 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] recovery
P_037: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
P_043: 2 effects
  [EffectTiming.OnEnterFieldAnyone] recovery, return_to_deck
  [EffectTiming.OnDestroyedAnyone] change_dp (inherited)
P_053: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnAllyAttack] change_dp
P_054: 2 effects
  [EffectTiming.OnEnterFieldAnyone] recovery
  [EffectTiming.OnDestroyedAnyone] recovery
P_068: 2 effects
  [EffectTiming.SecuritySkill] no-action
  [EffectTiming.SecuritySkill] add_to_hand, change_security_attack
P_074: 3 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
P_081: 1 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp
P_084: 1 effects
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
P_087: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, suspend
  [factory] security_play
P_095: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] change_dp, disable_effect
  [EffectTiming.SecuritySkill] change_dp, add_to_hand
P_099: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
P_105: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDeclaration] digivolve
P_111: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
P_120: 1 effects
  [factory] barrier
P_122: 1 effects
  [EffectTiming.OnEnterFieldAnyone] recovery, add_to_hand, destroy_security
P_127: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] play_card, digivolve
  [factory] security_play
P_129: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] play_card, digivolve
  [factory] security_play
P_134: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
P_135: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, change_security_attack
  [factory] jamming
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
P_136: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend (1/turn)
  [factory] security_play
P_139: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] recovery (inherited)
P_146: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.SecuritySkill] change_security_attack (descriptive-tagged)
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.WhenPermanentWouldBeDeleted] add_to_security (inherited)
P_147: 4 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] no-action
  [factory] dp_modifier
P_153: 4 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEndAttack] add_to_security, unsuspend
P_165: 4 effects
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [factory] barrier
P_165_token: 2 effects
  [EffectTiming.OnDestroyedAnyone] change_dp
  [EffectTiming.OnEndTurn] delete
P_172: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
  [EffectTiming.OnDestroyedAnyone] change_dp, delete
P_191: 6 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEndTurn] play_card, force_attack
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged) (inherited) (1/turn)
P_194: 4 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] barrier
  [factory] barrier
P_195: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] play_card, digivolve
  [factory] security_play
P_197: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] digivolve
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
P_207: 5 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] play_card (1/turn)
P_218: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenLinked] gain_memory, suspend
  [factory] security_play
P_221: 6 effects
  [EffectTiming.None] jogress_condition
  [factory] partition
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
P_229: 3 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] digivolve
```
