# Scoped legacy metadata review

Read-only review of BT18-039, BT6-089, BT9-084, LM-045 and EX10-041.
Fresh complete source acquisition covered 4,474 official IDs (7,920 observations,
72 source locations) and 4,535 secondary IDs (47 source locations), with no failed
source locations. The acquisition adapter fix uses catalog discovery and bounded
exact-ID batches, preserving response digests and refusing truncated batches.

The revised evidence passed an independent second review. Zero new overrides
are needed or approved. The staged correction ledger remains byte-identical to
370 existing rows; its hash is in the attached evidence. Eight rejected overrides
were removed before the final preflight.

`expected-scoped-runtime-diffs.json` is a source-only, five-card preview of 15
field differences; it is not a generated full candidate or applied runtime result.
`complete-candidate-blocker.json` preserves the measured final-input diagnostic:
84 official condition conflicts outside these five IDs block the complete candidate.
No production metadata, adjudications or publication was changed. The repository
has no validated minimal text/security installation route. Supported installation
still depends on a complete candidate passing the existing preflight.

Raw source responses and the unchanged staged ledger remain local review artifacts
under `.codex/dynasmon-source-review` and `.codex/dynasmon-metadata-stage`.
