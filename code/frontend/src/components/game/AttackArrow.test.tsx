import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import { createRef } from 'react';
import { describe, expect, it } from 'vitest';
import { AttackArrow } from './AttackArrow';
import { ATTACK_TARGET_SECURITY } from '@/utils/constants';

function mockRect(el: Element, x: number, y: number, w = 96, h = 134) {
  el.getBoundingClientRect = () =>
    ({
      left: x,
      top: y,
      right: x + w,
      bottom: y + h,
      width: w,
      height: h,
      x,
      y,
      toJSON: () => {},
    }) as DOMRect;
}

describe('AttackArrow tracking', () => {
  it('tracks opponent attacker -> player Digimon (including redirects) and Security targets', () => {
    const containerRef = createRef<HTMLDivElement>();
    const { rerender } = render(
      <div ref={containerRef} data-testid="arena">
        <div data-slot-id="opponent-1" />
        <div data-slot-id="player-2" />
        <div data-testid="own-security-row" />
        <div data-testid="opponent-security-row" />
        <AttackArrow
          pendingAttack={{ attackerSlot: 1, targetSlot: 2 }}
          selectedAttacker={null}
          attackerIsOpponent
          containerRef={containerRef}
        />
      </div>,
    );

    const arena = screen.getByTestId('arena');
    mockRect(arena, 0, 0, 1200, 800);
    mockRect(arena.querySelector('[data-slot-id="opponent-1"]')!, 400, 120);
    mockRect(arena.querySelector('[data-slot-id="player-2"]')!, 600, 480);
    mockRect(arena.querySelector('[data-testid="own-security-row"]')!, 980, 520, 160, 60);
    mockRect(arena.querySelector('[data-testid="opponent-security-row"]')!, 980, 140, 160, 60);

    // Trigger re-measure with mocked rects
    rerender(
      <div ref={containerRef} data-testid="arena">
        <div data-slot-id="opponent-1" />
        <div data-slot-id="player-2" />
        <div data-testid="own-security-row" />
        <div data-testid="opponent-security-row" />
        <AttackArrow
          pendingAttack={{ attackerSlot: 1, targetSlot: 2 }}
          selectedAttacker={null}
          attackerIsOpponent
          boardLayoutKey="opp-redirect"
          containerRef={containerRef}
        />
      </div>,
    );

    const svg = screen.getByTestId('attack-arrow');
    expect(svg).toHaveAttribute('data-attacker-side', 'opponent');
    expect(svg).toHaveAttribute('data-target-slot', '2');
    const path = svg.querySelector('path')!;
    expect(path.getAttribute('d')).toContain('M 448 187');
    expect(path.getAttribute('d')).toContain('648 547');

    // Now test opponent attacking player's Security
    rerender(
      <div ref={containerRef} data-testid="arena">
        <div data-slot-id="opponent-1" />
        <div data-slot-id="player-2" />
        <div data-testid="own-security-row" />
        <div data-testid="opponent-security-row" />
        <AttackArrow
          pendingAttack={{ attackerSlot: 1, targetSlot: ATTACK_TARGET_SECURITY }}
          selectedAttacker={null}
          attackerIsOpponent
          boardLayoutKey="opp-sec"
          containerRef={containerRef}
        />
      </div>,
    );

    const secPath = screen.getByTestId('attack-arrow').querySelector('path')!;
    expect(secPath.getAttribute('d')).toContain('M 448 187');
    expect(secPath.getAttribute('d')).toContain('1060 550');
  });

  it('compensates for CanvasScaler CSS transform scaling when computing SVG coordinates', () => {
    const containerRef = createRef<HTMLDivElement>();
    const { rerender } = render(
      <div ref={containerRef} data-testid="arena">
        <div data-slot-id="player-0" />
        <div data-slot-id="opponent-1" />
        <AttackArrow
          pendingAttack={{ attackerSlot: 0, targetSlot: 1 }}
          selectedAttacker={null}
          attackerIsOpponent={false}
          containerRef={containerRef}
        />
      </div>,
    );

    const arena = screen.getByTestId('arena');
    // Canvas is 1920x1080 unscaled, rendered at 0.5x scale (960x540 in viewport)
    Object.defineProperty(arena, 'clientWidth', { configurable: true, value: 1920 });
    Object.defineProperty(arena, 'clientHeight', { configurable: true, value: 1080 });
    mockRect(arena, 100, 50, 960, 540);
    // Viewport rect at (100 + 200, 50 + 300) size 48x67 -> unscaled (400, 600) size 96x134 -> center (448, 667)
    mockRect(arena.querySelector('[data-slot-id="player-0"]')!, 300, 350, 48, 67);
    // Viewport rect at (100 + 400, 50 + 100) size 48x67 -> unscaled (800, 200) size 96x134 -> center (848, 267)
    mockRect(arena.querySelector('[data-slot-id="opponent-1"]')!, 500, 150, 48, 67);

    rerender(
      <div ref={containerRef} data-testid="arena">
        <div data-slot-id="player-0" />
        <div data-slot-id="opponent-1" />
        <AttackArrow
          pendingAttack={{ attackerSlot: 0, targetSlot: 1 }}
          selectedAttacker={null}
          attackerIsOpponent={false}
          boardLayoutKey="scaled-0.5"
          containerRef={containerRef}
        />
      </div>,
    );

    const path = screen.getByTestId('attack-arrow').querySelector('path')!;
    expect(path.getAttribute('d')).toContain('M 448 667');
    expect(path.getAttribute('d')).toContain('848 267');
  });
});
