# guest-session

## ADDED Requirements

### Requirement: Guest identity without accounts
The client SHALL obtain an anonymous guest identity from `POST /auth/guest` and use its bearer token for all authenticated calls (lobby, WebSocket games, matchmaking) without requiring registration or login at any point in the PvP flow.

#### Scenario: First launch mints a guest
- **WHEN** the app launches with no cached guest session and the hosted API is reachable
- **THEN** the client mints a guest via `POST /auth/guest`, persists the token/user-id/display-name, and subsequent authenticated calls succeed with that bearer token

#### Scenario: Cached guest is reused across launches
- **WHEN** the app launches with a previously persisted real guest session
- **THEN** the cached identity is reused without minting a new guest

### Requirement: Offline sentinel is never persisted
When the guest mint fails (API unreachable or non-2xx), the client SHALL fall back to an in-memory offline session for that launch only and SHALL NOT write the offline sentinel to persistent storage, so the next launch retries a real mint.

#### Scenario: Mint failure does not poison future launches
- **WHEN** the first-ever mint attempt fails and the app is relaunched with the API reachable
- **THEN** the relaunch mints a real guest session (the offline fallback left no persisted token)

#### Scenario: Legacy poisoned token is migrated away
- **WHEN** the app launches with the legacy literal `offline-guest-token` in persistent storage
- **THEN** the client treats the cached session as absent and mints a real guest

### Requirement: Guest 401 recovery by re-mint
When an authenticated request fails with 401 and the stored token is a guest token (`is_guest` claim), the client SHALL discard the cached guest session, mint a fresh guest, and retry the failed request exactly once. Registered-user 401 handling (refresh-token flow) is unchanged.

#### Scenario: Stale guest token recovers transparently
- **WHEN** a lobby request returns 401 because the cached guest token is no longer valid (e.g. server DB reset)
- **THEN** the client mints a new guest identity and the retried request succeeds without user intervention

#### Scenario: Re-mint is attempted at most once per request
- **WHEN** the retried request after a re-mint also returns 401
- **THEN** the client surfaces the error instead of looping on mint attempts
