# model-distribution-tiers

## ADDED Requirements

### Requirement: Every model has a distribution tier
Each model record SHALL carry a `distribution` of either `download` or `stream`. A newly uploaded model SHALL be `stream` unless an administrator explicitly promotes it.

#### Scenario: New upload defaults to stream
- **WHEN** an administrator uploads a new model and confirms the upload
- **THEN** the model's `distribution` is `stream` and no publicly readable copy of its file exists

#### Scenario: Tier is reported on the model record
- **WHEN** an administrator lists or fetches models through the admin API
- **THEN** each record includes its `distribution`

### Requirement: Stream model files are never publicly readable
A `stream` model's file SHALL be stored only in private storage that has no CDN and serves no unauthenticated reads. No API response SHALL include a URL, presigned URL, or file checksum for a `stream` model.

#### Scenario: Upload goes to private storage
- **WHEN** the server issues an upload URL for a model
- **THEN** the URL targets the private bucket and does not grant public read access

#### Scenario: Direct fetch of a stream model's storage path fails
- **WHEN** an unauthenticated client requests the CDN or origin URL a `stream` model would have if it were public
- **THEN** the request does not return the file

#### Scenario: No endpoint leaks a stream model's location
- **WHEN** any public or user-authenticated endpoint returns information about a `stream` model
- **THEN** the response contains no file URL and no `file_sha256`

### Requirement: Manifest lists both tiers with tier-appropriate fields
`GET /models/manifest.json` SHALL list every published, contract-compatible model with its `distribution` and its contract metadata. `download` entries SHALL include `url`, `file_sha256` and `file_size_bytes`; `stream` entries SHALL omit all three. For a request that does not declare the `model-tiers` client capability, the manifest SHALL omit `stream` entries.

#### Scenario: Tier-aware client sees both tiers
- **WHEN** a client requests the manifest declaring the `model-tiers` capability
- **THEN** the response contains download entries with `url`/`file_sha256`/`file_size_bytes` and stream entries with `distribution: "stream"` and no file fields

#### Scenario: Legacy client is not broken by stream entries
- **WHEN** a client requests the manifest without declaring the `model-tiers` capability
- **THEN** the response contains only `download` entries, each with a `url`

### Requirement: Promotion publishes a copy; demotion removes it
Promoting a model from `stream` to `download` SHALL copy its file into public storage and record the public location. Demoting from `download` to `stream` SHALL delete the public copy and purge it from the CDN. Both SHALL be recorded in the audit log in the same transaction as the tier change.

#### Scenario: Promote a stream model
- **WHEN** an administrator promotes a `stream` model and confirms
- **THEN** a public copy exists, the manifest lists the model with a `url` whose content hashes to `file_sha256`, and an audit entry records the promotion

#### Scenario: Demote a download model
- **WHEN** an administrator demotes a `download` model and confirms
- **THEN** the public copy is deleted, a CDN purge is requested for its path, the manifest entry no longer carries a `url`, and an audit entry records the demotion

#### Scenario: Failed copy leaves the model unchanged
- **WHEN** the public copy fails during promotion
- **THEN** the model remains `stream`, no audit entry claims a promotion, and the administrator sees the error

### Requirement: Server-side play works for both tiers
The server SHALL be able to load and run any published model regardless of its tier, reading the canonical copy from private storage.

#### Scenario: Server-side game uses a stream model
- **WHEN** a hosted-API game is created against a `stream` model
- **THEN** the server resolves the model from private storage and the game runs

### Requirement: Existing models migrate to stream safely
The migration SHALL mark every existing model `stream` and copy its file into private storage (verifying the checksum) without deleting any public copy. Public copies of models still `stream` SHALL be removed only by a separate, explicit step.

#### Scenario: Migration preserves public copies until told otherwise
- **WHEN** the migration runs its default step
- **THEN** every model is `stream`, every file exists in private storage with a matching checksum, and existing public copies still exist

#### Scenario: Explicit cleanup removes public copies of stream models
- **WHEN** the migration's public-copy cleanup step runs after the owner has promoted some models
- **THEN** public copies of models still `stream` are deleted and purged, and promoted models keep theirs

#### Scenario: Migration is idempotent
- **WHEN** the migration is run a second time
- **THEN** no files are duplicated or altered and no model changes tier
