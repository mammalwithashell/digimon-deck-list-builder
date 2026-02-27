# EX8 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 75
- Scripts with effects: 75
- Total effects: 254
- Factory effects: 93
- Activate effects: 161

## Per-Card Breakdown

```
EX8_005: 1 effects
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
EX8_046: 2 effects
  [EffectTiming.OnDestroyedAnyone] draw, trash_from_hand
  [factory] blocker
EX8_047: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDigivolutionCardDiscarded] delete (inherited)
EX8_048: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDigivolutionCardDiscarded] delete (inherited)
EX8_049: 3 effects
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnDestroyedAnyone] de_digivolve
  [factory] blocker
EX8_050: 3 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
EX8_051: 3 effects
  [factory] collision
  [factory] fragment
  [EffectTiming.OnDigivolutionCardDiscarded] de_digivolve (inherited)
EX8_052: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnAllyAttack] destroy_security (inherited) (1/turn)
EX8_053: 2 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] play_card
EX8_054: 5 effects
  [factory] alt_digivolve_req
  [factory] rush
  [factory] security_attack_plus
  [EffectTiming.OnAllyAttack] no-action (1/turn)
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged) (1/turn)
EX8_055: 4 effects
  [factory] fragment
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, unsuspend
  [EffectTiming.OnAllyAttack] trash_digivolution_cards, unsuspend
  [EffectTiming.OnEndTurn] no-action (1/turn)
EX8_067: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] security_play
EX8_070: 2 effects
  [EffectTiming.OptionSkill] trash_digivolution_cards, gain_keyword_piercing, gain_keyword_reboot, gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck, grant_skill, grant_bounce_immunity, effect_immunity
  [EffectTiming.SecuritySkill] delete
EX8_002: 1 effects
  [EffectTiming.OnAllyAttack] gain_memory (inherited) (1/turn)
EX8_017: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker
  [factory] jamming
EX8_018: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
EX8_019: 3 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited)
EX8_020: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
EX8_021: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] gain_memory (1/turn)
  [factory] jamming
EX8_022: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_digivolution_cards
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited)
EX8_023: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, disable_effect, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, disable_effect, effect_immunity
  [factory] security_attack_plus
EX8_024: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnAllyAttack] effect_immunity (1/turn)
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
EX8_025: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndAttack] play_card (1/turn)
  [EffectTiming.None] target_lock (inherited)
EX8_026: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, bounce
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, bounce
  [EffectTiming.None] effect_immunity
EX8_027: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card, force_attack (1/turn)
EX8_028: 5 effects
  [factory] alt_digivolve_req
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] unsuspend, put_to_security, effect_immunity (1/turn)
  [EffectTiming.OnAllyAttack] unsuspend, put_to_security, effect_immunity (1/turn)
EX8_029: 5 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] play_card, bounce
  [EffectTiming.None] effect_immunity
  [EffectTiming.None] disable_effect, effect_immunity
EX8_066: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] suspend, trash_digivolution_cards
  [factory] security_play
EX8_068: 2 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.SecuritySkill] play_card
EX8_004: 1 effects
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged) (inherited) (1/turn)
EX8_038: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] retaliation
EX8_039: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
EX8_040: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] dp_modifier
EX8_041: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [factory] retaliation
EX8_042: 3 effects
  [factory] alt_digivolve_req
  [factory] dp_modifier
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
EX8_043: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, suspend, gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck, grant_bounce_immunity
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, suspend, gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck, grant_bounce_immunity
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
EX8_044: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend, effect_immunity
  [EffectTiming.OnTappedAnyone] change_dp, gain_keyword_piercing (1/turn)
EX8_045: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] suspend, bounce
  [factory] security_attack_plus
  [factory] dp_modifier
EX8_069: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [factory] alliance
  [EffectTiming.None] grant_skill
  [EffectTiming.SecuritySkill] play_card
EX8_074: 6 effects
  [EffectTiming.BeforePayCost] suspend, cost_reduction, effect_immunity
  [EffectTiming.None] cost_reduction, effect_immunity
  [factory] alliance
  [factory] vortex
  [EffectTiming.OnEnterFieldAnyone] delete, suspend
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
EX8_006: 1 effects
  [EffectTiming.OnAllyAttack] delete, trash_from_hand (inherited) (1/turn)
EX8_056: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] draw, trash_from_hand (1/turn)
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
EX8_057: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] draw, trash_from_hand (inherited) (1/turn)
EX8_058: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] gain_memory
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
EX8_059: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_temp_effect, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_temp_effect, effect_immunity
  [EffectTiming.OnAllyAttack] draw, trash_from_hand (inherited) (1/turn)
EX8_060: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card, force_attack (1/turn)
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
EX8_061: 4 effects
  [factory] alt_digivolve_req
  [factory] scapegoat
  [EffectTiming.OnAllyAttack] play_card (1/turn)
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
EX8_062: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnDestroyedAnyone] play_card (1/turn)
EX8_063: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand (1/turn)
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand (1/turn)
  [EffectTiming.OnDiscardHand] destroy_security (1/turn)
EX8_064: 4 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] play_card, de_digivolve
  [EffectTiming.OnDestroyedAnyone] destroy_security (1/turn)
EX8_071: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [factory] scapegoat
  [EffectTiming.None] grant_skill
  [EffectTiming.SecuritySkill] play_card
EX8_072: 3 effects
  [EffectTiming.OnEnterFieldAnyone] return_to_deck
  [EffectTiming.OptionSkill] delete, trash_from_hand
  [factory] security_play
EX8_001: 1 effects
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
EX8_007: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
EX8_008: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] gain_memory
  [factory] dp_modifier
EX8_009: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited) (1/turn)
EX8_010: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] delete
  [factory] dp_modifier
EX8_011: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnStartMainPhase] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [factory] dp_modifier
EX8_012: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] play_card, add_temp_effect, effect_immunity
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited) (1/turn)
EX8_013: 2 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
EX8_014: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] security_attack_plus
EX8_015: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete, gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck, grant_bounce_immunity
  [factory] security_attack_plus
EX8_016: 4 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] delete, suspend
  [EffectTiming.OnEnterFieldAnyone] delete, suspend
EX8_065: 3 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAllyAttack] suspend, digivolve
EX8_073: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnAllyAttack] change_dp
  [EffectTiming.OnEnterFieldAnyone] destroy_security, unsuspend (1/turn)
  [EffectTiming.OnEndAttack] destroy_security, unsuspend (1/turn)
  [EffectTiming.None] effect_immunity
EX8_003: 1 effects
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX8_030: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
EX8_031: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnUseOption] change_dp (inherited) (1/turn)
EX8_032: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX8_033: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnDestroyedAnyone] change_dp
  [EffectTiming.OnDestroyedAnyone] recovery (inherited)
EX8_034: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] change_security_attack (descriptive-tagged)
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
EX8_035: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.SecuritySkill] add_to_hand, change_security_attack
  [EffectTiming.None] disable_effect, effect_immunity
EX8_036: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] recovery
EX8_037: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] unsuspend (1/turn)
EX8_037_token: 1 effects
  [factory] rush
```


## Cross-Validation Results

Checked 74 cards against digimoncard.io effect text.

### Forward Mismatches (API mentions X, script missing)

```
EX8-014: API has 'fortitude' but script missing implementation
EX8-016: API has 'fortitude' but script missing implementation
EX8-023: API has 'piercing' but script missing implementation
EX8-031: API has 'bounce' but script missing implementation
EX8-033: API has 'bounce' but script missing implementation
EX8-037: API has 'play' but script missing implementation
EX8-037: API has 'token_play' but script missing implementation
EX8-042: API has 'fortitude' but script missing implementation
EX8-045: API has 'piercing' but script missing implementation
EX8-050: API has 'reveal_top' but script missing implementation
EX8-051: API has 'piercing' but script missing implementation
EX8-053: API has 'dp_modification' but script missing implementation
EX8-053: API has 'reveal_top' but script missing implementation
EX8-054: API has 'piercing' but script missing implementation
EX8-064: API has 'dp_modification' but script missing implementation
EX8-067: API has 'digivolve_into' but script missing implementation
EX8-070: API has 'collision' but script missing implementation
EX8-070: API has 'dp_modification' but script missing implementation
EX8-072: API has 'security_trash' but script missing implementation
EX8-073: API has 'delete_opponent' but script missing implementation
```

### Reverse Mismatches (Script claims X, API doesn't mention)

```
EX8-051: script has '_is_fragment' but API text doesn't mention it
EX8-055: script has '_is_fragment' but API text doesn't mention it
```

### Timing Mismatches

```
EX8-023: has inherited effect text but no is_inherited_effect flag
EX8-026: has inherited effect text but no is_inherited_effect flag
EX8-044: has inherited effect text but no is_inherited_effect flag
EX8-062: has inherited effect text but no is_inherited_effect flag
```

### Structural Warnings

```
EX8-023: API has inherited effect but script has no is_inherited_effect
EX8-026: API has inherited effect but script has no is_inherited_effect
EX8-044: API has inherited effect but script has no is_inherited_effect
EX8-062: API has inherited effect but script has no is_inherited_effect
```

