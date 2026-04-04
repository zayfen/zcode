import type { BallId, BallGroup } from '../types';

// Standard billiard ball colors
export const BALL_COLORS: Record<BallId, string> = {
  0: '#FFFFFF',  // Cue ball (white)
  1: '#FFD700',  // Yellow (solid)
  2: '#0000CD',  // Blue (solid)
  3: '#FF0000',  // Red (solid)
  4: '#800080',  // Purple (solid)
  5: '#FF6600',  // Orange (solid)
  6: '#006400',  // Green (solid)
  7: '#8B0000',  // Maroon (solid)
  8: '#000000',  // Black (8-ball)
  9: '#FFD700',  // Yellow (stripe)
  10: '#0000CD', // Blue (stripe)
  11: '#FF0000', // Red (stripe)
  12: '#800080', // Purple (stripe)
  13: '#FF6600', // Orange (stripe)
  14: '#006400', // Green (stripe)
  15: '#8B0000', // Maroon (stripe)
};

// Ball group assignment
export function getBallGroup(id: BallId): BallGroup {
  if (id === 0 || id === 8) return null;
  if (id >= 1 && id <= 7) return 'solids';
  return 'stripes';
}

// Check if ball is a stripe
export function isStripe(id: BallId): boolean {
  return id >= 9 && id <= 15;
}

// Check if ball is a solid
export function isSolid(id: BallId): boolean {
  return id >= 1 && id <= 7;
}

// Standard triangle rack positions
// The rack is positioned at the foot spot (3/4 of table length from head)
// Balls arranged in triangle with 8-ball at center
// Origin is at table center; Z is the long axis
import { TABLE_LENGTH, BALL_RADIUS } from './table';

const RACK_Z = TABLE_LENGTH * 0.25; // Foot spot at 3/4 of table (from head at -TABLE_LENGTH/2)
const ROW_SPACING = BALL_RADIUS * 2 * Math.cos(Math.PI / 6); // sqrt(3) * radius
const COL_SPACING = BALL_RADIUS * 2;

// Standard 8-ball rack order (row by row, each row left to right)
// Row 0: 1 ball, Row 1: 2 balls, ..., Row 4: 5 balls
// 8-ball must be in the middle of row 2 (3rd row)
// One solid and one stripe in bottom corners
// 1-ball at the apex (facing the cue ball)
const RACK_ORDER: BallId[] = [1, 9, 2, 10, 8, 11, 3, 14, 6, 12, 4, 13, 7, 15, 5];

export function getRackPositions(): Record<BallId, [number, number, number]> {
  const positions: Record<number, [number, number, number]> = {};
  const cuePosition: [number, number, number] = [0, BALL_RADIUS, -TABLE_LENGTH * 0.25];

  positions[0] = cuePosition;

  let ballIndex = 0;
  for (let row = 0; row < 5; row++) {
    const numInRow = row + 1;
    for (let col = 0; col < numInRow; col++) {
      const x = (col - (numInRow - 1) / 2) * COL_SPACING;
      const z = RACK_Z + row * ROW_SPACING;
      const ballId = RACK_ORDER[ballIndex];
      positions[ballId] = [x, BALL_RADIUS, z];
      ballIndex++;
    }
  }

  return positions as Record<BallId, [number, number, number]>;
}

// All ball IDs
export const ALL_BALL_IDS: BallId[] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

// Object ball IDs (excluding cue)
export const OBJECT_BALL_IDS: BallId[] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
