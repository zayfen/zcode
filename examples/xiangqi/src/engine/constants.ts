import type { Board, BoardState, Player, PieceType, Piece } from './types';

// ── Board dimensions ──
export const ROWS = 10; // 10 rows (0–9)
export const COLS = 9;  // 9 columns (0–8)

// ── Palace boundaries ──
export const RED_PALACE = { minRow: 7, maxRow: 9, minCol: 3, maxCol: 5 } as const;
export const BLACK_PALACE = { minRow: 0, maxRow: 2, minCol: 3, maxCol: 5 } as const;

// ── River ──
export const RIVER_ROW_MIN = 4; // row indices 4 and 5 form the river boundary
export const RIVER_ROW_MAX = 5;

// ── Chinese characters for pieces ──
const PIECE_CHARS: Record<Player, Record<PieceType, string>> = {
  red: {
    general: '帥',
    advisor: '仕',
    elephant: '相',
    knight: '馬',
    chariot: '車',
    cannon: '炮',
    soldier: '兵',
  },
  black: {
    general: '將',
    advisor: '士',
    elephant: '象',
    knight: '馬',
    chariot: '車',
    cannon: '砲',
    soldier: '卒',
  },
};

export function getPieceChar(player: Player, type: PieceType): string {
  return PIECE_CHARS[player][type];
}

/** Check if a position is inside the palace for the given player. */
export function isInPalace(pos: { row: number; col: number }, player: Player): boolean {
  const palace = player === 'red' ? RED_PALACE : BLACK_PALACE;
  return (
    pos.row >= palace.minRow &&
    pos.row <= palace.maxRow &&
    pos.col >= palace.minCol &&
    pos.col <= palace.maxCol
  );
}

/** Check if a row is on a given player's side of the river. */
export function isOnSide(row: number, player: Player): boolean {
  return player === 'red' ? row >= RIVER_ROW_MAX : row <= RIVER_ROW_MIN;
}

/** Check if the position is within board bounds. */
export function isInBounds(row: number, col: number): boolean {
  return row >= 0 && row < ROWS && col >= 0 && col < COLS;
}

// ── Initial board setup ──

/** Describes a single piece's starting placement on the board. */
export interface InitialPlacement {
  readonly row: number;
  readonly col: number;
  readonly type: PieceType;
  readonly player: Player;
}

/**
 * Starting positions for all 32 Xiangqi pieces.
 *
 * Layout (row 0 = black back rank at top, row 9 = red back rank at bottom):
 *
 *   Row 0: 車 馬 象 士 將 士 象 馬 車   (black back rank)
 *   Row 1: · · · · · · · · ·
 *   Row 2: · 砲 · · · · · 砲 ·           (black cannons)
 *   Row 3: 卒 · 卒 · 卒 · 卒 · 卒       (black soldiers)
 *   Row 4: · · · · · · · · ·  ─ river
 *   Row 5: · · · · · · · · ·  ─ river
 *   Row 6: 兵 · 兵 · 兵 · 兵 · 兵       (red soldiers)
 *   Row 7: · 炮 · · · · · 炮 ·           (red cannons)
 *   Row 8: · · · · · · · · ·
 *   Row 9: 車 馬 相 仕 帥 仕 相 馬 車   (red back rank)
 */
export const INITIAL_POSITIONS: readonly InitialPlacement[] = [
  // ── Black back rank (row 0) ──
  { row: 0, col: 0, type: 'chariot',  player: 'black' },
  { row: 0, col: 1, type: 'knight',   player: 'black' },
  { row: 0, col: 2, type: 'elephant', player: 'black' },
  { row: 0, col: 3, type: 'advisor',  player: 'black' },
  { row: 0, col: 4, type: 'general',  player: 'black' },
  { row: 0, col: 5, type: 'advisor',  player: 'black' },
  { row: 0, col: 6, type: 'elephant', player: 'black' },
  { row: 0, col: 7, type: 'knight',   player: 'black' },
  { row: 0, col: 8, type: 'chariot',  player: 'black' },

  // ── Black cannons (row 2) ──
  { row: 2, col: 1, type: 'cannon', player: 'black' },
  { row: 2, col: 7, type: 'cannon', player: 'black' },

  // ── Black soldiers (row 3, even columns) ──
  { row: 3, col: 0, type: 'soldier', player: 'black' },
  { row: 3, col: 2, type: 'soldier', player: 'black' },
  { row: 3, col: 4, type: 'soldier', player: 'black' },
  { row: 3, col: 6, type: 'soldier', player: 'black' },
  { row: 3, col: 8, type: 'soldier', player: 'black' },

  // ── Red soldiers (row 6, even columns) ──
  { row: 6, col: 0, type: 'soldier', player: 'red' },
  { row: 6, col: 2, type: 'soldier', player: 'red' },
  { row: 6, col: 4, type: 'soldier', player: 'red' },
  { row: 6, col: 6, type: 'soldier', player: 'red' },
  { row: 6, col: 8, type: 'soldier', player: 'red' },

  // ── Red cannons (row 7) ──
  { row: 7, col: 1, type: 'cannon', player: 'red' },
  { row: 7, col: 7, type: 'cannon', player: 'red' },

  // ── Red back rank (row 9) ──
  { row: 9, col: 0, type: 'chariot',  player: 'red' },
  { row: 9, col: 1, type: 'knight',   player: 'red' },
  { row: 9, col: 2, type: 'elephant', player: 'red' },
  { row: 9, col: 3, type: 'advisor',  player: 'red' },
  { row: 9, col: 4, type: 'general',  player: 'red' },
  { row: 9, col: 5, type: 'advisor',  player: 'red' },
  { row: 9, col: 6, type: 'elephant', player: 'red' },
  { row: 9, col: 7, type: 'knight',   player: 'red' },
  { row: 9, col: 8, type: 'chariot',  player: 'red' },
] as const;

// Runtime assertion: exactly 32 starting pieces
if (INITIAL_POSITIONS.length !== 32) {
  throw new Error(`INITIAL_POSITIONS must contain exactly 32 entries, got ${INITIAL_POSITIONS.length}`);
}

function p(type: PieceType, player: Player): Piece {
  return { type, player };
}

/**
 * Returns the initial Xiangqi board (10 rows × 9 cols).
 * Row 0 = Black back rank (top), Row 9 = Red back rank (bottom).
 */
export function createInitialBoard(): Board {
  const board: (Piece | null)[][] = Array.from({ length: ROWS }, () =>
    Array.from({ length: COLS }, () => null),
  );

  for (const { row, col, type, player } of INITIAL_POSITIONS) {
    board[row]![col] = p(type, player);
  }

  return board;
}

export function createInitialBoardState(): BoardState {
  return {
    board: createInitialBoard(),
    currentPlayer: 'red',
  };
}
