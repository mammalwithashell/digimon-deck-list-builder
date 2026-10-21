---
name: digimon-rules
description: Use when reasoning about, implementing, debugging, or QA'ing Digimon TCG RULES, KEYWORDS, or TIMING — or when the user wants a deep TCG thinking partner. Resolves a keyword / rule number / topic to the exact authoritative rules-manual pages and reads them; `deep` mode loads the full verified rules digest. Triggers on keyword semantics ("is Save optional?", "how does Piercing resolve?"), rule numbers (16-36), timing / processing-order / optional-vs-mandatory questions, attack/block/security/battle flow, memory or digivolution rules, or "be my Digimon rules thinking partner". Reads local files + the base-repo PDF only — no Pinecone / network.
---

# Digimon TCG Rules Lookup

The authoritative source for rules, keyword semantics, and timing is the **Comprehensive
Rules Manual — Ver. 4.3 (Last updated 2026/09/18)**. The PDFs live **base-only** (git-ignored,
not in worktrees — rule 29):

```bash
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
# "$BASE/Digimon TCG resources/general_rule.pdf"  -> Ver. 4.3 timing / keyword / effect rules (authoritative text)
# "$BASE/Digimon TCG resources/general_rule_v3.6.pdf" -> the superseded 3.6 manual (for reading old citations only)
# "$BASE/Digimon TCG resources/glossary.pdf"      -> keyword definitions + areas/states/phases/timings
# "$BASE/Digimon TCG resources/manual.pdf"        -> image-heavy, visual / UI reference
```

Committed quick-reference artifacts (present in **every** worktree — read these first, they are cheap):
- `docs/digimon-rules/keyword-semantics.md` — compact per-keyword table (optional/mandatory + targets + rule §), 44 keywords.
- `docs/digimon-rules/rules-index.json` — keyword / topic / rule-number → `{pdf, pages, section}`.
- `docs/digimon-rules/digest.md` — the full verified deep digest (cited).
- `docs/digimon-rules/cr-4.3.txt` — the **full manual text** (`pdftotext -raw`); `grep -n "^16-37" docs/digimon-rules/cr-4.3.txt`
  gives the exact rule wording without the PDF. Prefer the PDF pages when they are reachable; the text copy is the
  worktree fallback and the grep-able index.
- `docs/digimon-rules/changelog-3.6-to-4.3.md` — renumber map (3.6 → 4.3) and every semantic change. **Any
  rule number quoted from older code, docs, memories or chat may be a 3.6 number** — translate it before
  reading (§16 keywords shifted +1: 3.6's 16-36 Fragment is 4.3's 16-37; §15-7-4 reversed its meaning).
- `python code/tools/rules/check_rules_freshness.py [--online]` — exits non-zero if the committed stamp lags the PDF /
  the official site.

## Mode A — Lookup (default): `/digimon-rules <keyword | rule-# | topic>`

1. Open `docs/digimon-rules/rules-index.json`. Find the entry whose `keywords`/`topics`
   `names` match the query, or — for a rule number like `16-37` — the enclosing `sections`
   entry (split on the first `-`; `16-37` → section `16`).
2. Resolve `BASE` (command above). `Read` the entry's `pdf` at its `pages` using the Read
   tool's `pages` arg (e.g. `pages: "35-42"`). In a worktree without the PDF, grep the rule
   number in `docs/digimon-rules/cr-4.3.txt` instead (`grep -n -A12 "^16-37\." docs/digimon-rules/cr-4.3.txt`).
3. Answer from the **printed rule text**, citing the rule number. Cross-check
   `keyword-semantics.md` for the optional/mandatory kind. If the printed rule text is
   terse, also read `glossary.pdf` for the player-facing definition.
4. A question about a specific *card's* behavior still defers to DCGO C# / the card image
   (CLAUDE.md source priority). This skill governs *rules*, not card-specific resolution.

## Mode B — Deep / thinking partner: `/digimon-rules deep`

`Read` the whole of `docs/digimon-rules/digest.md` into context, then act as a
sparring/thinking partner who knows how the game is actually played (turn flow, memory,
digivolution, attack/block/security/battle, effect timing & processing order, common
interaction gotchas). For any specific rule you cite, open the underlying PDF pages (via
`rules-index.json`) to confirm before asserting.

## Rules for this skill
- Read local files + the base-repo PDF only. **Never** query Pinecone or the network; this
  works with `PINECONE_API_KEY` unset.
- Never assert a rule from memory — cite the page/rule § you read.
- The official `general_rule.pdf` outranks `cards.json` and the (retired) `RULES_CONTEXT.md`;
  when DCGO behavior and the PDF disagree on a *rules* question, the PDF governs (rule 1 of
  the source priority). For *card behavior*, DCGO governs.
- If an artifact is missing (older branch), fall back to reading `general_rule.pdf` directly
  via `BASE` (Ver. 4.3 TOC: §4 Basic Terminology p.8, §8 Digivolution p.18, §11 Attacking p.22,
  §13 Security p.23, §15 Effect Rules p.24, §16 Keyword Effects p.35, §17 Rule Checks p.42).
- A citation that reads oddly against the text (e.g. "16-36 Fragment" landing on `<Decode>`) is
  almost certainly a pre-4.0 number: look it up in `changelog-3.6-to-4.3.md` §1 (renumber map).
