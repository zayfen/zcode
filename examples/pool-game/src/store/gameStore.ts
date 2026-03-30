/**
 * Zustand game store — Phase 5 (T14) + Phase 6 aiming enhancements + Phase 9 (T27-T28)
 *
 * State machine:  IDLE → AIMING → POWER → SIMULATING → EVALUATING → (IDLE | GAME_OVER)
 *
 * Exposes the full game state plus actions for every phase transition,
 * ball tracking, foul handling, group assignment, ball-in-hand, and
 * undo (single-step shot history with full restore).
 */
import { create } from 'zustand';
import type {
  GamePhase,
  Player,
  BallGroup,
  BallId,
  FoulType,
  Vec3Tuple,
  ShotSnapshot,
  BallState,
} from '../types';
import { getRackPositions } from '../constants/balls';
import {
  HALF_WIDTH,
  HALF_LENGTH,
  CUSHION_THICKNESS,
  BALL_RADIUS,
} from '../constants/table';

// ---------------------------------------------------------------------------
// Store shape
// ---------------------------------------------------------------------------

interface GameStoreState {
  /** Current state-machine phase */
  phase: GamePhase;

  /** Which player is at the table */
  currentPlayer: Player;

  /** Group assigned to each player (null = not yet assigned) */
  playerGroups: Record<Player, BallGroup>;

  /** All balls pocketed so far this game */
  pocketedBalls: BallId[];

  /** Per-ball state (position, velocity, pocketed flag) */
  ballStates: Record<BallId, BallState>;

  /** Balls pocketed during the current shot */
  ballsPocketedThisShot: BallId[];

  /** First ball contacted by cue ball this shot */
  firstContact: BallId | null;

  /** Whether a rail was contacted after the first ball hit */
  railContacted: boolean;

  /** Current foul (null = none) */
  foul: FoulType;

  /** Winner (null while game in progress) */
  winner: Player | null;

  /** Ball-in-hand mode (player must place cue ball) */
  ballInHand: boolean;
  ballInHandPosition: Vec3Tuple | null;

  /** Shot history for single-step undo */
  shotHistory: ShotSnapshot[];

  /** Shot number (0 = before break) */
  shotNumber: number;

  /**
   * Real-time cue ball position (updated by physics subscription).
   * Used by the aiming system to compute aim direction from the cue ball.
   */
  cueBallPosition: Vec3Tuple;
}

interface GameStoreActions {
  // ── Phase transitions ──────────────────────────────────────────────────
  startAiming: () => void;
  startPower: () => void;
  shoot: (impulse: Vec3Tuple) => void;
  setSimulating: () => void;
  evaluateShot: () => void;
  nextTurn: (nextPlayer: Player) => void;
  setGameOver: (winner: Player) => void;
  resetGame: () => void;

  // ── Ball tracking ──────────────────────────────────────────────────────
  pocketBall: (id: BallId) => void;
  setFirstContact: (id: BallId) => void;
  setRailContacted: () => void;
  updateBallState: (id: BallId, position: Vec3Tuple, velocity: Vec3Tuple) => void;
  removeBallFromPlay: (id: BallId) => void;
  setCueBallPosition: (pos: Vec3Tuple) => void;

  // ── Ball-in-hand ───────────────────────────────────────────────────────
  setBallInHand: (enabled: boolean) => void;
  placeBallInHand: (pos: Vec3Tuple) => void;

  // ── Snapshot / undo ────────────────────────────────────────────────────
  takeSnapshot: () => void;
  undo: () => ShotSnapshot | null;
  restoreSnapshot: (snapshot: ShotSnapshot) => void;

  // ── Helpers ────────────────────────────────────────────────────────────
  setFoul: (foul: FoulType) => void;
  assignGroups: (player: Player, group: BallGroup) => void;
}

export type GameStore = GameStoreState & GameStoreActions;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const ALL_IDS: BallId[] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

/** Create the initial ball states from rack positions. */
function createInitialBallStates(): Record<BallId, BallState> {
  const positions = getRackPositions();
  const states: Partial<Record<BallId, BallState>> = {};
  for (const id of ALL_IDS) {
    const pos = positions[id as BallId];
    states[id as BallId] = {
      id: id as BallId,
      position: pos,
      velocity: [0, 0, 0],
      pocketed: false,
    };
  }
  return states as Record<BallId, BallState>;
}

/** Build a fresh default state object (used for resetGame). */
function freshState(): GameStoreState {
  const ballStates = createInitialBallStates();
  return {
    phase: 'IDLE',
    currentPlayer: 1,
    playerGroups: { 1: null, 2: null },
    pocketedBalls: [],
    ballStates,
    ballsPocketedThisShot: [],
    firstContact: null,
    railContacted: false,
    foul: null,
    winner: null,
    ballInHand: false,
    ballInHandPosition: null,
    shotHistory: [],
    shotNumber: 0,
    cueBallPosition: [...ballStates[0].position] as Vec3Tuple,
  };
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useGameStore = create<GameStore>((set, get) => ({
  ...freshState(),

  // ── Phase transitions ──────────────────────────────────────────────────

  startAiming: () => {
    const { phase, ballInHand } = get();
    if (phase === 'IDLE' && !ballInHand) {
      set({ phase: 'AIMING' });
    }
  },

  startPower: () => {
    const { phase } = get();
    if (phase === 'AIMING') {
      set({ phase: 'POWER' });
    }
  },

  shoot: (impulse: Vec3Tuple) => {
    const { phase, ballStates, shotNumber } = get();
    if (phase !== 'POWER') return;

    // Take snapshot for undo BEFORE the shot
    get().takeSnapshot();

    // Reset shot-specific tracking
    set({
      phase: 'SIMULATING',
      ballsPocketedThisShot: [],
      firstContact: null,
      railContacted: false,
      foul: null,
      shotNumber: shotNumber + 1,
    });

    // Apply the impulse to the cue ball's velocity in ballStates
    const cueState = ballStates[0];
    if (cueState && !cueState.pocketed) {
      set({
        ballStates: {
          ...ballStates,
          0: {
            ...cueState,
            velocity: impulse,
          },
        },
      });
    }
  },

  setSimulating: () => {
    set({ phase: 'SIMULATING' });
  },

  evaluateShot: () => {
    const { phase } = get();
    if (phase === 'SIMULATING') {
      set({ phase: 'EVALUATING' });
    }
  },

  nextTurn: (nextPlayer: Player) => {
    set({
      phase: 'IDLE',
      currentPlayer: nextPlayer,
      ballsPocketedThisShot: [],
      firstContact: null,
      railContacted: false,
    });
  },

  setGameOver: (winner: Player) => {
    set({ phase: 'GAME_OVER', winner });
  },

  resetGame: () => {
    set(freshState());
  },

  // ── Ball tracking ──────────────────────────────────────────────────────

  pocketBall: (id: BallId) => {
    const { pocketedBalls, ballsPocketedThisShot, ballStates } = get();
    if (pocketedBalls.includes(id)) return;

    set({
      pocketedBalls: [...pocketedBalls, id],
      ballsPocketedThisShot: [...ballsPocketedThisShot, id],
      ballStates: {
        ...ballStates,
        [id]: {
          ...ballStates[id],
          pocketed: true,
          velocity: [0, 0, 0],
        },
      },
    });
  },

  setFirstContact: (id: BallId) => {
    const { firstContact } = get();
    if (firstContact === null) {
      set({ firstContact: id });
    }
  },

  setRailContacted: () => {
    set({ railContacted: true });
  },

  updateBallState: (id: BallId, position: Vec3Tuple, velocity: Vec3Tuple) => {
    const { ballStates } = get();
    set({
      ballStates: {
        ...ballStates,
        [id]: {
          ...ballStates[id],
          position,
          velocity,
        },
      },
    });
  },

  removeBallFromPlay: (id: BallId) => {
    const { ballStates } = get();
    const current = ballStates[id];
    if (!current) return;

    set({
      ballStates: {
        ...ballStates,
        [id]: {
          ...current,
          position: [current.position[0], -0.5, current.position[2]],
          velocity: [0, 0, 0],
          pocketed: true,
        },
      },
    });
  },

  setCueBallPosition: (pos: Vec3Tuple) => {
    set({ cueBallPosition: pos });
  },

  // ── Ball-in-hand ───────────────────────────────────────────────────────

  setBallInHand: (enabled: boolean) => {
    set({
      ballInHand: enabled,
      phase: enabled ? 'IDLE' : get().phase,
      ballInHandPosition: enabled ? null : get().ballInHandPosition,
    });
  },

  placeBallInHand: (pos: Vec3Tuple) => {
    const { ballStates } = get();

    const clampedPos: Vec3Tuple = [
      Math.max(-(HALF_WIDTH - CUSHION_THICKNESS - BALL_RADIUS),
        Math.min(HALF_WIDTH - CUSHION_THICKNESS - BALL_RADIUS, pos[0])),
      BALL_RADIUS,
      Math.max(-(HALF_LENGTH - CUSHION_THICKNESS - BALL_RADIUS),
        Math.min(HALF_LENGTH - CUSHION_THICKNESS - BALL_RADIUS, pos[2])),
    ];

    set({
      ballInHand: false,
      ballInHandPosition: clampedPos,
      cueBallPosition: clampedPos,
      ballStates: {
        ...ballStates,
        0: {
          ...ballStates[0],
          id: 0,
          position: clampedPos,
          velocity: [0, 0, 0],
          pocketed: false,
        },
      },
    });
  },

  // ── Snapshot / undo ────────────────────────────────────────────────────

  takeSnapshot: () => {
    const state = get();
    const ballPositions: Record<number, Vec3Tuple> = {};
    const ballPocketed: Record<number, boolean> = {};
    for (const id of ALL_IDS) {
      ballPositions[id] = [...state.ballStates[id as BallId].position];
      ballPocketed[id] = state.ballStates[id as BallId].pocketed;
    }

    const snapshot: ShotSnapshot & { ballPocketed: Record<number, boolean> } = {
      ballPositions,
      ballPocketed,
      state: {
        currentPlayer: state.currentPlayer,
        playerGroups: { ...state.playerGroups },
        pocketedBalls: [...state.pocketedBalls],
      },
    };

    // Keep only the most recent snapshot (single-step undo)
    set({ shotHistory: [snapshot] });
  },

  undo: (): ShotSnapshot | null => {
    const { shotHistory } = get();
    if (shotHistory.length === 0) return null;
    return shotHistory[0];
  },

  restoreSnapshot: (snapshot: ShotSnapshot) => {
    const ballStates = get().ballStates;
    const restoredBallStates = { ...ballStates };

    for (const id of ALL_IDS) {
      const bid = id as BallId;
      restoredBallStates[bid] = {
        ...restoredBallStates[bid],
        position: [...snapshot.ballPositions[id]],
        velocity: [0, 0, 0],
        pocketed: snapshot.ballPocketed?.[id] ?? false,
      };
    }

    set({
      phase: 'IDLE',
      currentPlayer: snapshot.state.currentPlayer,
      playerGroups: { ...snapshot.state.playerGroups },
      pocketedBalls: [...snapshot.state.pocketedBalls],
      ballStates: restoredBallStates,
      cueBallPosition: [...snapshot.ballPositions[0]] as Vec3Tuple,
      ballsPocketedThisShot: [],
      firstContact: null,
      foul: null,
      ballInHand: false,
      shotHistory: [],
    });
  },

  // ── Helpers ────────────────────────────────────────────────────────────

  setFoul: (foul: FoulType) => {
    set({ foul });
  },

  assignGroups: (player: Player, group: BallGroup) => {
    const otherPlayer: Player = player === 1 ? 2 : 1;
    const otherGroup: BallGroup = group === 'solids' ? 'stripes' : 'solids';
    set({
      playerGroups: {
        [player]: group,
        [otherPlayer]: otherGroup,
      } as Record<Player, BallGroup>,
    });
  },
}));
