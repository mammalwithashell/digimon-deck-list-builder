"""Explicit fresh v3 evidence for synthetic tests, never campaign migration."""
from tools.clause_coverage.semantic_fingerprint import semantic_digest


def interaction(iid, row, clauses=None):
    row = dict(row)
    if row["source"] == "qa":
        row["q_id"] = iid.split(":", 1)[1]
        payload = ["digimon-qa-semantics", 1, row["q_id"], sorted(row["card_ids"]),
                   row["kind"], row.get("text_sha256", "fixture question") + "\n"]
    else:
        row.setdefault("clause_id", iid.split(":")[1])
        if clauses is None:
            from tools.clause_coverage.extract import run
            clauses = {r["id"]: r for r in run(row["card_ids"], "fresh synthetic interaction source")["clauses"]}
        clause_sha = clauses.get(row["clause_id"], {}).get("semantic_sha256")
        row.setdefault("family", "fixture")
        row.setdefault("family_version", "families@1")
        payload = ["digimon-interaction-semantics", 1, iid, sorted(row["card_ids"]), "probe",
                   row["kind"], row["family"], row["family_version"],
                   clause_sha or semantic_digest([row.get("text_sha256", "fixture clause")])]
    row.update(semantic_fingerprint_version=1, semantic_sha256=semantic_digest(payload),
               semantic_payload=payload)
    return row


def write_current_qa(path, rows):
    """Explicit current Q&A source for fresh synthetic test interactions."""
    import json
    qa, cards = {}, {}
    for iid, row in rows.items():
        if row.get("source") != "qa": continue
        q = iid.split(":",1)[1]
        qa[q] = {"question": row.get("text_sha256", "fixture question"), "answer": "", "card_ids": row["card_ids"]}
        for card in row["card_ids"]: cards.setdefault(card, []).append(q)
    path.write_text(json.dumps({"qa":qa,"cards":cards}), encoding="utf-8")
