# Digimon TCG Rules Reference — MOVED (retired 2026-06-16)

This file was an **LLM-generated decomposition** of `general_rule.pdf` and has been
**retired** in favor of verified, PDF-derived artifacts that are read directly from the
manual (each claim citing a rule §):

- **`docs/digimon-rules/keyword-semantics.md`** — compact per-keyword table (every §16
  keyword + optional/mandatory + targets + rule §).
- **`docs/digimon-rules/rules-index.json`** — keyword / topic / rule-number → exact PDF pages.
- **`docs/digimon-rules/digest.md`** — deep, cited rules digest (turn flow, memory,
  digivolution, attack/block/security/battle, effect timing & processing order, gotchas).

The **authoritative** source remains `Digimon TCG resources/general_rule.pdf` (base repo
only — resolve via `$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")`;
see CLAUDE.md rule 32).

Use **`/digimon-rules <query>`** to read the right pages, or **`/digimon-rules deep`** to
load the full digest. See CLAUDE.md "Source priority for card / keyword / rules questions"
(item #5) and rule 32. None of this depends on Pinecone.
