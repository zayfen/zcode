/**
 * Ball ID: 0 = cue ball, 1-7 = solids, 8 = eight ball, 9-15 = stripes
 */
export type BallId = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15;

/**
 * Ball group classification
 */
export type BallGroup = 'solids' | 'stripes' | null;

/**
 * Game phase as defined by the state machine:
 *
 * IDLE -> AIMING -> POWER -> SIMULATING -> EVALUATING -> (IDLE or GAME_OVER)
 */
export type GamePhase =
  | 'IDLE'
  | 'AIMING'
  | 'POWER'
  | 'SIMULATING'
  | 'EVALUATING'
  | 'GAME_OVER';

/**
 * Player identifier
 */
export type Player = 1 | 2;

/**
 * Foul types in 8-ball pool
 */
export type FoulType =
  | 'SCRATCH'
  | 'NO_RAIL_CONTACT'
  | 'WRONG_BALL_FIRST'
  | 'NO_BALL_HIT'
  | 'EIGHT_EARLY'
  | null;

/**
 * 3D vector as a tuple (compatible with Three.js and cannon-es)
 */
export type Vec3Tuple = [number, number, number];

/**
 * State of a single ball on the table
 */
export interface BallState {
  id: BallId;
  position: Vec3Tuple;
  velocity: Vec3Tuple;
  pocketed: boolean;
}

/**
 * Result of a single shot, used by the rules engine
 */
export interface ShotResult {
  pocketed: BallId[];
  firstContact: BallId | null;
  cueBallFinal: Vec3Tuple;
  foul: FoulType;
}

/**
 * Snapshot for undo support
 */
export interface ShotSnapshot {
  ballPositions: Record<number, Vec3Tuple>;
  ballPocketed?: Record<number, boolean>;
  state: {
    currentPlayer: Player;
    playerGroups: Record<Player, BallGroup>;
    pocketedBalls: BallId[];
  };
}

/**
 * Complete game state managed by Zustand
 */
export interface GameState {
  phase: GamePhase;
  currentPlayer: Player;
  playerGroups: Record<Player, BallGroup>;
  pocketedBalls: BallId[];
  foul: FoulType;
  winner: Player | null;
  ballInHand: boolean;
  ballInHandPosition: Vec3Tuple | null;
  shotHistory: ShotSnapshot[];
  ballsPocketedThisShot: BallId[];
}

/**
 * Aim state managed by a separate Zustand store
 * (updates every frame on mouse movement)
 */
export interface AimState {
  direction: Vec3Tuple;
  power: number;
  isCharging: boolean;
  cameraMode: 'orbit' | 'topdown';
}

// Utility constants
export const ALL_BALL_IDS: BallId[] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
export const SOLIDS: BallId[] = [1, 2, 3, 4, 5, 6, 7];
export const STRIPES: BallId[] = [9, 10, 11, 12, 13, 14, 15];
export const EIGHT: BallId = 8;

export function getBallGroup(id: BallId): BallGroup {
  if (id >= 1 && id <= 7) return 'solids';
  if (id >= 9 && id <= 15) return 'stripes';
  return null;
}

export function isStripe(id: BallId): boolean {
  return id >= 9 && id <= 15;
}

export function isSolid(id: BallId): boolean {
  return id >= 1 && id <= 7;
}
