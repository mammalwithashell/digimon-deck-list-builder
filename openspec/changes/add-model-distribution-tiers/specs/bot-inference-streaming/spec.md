# bot-inference-streaming

## ADDED Requirements

### Requirement: Move endpoint for published models
The server SHALL expose `POST /bots/{model_id}/act` that accepts an observation, a legal-action mask and, for recurrent models, the recurrent state, and returns the chosen action and the next recurrent state. It SHALL serve any published model of either tier, and SHALL return 404 for unknown or unpublished models.

#### Scenario: MLP model returns an action
- **WHEN** an authenticated client posts a valid observation and mask to a published MLP model
- **THEN** the response is 200 with an `action` that is legal under the mask

#### Scenario: LSTM model round-trips its state
- **WHEN** an authenticated client posts an observation, mask and null state to a published LSTM model, then posts the next observation with the returned `h` and `c`
- **THEN** both responses are 200, each returns `h` and `c` of the model's hidden size, and the second call's action reflects the state passed in

#### Scenario: Unpublished model is refused
- **WHEN** a client posts to an unpublished or unknown model id
- **THEN** the response is 404

### Requirement: Server keeps no per-game state
The endpoint SHALL be stateless with respect to games: everything needed to choose the next action SHALL arrive in the request. Any API instance SHALL be able to serve any call.

#### Scenario: Calls survive a server restart mid-game
- **WHEN** the API restarts between two calls of the same game and the client sends the state it last received
- **THEN** the second call succeeds and returns the same action it would have without the restart

### Requirement: Streamed moves match local moves
For the same model, observation, mask and recurrent state, the endpoint SHALL return the same action and next state as local inference through the Rust engine's policy, by using that same policy implementation through the engine bindings.

#### Scenario: Parity with local inference
- **WHEN** a recorded sequence of observations and masks is run through local inference and through the endpoint for the same model
- **THEN** every action is identical and the recurrent states match within floating-point tolerance

### Requirement: Contract check
Each request SHALL declare the client's observation and action contract (`tensor_layout_hash`, `action_schema_version`, `tensor_size`, `action_space_size`). The endpoint SHALL return 409 when it does not match the model's recorded contract, and 422 when the observation or mask length does not match.

#### Scenario: Incompatible client is refused clearly
- **WHEN** a client whose `tensor_layout_hash` differs from the model's posts a move request
- **THEN** the response is 409 with a body naming the mismatched fields, and no inference runs

#### Scenario: Wrong observation length
- **WHEN** a request's observation length differs from the declared `tensor_size`
- **THEN** the response is 422

### Requirement: Authentication and per-account rate limits
The endpoint SHALL require an authenticated user (guest or registered). It SHALL require a registered account for models flagged `requires_account`. It SHALL enforce a per-user, per-model limit (the model's `rate_limit_per_minute`) and a global per-user cap across models, returning 429 with `Retry-After` when exceeded.

#### Scenario: Anonymous request is refused
- **WHEN** a request without a valid bearer token is posted
- **THEN** the response is 401

#### Scenario: Guest refused on an account-only model
- **WHEN** a guest token posts to a model flagged `requires_account`
- **THEN** the response is 403

#### Scenario: Rate limit enforced per user and model
- **WHEN** one user exceeds a model's per-minute limit
- **THEN** further calls from that user to that model return 429 with `Retry-After`, while other users and that user's calls to other models are unaffected until the global cap

### Requirement: Clients choose local or streamed inference by tier
A client that runs games locally SHALL use local inference for a `download` model whose cached file matches the manifest checksum and contract, SHALL download then run locally when a `download` model is not yet cached, SHALL use the move endpoint for a `stream` model, and SHALL show the model as unavailable with a reason when neither is possible (offline for a stream model, contract mismatch, account required).

#### Scenario: Desktop plays a stream model
- **WHEN** a desktop player starts a game against a `stream` model while online
- **THEN** the bot's decisions come from the move endpoint and the game plays normally

#### Scenario: Desktop offline with a stream model
- **WHEN** a desktop player selects a `stream` model while offline
- **THEN** the model is shown as unavailable offline and the player is offered a download model instead of a failed game

#### Scenario: Rate limit during a game
- **WHEN** the move endpoint returns 429 during a game
- **THEN** the bot pauses for the `Retry-After` period with a visible notice and resumes, instead of the game failing

#### Scenario: Stream models are labelled
- **WHEN** a player opens the model picker
- **THEN** stream models are visibly marked as online-only
