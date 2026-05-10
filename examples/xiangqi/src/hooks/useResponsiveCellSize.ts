import { useEffect, useState } from 'react';

// ── Responsive cell-size computation ──

const COLS = 9;
const ROWS = 10;

const MIN_CELL = 36;
const MAX_CELL = 120;

/** Fraction of viewport the board may occupy (width). */
const WIDTH_FRACTION = 0.88;
/** Fraction of viewport the board may occupy (height). */
const HEIGHT_FRACTION = 0.86;
/** Approximate vertical chrome (header, turn indicator, buttons) in px. */
const VERTICAL_CHROME = 180;

function computeCellSize(): number {
  const maxWidth = window.innerWidth * WIDTH_FRACTION;
  const maxHeight = (window.innerHeight - VERTICAL_CHROME) * HEIGHT_FRACTION;

  const byW = Math.floor(maxWidth / COLS);
  const byH = Math.floor(maxHeight / ROWS);

  const raw = Math.min(byW, byH);
  return Math.max(MIN_CELL, Math.min(raw, MAX_CELL));
}

/**
 * Returns a `cellSize` (px) that scales the board to fit comfortably
 * within the current viewport. Updates on window resize.
 */
export function useResponsiveCellSize(): number {
  const [cellSize, setCellSize] = useState(() => computeCellSize());

  useEffect(() => {
    let rafId = 0;

    const handleResize = () => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        setCellSize(computeCellSize());
      });
    };

    window.addEventListener('resize', handleResize, { passive: true });
    return () => {
      window.removeEventListener('resize', handleResize);
      cancelAnimationFrame(rafId);
    };
  }, []);

  return cellSize;
}
