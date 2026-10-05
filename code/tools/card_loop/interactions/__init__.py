"""Interaction exams: the gating denominator, risk probes, Q&A packets, outcomes.

Design: openspec/changes/add-card-authoring-loop/design.md D6 (deterministic
denominator), D7 (three-way comparison), D8 (scenario / verdict schema).

    probes.py       families@1 rule-based probe generator over extracted clauses
    denominator.py  card_qa.json + promoted probes -> data/interaction_denominator.json
                    (`python -m tools.card_loop interactions build|--check|promotion-report`)
    packets.py      classify_qa / encode_ruling task-packet inputs (terminating calls)
    outcome.py      the D7 three-way outcome table + DCGO fork-candidate export

Interaction ids (stable, sorted naturally, never invented elsewhere):

    qa:<Q-number>                        one official ruling, shared by every card it lists
    probe:<clause-id>:<family>[:neg]     one generated risk probe on one clause
    combo:<slug>                         model-authored combo -- examined, NEVER gating
"""

QA_PREFIX = "qa:"
PROBE_PREFIX = "probe:"
COMBO_PREFIX = "combo:"
