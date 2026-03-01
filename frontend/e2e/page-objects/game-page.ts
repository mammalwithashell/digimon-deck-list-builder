import { type Page, type Locator, expect } from '@playwright/test';

export class GamePageObject {
  readonly page: Page;
  readonly board: Locator;
  readonly phaseIndicator: Locator;
  readonly turnCount: Locator;
  readonly actionBar: Locator;

  constructor(page: Page) {
    this.page = page;
    this.board = page.getByTestId('game-board');
    this.phaseIndicator = page.getByTestId('phase-indicator');
    this.turnCount = page.getByTestId('turn-count');
    this.actionBar = page.getByTestId('action-bar');
  }

  async navigateToGame(gameId: string) {
    await this.page.goto(`/game/${gameId}`);
    await this.board.waitFor({ state: 'visible', timeout: 15_000 });
  }

  async getTurnCount(): Promise<string> {
    return (await this.turnCount.textContent()) ?? '';
  }

  async getPhase(): Promise<string> {
    const el = this.page.getByTestId('current-phase');
    return (await el.textContent()) ?? '';
  }

  fieldSlot(player: 1 | 2, index: number): Locator {
    return this.page.getByTestId(`field-slot-p${player}-${index}`);
  }

  async assertBoardVisible() {
    await expect(this.board).toBeVisible();
  }

  async assertPhaseIndicatorVisible() {
    await expect(this.phaseIndicator).toBeVisible();
  }
}
