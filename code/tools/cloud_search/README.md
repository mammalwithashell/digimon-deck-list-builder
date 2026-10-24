# CPU search experiments

Publish `training-search-v*` tags through `.github/workflows/training-image.yml`.
The CPU image contains the native search CLI, matching PyO3 wheel, model
exporter, AlphaZero trainer, scheduler and anchored evaluator. The existing
`training-v*` GPU image workflow is unchanged. The CPU workflow smoke-checks
registry/contract parity and native shard generation before pushing the exact
tested local image. The smoke's deliberately capped games are not training or
playing-strength evidence.

Deploy by registry digest, recording the CI run, image source commit, digest,
model/data hashes and mounted deployment-controller hash in the experiment's
provenance. `run_experiment.py` is a separately versioned, read-only mounted
deployment controller; it uses the engine and training tools inside the image.
It validates inputs, checks serial/batched targets, compares worker counts,
checks 100 complete-game attempts each at 160/4 and 320/4, and only then runs
one approved generation at 320/4/batch 4. Either health panel exceeding 5%
truncation fails the launch. Generation uses dynamic four-game chunks, bounded
retries and CPU fitting. No model is promoted automatically.

The native pool samples slots uniformly. Use two slots per archetype when the
meta archetypes have two distinct lists and pet archetypes have one. Repeating
a pet list in two sampling slots gives it equal archetype weight; it is still
one distinct list. Canonical pool snapshots retain each distinct list once.
Preserve exact card counts and normalize only alternate-art suffixes. Do not
repair inadmissible personal lists by silently substituting cards.

Evaluate the meta validation panel and the exact pet lists separately. The
existing eight-list panel was used for budget tuning; neither that panel nor
same-list pet evaluation is untouched promotion evidence. Add distinct variants
later to test deck-list generalization.

`experiment.service` supervises the container without automatic restart and
stops it after 48 hours. This is a workload limit, **not a billing limit**:
Hetzner charges until the server is deleted. Keep it until artifacts are
harvested and verified. The bind-mounted experiment directory survives a
container exit and a server reboot, but not server deletion.

Run `backup_experiment.py --host root@HOST --key PATH --out PATH --watch` on the
development host to copy immutable committed chunks and checkpoint archives
every ten minutes. Transfers are hash-verified. Backups pause if that machine
is offline; the remote experiment continues. The helper never deletes a server
or remote data. The controller and backup helper refuse competing ownership or
implicit experiment restart. Investigate failed runs before explicit resume.
