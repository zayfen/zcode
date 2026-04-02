import type { BallId, Vec3Tuple } from '../types';
import { BALL_RADIUS, HALF_LENGTH } from './table';

// ─────────────────────────────────────────────────────────────────────────────
// 16 ball colors (standard 8-ball palette)
// ─────────────────────────────────────────────────────────────────────────────
export const BALL_COLORS: Record<BallId, string> = {
  0: '#FFFFFF',  // Cue ball
  1: '#FFD700',  // Yellow (solid)
  2: '#0000FF',  // Blue (solid)
  3: '#FF0000',  // Red (solid)
  4: '#800080',  // Purple (solid)
  5: '#FF8C00',  // Orange (solid)
  6: '#006400',  // Green (solid)
  7: '#8B0000',  // Maroon (solid)
  8: '#000000',  // Eight ball (black)
  9: '#FFD700',  // Yellow (stripe)
  10: '#0000FF', // Blue (stripe)
  11: '#FF0000', // Red (stripe)
  12: '#800080', // Purple (stripe)
  13: '#FF8C00', // Orange (stripe)
  14: '#006400', // Green (stripe)
  15: '#8B0000', // Maroon (stripe)
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers – ball classification
// ─────────────────────────────────────────────────────────────────────────────
/** Balls 1-7 are solids */
export function isSolid(id: BallId): boolean {
  return id >= 1 && id <= 7;
}

/** Balls 9-15 are stripes */
export function isStripe(id: BallId): boolean {
  return id >= 9 && id <= 15;
}

/** Ball 8 is the eight ball */
export function isEightBall(id: BallId): boolean {
  return id === 8;
}

/** Ball 0 is the cue ball */
export function isCueBall(id: BallId): boolean {
  return id === 0;
}

// ─────────────────────────────────────────────────────────────────────────────
// Standard 8-ball rack layout (triangle at foot spot)
//
// Row 0 (apex):  1 ball   →   solid
// Row 1:         2 balls  →   stripe, solid
// Row 2:         3 balls  →   solid, EIGHT, stripe
// Row 3:         4 balls  →   mixed
// Row 4 (base):  5 balls  →   corners: one solid + one stripe
// ─────────────────────────────────────────────────────────────────────────────
const RACK_ORDER: BallId[] = [
  // row 0
  1,
  // row 1
  9, 2,
  // row 2 (8-ball in centre)
  10, 8, 11,
  // row 3
  3, 14, 6, 15,
  // row 4 (corners: solid + stripe)
  4, 12, 7, 13, 5,
];

/**
 * Returns the 16 ball starting positions as `[x, y, z]` tuples.
 *
 * The apex ball sits on the foot spot; each subsequent row is offset along
 * the Z-axis by `BALL_RADIUS * 2 * cos(30°)`.  Columns within a row are
 * centred around X = 0 with `BALL_RADIUS * 2` spacing.
 */
export function getRackPositions(): Record<BallId, Vec3Tuple> {
  const positions: Record<number, Vec3Tuple> = {};
  const diameter = BALL_RADIUS * 2;
  const spacing = diameter * 1.02; // tiny gap to avoid initial overlap
  const rowDepth = spacing * Math.cos(Math.PI / 6); // cos(30°) ≈ 0.866

  let idx = 0;
  for (let row = 0; row < 5; row++) {
    const count = row + 1;
    const z = HALF_LENGTH / 2 + row * rowDepth;
    for (let col = 0; col < count; col++) {
      const x = (col - (count - 1) / 2) * spacing;
      positions[RACK_ORDER[idx]] = [x, BALL_RADIUS, z];
      idx++;
    }
  }

  // Cue ball behind the head string (~40 % from head rail)
  positions[0] = [0, BALL_RADIUS, -HALF_LENGTH * 0.4];

  return positions as Record<BallId, Vec3Tuple>;
}

/** Convenience: look up a ball's hex colour string */
export function getBallColor(id: BallId): string {
  return BALL_COLORS[id];
}
