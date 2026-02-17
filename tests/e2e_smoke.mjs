/**
 * E2E smoke test: register, login, import deck, save, start game vs greedy agent.
 * Run: node tests/e2e_smoke.mjs
 */
import { chromium } from 'playwright';

const BASE = process.env.BASE_URL || 'http://localhost:5174';
const DECK_JSON = `["Exported from https://digimoncard.io","BT23-005","BT23-005","BT23-005","BT21-008","BT21-008","BT21-008","BT21-008","BT24-008","BT24-008","BT24-008","BT21-017","BT21-017","BT21-017","BT21-017","BT24-012","BT24-012","BT24-012","P-189","P-189","BT24-016","BT24-016","BT24-016","BT24-016","BT21-025","BT24-017","BT24-017","BT24-017","BT24-017","BT24-018","BT24-018","BT24-018","P-103","P-103","P-103","P-103","LM-027","LM-027","BT24-089","BT24-089","BT24-089","BT8-097","BT8-097","BT21-093","BT21-081","BT21-081","BT24-082","BT24-082","BT24-082","BT18-087","BT18-087","BT21-001","BT21-001","BT21-001","BT21-001"]`;

const USERNAME = `testuser_${Date.now()}`;
const EMAIL = `${USERNAME}@test.com`;
const PASSWORD = 'TestPass123!';

async function run() {
  const browser = await chromium.launch({ headless: false, slowMo: 200 });
  const page = await browser.newPage();

  // Capture console errors
  page.on('console', msg => {
    if (msg.type() === 'error') {
      console.log(`  [BROWSER ERROR] ${msg.text()}`);
    }
  });
  page.on('pageerror', err => {
    console.log(`  [PAGE ERROR] ${err.message}`);
  });
  // Capture failed network requests
  page.on('response', response => {
    if (response.status() >= 400) {
      console.log(`  [HTTP ${response.status()}] ${response.url()}`);
    }
  });

  try {
    // ─── 1. Register ───────────────────────────────────────────────
    console.log('→ Step 1: Register');
    await page.goto(`${BASE}/register`);
    await page.waitForLoadState('networkidle');

    await page.fill('#reg-username', USERNAME);
    await page.fill('#reg-email', EMAIL);
    await page.fill('#reg-password', PASSWORD);

    await page.click('button[type="submit"]');
    await page.waitForURL(url => !url.toString().includes('/register'), { timeout: 10000 });
    console.log(`  ✓ Registered as ${USERNAME}, redirected to: ${page.url()}`);

    // ─── 2. Log in ─────────────────────────────────────────────────
    console.log('→ Step 2: Log in');
    if (!page.url().includes('/login')) {
      await page.goto(`${BASE}/login`);
    }
    await page.waitForLoadState('networkidle');

    await page.fill('#username', USERNAME);
    await page.fill('#password', PASSWORD);
    await page.click('button[type="submit"]');

    await page.waitForURL(url => !url.toString().includes('/login'), { timeout: 10000 });
    console.log(`  ✓ Logged in, redirected to: ${page.url()}`);

    // ─── 3. Navigate to Deck Builder ───────────────────────────────
    console.log('→ Step 3: Navigate to Deck Builder');
    await page.goto(`${BASE}/deckbuilder`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    console.log(`  ✓ On: ${page.url()}`);

    // ─── 4. Import deck ────────────────────────────────────────────
    console.log('→ Step 4: Import deck');
    const importBtn = page.locator('button', { hasText: /import/i });
    await importBtn.first().click();
    await page.waitForTimeout(500);

    const textarea = page.locator('textarea');
    await textarea.fill(DECK_JSON);

    const modalImportBtn = page.locator('button', { hasText: /^Import$/ });
    await modalImportBtn.click();

    // Wait for import (fetches card data from digimoncard.io)
    await page.waitForTimeout(8000);
    console.log('  ✓ Deck import completed');

    // Close modal if warnings shown
    const doneBtn = page.locator('button', { hasText: /^Done$/ });
    if (await doneBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await doneBtn.click();
      await page.waitForTimeout(500);
    }

    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);

    // ─── 5. Name and save deck ─────────────────────────────────────
    console.log('→ Step 5: Save deck');
    const nameInput = page.locator('input[type="text"]').first();
    await nameInput.click({ clickCount: 3 });
    await nameInput.fill('Medusa Test Deck');
    await page.waitForTimeout(300);

    const saveBtn = page.locator('button', { hasText: /^Save$/ });
    await saveBtn.click();
    await page.waitForTimeout(2000);
    console.log('  ✓ Deck saved');

    // ─── 6. Start game ─────────────────────────────────────────────
    console.log('→ Step 6: Start game');
    await page.goto(`${BASE}/game`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Select deck
    const deckSelect = page.locator('select').first();
    const options = await deckSelect.locator('option').allTextContents();
    console.log('  Available decks:', options);

    const deckOptions = deckSelect.locator('option:not([value=""])');
    const deckCount = await deckOptions.count();
    if (deckCount > 0) {
      const val = await deckOptions.first().getAttribute('value');
      await deckSelect.selectOption(val);
      console.log('  ✓ Selected deck');
    } else {
      throw new Error('No decks available in dropdown');
    }

    // Greedy agent
    const agentSelect = page.locator('select').nth(1);
    await agentSelect.selectOption('greedy');

    // Start
    console.log('  Clicking Start Game...');
    const startBtn = page.locator('button', { hasText: /start game/i });
    await startBtn.click();

    // Wait and watch for errors
    await page.waitForTimeout(10000);

    await page.screenshot({ path: 'tests/e2e_game_board.png', fullPage: true });
    console.log('  ✓ Screenshot: tests/e2e_game_board.png');

    const body = await page.textContent('body');
    if (body.includes('Turn') || body.includes('Phase') || body.includes('Memory') || body.includes('Pass') || body.includes('Breeding')) {
      console.log('  ✓ Game board is visible!');
    } else if (body.includes('Start a Game') || body.includes('Starting')) {
      console.log('  ✗ Still on Start Game screen — game creation likely failed');
    } else {
      console.log('  ⚠ Unknown state — check screenshot');
    }

    console.log('\n✅ E2E smoke test completed');

  } catch (err) {
    console.error('\n❌ E2E smoke test FAILED:', err.message);
    await page.screenshot({ path: 'tests/e2e_failure.png', fullPage: true });
    console.log('  Screenshot saved to tests/e2e_failure.png');
  } finally {
    await page.waitForTimeout(5000);
    await browser.close();
  }
}

run();
