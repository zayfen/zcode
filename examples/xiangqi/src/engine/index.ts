// ── Public Xiangqi Engine API ──

import type { Board, BoardState, GameStatus, Move, MoveResult, Player } from './types';
import { isInCheck } from './checkDetection';
import { applyMove, getLegalMoves, hasAnyLegalMoves } from './specialRules';

// ── Move execution ──

/**
 * Attempt to execute `move` on the given `state`.
 * Returns a `MoveResult` indicating whether the move was valid and,
 * if so, the new `BoardState` and any captured piece.
 */
export function makeMove(state: BoardState, move: Move): MoveResult {
  const { board, currentPlayer } = state;
  const piece = board[move.from.row]?.[move.from.col];

  // Validate: there must be a piece belonging to the current player
  if (!piece || piece.player !== currentPlayer) {
    return { valid: false };
  }

  // Validate: destination must be a legal move
  const legal = getLegalMoves(board, move.from);
  const isLegal = legal.some(
    (p) => p.row === move.to.row && p.col === move.to.col,
  );
  if (!isLegal) {
    return { valid: false };
  }

  // Determine captured piece (if any)
  const targetPiece = board[move.to.row]?.[move.to.col];
  const captured = targetPiece ?? undefined;

  // Apply move
  const newBoard = applyMove(board, move);

  const opponent: Player = currentPlayer === 'red' ? 'black' : 'red';

  const newState: BoardState = {
    board: newBoard,
    currentPlayer: opponent,
  };

  return { valid: true, newState, captured };
}

// ── Game-over detection ──

/**
 * Checkmate: the player is in check AND has no legal moves.
 */
export function isCheckmate(board: Board, player: Player): boolean {
  return isInCheck(board, player) && !hasAnyLegalMoves(board, player);
}

/**
 * Stalemate: the player is NOT in check but has no legal moves.
 * (In Xiangqi, stalemate is a loss for the stalemated player.)
 */
export function isStalemate(board: Board, player: Player): boolean {
  return !isInCheck(board, player) && !hasAnyLegalMoves(board, player);
}

/**
 * Evaluate the current game status.
 * Checks whether the `currentPlayer` (the one who must move next)
 * is in checkmate or stalemate.
 */
export function getGameStatus(state: BoardState): GameStatus {
  const { board, currentPlayer } = state;
  const opponent: Player = currentPlayer === 'red' ? 'black' : 'red';

  if (isCheckmate(board, currentPlayer)) {
    return { type: 'checkmate', winner: opponent };
  }
  if (isStalemate(board, currentPlayer)) {
    return { type: 'stalemate', loser: currentPlayer };
  }
  return { type: 'playing' };
}

// ── Re-exports for convenience ──

export type { Board, BoardState, GameStatus, Move, MoveResult, Piece, PieceType, Player, Position } from './types';
export { createInitialBoard, createInitialBoardState, INITIAL_POSITIONS } from './constants';
export type { InitialPlacement } from './constants';
export { isInCheck, isSquareAttackedBy, findGeneralPosition } from './checkDetection';
export { getLegalMoves, hasAnyLegalMoves, isFlyingGeneral } from './specialRules';
