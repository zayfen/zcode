// src/types/index.ts

export type BallId = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15;
export type BallGroup = 'solids' | 'stripes' | null;
export type GamePhase = 'IDLE' | 'AIMING' | 'POWER' | 'SIMULATING' | 'EVALUATING' | 'GAME_OVER';
export type Player = 1 | 2;
export type Foul = 'SCRATCH' | 'NO_RAIL_CONTACT' | 'WRONG_BALL_FIRST' | 'NO_BALL_HIT' | null;

export interface BallState {
  id: BallId;
  position: [number, number, number];
  velocity: [number, number, number];
  pocketed: boolean;
}

export interface ShotResult {
  pocketed: BallId[];
  firstContact: BallId | null;
  cueBallStopped: { x: number; y: number; z: number };
  foul: Foul;
}

export interface GameState {
  phase: GamePhase;
  currentPlayer: Player;
  ballGroups: { player1: BallGroup; player2: BallGroup };
  pocketedBalls: BallId[];
  scores: { player1: number; player2: number };
  foul: Foul;
  winner: Player | null;
  ballInHand: boolean;
  ballInHandPosition: [number, number, number] | null;
}

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export const BALL_IDS: BallId[] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
export const SOLIDS: BallId[] = [1, 2, 3, 4, 5, 6, 7];
export const STRIPES: BallId[] = [9, 10, 11, 12, 13, 14, 15];
export const EIGHT_BALL: BallId = 8;
