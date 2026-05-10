import { renderHook, act } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { useAnimation } from '../useAnimation';
import type { Piece, Position } from '../../engine';

// ── Helpers ──

const redPiece: Piece = { type: 'chariot', player: 'red' };
const from: Position = { row: 9, col: 0 };
const to: Position = { row: 5, col: 0 };

describe('useAnimation', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ── Initial state ──

  it('starts with no animation', () => {
    const { result } = renderHook(() => useAnimation());
    expect(result.current.animation).toBeNull();
    expect(result.current.isAnimating).toBe(false);
    expect(result.current.hiddenPosition).toBeNull();
    expect(result.current.duration).toBeGreaterThanOrEqual(200);
  });

  // ── Default duration ──

  it('uses 250 ms default duration (≥ 200)', () => {
    const { result } = renderHook(() => useAnimation());
    expect(result.current.duration).toBe(250);
  });

  // ── Custom duration ──

  it('respects custom duration but enforces minimum of 200 ms', () => {
    const { result: r1 } = renderHook(() => useAnimation(300));
    expect(r1.current.duration).toBe(300);

    const { result: r2 } = renderHook(() => useAnimation(100));
    expect(r2.current.duration).toBe(200); // clamped to minimum
  });

  // ── startAnimation ──

  it('sets animation state when startAnimation is called', () => {
    const { result } = renderHook(() => useAnimation());

    act(() => {
      result.current.startAnimation(from, to, redPiece);
    });

    expect(result.current.animation).not.toBeNull();
    expect(result.current.animation!.from).toEqual(from);
    expect(result.current.animation!.to).toEqual(to);
    expect(result.current.animation!.piece).toEqual(redPiece);
    expect(result.current.animation!.startTime).toBeGreaterThan(0);
    expect(result.current.isAnimating).toBe(true);
    expect(result.current.hiddenPosition).toEqual(to);
  });

  // ── clearAnimation ──

  it('clears animation when clearAnimation is called', () => {
    const { result } = renderHook(() => useAnimation());

    act(() => {
      result.current.startAnimation(from, to, redPiece);
    });
    expect(result.current.isAnimating).toBe(true);

    act(() => {
      result.current.clearAnimation();
    });
    expect(result.current.animation).toBeNull();
    expect(result.current.isAnimating).toBe(false);
    expect(result.current.hiddenPosition).toBeNull();
  });

  // ── Fallback timer ──

  it('auto-clears animation after duration + 50 ms (fallback timer)', () => {
    const { result } = renderHook(() => useAnimation(250));

    act(() => {
      result.current.startAnimation(from, to, redPiece);
    });
    expect(result.current.isAnimating).toBe(true);

    // Advance just before the fallback fires
    act(() => {
      vi.advanceTimersByTime(299);
    });
    expect(result.current.isAnimating).toBe(true);

    // Advance past the fallback
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.isAnimating).toBe(false);
    expect(result.current.animation).toBeNull();
  });

  // ── Cancelling previous animation ──

  it('cancels previous animation when a new one starts', () => {
    const { result } = renderHook(() => useAnimation());

    const to2: Position = { row: 3, col: 5 };

    act(() => {
      result.current.startAnimation(from, to, redPiece);
    });
    expect(result.current.animation!.to).toEqual(to);

    act(() => {
      result.current.startAnimation(from, to2, redPiece);
    });
    expect(result.current.animation!.to).toEqual(to2);
    expect(result.current.isAnimating).toBe(true);
  });

  // ── Hidden position tracks animation destination ──

  it('hiddenPosition is null when no animation, equals animation.to when animating', () => {
    const { result } = renderHook(() => useAnimation());

    expect(result.current.hiddenPosition).toBeNull();

    act(() => {
      result.current.startAnimation(from, to, redPiece);
    });
    expect(result.current.hiddenPosition).toEqual(to);

    act(() => {
      result.current.clearAnimation();
    });
    expect(result.current.hiddenPosition).toBeNull();
  });

  // ── Multiple start/clear cycles ──

  it('supports multiple start/clear cycles', () => {
    const { result } = renderHook(() => useAnimation(200));

    for (let i = 0; i < 3; i++) {
      act(() => {
        result.current.startAnimation(from, to, redPiece);
      });
      expect(result.current.isAnimating).toBe(true);

      act(() => {
        result.current.clearAnimation();
      });
      expect(result.current.isAnimating).toBe(false);
    }
  });
});
