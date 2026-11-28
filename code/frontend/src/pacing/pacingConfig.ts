import type { BotSpeed, Motion } from '@/stores/uiStore';

/**
 * Central timing table for the all-mode client-side presentation queue.
 *
 * Modeled on Aegis's stacked presentation pacing (`pacing.ts`):
 * - Observable opponent plays (brief showcase hold so card art/name register)
 * - Readable effect source focus + scrollable recent clause toasts
 * - Chain acceleration after effect #2 (40% beat length) so long combos stay
 *   snappy rather than glacial
 * - Respects user Speed (`normal` | `fast` | `instant`) and `Motion` settings
 */
export interface PacingDurations {
  turnStartMs: number;
  phaseTickMs: number;
  hatchOrMoveMs: number;
  ownPlayMs: number;
  oppPlayShowcaseMs: number;
  ownDigivolveMs: number;
  oppDigivolveMs: number;
  attackDeclareMs: number;
  securityRevealMs: number;
  effectSourceMs: number;
  revealCardsMs: number;
  cardMoveMs: number;
  settleGapMs: number;
  toastLifetimeMs: number;
  chainTailFrom: number;
  chainTailScale: number;
}

export const BASE_PACING: PacingDurations = {
  turnStartMs: 760,
  phaseTickMs: 280,
  hatchOrMoveMs: 620,
  ownPlayMs: 300,
  oppPlayShowcaseMs: 1150,
  ownDigivolveMs: 380,
  oppDigivolveMs: 1050,
  attackDeclareMs: 1150,
  securityRevealMs: 1450,
  effectSourceMs: 1050,
  revealCardsMs: 780,
  cardMoveMs: 480,
  settleGapMs: 200,
  toastLifetimeMs: 7200,
  chainTailFrom: 3,
  chainTailScale: 0.72,
};

export function speedScale(speed: BotSpeed, motion: Motion): number {
  if (motion === 'off' || speed === 'instant') return 0;
  const motionFactor = motion === 'reduced' ? 0.65 : 1.0;
  switch (speed) {
    case 'fast':
      return 0.55 * motionFactor;
    case 'slow':
      return 1.25 * motionFactor;
    case 'normal':
    default:
      return 1.0 * motionFactor;
  }
}

export function scaleDuration(
  baseMs: number,
  speed: BotSpeed,
  motion: Motion,
  chainIndex = 0,
): number {
  const factor = speedScale(speed, motion);
  if (factor === 0) return 0;
  const tailFactor =
    chainIndex >= BASE_PACING.chainTailFrom ? BASE_PACING.chainTailScale : 1.0;
  return Math.max(60, Math.round(baseMs * factor * tailFactor));
}
