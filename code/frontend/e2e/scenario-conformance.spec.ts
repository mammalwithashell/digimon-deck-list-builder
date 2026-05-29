import { test, expect } from '@playwright/test';
import {
  loadScenario,
  stageScenario,
  type ScenarioFixture,
} from './fixtures/debug-game';

/**
 * Scenario-conformance e2e (add-ui-scenario-test-substrate, group 7/8).
 *
 * Drives the shared `qa/scenarios/*.json` corpus through the exact path the
 * UI fixture uses: load the fixture → stage it via the Rust-backed
 * `/debug/games` router → evaluate the fixture's `engine` assertions
 * server-side. This is the Playwright half of the two-layer harness (the
 * Rust `cargo test --test scenario_corpus` is the other half); both consume
 * the same fixture files, so engine-correctness has one source of truth.
 *
 * These tests are API-driven (Playwright `page.request`) and need only the
 * FastAPI server up — no frontend. DOM-level UI assertions (DNA option
 * present, target highlighting) live in the game-flow specs that navigate
 * the rendered board.
 */

// Fixtures to run through the engine layer. `expected_pass` must pass;
// `blocked_on_card_impl` is reported but not asserted.
const CORPUS = ['q16-paildramon-staging', 'q16-partition-self-deletion', 'dna-paildramon-hand'];

test.describe('Scenario corpus — engine assertions', () => {
  for (const name of CORPUS) {
    test(`fixture: ${name}`, async ({ page }) => {
      const fixture: ScenarioFixture = loadScenario(name);
      const game = await stageScenario(page, fixture);
      try {
        const engineAssertions = fixture.assertions.engine ?? [];
        const result = await game.evaluate(engineAssertions);

        // Surface per-assertion detail on failure.
        const failed = result.results.filter((r) => !r.passed);
        const detail = failed.map((r) => `${r.kind}: ${r.message}`).join('\n');

        if (fixture.readiness === 'blocked_on_card_impl') {
          test.info().annotations.push({
            type: 'pending',
            description: `${name} is blocked_on_card_impl (${failed.length} unmet)`,
          });
          return;
        }

        expect(result.all_passed, `engine assertions failed:\n${detail}`).toBe(true);
      } finally {
        await game.delete();
      }
    });
  }
});
