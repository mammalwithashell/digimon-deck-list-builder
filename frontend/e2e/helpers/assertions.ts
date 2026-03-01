import { expect, type Page } from '@playwright/test';

const API_BASE = process.env.API_BASE ?? 'http://localhost:8000';

export async function assertMemory(page: Page, gameId: string, expected: number) {
  const resp = await page.request.get(`${API_BASE}/debug/games/${gameId}/internal-state`);
  const state = await resp.json();
  expect(state.memory, `Expected memory=${expected}, got ${state.memory}`).toBe(expected);
}

export async function assertPhase(page: Page, gameId: string, expectedPhase: string) {
  const resp = await page.request.get(`${API_BASE}/debug/games/${gameId}/internal-state`);
  const state = await resp.json();
  expect(state.phase, `Expected phase=${expectedPhase}, got ${state.phase}`).toBe(expectedPhase);
}

export async function assertCardInZone(
  page: Page,
  gameId: string,
  playerId: 1 | 2,
  cardId: string,
  zone: 'hand' | 'trash' | 'security_cards',
) {
  const resp = await page.request.get(`${API_BASE}/debug/games/${gameId}/internal-state`);
  const state = await resp.json();
  const key = `${zone}_p${playerId}`;
  expect(state[key], `Expected ${cardId} in ${key}`).toContain(cardId);
}

export async function getInternalState(page: Page, gameId: string) {
  const resp = await page.request.get(`${API_BASE}/debug/games/${gameId}/internal-state`);
  return resp.json();
}
