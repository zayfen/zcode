import type { BallId, BallGroup, GamePhase, Player, Foul, GameState, BallState, GameSnapshot } from '../types';
import { create } from 'zustand';
import { getRackPositions } from '../constants/balls';

interface GameActions {
  startAiming: () => void;
  startPower: () => void;
  shoot: () => void;
  setSimulating: () => void;
  evaluateShot: () => void;
  nextTurn: (nextPlayer: Player) => void;
  setGameOver: (winner: Player) => void;
  resetGame: () => void;
  setFoul: (foul: Foul) => void;
  setPhase: (phase: GamePhase) => void;
  assignGroups: (player: Player, group: BallGroup) => void;
  pocketBall: (ballId: BallId) => void;
  setBallInHand: (enabled: boolean, position?: { x: number; y: number; z: number }) => void;
  setBreakShot: (value: boolean) => void;
  saveSnapshot: (ballStates: BallState[]) => void;
  undo: () => GameSnapshot | null;
}

const initialState: GameState = {
  phase: 'IDLE',
  currentPlayer: 1,
  playerGroups: { 1: null, 2: null },
  pocketedBalls: [],
  scores: { 1: 0, 2: 0 },
  foul: null,
  winner: null,
  ballInHand: false,
  ballInHandPosition: null,
  breakShot: true,
  groupsAssigned: false,
};

let snapshotStack: GameSnapshot[] = [];

export const useGameStore = create<GameState & GameActions>((set, get) => ({
  ...initialState,

  startAiming: () => {
    const state = get();
    if (state.phase === 'IDLE') {
      set({ phase: 'AIMING' });
    }
  },

  startPower: () => {
    const state = get();
    if (state.phase === 'AIMING') {
      set({ phase: 'POWER' });
    }
  },

  shoot: () => {
    const state = get();
    if (state.phase === 'POWER') {
      set({ phase: 'SIMULATING', foul: null });
    }
  },

  setSimulating: () => {
    set({ phase: 'SIMULATING' });
  },

  evaluateShot: () => {
    set({ phase: 'EVALUATING' });
  },

  nextTurn: (nextPlayer: Player) => {
    set({
      phase: 'IDLE',
      currentPlayer: nextPlayer,
      foul: null,
      ballInHand: false,
      ballInHandPosition: null,
    });
  },

  setGameOver: (winner: Player) => {
    set({ phase: 'GAME_OVER', winner });
  },

  resetGame: () => {
    snapshotStack = [];
    set({ ...initialState });
  },

  setFoul: (foul: Foul) => {
    set({ foul });
  },

  setPhase: (phase: GamePhase) => {
    set({ phase });
  },

  assignGroups: (player: Player, group: BallGroup) => {
    const otherGroup: BallGroup = group === 'solids' ? 'stripes' : 'solids';
    const otherPlayer: Player = player === 1 ? 2 : 1;
    set({
      playerGroups: { [player]: group, [otherPlayer]: otherGroup } as Record<Player, BallGroup>,
      groupsAssigned: true,
    });
  },

  pocketBall: (ballId: BallId) => {
    const state = get();
    if (!state.pocketedBalls.includes(ballId)) {
      set({ pocketedBalls: [...state.pocketedBalls, ballId] });
    }
  },

  setBallInHand: (enabled: boolean, position?: { x: number; y: number; z: number }) => {
    set({
      ballInHand: enabled,
      ballInHandPosition: position ?? null,
    });
  },

  setBreakShot: (value: boolean) => {
    set({ breakShot: value });
  },

  saveSnapshot: (ballStates: BallState[]) => {
    const state = get();
    const snapshot: GameSnapshot = {
      gameState: {
        phase: state.phase,
        currentPlayer: state.currentPlayer,
        playerGroups: { ...state.playerGroups },
        pocketedBalls: [...state.pocketedBalls],
        scores: { ...state.scores },
        foul: state.foul,
        winner: state.winner,
        ballInHand: state.ballInHand,
        ballInHandPosition: state.ballInHandPosition
          ? { ...state.ballInHandPosition }
          : null,
        breakShot: state.breakShot,
        groupsAssigned: state.groupsAssigned,
      },
      ballStates: ballStates.map((b) => ({ ...b })),
    };
    snapshotStack.push(snapshot);
    if (snapshotStack.length > 2) snapshotStack.shift();
  },

  undo: () => {
    if (snapshotStack.length === 0) return null;
    const snapshot = snapshotStack.pop()!;
    set({ ...snapshot.gameState });
    return snapshot;
  },
}));
