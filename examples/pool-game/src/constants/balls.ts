import * as THREE from 'three';

// Ball colors: standard billiard colors
export const BALL_COLORS: Record<number, string> = {
  0: '#FFFFFF',  // Cue ball (white)
  1: '#FFD700',  // Yellow (solid)
  2: '#0000FF',  // Blue (solid)
  3: '#FF0000',  // Red (solid)
  4: '#800080',  // Purple (solid)
  5: '#FF8C00',  // Orange (solid)
  6: '#006400',  // Green (solid)
  7: '#8B0000',  // Maroon (solid)
  8: '#000000',  // Black (eight ball)
  9: '#FFD700',  // Yellow (stripe)
  10: '#0000FF', // Blue (stripe)
  11: '#FF0000', // Red (stripe)
  12: '#800080', // Purple (stripe)
  13: '#FF8C00', // Orange (stripe)
  14: '#006400', // Green (stripe)
  15: '#8B0000', // Maroon (stripe)
};

// Ball group assignment
export function getBallGroup(ballId: number): 'solids' | 'stripes' | 'eight' | 'cue' {
  if (ballId === 0) return 'cue';
  if (ballId === 8) return 'eight';
  if (ballId >= 1 && ballId <= 7) return 'solids';
  return 'stripes';
}

export function isStripe(ballId: number): boolean {
  return ballId >= 9 && ballId <= 15;
}

// All ball IDs (0-15)
export const ALL_BALL_IDS: number[] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

// Standard triangle rack positions
// The rack is positioned at the foot spot (3/4 of the table length from the head)
// Row layout: 1-2-3-4-5 balls

import { TABLE_LENGTH, BALL_RADIUS, TABLE_HEIGHT } from './table';
import type { BallId } from '../types';


const FOOT_SPOT_Z = -TABLE_LENGTH / 4;  // 1/4 from center on Z axis (negative Z is far from camera)
const ROW_SPACING = BALL_RADIUS * 2.02 * Math.cos(Math.PI / 6); // Add 0.02 gap to prevent penetration explosion
const COL_SPACING = BALL_RADIUS * 2.02;

// Standard rack order: 
// Row 0: 1
// Row 1: 9, 2
// Row 2: 10, 8, 3  (8-ball at center)
// Row 3: 11, 4, 12, 5
// Row 4: 13, 6, 14, 7, 15
// Corner balls must be one solid and one stripe

const RACK_ORDER = [1, 9, 2, 10, 8, 3, 11, 4, 12, 5, 13, 6, 14, 7, 15];

export function getRackPositions(): { [key: number]: THREE.Vector3 } {
  const positions: { [key: number]: THREE.Vector3 } = {};
  let ballIndex = 0;

  for (let row = 0; row < 5; row++) {
    const ballsInRow = row + 1;
    for (let col = 0; col < ballsInRow; col++) {
      const z = FOOT_SPOT_Z - row * ROW_SPACING; // Grow towards negative Z (away from cue)
      const x = (col - (ballsInRow - 1) / 2) * COL_SPACING;
      const ballId = RACK_ORDER[ballIndex];
      positions[ballId] = new THREE.Vector3(x, TABLE_HEIGHT + BALL_RADIUS, z);
      ballIndex++;
    }
  }

  // Cue ball at head spot (near camera)
  positions[0] = new THREE.Vector3(0, TABLE_HEIGHT + BALL_RADIUS, TABLE_LENGTH / 4);

  return positions;
}

// Initial ball positions as plain arrays for serialization
export function getInitialBallPositions(): { [key: number]: [number, number, number] } {
  const vecPositions = getRackPositions();
  const positions: { [key: number]: [number, number, number] } = {};
  for (const [id, pos] of Object.entries(vecPositions)) {
    positions[Number(id)] = [pos.x, pos.y, pos.z];
  }
  return positions;
}

