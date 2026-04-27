import { describe, expect, it } from 'vitest';
import type { ActionTrace, TensorSummary } from '@/types/game';

describe('rust trace types', () => {
  it('supports agent traces with tensor summaries', () => {
    const summary: TensorSummary = {
      playerId: 1,
      tensorSize: 1375,
      maskSize: 2168,
      legalActionCount: 4,
      turnCount: 3,
      phase: 'Main',
      memory: 2,
      tensorHead: [0.1, 3, 0.2],
    };
    const trace: ActionTrace = {
      actor: 'agent_trained',
      playerId: 1,
      actionId: 62,
      decoded: {
        actionId: 62,
        playerId: 1,
        phase: 'Main',
        kind: 'pass',
        label: 'Pass / decline',
        sourceZone: null,
        sourceIndex: null,
        targetZone: null,
        targetIndex: null,
        cardId: null,
        cardName: null,
      },
      tensorSummary: summary,
    };

    expect(trace.tensorSummary?.tensorSize).toBe(1375);
    expect(trace.tensorSummary?.maskSize).toBe(2168);
    expect(trace.decoded.label).toBe('Pass / decline');
  });
});
