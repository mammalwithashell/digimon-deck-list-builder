# model-admin-dashboard

## ADDED Requirements

### Requirement: Catalog view with tiers
The admin model page SHALL list every model with its name, architecture, tier (`download` / `stream`), published and deprecated state, contract compatibility with the current engine, upload date, and the last 7 days of move-endpoint calls. It SHALL be filterable by tier, published state and compatibility. It SHALL be visible only to administrators.

#### Scenario: Owner sees which models are protected
- **WHEN** an administrator opens the model page
- **THEN** each model shows its tier with a distinct visual treatment, and the list can be filtered to `stream` or `download` only

#### Scenario: Non-admin is refused
- **WHEN** a non-admin user requests the model page or its admin API
- **THEN** access is denied

### Requirement: Promotion requires deliberate confirmation
Promoting a model to `download` SHALL require a confirmation step that states the file will become publicly downloadable and cannot be recalled once downloaded, and that requires the administrator to type the model's name. Demotion SHALL require a confirmation that states previously downloaded copies remain with whoever downloaded them.

#### Scenario: Promotion blocked until confirmed
- **WHEN** an administrator starts a promotion but has not typed the model name exactly
- **THEN** the confirm control stays disabled and no change is made

#### Scenario: Promotion completes after confirmation
- **WHEN** the administrator types the model name and confirms
- **THEN** the model becomes `download`, the page shows its new tier, and the audit log gains a promotion entry naming the administrator

#### Scenario: Demotion warns about existing copies
- **WHEN** an administrator starts a demotion
- **THEN** the dialog states that copies already downloaded cannot be recalled before the administrator confirms

### Requirement: Visibility and lifecycle controls
The page SHALL let an administrator publish and unpublish a model, deprecate and undeprecate it, toggle `requires_account`, and set the per-model rate limit. Each change SHALL be written to the audit log.

#### Scenario: Unpublish hides a model everywhere
- **WHEN** an administrator unpublishes a model
- **THEN** it disappears from the manifest and the move endpoint returns 404 for it, and the change is audited

#### Scenario: Rate limit override takes effect
- **WHEN** an administrator changes a model's rate limit
- **THEN** subsequent move-endpoint calls are limited at the new rate and the old and new values are audited

### Requirement: Usage and rate-limit statistics
For each model, the page SHALL show daily move-endpoint calls, distinct users, rate-limited calls, and average and maximum latency for at least the last 30 days. For `download` models it SHALL show manifest downloads where the storage backend reports them, or state that downloads are not tracked.

#### Scenario: Owner reviews a stream model's load
- **WHEN** an administrator opens a model's detail view
- **THEN** a 30-day daily series of calls, distinct users, rate-limited calls and latency is shown

#### Scenario: Statistics survive restarts
- **WHEN** the API restarts
- **THEN** previously flushed daily statistics are still shown, with at most the last minute of counts lost

### Requirement: Audit log
The page SHALL show an audit log, per model and across all models, of uploads, tier changes, publish/unpublish, deprecate/undeprecate, `requires_account` changes and rate-limit changes, each with time, administrator, previous value and new value. Entries SHALL NOT be editable or deletable from the UI or API.

#### Scenario: Every tier change is attributable
- **WHEN** an administrator views a model's history
- **THEN** every tier change since the model was uploaded is listed with who made it and when

#### Scenario: Audit entries are append-only
- **WHEN** any client attempts to modify or delete an audit entry through the API
- **THEN** no such endpoint exists and the entry is unchanged
