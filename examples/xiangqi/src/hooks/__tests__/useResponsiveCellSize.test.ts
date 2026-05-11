import { renderHook, waitFor, act } from '@testing-library/react';
import { describe, expect, it, afterEach, vi } from 'vitest';
import { useResponsiveCellSize } from '../useResponsiveCellSize';

// ── Helpers ──

function mockViewport(width: number, height: number) {
  Object.defineProperty(window, 'innerWidth', {
    value: width,
    writable: true,
    configurable: true,
  });
  Object.defineProperty(window, 'innerHeight', {
    value: height,
    writable: true,
    configurable: true,
  });
}

// ── Constants matching the hook implementation ──
// WIDTH_FRACTION = 0.88, HEIGHT_FRACTION = 0.86, VERTICAL_CHROME = 180
// MIN_CELL = 36, MAX_CELL = 120
// byW = floor(width * 0.88 / 9)
// byH = floor((height - 180) * 0.86 / 10)
// raw = min(byW, byH)
// result = max(36, min(raw, 120))

describe('useResponsiveCellSize', () => {
  afterEach(() => {
    // Reset to a reasonable default
    mockViewport(1024, 768);
  });

  // ── Basic range check ──

  it('returns a number within the expected range', () => {
    mockViewport(1366, 768);
    const { result } = renderHook(() => useResponsiveCellSize());
    const size = result.current;
    expect(size).toBeGreaterThanOrEqual(36);
    expect(size).toBeLessThanOrEqual(120);
  });

  // ── 1920 × 1080 viewport ──

  it('computes correct cellSize on a 1920 × 1080 viewport', () => {
    mockViewport(1920, 1080);
    const { result } = renderHook(() => useResponsiveCellSize());
    // byW = floor(1920 * 0.88 / 9) = floor(187.91) = 187
    // byH = floor((1080 - 180) * 0.86 / 10) = floor(77.4) = 77
    // raw = min(187, 77) = 77
    expect(result.current).toBe(77);
  });

  // ── 1440 × 900 viewport ──

  it('computes correct cellSize on a 1440 × 900 viewport', () => {
    mockViewport(1440, 900);
    const { result } = renderHook(() => useResponsiveCellSize());
    // byW = floor(1440 * 0.88 / 9) = floor(140.8) = 140
    // byH = floor((900 - 180) * 0.86 / 10) = floor(61.92) = 61
    // raw = min(140, 61) = 61
    expect(result.current).toBe(61);
  });

  // ── 1366 × 768 viewport ──

  it('computes correct cellSize on a 1366 × 768 viewport', () => {
    mockViewport(1366, 768);
    const { result } = renderHook(() => useResponsiveCellSize());
    // byW = floor(1366 * 0.88 / 9) = floor(133.49) = 133
    // byH = floor((768 - 180) * 0.86 / 10) = floor(50.56) = 50
    // raw = min(133, 50) = 50
    expect(result.current).toBe(50);
  });

  // ── 1024 × 768 viewport ──

  it('computes correct cellSize on a 1024 × 768 viewport', () => {
    mockViewport(1024, 768);
    const { result } = renderHook(() => useResponsiveCellSize());
    // byW = floor(1024 * 0.88 / 9) = floor(100.13) = 100
    // byH = floor((768 - 180) * 0.86 / 10) = floor(50.56) = 50
    // raw = min(100, 50) = 50
    expect(result.current).toBe(50);
  });

  // ── Very small viewport hits the minimum clamp ──

  it('clamps to minimum on a very small viewport', () => {
    mockViewport(320, 240);
    const { result } = renderHook(() => useResponsiveCellSize());
    // byW = floor(320 * 0.88 / 9) = floor(31.28) = 31
    // byH = floor((240 - 180) * 0.86 / 10) = floor(5.16) = 5
    // raw = min(31, 5) = 5 → clamped to 36
    expect(result.current).toBe(36);
  });

  // ── Very large viewport hits the maximum clamp ──

  it('clamps to maximum (120) on a very large viewport', () => {
    mockViewport(3840, 2160); // 4K
    const { result } = renderHook(() => useResponsiveCellSize());
    // byW = floor(3840 * 0.88 / 9) = floor(375.47) = 375
    // byH = floor((2160 - 180) * 0.86 / 10) = floor(169.92) = 169
    // raw = min(375, 169) = 169 → clamped to 120
    expect(result.current).toBe(120);
  });

  // ── Updates on resize ──

  it('updates cellSize when the viewport is resized', async () => {
    mockViewport(1024, 768);
    const { result } = renderHook(() => useResponsiveCellSize());
    expect(result.current).toBe(50);

    // Simulate resize
    mockViewport(1920, 1080);
    act(() => {
      window.dispatchEvent(new Event('resize'));
    });

    // Wait for the requestAnimationFrame to fire and state to update
    await waitFor(() => {
      expect(result.current).toBe(77);
    });
  });
});
