## ADDED Requirements

### Requirement: Pods are selected by effective CPU cores, not host nproc
The provisioning layer SHALL determine a pod's usable CPU from the container's `cpu.max` cgroup quota (effective cores = quota ÷ period), MUST NOT treat `nproc` as the usable core count, and SHALL reject or re-roll any pod whose effective cores fall below a configurable floor.

#### Scenario: Throttled pod is rejected despite high nproc
- **WHEN** a provisioned pod reports `nproc=96` but `cat /sys/fs/cgroup/cpu.max` yields `765000 100000` (≈7.65 effective cores) and the floor is 16
- **THEN** the provisioner records the effective-core count, rejects the pod as under-resourced, and provisions a replacement (or surfaces the shortfall to the operator)

#### Scenario: vCPU pre-filter then in-container verify
- **WHEN** selecting a candidate pod
- **THEN** the provisioner MAY use `runpodctl vcpuCount` as a pre-filter (it reflects the real quota) but SHALL verify `cpu.max` inside the running container before admitting the pod for training

### Requirement: Provisioning surfaces CPU throttling and memory headroom
The provisioning layer SHALL report the container's CPU throttling (`cpu.stat` `nr_throttled` / `throttled_usec`) and its memory cgroup limit and headroom so throughput numbers are never compared across differently-resourced pods unknowingly.

#### Scenario: Throttling is visible before a run is trusted
- **WHEN** a pod has accumulated significant `throttled_usec` or its `memory.max` is far below the host RAM
- **THEN** the provisioner logs the effective CPU quota and memory cap alongside any reported throughput, so a slow run is attributable to the quota rather than the code

### Requirement: Avoid the A40-with-volume provisioning hang; harvest before terminate
The provisioning layer SHALL default to container-disk-only pods (no attached `--volume-in-gb`) because attaching a volume to A40-secure pods hangs provisioning, and therefore SHALL treat pods as ephemeral: every artifact (`final.zip`, its `.meta.json`, registry deltas) MUST be copied to durable storage before the pod terminates.

#### Scenario: No-volume pod with auto-terminate guard
- **WHEN** a training pod is created
- **THEN** it is created without a network volume, with an `--terminate-after` guard to cap idle billing, and the launcher records the deadline

#### Scenario: Artifacts secured before teardown
- **WHEN** a pod's training completes (or the terminate deadline approaches)
- **THEN** the launcher downloads the pod's artifacts to durable storage and verifies them locally BEFORE deleting the pod
