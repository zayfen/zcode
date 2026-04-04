import type { BallId, BallGroup, Foul, GameState, ShotResult } from '../types';
import { SOLIDS, STRIPES, EIGHT_BALL } from '../types';

interface ShotEvaluation {
  foul: Foul;
  nextPlayer: 1 | 2;
  pocketed: BallId[];
  gameOver: boolean;
  winner: 1 | 2 | null;
  assignGroup?: { player: 1 | 2; group: BallGroup };
}

export function evaluateShotResult(
  shot: ShotResult,
  state: GameState,
  remainingBalls: BallId[]
): ShotEvaluation {
  const result: ShotEvaluation = {
    foul: null,
    nextPlayer: state.currentPlayer,
    pocketed: shot.pocketed,
    gameOver: false,
    winner: null,
  };

  const currentPlayerGroup = state.currentPlayer === 1 ? state.ballGroups.player1 : state.ballGroups.player2;
  const opponentGroup = state.currentPlayer === 1 ? state.ballGroups.player2 : state.ballGroups.player1;

  // Check for scratch (cue ball pocketed)
  const cuePocketed = shot.pocketed.includes(0);
  if (cuePocketed) {
    result.foul = 'SCRATCH';
  }

  // Check if no ball was hit
  if (shot.firstContact === null) {
    result.foul = 'NO_BALL_HIT';
  }

  // Check wrong ball first contact
  if (shot.firstContact !== null && currentPlayerGroup !== null && !cuePocketed) {
    const firstBallGroup = getGroupForBall(shot.firstContact);
    const playerRemaining = remainingBalls.filter(id => getGroupForBall(id) === currentPlayerGroup);

    if (playerRemaining.length > 0) {
      // Player still has balls of their group to pocket
      if (firstBallGroup !== currentPlayerGroup && firstBallGroup !== 'eight') {
        result.foul = 'WRONG_BALL_FIRST';
      }
    } else {
      // Player has cleared their group, must hit 8-ball
      if (firstBallGroup !== 'eight') {
        result.foul = 'WRONG_BALL_FIRST';
      }
    }
  }

  // Check if 8-ball was pocketed
  const eightBallPocketed = shot.pocketed.includes(EIGHT_BALL);
  if (eightBallPocketed) {
    result.gameOver = true;
    if (result.foul !== null) {
      // Pocketed 8-ball with a foul = lose
      result.winner = state.currentPlayer === 1 ? 2 : 1;
    } else if (currentPlayerGroup !== null) {
      const playerRemaining = remainingBalls.filter(id => getGroupForBall(id) === currentPlayerGroup);
      if (playerRemaining.length > 0) {
        // Pocketed 8-ball before clearing group = lose
        result.winner = state.currentPlayer === 1 ? 2 : 1;
      } else {
        // Legal 8-ball pocket = win
        result.winner = state.currentPlayer;
      }
    } else {
      // Groups not assigned yet, 8-ball on break or early = lose
      result.winner = state.currentPlayer === 1 ? 2 : 1;
    }
  }

  // Assign groups on first legal pocket (if unassigned)
  if (!result.gameOver && state.ballGroups.player1 === null) {
    const legalPocketed = shot.pocketed.filter(id => id !== 0);
    if (legalPocketed.length > 0 && result.foul === null) {
      const firstPocketed = legalPocketed[0];
      const group = getGroupForBall(firstPocketed);
      if (group === 'solids' || group === 'stripes') {
        result.assignGroup = {
          player: state.currentPlayer,
          group: group,
        };
      }
    }
  }

  // Determine next player
  if (!result.gameOver) {
    if (result.foul !== null) {
      result.nextPlayer = state.currentPlayer === 1 ? 2 : 1;
    } else {
      const legalPocketed = shot.pocketed.filter(id => id !== 0);
      const playerPocketedOwnBall = legalPocketed.some(id => {
        const g = getGroupForBall(id);
        return g === currentPlayerGroup;
      });
      if (playerPocketedOwnBall && currentPlayerGroup !== null) {
        result.nextPlayer = state.currentPlayer;
      } else {
        result.nextPlayer = state.currentPlayer === 1 ? 2 : 1;
      }
    }
  }

  return result;
}

function getGroupForBall(ballId: BallId): 'solids' | 'stripes' | 'eight' | 'cue' {
  if (ballId === 0) return 'cue';
  if (ballId === 8) return 'eight';
  if (ballId >= 1 && ballId <= 7) return 'solids';
  return 'stripes';
}
