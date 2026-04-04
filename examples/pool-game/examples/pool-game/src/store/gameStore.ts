// src/store/gameStore.ts
import { create } from 'zustand';
import { GamePhase, Player, Foul, BallId, BallGroup } from '../types';
import { getInitialBallPositions } from '../constants/balls';

export interface GameStoreState {
  phase: GamePhase;
  currentPlayer: Player;
  ballGroups: { player1: BallGroup; player2: BallGroup };
  pocketedBalls: BallId[];
  scores: { player1: number; player2: number };
  foul: Foul;
  winner: Player | null;
  ballInHand: boolean;
  ballInHandPosition: [number, number, number] | null;
  ballPositions: { [key: number]: [number, number, number] };
  snapshot: {
    ballPositions: { [key: number]: [number, number, number] } | null;
    state: GameStoreState | null;
  };
}

export interface GameStoreActions {
  startAiming: () => void;
  startPower: () => void;
  shoot: () => void;
  setSimulating: () => void;
  evaluateShot: (pocketed: BallId[], firstContact: BallId | null, foul: Foul) => void;
  nextTurn: () => void;
  setGameOver: (winner: Player) => void;
  resetGame: () => void;
  setPhase: (phase: GamePhase) => void;
  setBallInHand: (position: [number, number, number] | null) => void;
  updateBallPosition: (id: number, pos: [number, number, number]) => void;
  takeSnapshot: () => void;
  undo: () => void;
}

const initialState = (): Omit<GameStoreState, never> => ({
  phase: 'IDLE',
  currentPlayer: 1,
  ballGroups: { player1: null, player2: null },
  pocketedBalls: [],
  scores: { player1: 0, player2: 0 },
  foul: null,
  winner: null,
  ballInHand: false,
  ballInHandPosition: null,
  ballPositions: getInitialBallPositions(),
  snapshot: { ballPositions: null, state: null },
});

export const useGameStore = create<GameStoreState & GameStoreActions>((set, get) => ({
  ...initialState(),

  startAiming: () => set({ phase: 'AIMING' }),

  startPower: () => {
    const state = get();
    if (state.phase === 'AIMING') {
      set({ phase: 'POWER' });
    }
  },

  shoot: () => set({ phase: 'SIMULATING' }),

  setSimulating: () => set({ phase: 'SIMULATING' }),

  evaluateShot: (pocketed, firstContact, foul) => {
    const state = get();
    const allPocketed = [...state.pocketedBalls, ...pocketed];

    let newGroups = { ...state.ballGroups };
    let nextPlayer: Player = state.currentPlayer === 1 ? 2 : 1;
    let newFoul: Foul = foul;
    let newBallInHand = false;
    let gameOver = false;
    let winner: Player | null = null;

    // Check for scratch (cue ball pocketed)
    if (pocketed.includes(0)) {
      newFoul = 'SCRATCH';
      newBallInHand = true;
    }

    // Check if 8-ball was pocketed
    if (pocketed.includes(8)) {
      const currentGroup = state.currentPlayer === 1 ? state.ballGroups.player1 : state.ballGroups.player2;
      const playerBalls = currentGroup === 'solids'
        ? [1, 2, 3, 4, 5, 6, 7]
        : currentGroup === 'stripes'
        ? [9, 10, 11, 12, 13, 14, 15]
        : [];

      const allPlayerBallsPocketed = playerBalls.every(b => allPocketed.includes(b as BallId));

      if (currentGroup === null || !allPlayerBallsPocketed || newFoul !== null) {
        // Illegal 8-ball pocket - current player loses
        winner = state.currentPlayer === 1 ? 2 : 1;
      } else {
        // Legal 8-ball pocket - current player wins
        winner = state.currentPlayer;
      }
      gameOver = true;
    }

    // Assign ball groups on first legal pocket (if not assigned)
    if (!gameOver && newGroups.player1 === null && pocketed.length > 0) {
      const firstPocketed = pocketed.find(id => id !== 0 && id !== 8);
      if (firstPocketed !== undefined && newFoul === null) {
        const group: BallGroup = firstPocketed >= 1 && firstPocketed <= 7 ? 'solids' : 'stripes';
        if (state.currentPlayer === 1) {
          newGroups = { player1: group, player2: group === 'solids' ? 'stripes' : 'solids' };
        } else {
          newGroups = { player1: group === 'solids' ? 'stripes' : 'solids', player2: group };
        }
      }
    }

    // If no foul and pocketed own balls, same player continues
    if (!gameOver && newFoul === null && pocketed.length > 0) {
      const currentGroup = state.currentPlayer === 1 ? newGroups.player1 : newGroups.player2;
      const pocketedOwnBall = pocketed.some(id => {
        if (id === 0 || id === 8) return false;
        if (currentGroup === 'solids') return id >= 1 && id <= 7;
        if (currentGroup === 'stripes') return id >= 9 && id <= 15;
        return true; // before group assignment, any pocket counts
      });
      if (pocketedOwnBall) {
        nextPlayer = state.currentPlayer;
      }
    }

    if (gameOver) {
      set({
        phase: 'GAME_OVER',
        pocketedBalls: allPocketed,
        foul: newFoul,
        winner,
        ballGroups: newGroups,
        ballInHand: newBallInHand,
      });
    } else {
      set({
        phase: newBallInHand ? 'IDLE' : 'AIMING',
        pocketedBalls: allPocketed,
        foul: newFoul,
        currentPlayer: nextPlayer,
        ballGroups: newGroups,
        ballInHand: newBallInHand,
      });
    }
  },

  nextTurn: () => {
    const state = get();
    set({
      currentPlayer: state.currentPlayer === 1 ? 2 : 1,
      phase: 'AIMING',
    });
  },

  setGameOver: (winner) => set({ phase: 'GAME_OVER', winner }),

  resetGame: () => set(initialState()),

  setPhase: (phase) => set({ phase }),

  setBallInHand: (position) => set({
    ballInHand: position !== null,
    ballInHandPosition: position,
    phase: position !== null ? 'IDLE' : 'AIMING',
  }),

  updateBallPosition: (id, pos) => {
    const state = get();
    set({
      ballPositions: { ...state.ballPositions, [id]: pos },
    });
  },

  takeSnapshot: () => {
    const state = get();
    set({
      snapshot: {
        ballPositions: { ...state.ballPositions },
        state: { ...state } as any,
      },
    });
  },

  undo: () => {
    const state = get();
    if (state.snapshot.ballPositions && state.snapshot.state) {
      const snap = state.snapshot.state;
      set({
        ballPositions: state.snapshot.ballPositions,
        phase: snap.phase,
        currentPlayer: snap.currentPlayer,
        ballGroups: snap.ballGroups,
        pocketedBalls: snap.pocketedBalls,
        scores: snap.scores,
        foul: null,
        winner: null,
        ballInHand: false,
        ballInHandPosition: null,
        snapshot: { ballPositions: null, state: null },
      });
    }
  },
}));
