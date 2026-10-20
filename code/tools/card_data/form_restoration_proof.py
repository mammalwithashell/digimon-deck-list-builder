"""Conservative Form family proof; never publishes or changes approval policy.

prove() emits no adjudications. emit_reviewed() recomputes that exact proof and
requires an independent, digest-bound family review. Callers must separately
review any output before merging it into a candidate ledger. Only four simple
English printed Forms are covered; this is substantive runtime data repair.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
from datetime import datetime
import hashlib
import json
from pathlib import Path
import re
import unicodedata
from urllib.parse import urljoin, urlparse

from bs4 import BeautifulSoup
from . import models, serialization, adjudications
from .models import AdjudicationKind, MigrationAdjudication
from .serialization import canonicalize, canonical_json_bytes, content_digest

PREDICATE_VERSION = "simple-printed-form-v1"
LABELS = frozenset({"Rookie", "Champion", "Ultimate", "Mega"})
KNOWN_SOURCE_GAPS = frozenset({"BT10-111"})
HEX = re.compile(r"^[0-9a-f]{64}$")
RULE = re.compile(r"(?:\[|\(|<)\s*Rule\s*(?:\]|\)|>)|\bRule\s*:?\s*Trait\b", re.I)


class ProofError(ValueError):
    """Invalid proof inputs or missing independent approval; emit nothing."""


def _require(condition, message):
    if not condition:
        raise ProofError(message)


def _sha(raw):
    return hashlib.sha256(raw).hexdigest()


def _load(raw):
    try:
        value = json.loads(raw)
    except (ValueError, UnicodeError) as error:
        raise ProofError("invalid JSON proof input") from error
    _require(isinstance(value, dict), "proof input must be an object")
    return value


def _origin(url):
    parsed = urlparse(url)
    return parsed.scheme == "https" and parsed.netloc == "world.digimoncard.com" and parsed.path == "/cards/"


def _raw_reasons(observation, soup):
    e = observation["extracted"]
    cid, printing = e["card_id"], e["printing"]["printing_id"]
    blocks = soup.find_all("div", class_="popupCol", id=printing)
    if len(blocks) != 1:
        return ["ambiguous_raw_popup"]
    block = blocks[0]
    reasons = []
    if [n.get_text(" ", strip=True) for n in block.select("li.cardNo")] != [cid]:
        reasons.append("raw_card_identity")
    if [n.get_text(" ", strip=True) for n in block.select("li.cardType")] != ["Digimon"]:
        reasons.append("raw_card_type")
    images = [urljoin(observation["source_url"], i.get("src", "")) for i in block.select("div.cardImg img")]
    if images != [e["printing"].get("image_url")]:
        reasons.append("raw_image_identity")
    rows = [n for n in block.select("dt.cardInfoTit") if n.get_text(" ", strip=True) == "Form"]
    if len(rows) != 1:
        reasons.append("raw_form_row_count")
    else:
        value = rows[0].find_next_sibling("dd")
        # A literal text row only: image/hidden/nested markup needs a different proof.
        if value is None or value.find(True) is not None or value.has_attr("hidden") or value.get("style") or value.get_text(" ", strip=True) != e["form"]:
            reasons.append("raw_form_not_literal_exact")
    raw_text = unicodedata.normalize("NFKC", block.get_text(" ", strip=True) + " " + " ".join(n.get("alt", "") for n in block.select("img")))
    if RULE.search(raw_text) or any("rule" in n.get_text(" ", strip=True).lower() for n in block.select("dt, .cardInfoTitMedium")):
        reasons.append("raw_rule_section")
    if block.select(".cardDetailColInnerDual, .cardDetailColInner_dual"):
        reasons.append("raw_dual_layout")
    return reasons


def prove(*, report_bytes: bytes, snapshot_bytes: bytes, raw_pages: dict[str, bytes]):
    """Rebuild the complete Form denominator; no selected-row input is accepted."""
    report, snapshot = _load(report_bytes), _load(snapshot_bytes)
    for name in ("old_input_revision", "new_input_revision"):
        _require(isinstance(report.get(name), str) and HEX.fullmatch(report[name]), "invalid revision identity")
    manifest = snapshot.get("manifest", {})
    _require(manifest.get("complete") is True and manifest.get("failed_ids") == [], "incomplete official scope")
    observations = snapshot.get("observations", [])
    by_id, by_card = {}, defaultdict(list)
    page_hashes = {}
    for observation in observations:
        try:
            ident = observation["observation_id"]
            payload = {k: observation[k] for k in ("raw_payload_digest", "parser_version", "source_schema_version", "extracted")}
            _require(content_digest(payload, exclude_operational=True) == ident, "observation identity mismatch")
            _require(ident not in by_id and observation["source"] == "official", "duplicate or nonofficial observation")
            _require(observation["source_schema_version"] == "world-card-list-html-v1", "unreviewed official schema")
            _require(observation["parser_version"] in {"official-html-v4", "official-html-v5"}, "unreviewed official parser")
            url = observation["source_url"]
            _require(_origin(url), "unreviewed official origin")
            raw = raw_pages.get(url)
            _require(isinstance(raw, bytes), "missing raw page")
            if url not in page_hashes:
                page_hashes[url] = _sha(raw)
            _require(page_hashes[url] == observation["raw_payload_digest"], "raw payload digest mismatch")
            e = observation["extracted"]
            _require(isinstance(e["card_id"], str) and models.CARD_ID_RE.fullmatch(e["card_id"]), "invalid observed card ID")
            _require(isinstance(e["printing"]["printing_id"], str), "missing printing identity")
            by_id[ident] = observation
            by_card[e["card_id"]].append(observation)
        except (KeyError, TypeError) as error:
            raise ProofError("malformed official observation") from error
    declared = manifest.get("observation_ids", [])
    refs = manifest.get("observation_refs", [])
    _require(len(declared) == len(set(declared)) and set(declared) == set(by_id), "official observation closure mismatch")
    _require(len(refs) == len(by_id) and {r.get("observation_id") for r in refs} == set(by_id), "official printing closure mismatch")
    for ref in refs:
        e = by_id[ref["observation_id"]]["extracted"]
        _require(ref.get("gameplay_card_id") == e["card_id"] and ref.get("printing_id") == e["printing"]["printing_id"], "official reference identity mismatch")
    _require(type(manifest.get("expected_count")) is int and manifest["expected_count"] == len(by_card), "official universe count mismatch")
    corrected = {r["card_id"] for r in report.get("corrections", [])}
    conflicts = {r["card_id"] for r in report.get("canonical_conflicts", []) if r.get("field_path") == "form_eng" and r.get("approval_required")}
    diffs = [r for r in report["runtime_field_diffs"] if r["field_path"] == "form_eng"]
    _require(len({r["card_id"] for r in diffs}) == len(diffs), "duplicate Form diff")
    eligible, excluded, soups = [], [], {}
    for diff in sorted(diffs, key=lambda row: row["card_id"]):
        cid = diff["card_id"]
        _require(diff.get("old_digest") == content_digest(diff.get("old_value")) and diff.get("new_digest") == content_digest(diff.get("new_value")), "runtime field digest mismatch")
        reasons = []
        if "old_value" not in diff or diff["old_value"] != []:
            reasons.append("old_value_not_empty_list")
        new = diff.get("new_value")
        if not isinstance(new, list) or len(new) != 1 or new[0] not in LABELS:
            reasons.append("not_single_allowed_label")
        if cid in corrected: reasons.append("card_has_correction")
        if cid in conflicts: reasons.append("required_form_conflict")
        if cid in KNOWN_SOURCE_GAPS: reasons.append("known_adjacent_source_image_disagreement")
        official = by_card.get(cid, [])
        if not official: reasons.append("no_official_printing")
        evidence = []
        for observation in official:
            e = observation["extracted"]
            if e.get("form") != (new[0] if isinstance(new, list) and len(new) == 1 else None): reasons.append("official_printing_disagrees")
            if e.get("card_type") != "Digimon": reasons.append("non_English_or_missing_card_type")
            if e.get("layout") != "single" or len(e.get("faces", [None])) != 1: reasons.append("non_single_layout")
            grants = e.get("rule_grants", {})
            if not isinstance(grants, dict) or any(grants.values()): reasons.append("rule_grants")
            if any(RULE.search(unicodedata.normalize("NFKC", t.get("text", ""))) or "rule" in t.get("label", "").lower() for t in e.get("text_sections", [])): reasons.append("rule_text_or_section")
            if any(g.get("field") in {"form", "form_eng"} for g in e.get("printing_field_gaps", [])): reasons.append("printing_form_gap")
            evidence.append({k: observation[k] for k in ("observation_id", "raw_payload_digest", "source_record_key", "source_url") } | {"printing_id": e["printing"]["printing_id"], "printed_form": e.get("form")})
        _require({o["observation_id"] for o in official} <= set(diff.get("provenance_observation_ids", [])), "diff omits official printing provenance")
        if not reasons:
            for observation in official:
                url = observation["source_url"]
                if url not in soups:
                    soups[url] = BeautifulSoup(raw_pages[url], "html.parser")
                reasons.extend(_raw_reasons(observation, soups[url]))
        if reasons:
            excluded.append({"card_id": cid, "reasons": sorted(set(reasons))})
        else:
            eligible.append({"diff": diff, "official_evidence": sorted(evidence, key=lambda e: e["observation_id"])})
    return {"status": "PROOF_ONLY_NO_APPROVALS", "predicate_version": PREDICATE_VERSION,
            "predicate_sha256": _sha(Path(__file__).read_bytes()),
            "dependency_sha256": {"models.py": _sha(Path(models.__file__).read_bytes()), "serialization.py": _sha(Path(serialization.__file__).read_bytes()), "adjudications.py": _sha(Path(adjudications.__file__).read_bytes())},
            "candidate_report_sha256": _sha(report_bytes), "official_snapshot_sha256": _sha(snapshot_bytes),
            "raw_page_sha256": dict(sorted(page_hashes.items())),
            "old_input_revision": report["old_input_revision"], "new_input_revision": report["new_input_revision"],
            "total_form_diffs": len(diffs), "eligible": eligible, "excluded": excluded,
            "membership_sha256": content_digest([r["diff"]["card_id"] for r in eligible])}


def emit_reviewed(*, report_bytes, snapshot_bytes, raw_pages, review, existing_adjudications=()):
    """Emit only ordinary exact-bound rows after a separately reviewed proof."""
    proof = prove(report_bytes=report_bytes, snapshot_bytes=snapshot_bytes, raw_pages=raw_pages)
    _require(isinstance(review, dict), "independent family review required")
    expected = {"status": "APPROVED", "proof_sha256": content_digest(proof), "predicate_sha256": proof["predicate_sha256"],
                "old_input_revision": proof["old_input_revision"], "new_input_revision": proof["new_input_revision"],
                "membership_sha256": proof["membership_sha256"], "count": len(proof["eligible"])}
    _require(all(review.get(k) == v for k, v in expected.items()) and type(review.get("count")) is int, "stale or mismatched family review")
    _require(isinstance(review.get("reviewer"), str) and bool(review["reviewer"].strip()), "reviewer required")
    try:
        stamp = datetime.fromisoformat(review.get("reviewed_at", "").replace("Z", "+00:00"))
        _require(stamp.tzinfo is not None, "review timestamp requires timezone")
    except (ValueError, AttributeError) as error:
        raise ProofError("valid review timestamp required") from error
    existing = {(r["card_id"], r.get("field_path"), r["old_input_revision"], r["new_input_revision"]) for r in existing_adjudications}
    output = []
    for row in proof["eligible"]:
        d = row["diff"]
        key = (d["card_id"], "form_eng", proof["old_input_revision"], proof["new_input_revision"])
        _require(key not in existing, "existing adjudication would be duplicated")
        item = MigrationAdjudication(schema_version=1,
            adjudication_id=f"{PREDICATE_VERSION}-{d['card_id']}-{expected['proof_sha256'][:16]}",
            kind=AdjudicationKind.RUNTIME_FIELD_CUTOVER, card_id=d["card_id"], field_path="form_eng",
            old_input_revision=proof["old_input_revision"], new_input_revision=proof["new_input_revision"],
            old_digest=d["old_digest"], new_digest=d["new_digest"], old_value=d["old_value"], new_value=d["new_value"],
            provenance_observation_ids=tuple(e["observation_id"] for e in row["official_evidence"]),
            reason=f"Substantive printed Form restoration; independently reviewed family proof {expected['proof_sha256']}; no approval of adjacent fields or behavior.",
            reviewer=review["reviewer"], reviewed_at=review["reviewed_at"])
        output.append(canonicalize(item))
    return output


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("prove", "emit-reviewed"))
    for name in ("report", "snapshot", "cache-index", "out"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--review", type=Path)
    parser.add_argument("--existing-adjudications", type=Path)
    args = parser.parse_args(argv)
    _require(not args.out.exists(), "refusing to overwrite proof or approval output")
    index = json.loads(args.cache_index.read_text(encoding="utf-8"))
    pages = {}
    for item in index:
        _require(item["url"] not in pages, "duplicate cache URL")
        path = (args.cache_index.parent / item["file"]).resolve()
        _require(path.is_relative_to(args.cache_index.parent.resolve()), "cache path escapes inventory")
        pages[item["url"]] = path.read_bytes()
    kwargs = dict(report_bytes=args.report.read_bytes(), snapshot_bytes=args.snapshot.read_bytes(), raw_pages=pages)
    if args.mode == "prove":
        _require(args.review is None, "prove never consumes approvals")
        value = prove(**kwargs)
    else:
        _require(args.review is not None, "independent review file required")
        _require(args.existing_adjudications is not None and args.existing_adjudications.is_file(), "existing ledger inventory required")
        existing = [canonicalize(row) for row in adjudications.load_migration_adjudications(args.existing_adjudications)]
        rows = emit_reviewed(**kwargs, review=json.loads(args.review.read_text(encoding="utf-8")), existing_adjudications=existing)
        value = {"schema_version": 1, "adjudications": rows}
    with args.out.open("xb") as output:
        output.write(canonical_json_bytes(value))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
