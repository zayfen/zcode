import { describe, it, expect } from 'vitest';
import { createInitialBoard, createInitialBoardState } from '../constants';
import type { Board, BoardState, Piece, Player, PieceType } from '../types';

// ── Helpers ──

/** Count all pieces on the board, optionally filtered by player. */
function countPieces(board: Board, player?: Player): number {
  let count = 0;
  for (const row of board) {
    for (const cell of row) {
      if (cell && (player === undefined || cell.player === player)) {
        count++;
      }
    }
  }
  return count;
}

/** Get the piece at a given position, or null. */
function pieceAt(board: Board, row: number, col: number): Piece | null {
  return board[row]?.[col] ?? null;
}

/** Assert that a cell contains a specific piece type and player. */
function expectPiece(
  board: Board,
  row: number,
  col: number,
  type: PieceType,
  player: Player,
): void {
  const p = pieceAt(board, row, col);
  expect(p).not.toBeNull();
  expect(p!.type).toBe(type);
  expect(p!.player).toBe(player);
}

// ── Tests ──

describe('createInitialBoard', () => {
  it('returns a 10×9 board', () => {
    const board = createInitialBoard();
    expect(board).toHaveLength(10);
    for (const row of board) {
      expect(row).toHaveLength(9);
    }
  });

  it('places exactly 32 pieces (16 red, 16 black)', () => {
    const board = createInitialBoard();
    expect(countPieces(board)).toBe(32);
    expect(countPieces(board, 'red')).toBe(16);
    expect(countPieces(board, 'black')).toBe(16);
  });

  // ── Black pieces ──

  it('places black back rank correctly (row 0)', () => {
    const board = createInitialBoard();
    expectPiece(board, 0, 0, 'chariot', 'black');
    expectPiece(board, 0, 1, 'knight', 'black');
    expectPiece(board, 0, 2, 'elephant', 'black');
    expectPiece(board, 0, 3, 'advisor', 'black');
    expectPiece(board, 0, 4, 'general', 'black');
    expectPiece(board, 0, 5, 'advisor', 'black');
    expectPiece(board, 0, 6, 'elephant', 'black');
    expectPiece(board, 0, 7, 'knight', 'black');
    expectPiece(board, 0, 8, 'chariot', 'black');
  });

  it('places black cannons at row 2, cols 1 and 7', () => {
    const board = createInitialBoard();
    expectPiece(board, 2, 1, 'cannon', 'black');
    expectPiece(board, 2, 7, 'cannon', 'black');
  });

  it('places black soldiers at row 3, even cols', () => {
    const board = createInitialBoard();
    for (let c = 0; c < 9; c += 2) {
      expectPiece(board, 3, c, 'soldier', 'black');
    }
    // Odd cols on row 3 should be empty
    for (let c = 1; c < 9; c += 2) {
      expect(pieceAt(board, 3, c)).toBeNull();
    }
  });

  // ── Red pieces ──

  it('places red back rank correctly (row 9)', () => {
    const board = createInitialBoard();
    expectPiece(board, 9, 0, 'chariot', 'red');
    expectPiece(board, 9, 1, 'knight', 'red');
    expectPiece(board, 9, 2, 'elephant', 'red');
    expectPiece(board, 9, 3, 'advisor', 'red');
    expectPiece(board, 9, 4, 'general', 'red');
    expectPiece(board, 9, 5, 'advisor', 'red');
    expectPiece(board, 9, 6, 'elephant', 'red');
    expectPiece(board, 9, 7, 'knight', 'red');
    expectPiece(board, 9, 8, 'chariot', 'red');
  });

  it('places red cannons at row 7, cols 1 and 7', () => {
    const board = createInitialBoard();
    expectPiece(board, 7, 1, 'cannon', 'red');
    expectPiece(board, 7, 7, 'cannon', 'red');
  });

  it('places red soldiers at row 6, even cols', () => {
    const board = createInitialBoard();
    for (let c = 0; c < 9; c += 2) {
      expectPiece(board, 6, c, 'soldier', 'red');
    }
    // Odd cols on row 6 should be empty
    for (let c = 1; c < 9; c += 2) {
      expect(pieceAt(board, 6, c)).toBeNull();
    }
  });

  // ── Empty rows ──

  it('leaves rows 1, 4, 5, and 8 completely empty', () => {
    const board = createInitialBoard();
    for (const row of [1, 4, 5, 8]) {
      for (let c = 0; c < 9; c++) {
        expect(pieceAt(board, row, c)).toBeNull();
      }
    }
  });

  it('places generals inside their palaces', () => {
    const board = createInitialBoard();
    // Black general at (0, 4) — inside black palace [0-2, 3-5]
    expectPiece(board, 0, 4, 'general', 'black');
    // Red general at (9, 4) — inside red palace [7-9, 3-5]
    expectPiece(board, 9, 4, 'general', 'red');
  });

  it('is symmetric — mirrors red/black positions across the river', () => {
    const board = createInitialBoard();
    // Back rank symmetry: row 0 ↔ row 9
    for (let c = 0; c < 9; c++) {
      const top = pieceAt(board, 0, c)!;
      const bottom = pieceAt(board, 9, c)!;
      expect(top.type).toBe(bottom.type);
      expect(top.player).toBe('black');
      expect(bottom.player).toBe('red');
    }
    // Cannon symmetry: row 2 ↔ row 7
    expect(pieceAt(board, 2, 1)!.type).toBe('cannon');
    expect(pieceAt(board, 7, 1)!.type).toBe('cannon');
    expect(pieceAt(board, 2, 7)!.type).toBe('cannon');
    expect(pieceAt(board, 7, 7)!.type).toBe('cannon');
    // Soldier symmetry: row 3 ↔ row 6
    for (let c = 0; c < 9; c += 2) {
      expect(pieceAt(board, 3, c)!.type).toBe('soldier');
      expect(pieceAt(board, 6, c)!.type).toBe('soldier');
    }
  });
});

describe('createInitialBoardState', () => {
  it('returns a BoardState with the initial board', () => {
    const state: BoardState = createInitialBoardState();
    expect(state.board).toHaveLength(10);
    expect(state.board[0]).toHaveLength(9);
    expect(countPieces(state.board)).toBe(32);
  });

  it('sets currentPlayer to red (red moves first per Xiangqi rules)', () => {
    const state = createInitialBoardState();
    expect(state.currentPlayer).toBe('red');
  });

  it('returns a fresh board each call (no shared mutation)', () => {
    const state1 = createInitialBoardState();
    const state2 = createInitialBoardState();
    // They should be structurally equal but distinct objects
    expect(state1).toEqual(state2);
    expect(state1).not.toBe(state2);
    expect(state1.board).not.toBe(state2.board);
  });
});
