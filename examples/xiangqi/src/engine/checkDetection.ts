import type { Board, Player, Position } from './types';
import { COLS, ROWS } from './constants';
import { getRawMoves } from './moveValidation';

// ── Attack & Check Detection ──

/**
 * Return true if ANY piece belonging to `attacker` can reach `pos`
 * via a raw move (i.e. the square is attacked).
 *
 * This iterates every board square, and for each piece owned by `attacker`
 * generates its raw moves. If any raw move targets `pos`, the square is
 * considered attacked.  Raw moves already encode piece-specific rules
 * (line-of-sight blocking, cannon screens, knight leg-blocks, etc.), so
 * no additional filtering is required here.
 */
export function isSquareAttackedBy(
  board: Board,
  pos: Position,
  attacker: Player,
): boolean {
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      const piece = board[r]?.[c];
      if (piece && piece.player === attacker) {
        const moves = getRawMoves(board, { row: r, col: c });
        for (const m of moves) {
          if (m.row === pos.row && m.col === pos.col) {
            return true;
          }
        }
      }
    }
  }
  return false;
}

/**
 * Find the position of the given player's general on the board.
 * Throws if the general is not found (should never happen in a valid game).
 */
export function findGeneralPosition(
  board: Board,
  player: Player,
): Position {
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      const piece = board[r]?.[c];
      if (piece && piece.type === 'general' && piece.player === player) {
        return { row: r, col: c };
      }
    }
  }
  throw new Error(`General not found for player ${player}`);
}

/** Return the opponent of the given player. */
function opponent(player: Player): Player {
  return player === 'red' ? 'black' : 'red';
}

/**
 * Flying-general helper: if both generals are on the same column with no
 * intervening piece, return the player whose general is "behind" (higher
 * row index) — that player is considered to be in check from the
 * "flying general" attack.
 * Returns `null` when the generals are not facing each other.
 */
function flyingGeneralTarget(board: Board): Player | null {
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

  if (!redGeneral || !blackGeneral) return null;
  if (redGeneral.col !== blackGeneral.col) return null;

  const minRow = Math.min(redGeneral.row, blackGeneral.row);
  const maxRow = Math.max(redGeneral.row, blackGeneral.row);

  for (let r = minRow + 1; r < maxRow; r++) {
    if (board[r]![redGeneral.col]) {
      return null; // a piece blocks the line of sight
    }
  }

  // Generals face each other — the one with the higher row is "in check"
  return redGeneral.row > blackGeneral.row ? 'red' : 'black';
}

/**
 * Flying-general rule: if both generals are on the same column with no
 * piece between them, the position is illegal.
 * Returns false if either general is missing from the board.
 */
export function isFlyingGeneral(board: Board): boolean {
  return flyingGeneralTarget(board) !== null;
}

/**
 * Return true if `player`'s general is currently in check.
 *
 * A player is in check when either:
 *  1. An opponent piece has a raw move targeting the general's square, OR
 *  2. The flying-general rule applies — both generals face each other on
 *     the same column with no intervening pieces — and `player` is the
 *     one whose general sits behind the other (higher row index).
 */
export function isInCheck(board: Board, player: Player): boolean {
  const generalPos = findGeneralPosition(board, player);

  // Standard check: an opponent piece can capture the general
  if (isSquareAttackedBy(board, generalPos, opponent(player))) {
    return true;
  }

  // Flying-general check: both generals facing each other is illegal.
  // Only the player whose general is "behind" (higher row) is in check.
  if (flyingGeneralTarget(board) === player) {
    return true;
  }

  return false;
}
