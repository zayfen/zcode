import type { Board, Move, Piece, Player, Position } from './types';
import { isInCheck } from './checkDetection';
import { COLS, ROWS } from './constants';
import { getRawMoves } from './moveValidation';

// ── Internal helpers ──

/** Create a deep copy of the board and apply the move. */
export function applyMove(board: Board, move: Move): Board {
  const newBoard: (Piece | null)[][] = board.map((row) =>
    row.map((cell) => cell),
  );
  const piece = newBoard[move.from.row]?.[move.from.col];
  newBoard[move.from.row]![move.from.col] = null;
  newBoard[move.to.row]![move.to.col] = piece ?? null;
  return newBoard;
}

/**
 * Flying-general rule: if both generals are on the same column with no
 * piece between them, the position is illegal.
 * Returns false if either general is missing from the board.
 */
export function isFlyingGeneral(board: Board): boolean {
  let redGeneral: Position | undefined;
  let blackGeneral: Position | undefined;

  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      const piece = board[r]?.[c];
      if (piece?.type === 'general') {
        if (piece.player === 'red') redGeneral = { row: r, col: c };
        else blackGeneral = { row: r, col: c };
      }
    }
  }

  if (!redGeneral || !blackGeneral) return false;
  if (redGeneral.col !== blackGeneral.col) return false;

  const minRow = Math.min(redGeneral.row, blackGeneral.row);
  const maxRow = Math.max(redGeneral.row, blackGeneral.row);

  for (let r = minRow + 1; r < maxRow; r++) {
    if (board[r]![redGeneral.col]) {
      return false; // a piece blocks the line of sight
    }
  }
  return true; // generals face each other — illegal
}

// ── Legal move generation ──

/**
 * Return all legal moves for the piece at `from`.
 * Raw moves are filtered to exclude those that:
 *  - leave the mover's own general in check, or
 *  - create a flying-general violation.
 */
export function getLegalMoves(board: Board, from: Position): Position[] {
  const piece = board[from.row]?.[from.col];
  if (!piece) return [];

  const raw = getRawMoves(board, from);

  return raw.filter((to) => {
    const move: Move = { from, to };
    const newBoard = applyMove(board, move);

    // Moving must not create flying-general
    if (isFlyingGeneral(newBoard)) return false;
    // Moving must not leave own general in check
    if (isInCheck(newBoard, piece.player)) return false;

    return true;
  });
}

/**
 * Return true if `player` has at least one legal move available.
 */
export function hasAnyLegalMoves(board: Board, player: Player): boolean {
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      const piece = board[r]?.[c];
      if (piece && piece.player === player) {
        if (getLegalMoves(board, { row: r, col: c }).length > 0) {
          return true;
        }
      }
    }
  }
  return false;
}
