# Examon resumed oracle campaign — 2026-10-10

This records the earlier resumed pass. Its blocker list below is historical;
the completed repair restored the seven cards, built an isolated EX13 player,
and confirmed all 122 core clauses. The final 51 backfilled scenarios pass
6,940 assertions. Current results and broader verification limits are in
`../../archetype-qa/dsl/examon-2026-10-10.md`; the repair plan is
`../../archetype-qa/dsl/examon-2026-10-10-blocker-repair-plan.md` and the adjacent
coverage/core-adjudication files. Repair runs use the isolated
`DCGO-CatalogRepair/EX13-0a6f9dcd7c804fde9a5aea08cc0e17fb` runtime, not the shared player.

The campaign uses the matching C: harness and scripted-v20 player (build 7221db7e525fa04217b1d0a6c04515b1cb3e479a, action hash 711d23bf24fb). A global MCP health response described a different harness/root, so live runs used the explicit CLI. No engine or card code changed during the resumed oracle pass.

## Fixtures and prompt mappings

`examon_historical_pool.json` remains the exact Sanchez 50-main/5-egg list. `examon_core_pool.json` adds explicitly named legal 50-card fixtures for deterministic breeding, vanilla targets, removal Options, a CS route and a security Option. These variants are scenario fixtures, not tournament lists. P-035 security was checked against its image and official observation 183f4c89f487eb44051d189b1ff8807b7c522f7ab65a0b83f244bc0277bb5ab2 (base-repository observations). ST2-09, ST1-16 and BT23-058 images were checked before use. P-036 has no implementation here and is never played in these lines.

Security is dealt from stack positions 5–9; the first card checked is position 9. The first P-035 fixture put it at position 5 and was corrected before the successful oracle run. Repeated identical target IDs can resolve different copies when field ordering differs. The EX3-074 draft exposed this ambiguity; replacing one target with a distinct identity produced a complete confirmed replay. This was a fixture correction, not an engine repair.

Engine-only / DCGO-only rows express observed prompt topology, never skip a gameplay decision on both engines. Examples: DCGO asks for the DNA result and both materials while the engine fixes the carrier; only the partner is player-selectable there. Partition has one source pair in this fixture, while the engine exposes pair/order prompts that DCGO omits. The final observable states must still compare. RAID and inherited triggers are chosen by identity/keyword rather than a guessed ordinal.

## Historical blockers at the end of the resumed pass

- **DCGO-RUNTIME-CATALOG-EX13**: all nine core EX13 cards were inserted individually into legal diagnostic decks. Every sim preparation passed, and the live player rejected each ID before gameplay. See `../../archetype-qa/dsl/examon-2026-10-10-evidence/verification/catalog-probes/`. These probes establish catalog rejection, not card behavior or missing C# scripts. All 45 affected clauses remain unmeasured.
- **EXAM-BLAST-DNA-HAND-RESOLUTION**: the BT20-045 Blast DNA line reaches a legal Counter mask containing action 63, but the symbolic Hand-card resolver cannot name the card. See `resume-BT20-045-effect3-sim-1791669656269487100.log`. The Hand / Counter / Blast DNA clauses remain unmeasured. Ordinary DNA and its highest-DP sweep have separate confirmed scenarios.
- **EXAM-ORDERED-REMAINDER-CLOSURE-IDENTITY**: EX3-037 search selects both eligible cards, then fails at the final `OrderedPermutation { remaining: 1 }` identity lookup. Individual picks and a full two-card ordering both failed. The closure-based remainder is not exposed through `PermutationStep` metadata (`selection_resolve.rs::permutation_remaining_ids`). No raw-action bypass or assumed identity was used. The search clause remains unmeasured.
- **Withdrawn production implementations**: seven core cards / 27 clauses cannot be compared through a faithful installed YAML. Current pack absence was checked. Their specific capability findings and prior runtime/structural evidence are in `qa/dsl-vocab-gaps.md`: effect cost choice (BT20-023/040), mutual battle deletion (BT20-044), paid Delay field-pair DNA (BT20-093), result-zone material identity (EX3-020/041), and opponent main-phase timing (EX3-024). This is a card-authoring blocker; it does not assert every clause is intrinsically unsupported or unreachable.

The five oracle verdict classes are preserved. Failed/truncated attempts, catalog failures and authoring exclusions do not become confirmed, unavailable or unreachable. The per-clause adjudication file separates these measured tool/runtime failures from structural authoring exclusions. The support/tech tail remains explicitly unattempted where no prior verdict exists.

## Evidence and verification

Every resumed attempt has a durable raw log and an append-only exam-log row. Completed/failed recording sidecars are copied into the evidence tree. Current verdicts refer only to completed comparisons; failed earlier attempts remain in the attempt ledger. The report and coverage JSON include exact text-hash counts and the current per-clause core disposition.

Backfilled assertions are replayed using the same harness. Unstacked hand draws are omitted by backfill rather than asserted from an irreproducible deck order; board, memory, sources and security remain checked. No all-scenarios pass is claimed for deliberately blocked drafts. Prior full-engine verification and its pre-existing replay-golden limitation are retained in the campaign report.
