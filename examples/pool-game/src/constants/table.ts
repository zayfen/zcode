import type { Vec3Tuple } from '../types';

// Table dimensions (meters)
export const TABLE_LENGTH = 2.24; // Z-axis (long dimension)
export const TABLE_WIDTH = 1.12;  // X-axis (short dimension)
export const TABLE_HEIGHT = 0.02; // Y-axis thickness of bed

// Rail dimensions
export const RAIL_HEIGHT = 0.05;
export const RAIL_WIDTH = 0.06;

// Ball
export const BALL_RADIUS = 0.0285;

// Pocket
export const POCKET_RADIUS = 0.047;

// Playing bounds (half extents)
export const HALF_LENGTH = TABLE_LENGTH / 2;
export const HALF_WIDTH = TABLE_WIDTH / 2;

// Playing surface bounds (where balls can travel, accounting for rail thickness)
export const CUSHION_THICKNESS = 0.03;
export const PLAYING_BOUNDS = {
  minX: -(HALF_WIDTH - CUSHION_THICKNESS),
  maxX: HALF_WIDTH - CUSHION_THICKNESS,
  minZ: -(HALF_LENGTH - CUSHION_THICKNESS),
  maxZ: HALF_LENGTH - CUSHION_THICKNESS,
};

// Head string: 1/4 of table length from head rail
// This is the line behind which the cue ball is placed for the break
export const HEAD_STRING_Z = -HALF_LENGTH / 2;

// Foot spot: center of the foot end (for racking)
// Located at the intersection of the long center line and a line drawn
// through the foot rail's center spot
export const FOOT_SPOT: Vec3Tuple = [0, 0, HALF_LENGTH / 2];

// Pocket center positions (6 pockets)
// 4 corner pockets + 2 side (middle) pockets
export const POCKET_POSITIONS: Vec3Tuple[] = [
  // Corner pockets
  [-HALF_WIDTH, 0, -HALF_LENGTH],  // head-left
  [HALF_WIDTH, 0, -HALF_LENGTH],   // head-right
  [-HALF_WIDTH, 0, HALF_LENGTH],   // foot-left
  [HALF_WIDTH, 0, HALF_LENGTH],    // foot-right
  // Side pockets (middle of long rails)
  [-HALF_WIDTH, 0, 0],             // mid-left
  [HALF_WIDTH, 0, 0],              // mid-right
];

// Pocket labels for readability
export const POCKET_LABELS = [
  'head-left',
  'head-right',
  'foot-left',
  'foot-right',
  'mid-left',
  'mid-right',
] as const;

// Cushion height (how tall the cushion is above the bed)
export const CUSHION_HEIGHT = BALL_RADIUS * 1.5;

// Floor
export const FLOOR_Y = -0.8;

// Camera defaults
export const CAMERA_DEFAULT_POSITION: Vec3Tuple = [0, 1.8, 1.6];
export const CAMERA_FOV = 45;
