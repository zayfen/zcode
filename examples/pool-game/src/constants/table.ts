/**
 * Table geometry constants.
 * Coordinate system: origin at center of table bed surface.
 * X = length (long), Z = width (short), Y = up.
 */

// Playing surface dimensions (meters) — regulation 7-foot table proportions
export const TABLE_LENGTH = 2.24; // X axis (long dimension)
export const TABLE_WIDTH = 1.12; // Z axis (short dimension)

export const BALL_RADIUS = 0.0285; // Standard billiard ball radius
export const POCKET_RADIUS = 0.047; // Distance threshold for pocket detection

// Table structure
export const TABLE_HEIGHT = 0.05;
export const FELT_THICKNESS = 0.02;
export const RAIL_HEIGHT = 0.05;
export const RAIL_THICKNESS = 0.08;
export const RAIL_WIDTH = 0.08;
export const CUSHION_HEIGHT = 0.035;
export const CUSHION_THICKNESS = 0.03;
export const CUSHION_WIDTH = 0.03;

// Half dimensions
const HL = TABLE_LENGTH / 2;
const HW = TABLE_WIDTH / 2;

// Pocket center positions (6 pockets: 4 corners + 2 side)
export const POCKET_POSITIONS: { x: number; y: number; z: number }[] = [
  // Corner pockets
  { x: -HL, y: 0, z: -HW },
  { x: HL, y: 0, z: -HW },
  { x: -HL, y: 0, z: HW },
  { x: HL, y: 0, z: HW },
  // Side pockets (middle of long rails)
  { x: -HL, y: 0, z: 0 },
  { x: HL, y: 0, z: 0 },
];

// Pocket positions as tuples for physics
export const POCKET_CENTERS: [number, number, number][] = [
  [-HL, 0, -HW],
  [HL, 0, -HW],
  [-HL, 0, HW],
  [HL, 0, HW],
  [-HL, 0, 0],
  [HL, 0, 0],
];

// Cushion segment definitions
export const CUSHION_SEGMENTS: {
  position: [number, number, number];
  size: [number, number, number];
}[] = [
  // Long rails (along X), split by side pockets
  // Top long rail (Z = -HW), left half
  { position: [-HL / 2, CUSHION_HEIGHT / 2, -HW + CUSHION_THICKNESS / 2], size: [HL - POCKET_RADIUS * 2, CUSHION_HEIGHT, CUSHION_THICKNESS] },
  // Top long rail, right half
  { position: [HL / 2, CUSHION_HEIGHT / 2, -HW + CUSHION_THICKNESS / 2], size: [HL - POCKET_RADIUS * 2, CUSHION_HEIGHT, CUSHION_THICKNESS] },
  // Bottom long rail (Z = +HW), left half
  { position: [-HL / 2, CUSHION_HEIGHT / 2, HW - CUSHION_THICKNESS / 2], size: [HL - POCKET_RADIUS * 2, CUSHION_HEIGHT, CUSHION_THICKNESS] },
  // Bottom long rail, right half
  { position: [HL / 2, CUSHION_HEIGHT / 2, HW - CUSHION_THICKNESS / 2], size: [HL - POCKET_RADIUS * 2, CUSHION_HEIGHT, CUSHION_THICKNESS] },
  // Left short rail (X = -HL)
  { position: [-HL + CUSHION_THICKNESS / 2, CUSHION_HEIGHT / 2, 0], size: [CUSHION_THICKNESS, CUSHION_HEIGHT, TABLE_WIDTH * 0.7] },
  // Right short rail (X = +HL)
  { position: [HL - CUSHION_THICKNESS / 2, CUSHION_HEIGHT / 2, 0], size: [CUSHION_THICKNESS, CUSHION_HEIGHT, TABLE_WIDTH * 0.7] },
];

// Head string position
export const HEAD_STRING_X = -TABLE_LENGTH / 4;

// Foot spot
export const FOOT_SPOT_X = TABLE_LENGTH / 4;
