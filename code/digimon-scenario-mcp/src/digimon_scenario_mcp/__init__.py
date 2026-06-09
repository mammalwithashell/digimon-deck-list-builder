"""digimon-scenario-mcp: WRITE-capable dev/test MCP for staging, snapshotting,
and authoring Digimon TCG game-state scenario tests.

Unlike the read-only operator MCPs (`digimon-engine-mcp`,
`digimon-training-mcp`), this server mutates game state and writes files
(`qa/scenarios/`, `code/frontend/e2e/`). It is dev/test-only: never bundled
into any production build, and it talks only to dev-gated surfaces (the
hosted-API `/debug` router in browser-dev mode, and the feature-gated Tauri
debug bridge on desktop).
"""

__version__ = "0.1.0"
