"""Versioned, language-neutral source binding; never upgrades historical verdicts.

The wire payload is compact UTF-8 JSON arrays. Only line endings normalize.
All spans in a zone participate, including empty timing spans governing siblings.
"""
from __future__ import annotations

import hashlib
import json
import re
from collections import defaultdict
from typing import Mapping, Iterable

VERSION = 1
SHA256 = re.compile(r"^[0-9a-f]{64}$")


def normalize(value):
    if isinstance(value, str):
        return value.replace("\r\n", "\n").replace("\r", "\n")
    if isinstance(value, (list, tuple)):
        return [normalize(item) for item in value]
    if value is None or type(value) is int:
        return value
    raise ValueError("semantic payload must contain only arrays, strings, integers and null")


def semantic_digest(payload) -> str:
    encoded = json.dumps(normalize(payload), ensure_ascii=False, separators=(",", ":"))
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


def span(row: Mapping) -> list:
    if row.get("kind") not in ("timing", "keyword", "untimed"):
        raise ValueError(f"{row.get('id')}: missing/invalid clause kind")
    if not isinstance(row.get("timings"), list) or not all(isinstance(t, str) for t in row["timings"]):
        raise ValueError(f"{row.get('id')}: missing/invalid timings")
    if "keyword" not in row or (row["keyword"] is not None and not isinstance(row["keyword"], str)):
        raise ValueError(f"{row.get('id')}: missing/invalid keyword")
    if not isinstance(row.get("text"), str):
        raise ValueError(f"{row.get('id')}: missing/invalid text")
    return [row["kind"], row["timings"], row["keyword"], row["text"]]


def bind_clause_semantics(rows: Iterable[Mapping]) -> list[dict]:
    """Return fresh source rows with verified positional identity and zone binding.

    Call only on a complete fresh extraction (or to validate a book); this
    function cannot establish that a historical extraction is current.
    """
    rows = [dict(row) for row in rows]
    groups = defaultdict(list)
    seen = set()
    for row in rows:
        cid, zone = row.get("card_id"), row.get("zone")
        if not isinstance(cid, str) or not cid or zone not in ("effect", "inherited", "security"):
            raise ValueError("missing/invalid clause card or zone")
        parts = str(row.get("id", "")).split("#")
        if len(parts) != 3 or parts[:2] != [cid, zone] or not parts[2].isdigit() or str(int(parts[2])) != parts[2]:
            raise ValueError(f"invalid positional clause identity: {row.get('id')}")
        if row["id"] in seen:
            raise ValueError(f"duplicate clause: {row['id']}")
        seen.add(row["id"])
        span(row)
        groups[cid, zone].append((int(parts[2]), row))
    for (cid, zone), group in groups.items():
        group.sort(key=lambda pair: pair[0])
        if [i for i, _ in group] != list(range(len(group))):
            raise ValueError(f"incomplete positional zone: {cid}#{zone}")
        spans = [span(row) for _, row in group]
        for index, row in group:
            payload = normalize(["digimon-clause-semantics", VERSION, cid, zone, index, span(row), spans])
            row.update(semantic_fingerprint_version=VERSION, semantic_sha256=semantic_digest(payload),
                       semantic_payload=payload)
    return rows


def binding_error(current: Mapping, recorded: Mapping) -> str | None:
    """Fail closed for historical or unknown bindings; do not alter either row."""
    if getattr(recorded, "legacy_unbound", False):
        return "stored verdict belongs to an unbound historical container"
    if type(current.get("semantic_fingerprint_version")) is not int or current.get("semantic_fingerprint_version") != VERSION:
        return "current source has missing or unsupported semantic fingerprint version"
    payload = current.get("semantic_payload")
    digest = current.get("semantic_sha256")
    if not isinstance(digest, str) or not SHA256.fullmatch(digest) or payload is None:
        return "current source has incomplete semantic binding"
    try:
        if semantic_digest(payload) != digest:
            return "current source semantic payload/hash mismatch"
    except ValueError:
        return "current source has invalid semantic payload"
    if type(recorded.get("semantic_fingerprint_version")) is not int or recorded.get("semantic_fingerprint_version") != VERSION:
        return "stored verdict has missing or unsupported semantic fingerprint version"
    if recorded.get("semantic_sha256") != digest:
        return "stored verdict semantic_sha256 mismatch"
    return None


def interaction_binding_error(interaction_id: str, current: Mapping, recorded: Mapping) -> str | None:
    error = binding_error(current, recorded)
    if error:
        return error
    payload = current["semantic_payload"]
    cards = sorted(set(current.get("card_ids", [])))
    if current.get("source") == "probe":
        expected = ["digimon-interaction-semantics", VERSION, interaction_id, cards, "probe",
                    current.get("kind"), current.get("family"), current.get("family_version")]
        if not isinstance(payload, list) or len(payload) != 9 or payload[:8] != expected or not isinstance(payload[8], str) or not SHA256.fullmatch(payload[8]):
            return "current probe semantic payload does not match denominator identity"
    elif current.get("source") == "qa":
        q_id = current.get("q_id")
        expected = ["digimon-qa-semantics", VERSION, q_id, cards, current.get("kind")]
        if interaction_id != f"qa:{q_id}" or not isinstance(payload, list) or len(payload) != 6 or payload[:5] != expected or not isinstance(payload[5], str):
            return "current Q&A semantic payload does not match denominator identity"
    else:
        return "unsupported semantic interaction source"
    return None
