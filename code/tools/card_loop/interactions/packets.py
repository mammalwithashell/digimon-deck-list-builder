"""Task-packet inputs for the two Q&A judgment stages (design D6, D7, D10).

``classify_qa``
    One official ruling -> ``behavioral | textual | not_examinable`` + reasoning
    + citation, BEFORE any oracle time is spent.
``encode_ruling``
    A behavioral ruling -> the scenario's ``expect_ruling:`` block (mode
    ``author``), or a yes/no on another family's block (mode ``verify``).

**Both are terminating calls** (`contracts.TERMINATING_STAGES`): a
``textual`` / ``not_examinable`` class removes the ruling from the oracle queue,
and the ruling encoding is the EXPECTED VALUE of the three-way comparison. So
the driver must send the SAME packet to BOTH model families, each blind to the
other's answer (`independent_pair`), and accept only when they agree; a
disagreement, a missing citation, or a single available family escalates
(design D5). For ``encode_ruling`` the agreement is author-then-verify: one
family authors, the other verifies the block without the author's reasoning.
`agree_classification` / `agree_encoding` decide it.

This module builds the INPUTS (what the worker sees and must cite) and the
references (files it reads rather than gets inlined); rendering into the
versioned `prompts/<stage>.md` templates is the driver's (group 6). Until those
templates land, `render_prompt` gives a deterministic default.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable, Mapping, Sequence

from tools.card_loop.contracts import FAMILIES, TERMINATING_STAGES, TaskPacket, item_id
from tools.card_loop.interactions import QA_PREFIX

SCHEMAS_DIR = Path(__file__).resolve().parents[1] / "schemas"
CLASSIFY_QA_SCHEMA = SCHEMAS_DIR / "classify_qa.json"
ENCODE_RULING_SCHEMA = SCHEMAS_DIR / "encode_ruling.json"

CLASSIFICATIONS = ("behavioral", "textual", "not_examinable")
#: Classes that END the ruling without an oracle run.
TERMINATING_CLASSES = ("textual", "not_examinable")

#: Projection keys an `expect_ruling` row may assert (Rust
#: `exam::assertions::ASSERTION_KEYS`, with `p{0,1}` expanded). Guarded against
#: drift by test_card_loop_interactions_packets.py.
ASSERTION_KEYS = (
    "turn", "phase", "memory", "p0.memory", "p1.memory",
    "p0.security", "p1.security", "p0.hand", "p1.hand",
    "p0.trash", "p1.trash", "p0.field", "p1.field",
)

#: Read-only references every Q&A packet points at (not inlined).
COMMON_REFERENCES = (
    "docs/digimon-rules/keyword-semantics.md",
    "docs/digimon-rules/digest.md",
)

CLASSIFY_TASK = (
    "Classify official ruling {q_id} for the cards listed. `behavioral`: the ruling says how "
    "play resolves (a trigger, a target, a timing, an interaction) -- it will be examined "
    "against our engine and DCGO. `textual`: it only clarifies PRINTED data (a name, trait, "
    "colour, cost, or wording) and is checked against the card data without an oracle run. "
    "`not_examinable`: it is about tournament procedure, or a game state no legal line of "
    "play can reach. textual and not_examinable END the ruling's examination, so another "
    "model family answers the same packet independently and both must agree; cite "
    "`qa:{q_id}` and, for those two classes, the printed field or rules section that "
    "justifies it."
)

ENCODE_AUTHOR_TASK = (
    "Encode the publisher's answer to ruling {q_id} as an `expect_ruling` block for the "
    "scenario given: the observable state after a given step that the answer implies, "
    "using only the listed projection keys. Encode exactly what the answer says -- no "
    "more (do not assert incidental state the answer does not decide), no less. Quote "
    "the answer words each observable encodes. Another model family will verify your "
    "block blind."
)

ENCODE_VERIFY_TASK = (
    "Another model encoded the publisher's answer to ruling {q_id} as the `expect_ruling` "
    "block given. Decide whether it encodes the answer -- no more, no less -- against the "
    "scenario's line. Set `agrees`, leave `expect_ruling` null, and quote the answer words "
    "the block gets wrong if you disagree."
)


@dataclass(frozen=True)
class PacketInputs:
    """Everything a Q&A worker sees, before rendering into a prompt."""

    stage: str                       # classify_qa | encode_ruling
    item: str                        # `interaction:qa:<Q>`
    schema_path: str
    task: str
    inputs: dict                     # inlined into the prompt as JSON
    references: tuple[str, ...] = ()
    terminating: bool = True
    requires_two_families: bool = True

    def __post_init__(self):
        if self.stage not in TERMINATING_STAGES:
            raise ValueError(f"{self.stage!r} is not a terminating Q&A stage")


def _ruling(card_qa: Mapping, q_id: str) -> dict:
    q_id = q_id[len(QA_PREFIX):] if q_id.startswith(QA_PREFIX) else q_id
    try:
        r = card_qa["qa"][q_id]
    except KeyError:
        raise KeyError(f"ruling {q_id!r} is not in card_qa.json") from None
    return {
        "q_id": q_id,
        "date": r.get("date"),
        "question": r.get("question", ""),
        "answer": r.get("answer", ""),
        "card_ids": list(r.get("card_ids") or []),
    }


def _card_context(card_ids: Sequence[str], clauses: Iterable[Mapping]) -> list[dict]:
    by_card: dict[str, list[dict]] = {c: [] for c in card_ids}
    for c in clauses:
        if c.get("card_id") in by_card:
            by_card[c["card_id"]].append({
                "id": c["id"], "label": c.get("label", ""),
                "timings": list(c.get("timings") or []), "keyword": c.get("keyword"),
                "text": c.get("text", ""),
            })
    return [{"card_id": cid, "bundle": f"data/card_bundles/{cid}.md", "clauses": by_card[cid]}
            for cid in card_ids]


def classify_qa_inputs(q_id: str, card_qa: Mapping, clauses: Iterable[Mapping]) -> PacketInputs:
    """Inputs for classifying one ruling. `clauses` are extracted clause
    records (`tools.clause_coverage`) for at least the ruling's cards."""
    ruling = _ruling(card_qa, q_id)
    return PacketInputs(
        stage="classify_qa",
        item=item_id("interaction", QA_PREFIX + ruling["q_id"]),
        schema_path=str(CLASSIFY_QA_SCHEMA),
        task=CLASSIFY_TASK.format(q_id=ruling["q_id"]),
        inputs={
            "ruling": ruling,
            "cards": _card_context(ruling["card_ids"], clauses),
            "classes": list(CLASSIFICATIONS),
        },
        references=tuple(f"data/card_bundles/{c}.md" for c in ruling["card_ids"]) + COMMON_REFERENCES,
    )


def encode_ruling_inputs(
    q_id: str,
    card_qa: Mapping,
    clauses: Iterable[Mapping],
    *,
    scenario_path: str,
    scenario_yaml: str,
    mode: str = "author",
    candidate: Mapping | None = None,
) -> PacketInputs:
    """Inputs for encoding (``author``) or verifying (``verify``) a ruling's
    `expect_ruling` block for the scenario at `scenario_path`.

    ``verify`` carries the candidate block ONLY -- never the author's
    reasoning or answer quote -- so the second family judges the encoding,
    not the argument for it.
    """
    if mode not in ("author", "verify"):
        raise ValueError(f"mode must be author or verify, not {mode!r}")
    if mode == "verify" and not candidate:
        raise ValueError("verify needs the candidate expect_ruling block")
    ruling = _ruling(card_qa, q_id)
    inputs = {
        "mode": mode,
        "ruling": ruling,
        "cards": _card_context(ruling["card_ids"], clauses),
        "scenario": {"path": scenario_path, "yaml": scenario_yaml},
        "assertion_keys": list(ASSERTION_KEYS),
    }
    if mode == "verify":
        inputs["candidate"] = {"q_id": ruling["q_id"], "assert": candidate.get("assert")}
    task = (ENCODE_AUTHOR_TASK if mode == "author" else ENCODE_VERIFY_TASK).format(q_id=ruling["q_id"])
    return PacketInputs(
        stage="encode_ruling",
        item=item_id("interaction", QA_PREFIX + ruling["q_id"]),
        schema_path=str(ENCODE_RULING_SCHEMA),
        task=task,
        inputs=inputs,
        references=(scenario_path,) + tuple(f"data/card_bundles/{c}.md" for c in ruling["card_ids"])
        + COMMON_REFERENCES + ("docs/DCGO_EXAM.md",),
    )


def render_prompt(p: PacketInputs) -> str:
    """Deterministic default rendering (task, references, inputs as JSON)."""
    refs = "\n".join(f"- {r}" for r in p.references)
    return (
        f"{p.task}\n\nRead (do not modify):\n{refs}\n\n"
        f"Inputs:\n```json\n{json.dumps(p.inputs, indent=2, sort_keys=True, ensure_ascii=False)}\n```\n\n"
        f"Answer with JSON matching {Path(p.schema_path).name}."
    )


def to_task_packet(
    p: PacketInputs, *, family: str, attempt_id: str, worktree: str, prompt_version: str,
    prompt: str | None = None, model: str | None = None, effort: str | None = None,
    budget_usd: float | None = None,
) -> TaskPacket:
    return TaskPacket(
        stage=p.stage, family=family, item=p.item, attempt_id=attempt_id,
        prompt=prompt if prompt is not None else render_prompt(p),
        prompt_version=prompt_version, schema_path=p.schema_path, worktree=worktree,
        references=p.references, model=model, effort=effort, budget_usd=budget_usd,
    )


def independent_pair(
    p: PacketInputs, *, attempt_ids: Mapping[str, str], worktrees: Mapping[str, str],
    prompt_version: str, prompt: str | None = None,
) -> tuple[TaskPacket, TaskPacket]:
    """The SAME packet for each family (design D5): identical prompt, schema
    and references, so neither answer can depend on the other."""
    missing = [f for f in FAMILIES if f not in attempt_ids or f not in worktrees]
    if missing:
        raise ValueError(
            f"a terminating call needs both families; missing {missing} -- escalate instead "
            "of degrading to single-model judgment"
        )
    text = prompt if prompt is not None else render_prompt(p)
    a, b = (to_task_packet(p, family=f, attempt_id=attempt_ids[f], worktree=worktrees[f],
                           prompt_version=prompt_version, prompt=text) for f in FAMILIES)
    return a, b


# ---------------------------------------------------------------------------
# Results -> decisions
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Agreement:
    agreed: bool
    value: str | None          # the agreed class / "encoded"; None when escalating
    escalate: bool
    reasons: tuple[str, ...] = field(default_factory=tuple)


def _cites_ruling(result: Mapping, q_id: str) -> bool:
    return f"{QA_PREFIX}{q_id}" in str(result.get("citation", ""))


def agree_classification(q_id: str, a: Mapping | None, b: Mapping | None) -> Agreement:
    """Two independent `classify_qa` results -> the decision.

    ``behavioral`` from either family is enough to EXAMINE (examining is not
    terminating); ending the ruling as textual / not_examinable needs both
    families to say the same terminating class, each citing ``qa:<Q>``.
    """
    if a is None or b is None:
        return Agreement(False, None, True, ("only one family answered",))
    classes = (a.get("classification"), b.get("classification"))
    if "behavioral" in classes:
        return Agreement(classes[0] == classes[1], "behavioral", False,
                         () if classes[0] == classes[1] else ("one family says behavioral: examine",))
    reasons = []
    if classes[0] != classes[1]:
        reasons.append(f"families disagree: {classes[0]} vs {classes[1]}")
    for name, r in (("first", a), ("second", b)):
        if not _cites_ruling(r, q_id):
            reasons.append(f"{name} family does not cite {QA_PREFIX}{q_id}")
    if reasons:
        return Agreement(False, None, True, tuple(reasons))
    return Agreement(True, classes[0], False)


def agree_encoding(q_id: str, author: Mapping | None, verifier: Mapping | None) -> Agreement:
    """An author's block plus the other family's verification -> accept or escalate."""
    if author is None or verifier is None:
        return Agreement(False, None, True, ("only one family answered",))
    problems = validate_encode_result(author, "author") + validate_encode_result(verifier, "verify")
    if problems:
        return Agreement(False, None, True, tuple(problems))
    if author["q_id"] != q_id or verifier["q_id"] != q_id:
        return Agreement(False, None, True, (f"result names a different ruling than {q_id}",))
    if verifier["agrees"] is not True:
        return Agreement(False, None, True, (f"verifier disagrees: {verifier['answer_quote']}",))
    return Agreement(True, "encoded", False)


def validate_encode_result(result: Mapping, mode: str) -> list[str]:
    """Cross-field rules the strict schema cannot express."""
    problems = []
    if result.get("mode") != mode:
        problems.append(f"expected mode {mode}, got {result.get('mode')!r}")
    if mode == "author":
        if not result.get("expect_ruling"):
            problems.append("author result has no expect_ruling block")
        if result.get("agrees") is not None:
            problems.append("author result must leave agrees null")
        for row in (result.get("expect_ruling") or {}).get("assert", []):
            for pair in row.get("that", []):
                if pair.get("key") not in ASSERTION_KEYS:
                    problems.append(f"unknown assertion key {pair.get('key')!r}")
                try:
                    json.loads(pair.get("value_json", ""))
                except (TypeError, ValueError):
                    problems.append(f"value_json for {pair.get('key')!r} is not JSON")
    else:
        if result.get("expect_ruling") is not None:
            problems.append("verify result must leave expect_ruling null")
        if not isinstance(result.get("agrees"), bool):
            problems.append("verify result must set agrees")
    return problems


def expect_ruling_block(result: Mapping) -> dict:
    """An accepted `author` result -> the scenario's `expect_ruling:` mapping
    (`{q_id, assert: [{at, that: {key: value}}]}`), ready to dump as YAML."""
    problems = validate_encode_result(result, "author")
    if problems:
        raise ValueError("; ".join(problems))
    rows = []
    for row in result["expect_ruling"]["assert"]:
        that = {}
        for pair in row["that"]:
            if pair["key"] in that:
                raise ValueError(f"step {row['at']}: {pair['key']} asserted twice")
            that[pair["key"]] = json.loads(pair["value_json"])
        rows.append({"at": int(row["at"]), "that": that})
    return {"q_id": result["q_id"], "assert": rows}
