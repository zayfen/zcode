// ─── Ball Types ───────────────────────────────────────────────────────────────
export type BallId = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15;

export type BallGroup = 'solids' | 'stripes' | null;

// ─── Game State Types ────────────────────────────────────────────────────────
export type GamePhase =
  | 'IDLE'
  | 'AIMING'
  | 'POWER'
  | 'SIMULATING'
  | 'EVALUATING'
  | 'GAME_OVER';

export type Player = 1 | 2;

// ─── Foul Types ──────────────────────────────────────────────────────────────
export type Foul = 'SCRATCH' | 'NO_RAIL_CONTACT' | 'WRONG_BALL_FIRST' | 'NO_BALL_HIT' | null;

// ─── Shot Result ─────────────────────────────────────────────────────────────
export interface ShotResult {
  pocketed: BallId[];
  firstContact: BallId | null;
  cueBallStopped: { x: number; y: number; z: number };
  foul: Foul;
}

// ─── Shot Evaluation ─────────────────────────────────────────────────────────
export interface ShotEvaluation {
  foul: Foul;
  nextPlayer: Player;
  pocketed: BallId[];
  gameOver: boolean;
  winner: Player | null;
  ballGroupAssigned: boolean;
}

// ─── Game State ──────────────────────────────────────────────────────────────
export interface GameState {
  phase: GamePhase;
  currentPlayer: Player;
  playerGroups: Record<Player, BallGroup>;
  pocketedBalls: BallId[];
  scores: Record<Player, number>;
  foul: Foul;
  winner: Player | null;
  ballInHand: boolean;
  ballInHandPosition: { x: number; y: number; z: number } | null;
  breakShot: boolean;
  groupsAssigned: boolean;
}

// ─── Ball State ──────────────────────────────────────────────────────────────
export interface BallState {
  id: BallId;
  position: [number, number, number];
  velocity: [number, number, number];
  angularVelocity: [number, number, number];
  pocketed: boolean;
}

// ─── Snapshot for Undo ───────────────────────────────────────────────────────
export interface GameSnapshot {
  gameState: GameState;
  ballStates: BallState[];
}
