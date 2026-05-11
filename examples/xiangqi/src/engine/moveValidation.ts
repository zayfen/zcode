import type { Board, Piece, Player, Position } from './types';
import { isInBounds, isInPalace, isOnSide, RIVER_ROW_MIN, RIVER_ROW_MAX } from './constants';

// ── Direction offsets ──

const KNIGHT_OFFSETS: ReadonlyArray<{ dr: number; dc: number; blockDr: number; blockDc: number }> = [
  { dr: -2, dc: -1, blockDr: -1, blockDc: 0 },
  { dr: -2, dc: 1, blockDr: -1, blockDc: 0 },
  { dr: -1, dc: -2, blockDr: 0, blockDc: -1 },
  { dr: -1, dc: 2, blockDr: 0, blockDc: 1 },
  { dr: 1, dc: -2, blockDr: 0, blockDc: -1 },
  { dr: 1, dc: 2, blockDr: 0, blockDc: 1 },
  { dr: 2, dc: -1, blockDr: 1, blockDc: 0 },
  { dr: 2, dc: 1, blockDr: 1, blockDc: 0 },
];

const ELEPHANT_OFFSETS: ReadonlyArray<{ dr: number; dc: number; eyeDr: number; eyeDc: number }> = [
  { dr: -2, dc: -2, eyeDr: -1, eyeDc: -1 },
  { dr: -2, dc: 2, eyeDr: -1, eyeDc: 1 },
  { dr: 2, dc: -2, eyeDr: 1, eyeDc: -1 },
  { dr: 2, dc: 2, eyeDr: 1, eyeDc: 1 },
];

const ADVISOR_OFFSETS: ReadonlyArray<{ dr: number; dc: number }> = [
  { dr: -1, dc: -1 },
  { dr: -1, dc: 1 },
  { dr: 1, dc: -1 },
  { dr: 1, dc: 1 },
];

const GENERAL_OFFSETS: ReadonlyArray<{ dr: number; dc: number }> = [
  { dr: -1, dc: 0 },
  { dr: 1, dc: 0 },
  { dr: 0, dc: -1 },
  { dr: 0, dc: 1 },
];

// ── Helper: get piece at position (null-safe) ──

function getPiece(board: Board, pos: Position): Piece | null {
  return board[pos.row]?.[pos.col] ?? null;
}

const CANNON_DIRECTIONS: ReadonlyArray<readonly [number, number]> = [[-1, 0], [1, 0], [0, -1], [0, 1]];

// ── Per-piece raw move generators ──

/**
 * Generate all raw (unfiltered) moves for the piece at `from`.
 * These do NOT check whether the move leaves the player's own general in check.
 */
export function getRawMoves(board: Board, from: Position): Position[] {
  const piece = getPiece(board, from);
  if (!piece) return [];

  switch (piece.type) {
    case 'chariot':
      return chariotMoves(board, from, piece.player);
    case 'knight':
      return knightMoves(board, from, piece.player);
    case 'elephant':
      return elephantMoves(board, from, piece.player);
    case 'advisor':
      return advisorMoves(board, from, piece.player);
    case 'general':
      return generalMoves(board, from, piece.player);
    case 'cannon':
      return cannonMoves(board, from, piece.player);
    case 'soldier':
      return soldierMoves(board, from, piece.player);
    default:
      return [];
  }
}

// ── Chariot (车): straight-line, blocked by first piece ──

function chariotMoves(board: Board, from: Position, player: Player): Position[] {
  const moves: Position[] = [];

  for (const [dr, dc] of CANNON_DIRECTIONS) {
    let r = from.row + dr;
    let c = from.col + dc;
    while (isInBounds(r, c)) {
      const target = board[r]?.[c] ?? null;
      if (target === null) {
        moves.push({ row: r, col: c });
      } else {
        if (target.player !== player) {
          moves.push({ row: r, col: c }); // capture
        }
        break; // blocked
      }
      r += dr;
      c += dc;
    }
  }

  return moves;
}

// ── Knight (马): L-shape, 蹩马腿 ──

function knightMoves(board: Board, from: Position, player: Player): Position[] {
  const moves: Position[] = [];

  for (const { dr, dc, blockDr, blockDc } of KNIGHT_OFFSETS) {
    const blockRow = from.row + blockDr;
    const blockCol = from.col + blockDc;

    // Check 蹩马腿
    if (!isInBounds(blockRow, blockCol)) continue;
    if (getPiece(board, { row: blockRow, col: blockCol }) !== null) continue;

    const toRow = from.row + dr;
    const toCol = from.col + dc;
    if (!isInBounds(toRow, toCol)) continue;

    const target = getPiece(board, { row: toRow, col: toCol });
    if (target === null || target.player !== player) {
      moves.push({ row: toRow, col: toCol });
    }
  }

  return moves;
}

// ── Elephant (象): diagonal-2 (田), 塞象眼, river boundary ──

function elephantMoves(board: Board, from: Position, player: Player): Position[] {
  const moves: Position[] = [];

  for (const { dr, dc, eyeDr, eyeDc } of ELEPHANT_OFFSETS) {
    const toRow = from.row + dr;
    const toCol = from.col + dc;
    if (!isInBounds(toRow, toCol)) continue;

    // Cannot cross river: destination must be on own side
    if (!isOnSide(toRow, player)) continue;

    // Check 塞象眼 (eye of the 田)
    const eyeRow = from.row + eyeDr;
    const eyeCol = from.col + eyeDc;
    if (getPiece(board, { row: eyeRow, col: eyeCol }) !== null) continue;

    const target = getPiece(board, { row: toRow, col: toCol });
    if (target === null || target.player !== player) {
      moves.push({ row: toRow, col: toCol });
    }
  }

  return moves;
}

// ── Advisor (士): one-step diagonal within palace ──

function advisorMoves(board: Board, from: Position, player: Player): Position[] {
  const moves: Position[] = [];

  for (const { dr, dc } of ADVISOR_OFFSETS) {
    const toRow = from.row + dr;
    const toCol = from.col + dc;
    if (!isInBounds(toRow, toCol)) continue;
    if (!isInPalace({ row: toRow, col: toCol }, player)) continue;

    const target = getPiece(board, { row: toRow, col: toCol });
    if (target === null || target.player !== player) {
      moves.push({ row: toRow, col: toCol });
    }
  }

  return moves;
}

// ── General (将/帅): one-step orthogonal within palace ──

function generalMoves(board: Board, from: Position, player: Player): Position[] {
  const moves: Position[] = [];

  for (const { dr, dc } of GENERAL_OFFSETS) {
    const toRow = from.row + dr;
    const toCol = from.col + dc;
    if (!isInBounds(toRow, toCol)) continue;
    if (!isInPalace({ row: toRow, col: toCol }, player)) continue;

    const target = getPiece(board, { row: toRow, col: toCol });
    if (target === null || target.player !== player) {
      moves.push({ row: toRow, col: toCol });
    }
  }

  return moves;
}

// ── Cannon (炮): straight-line; captures by jumping exactly one screen ──

function cannonMoves(board: Board, from: Position, player: Player): Position[] {
  const moves: Position[] = [];

  for (const [dr, dc] of CANNON_DIRECTIONS) {
    let r = from.row + dr;
    let c = from.col + dc;
    let jumped = false;

    while (isInBounds(r, c)) {
      const target = board[r]?.[c] ?? null;
      if (!jumped) {
        if (target === null) {
          moves.push({ row: r, col: c }); // non-capture move
        } else {
          jumped = true; // found screen
        }
      } else {
        if (target !== null) {
          if (target.player !== player) {
            moves.push({ row: r, col: c }); // capture over screen
          }
          break; // stop after first piece behind screen
        }
      }
      r += dr;
      c += dc;
    }
  }

  return moves;
}

// ── Soldier (兵/卒): forward before river; forward + sideways after river ──

function soldierMoves(board: Board, from: Position, player: Player): Position[] {
  const moves: Position[] = [];
  const forward = player === 'red' ? -1 : 1;
  const crossedRiver = player === 'red'
    ? from.row <= RIVER_ROW_MIN
    : from.row >= RIVER_ROW_MAX;

  // Forward move
  const fwdRow = from.row + forward;
  if (isInBounds(fwdRow, from.col)) {
    const target = getPiece(board, { row: fwdRow, col: from.col });
    if (target === null || target.player !== player) {
      moves.push({ row: fwdRow, col: from.col });
    }
  }

  // Sideways (only after crossing river)
  if (crossedRiver) {
    for (const dc of [-1, 1]) {
      const sideCol = from.col + dc;
      if (isInBounds(from.row, sideCol)) {
        const target = getPiece(board, { row: from.row, col: sideCol });
        if (target === null || target.player !== player) {
          moves.push({ row: from.row, col: sideCol });
        }
      }
    }
  }

  return moves;
}
