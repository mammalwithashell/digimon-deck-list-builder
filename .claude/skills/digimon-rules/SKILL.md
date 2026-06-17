---
name: digimon-rules
description: Use when reasoning about, implementing, debugging, or QA'ing Digimon TCG RULES, KEYWORDS, or TIMING — or when the user wants a deep TCG thinking partner. Resolves a keyword / rule number / topic to the exact authoritative rules-manual pages and reads them; `deep` mode loads the full verified rules digest. Triggers on keyword semantics ("is Save optional?", "how does Piercing resolve?"), rule numbers (16-36), timing / processing-order / optional-vs-mandatory questions, attack/block/security/battle flow, memory or digivolution rules, or "be my Digimon rules thinking partner". Reads local files + the base-repo PDF only — no Pinecone / network.
---

# Digimon TCG Rules Lookup

The authoritative source for rules, keyword semantics, and timing is the **Comprehensive
Rules Manual**. The PDFs live **base-only** (git-ignored, not in worktrees — rule 29):

```bash
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
# "$BASE/Digimon TCG resources/general_rule.pdf"  -> timing / keyword / effect rules (authoritative text)
# "$BASE/Digimon TCG resources/glossary.pdf"      -> keyword definitions + areas/states/phases/timings
# "$BASE/Digimon TCG resources/manual.pdf"        -> image-heavy, visual / UI reference
```

Committed quick-reference artifacts (present in **every** worktree — read these first, they are cheap):
- `docs/digimon-rules/keyword-semantics.md` — compact per-keyword table (optional/mandatory + targets + rule §).
- `docs/digimon-rules/rules-index.json` — keyword / topic / rule-number → `{pdf, pages, section}`.
- `docs/digimon-rules/digest.md` — the full verified deep digest (cited).

## Mode A — Lookup (default): `/digimon-rules <keyword | rule-# | topic>`

1. Open `docs/digimon-rules/rules-index.json`. Find the entry whose `keywords`/`topics`
   `names` match the query, or — for a rule number like `16-36` — the enclosing `sections`
   entry (split on the first `-`; `16-36` → section `16`).
2. Resolve `BASE` (command above). `Read` the entry's `pdf` at its `pages` using the Read
   tool's `pages` arg (e.g. `pages: "33-40"`).
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
  via `BASE` (TOC: §8 Digivolution p.16, §11 Attacking p.19, §13 Security p.21, §15 Effect
  Rules p.22, §16 Keyword Effects p.33).
