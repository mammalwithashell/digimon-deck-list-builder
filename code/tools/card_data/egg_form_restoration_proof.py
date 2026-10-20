"""Separate proof-only Digi-Egg Form restoration predicate.

The sealed Digimon v1 predicate remains unchanged. Its whole-input validation
is reused and pinned, but its eligibility is never treated as Egg approval.
This module has no approval emitter or publication path.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import json
from pathlib import Path
import unicodedata
from urllib.parse import urljoin

from bs4 import BeautifulSoup

from . import form_restoration_proof as base
from .serialization import canonical_json_bytes, content_digest

PREDICATE_VERSION = "simple-printed-egg-form-v1"
ProofError = base.ProofError


def _raw_reasons(observation, soup):
    e = observation["extracted"]
    blocks = soup.find_all("div", class_="popupCol", id=e["printing"]["printing_id"])
    if len(blocks) != 1:
        return ["ambiguous_raw_popup"]
    block = blocks[0]
    reasons = []
    if [n.get_text(" ", strip=True) for n in block.select("li.cardNo")] != [e["card_id"]]:
        reasons.append("raw_card_identity")
    types = block.select("li.cardType")
    if (len(types) != 1 or types[0].find(True) is not None
            or types[0].has_attr("hidden") or types[0].get("style")
            or types[0].get_text(" ", strip=True) != "Digi-Egg"):
        reasons.append("raw_card_type_not_literal_egg")
    images = [urljoin(observation["source_url"], n.get("src", ""))
              for n in block.select("div.cardImg img")]
    if images != [e["printing"].get("image_url")]:
        reasons.append("raw_image_identity")
    forms = [n for n in block.select("dt.cardInfoTit")
             if n.get_text(" ", strip=True) == "Form"]
    if len(forms) != 1:
        reasons.append("raw_form_row_count")
    else:
        value = forms[0].find_next_sibling("dd")
        if (value is None or value.find(True) is not None or value.has_attr("hidden")
                or value.get("style") or value.get_text(" ", strip=True) != "In-Training"):
            reasons.append("raw_form_not_literal_in_training")
    # Intentionally retain v1's conservative whole-popup Rule guard, including
    # FAQ mentions. FAQ-only eligibility is a different, unreviewed family.
    text = unicodedata.normalize("NFKC", block.get_text(" ", strip=True) + " "
                                + " ".join(n.get("alt", "") for n in block.select("img")))
    if base.RULE.search(text) or any("rule" in n.get_text(" ", strip=True).lower()
                                    for n in block.select("dt, .cardInfoTitMedium")):
        reasons.append("raw_rule_section")
    if block.select(".cardDetailColInnerDual, .cardDetailColInner_dual"):
        reasons.append("raw_dual_layout")
    return reasons


def prove(*, report_bytes: bytes, snapshot_bytes: bytes, raw_pages: dict[str, bytes]):
    """Classify every Form diff; never accepts an eligible-only input list."""
    # Reuse complete scope, raw digest, schema, observation/reference identity,
    # diff digest and official provenance closure checks without modifying v1.
    checked = base.prove(report_bytes=report_bytes, snapshot_bytes=snapshot_bytes,
                         raw_pages=raw_pages)
    report, snapshot = base._load(report_bytes), base._load(snapshot_bytes)
    by_card = defaultdict(list)
    for o in snapshot["observations"]:
        by_card[o["extracted"]["card_id"]].append(o)
    corrected = {r["card_id"] for key in ("corrections", "compatibility_corrections")
                 for r in report.get(key, [])}
    conflicts = {r["card_id"] for r in report.get("canonical_conflicts", [])
                 if r.get("approval_required")}
    diffs = [d for d in report["runtime_field_diffs"] if d["field_path"] == "form_eng"]
    eligible, excluded, soups = [], [], {}
    for diff in sorted(diffs, key=lambda d: d["card_id"]):
        cid, reasons, evidence = diff["card_id"], [], []
        if diff.get("old_value") != []:
            reasons.append("old_value_not_empty_list")
        if diff.get("new_value") != ["In-Training"]:
            reasons.append("not_exact_in_training_list")
        if cid in corrected:
            reasons.append("card_has_correction")
        if cid in conflicts:
            reasons.append("required_canonical_conflict")
        if cid in base.KNOWN_SOURCE_GAPS:
            reasons.append("known_adjacent_source_image_disagreement")
        official = by_card.get(cid, [])
        if not official:
            reasons.append("no_official_printing")
        for o in official:
            e = o["extracted"]
            if e.get("card_type") != "Digi-Egg":
                reasons.append("not_exact_digi_egg_type")
            if e.get("form") != "In-Training":
                reasons.append("official_printing_disagrees")
            if type(e.get("level")) is not int or e["level"] != 2:
                reasons.append("not_level_two_egg")
            faces = e.get("faces")
            if (e.get("layout") != "single" or not isinstance(faces, list) or len(faces) != 1
                    or faces[0].get("kind") != "digi_egg" or type(faces[0].get("level")) is not int
                    or faces[0]["level"] != 2):
                reasons.append("not_single_egg_face")
            if e.get("rule_grants") != {"traits": [], "attributes": [], "forms": []}:
                reasons.append("rule_grants_or_missing_grant_shape")
            if any(base.RULE.search(unicodedata.normalize("NFKC", t.get("text", "")))
                   or "rule" in t.get("label", "").lower() for t in e.get("text_sections", [])):
                reasons.append("rule_text_or_section")
            if e.get("printing_field_gaps"):
                reasons.append("printing_field_gap")
            evidence.append({k: o[k] for k in ("observation_id", "raw_payload_digest",
                                              "source_record_key", "source_url")} | {
                "printing_id": e["printing"]["printing_id"], "printed_form": e.get("form"),
                "printed_card_type": e.get("card_type")})
        if not reasons:
            for o in official:
                url = o["source_url"]
                if url not in soups:
                    soups[url] = BeautifulSoup(raw_pages[url].decode("utf-8"), "html.parser")
                reasons.extend(_raw_reasons(o, soups[url]))
        if reasons:
            excluded.append({"card_id": cid, "reasons": sorted(set(reasons))})
        else:
            eligible.append({"diff": diff, "official_evidence": sorted(evidence,
                             key=lambda e: e["observation_id"])})
    base._require(len(eligible) + len(excluded) == checked["total_form_diffs"],
                  "full Form denominator lost")
    return {"status": "PROOF_ONLY_NO_APPROVALS", "predicate_version": PREDICATE_VERSION,
            "predicate_sha256": base._sha(Path(__file__).read_bytes()),
            "dependency_sha256": {**checked["dependency_sha256"],
                                  "form_restoration_proof.py": base._sha(Path(base.__file__).read_bytes())},
            "candidate_report_sha256": checked["candidate_report_sha256"],
            "official_snapshot_sha256": checked["official_snapshot_sha256"],
            "raw_page_sha256": checked["raw_page_sha256"],
            "old_input_revision": checked["old_input_revision"],
            "new_input_revision": checked["new_input_revision"],
            "total_form_diffs": len(diffs), "eligible": eligible, "excluded": excluded,
            "membership_sha256": content_digest([r["diff"]["card_id"] for r in eligible])}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("prove",))
    for name in ("report", "snapshot", "cache-index", "out"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    base._require(not args.out.exists(), "refusing to overwrite proof output")
    pages = {}
    for item in json.loads(args.cache_index.read_text(encoding="utf-8")):
        base._require(item["url"] not in pages, "duplicate cache URL")
        path = (args.cache_index.parent / item["file"]).resolve()
        base._require(path.is_relative_to(args.cache_index.parent.resolve()), "cache path escapes inventory")
        pages[item["url"]] = path.read_bytes()
    proof = prove(report_bytes=args.report.read_bytes(), snapshot_bytes=args.snapshot.read_bytes(), raw_pages=pages)
    with args.out.open("xb") as out:
        out.write(canonical_json_bytes(proof))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
