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

// Standard triangle rack positions
// The rack is positioned at the foot spot (3/4 of the table length from the head)
// Row layout: 1-2-3-4-5 balls

import { TABLE_LENGTH, BALL_RADIUS } from './table';

const FOOT_SPOT_X = TABLE_LENGTH / 4;  // 3/4 from head = 1/4 from center (we use center as origin)
const ROW_SPACING = BALL_RADIUS * 2 * Math.cos(Math.PI / 6); // sqrt(3) * radius
const COL_SPACING = BALL_RADIUS * 2;

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
      const x = FOOT_SPOT_X + row * ROW_SPACING;
      const z = (col - (ballsInRow - 1) / 2) * COL_SPACING;
      const ballId = RACK_ORDER[ballIndex];
      positions[ballId] = new THREE.Vector3(x, BALL_RADIUS, z);
      ballIndex++;
    }
  }

  // Cue ball at head spot
  positions[0] = new THREE.Vector3(-TABLE_LENGTH / 4, BALL_RADIUS, 0);

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
