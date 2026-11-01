# Qualified panel host queues

`code/tools/cloud_search/frozen_mode_queue.py` sequences **already prepared, qualification-gated** panels. It does not qualify a runtime or create new scientific coverage. The two original scored queues were launched and are now terminal failures; they must never be restarted. Current status is in `execution-status.md` and the program ledger.

Before preparation, freeze a disjoint assignment of the existing 22 panel IDs to hosts and queue batches. Keep matched A/B controls together where possible. The host claim registry prevents duplicate panels on one host; it does not prove cross-host uniqueness. The global assignment must supply that proof before either host launches. Stage D requires its completed weight-training provenance proof. Use measured artifact sizes to estimate each batch, retaining at least 2 GiB of free disk; estimates are not hard bounds and there is no deletion or offload loop.

Prepare with `python -m tools.cloud_search.frozen_mode_queue prepare --root NEW_QUEUE --panels QUALIFIED_PANEL... --artifact-bytes-per-panel CONSERVATIVE_MEASURED_ESTIMATE --shared-lock ABSOLUTE_HOST_LOCK`. This copies and binds each immutable panel manifest, the runtime and supporting Python files. The frozen entry point is `host_controller.py`; naming it `queue.py` shadows Python's standard library and is prohibited.

Use one consistent lock path per host across all batches:

- Windows: `D:/wt/p3-constructs/runs/ex13-evaluation-program-20261029/host-queue.lock`
- Hetzner: `/opt/digimon-search/ex13-evaluation-program-20261029/host-queue.lock`

Run the queue controller **on the host**, with `psutil` available, so its process, CPU and memory inventory includes unrelated jobs and other containers. Do not run it in an isolated container. The host lock permits one new-program queue per host at a time. Before every panel, live CPU, available RAM, free artifact disk, recognized native workers and build processes must pass admission. It preserves 50% headroom over the sampled worker RSS plus a memory reserve. Waiting does not consume a panel attempt.

Windows invokes the frozen panel controller with hidden child processes. Linux invokes a retained Docker container with the immutable evaluation-image digest, network disabled, explicit CPU/memory caps and only the panel root mounted at `/campaign`. Containers and evidence remain after completion. Never use `--rm` or restart a scored container.

Each panel gets a durable claim under the shared lock directory before its first invocation. Any controller failure, nonzero/OOM container terminal state or failed frozen completion verification stops the queue. There is no queue resume or native retry path. A completed panel's own frozen verifier must succeed before the next panel starts. Reports and human-facing result markers remain separate from execution completion.

The implementation tests cover capacity waits, host-lock exclusion, durable duplicate-panel claims, no resume after failure, verifier failures, OOM failures, network-disabled retained containers, frozen-copy tampering, disk admission and standalone imports. Actual Windows/Linux qualification, host allocation, resource profiles and real scored artifacts remain separate gates.

## Frozen global assignment

`runs/ex13-closed-host-allocation-20261030/assignment.json` assigns the CP12-versus-CP8 A/B pair to Hetzner and the other 20 panels to local D:. The assignment covers all 11,200 games without duplicate host ownership and retains matched control hosts. It does not admit scoring. The measured estimate is 1 GiB of result growth per panel (twice the completed Linux qualification bytes/game, extrapolated to 520 games and rounded up), plus separate staging/report budgets and 2 GiB disk reserve. This estimate is not a hard upper bound; fresh queue admission remains required. No artifacts may be deleted automatically.

Before launching EACH prepared queue, run `python -m tools.cloud_search.frozen_mode_allocation --root runs/ex13-closed-host-allocation-20261030 --schedule runs/ex13-closed-scored-schedule-20261030 --host local|hetzner --queue PREPARED_QUEUE` and preserve its output. It checks the immutable global assignment and the queue's panel subset, platform, schedule, shared host lock and artifact budget. Run the queue's own frozen validation as well. Same-host durable claims prevent repeating a panel across batches. These checks passed for the original queues; a future queue would require fresh proofs and explicit failed-queue continuation policy resolution.

Hetzner host Python uses the private dependency directory `/opt/digimon-search/ex13-evaluation-program-20261029/host-runtime` on `PYTHONPATH`. Its pinned psutil 7.0.0 wheel and installed files are hash-recorded in the allocation preparation. No system Python packages were changed. The dependency is for host resource inspection only; model containers retain their frozen runtime and disabled network.

## Cross-platform deployment

Queue preparation validates the shared lock using the destination OS's path rules. A Linux queue prepared on Windows must retain `/opt/digimon-search/ex13-evaluation-program-20261029/host-queue.lock`; it must not gain a Windows drive or backslashes. Windows queues retain `D:/wt/p3-constructs/runs/ex13-evaluation-program-20261029/host-queue.lock`.

For a prepared, unattempted Linux queue, use `python -m tools.cloud_search.frozen_mode_queue archive --root QUEUE --out NEW_ARCHIVE.tar.gz`. The archive must be outside the queue root. This checks every frozen panel, refuses attempted queues/panels and symlinks, preserves empty queue and panel output directories, and explicitly assigns mode 0755 to each native executable. It returns both the archive SHA256 and queue-manifest SHA256; verify the archive hash before extraction on Hetzner and run the extracted frozen `host_controller.py validate` before launch. Do not use ad hoc Windows tar packaging. Containers and deployment artifacts remain retained.


## Receipt progress monitoring

Use `python -m tools.cloud_search.monitor_frozen_receipts --root PANEL --out NEW_RECORD --previous PREVIOUS_RECORD --process-evidence FRESH_PROCESS_SNAPSHOT`. Omit previous on the first observation. It checks new committed request/output hashes and unchanged existing receipt hashes, records rejection identities, and uses committed timestamps rather than controller heartbeat timestamps. After 30 minutes without commits it retains at most two 2 KiB stderr tails, while preserving the declared request deadline. Missing or stale process evidence never proves a crash. Linux container paths require separately matched host/container evidence; do not treat an unmapped process as absent. This monitor never retries, restarts, scores, or replaces the required frozen final verifier. Previously reported components are skipped.

Container evidence may include `containers[]` with `id`, `running`, `mounts[]` (`type=bind`, exact absolute host `source`, absolute container `destination`) and `processes[]` (`command` token arrays). Collect these from fresh Docker inspection and process evidence on the host. Only the native executable or controller at that exact destination counts; another panel mount, stopped container or unmapped command does not prove liveness. Remote snapshots must be evaluated with their host panel root, not a differently located local mirror. Ten focused monitor tests cover these boundaries; the original scored-container failure and live observations are preserved in the program evidence.

## Failed-queue continuation decision

The draft in `runs/ex13-failed-queue-continuation-policy-prep-20261030/proposal.md` partitions the frozen schedule into two permanently failed panels and twenty unattempted panels (10,160 games). It is not launch authorization. New candidate qualification, exact parity and measured profiles must pass before requesting the explicit continuation decision. No new queue may bypass this gate.

If later authorized, only four complete A/B pairs could support paired mode effects. The two B controls whose A counterparts failed cannot use partial or replacement A results. Stage A remains incomplete, even if all twenty unattempted panels succeed. Old queue identities, durable claims, failures and original prepared copies remain unchanged.


## Explicitly authorized local continuation

2026-10-30T12:02:56.774879+00:00: Human explicitly authorized local retry of OOM panel and continuation. New frozen local queue ex13-scored-queue-local-authorized-v2-20261030 launched once;21panels10680games,firsta-cp12-vs-cp8 one worker,others3. Live host64GiBtotal/~34GiBavailable;8GiBadmissionreserve. New globalassignment movesCP12-CP8A/Blocal for matched-runtimecomparison;allnewpanelslocal. Originalfailures/queues/claims retained;onlyOOMpanel authorized one freshrepeat,continuation-safetyfailedA16-12excluded. FivepotentialA/Bpairs,oneBstandalone. Newqueuefailstop/noautomaticretry.21/35tasksunchanged.

The earlier20-panel draft is historical. The new authorization adds one fresh local attempt of the OOM panel and moves its matched control locally. It does not erase the historical failure or authorize another automatic retry. The original global assignment remains preserved; the new assignment is frozen in runs/ex13-approved-local-continuation-20261030.


## Superseding iterative-repair authorization

The human instruction “Keep retrying the closed deck stuff and fixing the errors” authorizes fresh recorded retries of both failed closed panels and continuation of the existing finite program after concrete fixes and fresh qualification. It supersedes the preceding one-retry limit, permanent exclusion and additional continuation-approval requirement. It does not authorize resuming old controllers, clearing claims, merging partial results or weakening audits. Each fresh campaign binds failed-attempt lineage, qualified source/runtime, new global assignment and fresh capacity; prevent duplicate active panel execution. Repeated schedules remain correlated. Keep matched A/B comparisons on one qualified runtime/host. No training or promotion.
