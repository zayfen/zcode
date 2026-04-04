import type { BallId, BallGroup, Foul, GameState, ShotEvaluation, Player } from '../types';
import { getBallGroup } from '../constants/balls';

export interface ShotData {
  pocketed: BallId[];
  firstContact: BallId | null;
  cueBallPocketed: boolean;
  cueBallStopped: { x: number; y: number; z: number };
}

export function evaluateShotResult(shot: ShotData, state: GameState): ShotEvaluation {
  const { pocketed, firstContact, cueBallPocketed } = shot;
  const { currentPlayer, playerGroups, groupsAssigned, breakShot } = state;

  let foul: Foul = null;
  let gameOver = false;
  let winner: Player | null = null;
  let ballGroupAssigned = false;

  // --- Foul detection ---

  // 1. Scratch (cue ball pocketed)
  if (cueBallPocketed) {
    foul = 'SCRATCH';
  }

  // 2. No ball hit
  if (!firstContact && !foul) {
    foul = 'NO_BALL_HIT';
  }

  // 3. Wrong ball first contact (only if groups assigned and not break)
  if (firstContact !== null && groupsAssigned && !breakShot) {
    const playerGroup = playerGroups[currentPlayer];
    if (playerGroup !== null) {
      const contactGroup = getBallGroup(firstContact);
      // Must hit own group first, unless all own balls are pocketed
      const ownBallsRemaining = hasOwnBallsRemaining(state);
      if (ownBallsRemaining && contactGroup !== playerGroup) {
        foul = 'WRONG_BALL_FIRST';
      }
    }
  }

  // --- Group assignment (on first legal pocket after break) ---
  if (!foul && !groupsAssigned && pocketed.length > 0) {
    const firstPocketed = pocketed.find(id => id !== 0 && id !== 8);
    if (firstPocketed !== undefined) {
      const group = getBallGroup(firstPocketed);
      if (group !== null) {
        ballGroupAssigned = true;
        // Player who pockets gets that group
        // But we signal this so the caller can assign
      }
    }
  }

  // --- 8-ball game over checks ---
  const eightBallPocketed = pocketed.includes(8 as BallId);
  if (eightBallPocketed) {
    if (breakShot) {
      // 8-ball on break: re-spot it (no game over)
      // We handle this by not ending the game
    } else if (!groupsAssigned) {
      // Early 8-ball before groups assigned: lose
      gameOver = true;
      winner = currentPlayer === 1 ? 2 : 1;
    } else if (foul) {
      // Scratched while pocketing 8-ball: lose
      gameOver = true;
      winner = currentPlayer === 1 ? 2 : 1;
    } else {
      // Check if player has cleared their group
      const playerGroup = playerGroups[currentPlayer];
      if (playerGroup && !hasOwnBallsRemaining(state)) {
        // Legal 8-ball pocket: win!
        gameOver = true;
        winner = currentPlayer;
      } else {
        // Early 8-ball: lose
        gameOver = true;
        winner = currentPlayer === 1 ? 2 : 1;
      }
    }
  }

  // --- Next player ---
  let nextPlayer: Player;
  if (foul) {
    nextPlayer = currentPlayer === 1 ? 2 : 1;
  } else {
    // If player pocketed their own ball legally, they go again
    const pocketedOwnBall = pocketed.some(id => {
      if (id === 0 || id === 8) return false;
      const group = getBallGroup(id);
      if (!groupsAssigned) return true; // Before groups assigned, any pocket continues turn
      return group === playerGroups[currentPlayer];
    });
    nextPlayer = pocketedOwnBall ? currentPlayer : (currentPlayer === 1 ? 2 : 1);
  }

  return {
    foul,
    nextPlayer,
    pocketed,
    gameOver,
    winner,
    ballGroupAssigned,
  };
}

function hasOwnBallsRemaining(state: GameState): boolean {
  const { currentPlayer, playerGroups, pocketedBalls } = state;
  const group = playerGroups[currentPlayer];
  if (!group) return true;

  const groupBallIds = group === 'solids'
    ? [1, 2, 3, 4, 5, 6, 7] as BallId[]
    : [9, 10, 11, 12, 13, 14, 15] as BallId[];

  return groupBallIds.some(id => !pocketedBalls.includes(id));
}
