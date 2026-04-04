// Table dimensions in meters (regulation proportions)
export const TABLE_LENGTH = 2.24; // long axis (Z)
export const TABLE_WIDTH = 1.12; // short axis (X)
export const TABLE_HEIGHT = 0.78; // playing surface height
export const RAIL_HEIGHT = 0.05;
export const RAIL_WIDTH = 0.08;
export const CUSHION_HEIGHT = 0.032;
export const CUSHION_WIDTH = 0.05;

export const BALL_RADIUS = 0.0285;
export const POCKET_RADIUS = 0.057; // ~2× ball radius for gameplay feel

export const CUSHION_RESTITUTION = 0.6;

// Pocket center positions (6 pockets: 4 corners + 2 side)
export const POCKET_POSITIONS: [number, number, number][] = [
  // Corner pockets
  [-TABLE_WIDTH / 2, TABLE_HEIGHT, -TABLE_LENGTH / 2],
  [TABLE_WIDTH / 2, TABLE_HEIGHT, -TABLE_LENGTH / 2],
  [-TABLE_WIDTH / 2, TABLE_HEIGHT, TABLE_LENGTH / 2],
  [TABLE_WIDTH / 2, TABLE_HEIGHT, TABLE_LENGTH / 2],
  // Side pockets
  [TABLE_WIDTH / 2, TABLE_HEIGHT, 0],
  [-TABLE_WIDTH / 2, TABLE_HEIGHT, 0],
];

// Play area bounds (inside cushions)
export const PLAY_MIN_X = -TABLE_WIDTH / 2 + CUSHION_WIDTH;
export const PLAY_MAX_X = TABLE_WIDTH / 2 - CUSHION_WIDTH;
export const PLAY_MIN_Z = -TABLE_LENGTH / 2 + CUSHION_WIDTH;
export const PLAY_MAX_Z = TABLE_LENGTH / 2 - CUSHION_WIDTH;

// Table surface Y
export const TABLE_SURFACE_Y = TABLE_HEIGHT;
