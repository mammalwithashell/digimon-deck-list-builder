import { describe, expect, it } from 'vitest';
import { RESOLUTION_PRESETS, DESIGN_CANVAS } from '@/utils/constants';
import { __canvasScalerInternals } from './CanvasScaler';

const { computeScale } = __canvasScalerInternals;

/** Helper to set window inner dimensions for the next computeScale call. */
function setWindowSize(w: number, h: number) {
  Object.defineProperty(window, 'innerWidth', { value: w, configurable: true });
  Object.defineProperty(window, 'innerHeight', { value: h, configurable: true });
}

describe('CanvasScaler.computeScale', () => {
  it.each(RESOLUTION_PRESETS)(
    'computes scale for the %s × %s preset',
    (preset) => {
      // Preset typings: each entry is { width, height } — vitest passes the
      // object as a single param when iterating a typed array.
      const { width, height } = preset;
      setWindowSize(width, height);
      const scale = computeScale();
      const expected = Math.min(
        width / DESIGN_CANVAS.width,
        height / DESIGN_CANVAS.height,
      );
      expect(scale).toBeCloseTo(expected, 5);
    },
  );

  it('letterboxes ultrawide windows by picking the smaller axis', () => {
    setWindowSize(3440, 1440); // 21.5:9
    const scale = computeScale();
    // height-bound: 1440/1080 = 1.333…
    expect(scale).toBeCloseTo(1440 / 1080, 5);
    // Renders 1920*1.333 = 2560 wide, leaving (3440-2560)/2 = 440 per side.
    const renderedWidth = DESIGN_CANVAS.width * scale;
    expect(Math.round((3440 - renderedWidth) / 2)).toBe(440);
  });

  it('pillarboxes narrow windows by picking the smaller axis', () => {
    setWindowSize(1080, 1080); // square
    const scale = computeScale();
    // width-bound: 1080/1920 = 0.5625
    expect(scale).toBeCloseTo(1080 / 1920, 5);
  });

  it('produces scale 1.0 at the design canvas resolution', () => {
    setWindowSize(DESIGN_CANVAS.width, DESIGN_CANVAS.height);
    expect(computeScale()).toBe(1);
  });

  it('produces scale 2.0 at 3840×2160 (exact 2× canvas)', () => {
    setWindowSize(3840, 2160);
    expect(computeScale()).toBe(2);
  });
});
