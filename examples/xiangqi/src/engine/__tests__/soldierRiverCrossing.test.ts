import { describe, it, expect } from 'vitest';
import type { Board, Piece, Position } from '../types';
import { getRawMoves } from '../moveValidation';
import { createInitialBoard } from '../constants';

// ── Helpers ──

/** Create an empty 10×9 board. */
function emptyBoard(): Board {
  return Array.from({ length: 10 }, () =>
    Array.from({ length: 9 }, () => null),
  );
}

/** Place a piece on the board and return a new board. */
function place(
  board: Board,
  row: number,
  col: number,
  piece: Piece,
): Board {
  const b = board.map((r) => r.map((c) => c));
  b[row]![col] = piece;
  return b;
}

/** Shorthand to get moves as a set of "row,col" strings for easy comparison */
function moveSet(moves: Position[]): Set<string> {
  return new Set(moves.map((m) => `${m.row},${m.col}`));
}

// ── Piece constants ──

const redSoldier: Piece = { type: 'soldier', player: 'red' };
const blackSoldier: Piece = { type: 'soldier', player: 'black' };
const redChariot: Piece = { type: 'chariot', player: 'red' };
const blackChariot: Piece = { type: 'chariot', player: 'black' };

// ── Tests ──

describe('soldierRiverCrossing', () => {
  // ──────────────────────────────────────────────────────────────
  // Red Soldier — BEFORE crossing the river (home side, rows 6–9)
  // ──────────────────────────────────────────────────────────────

  describe('Red soldier before crossing the river', () => {
    it('at starting position (row 6) — can only move forward', () => {
      let board = emptyBoard();
      board = place(board, 6, 4, redSoldier);

      const moves = getRawMoves(board, { row: 6, col: 4 });
      const ms = moveSet(moves);

      // Only forward (row 5)
      expect(ms.has('5,4')).toBe(true);
      expect(moves).toHaveLength(1);

      // No sideways
      expect(ms.has('6,3')).toBe(false);
      expect(ms.has('6,5')).toBe(false);
    });

    it('at row 7 — can only move forward', () => {
      let board = emptyBoard();
      board = place(board, 7, 2, redSoldier);

      const moves = getRawMoves(board, { row: 7, col: 2 });
      const ms = moveSet(moves);

      expect(ms.has('6,2')).toBe(true);
      expect(moves).toHaveLength(1);
    });

    it('at row 8 (just before river) — can only move forward', () => {
      let board = emptyBoard();
      board = place(board, 8, 0, redSoldier);

      const moves = getRawMoves(board, { row: 8, col: 0 });
      const ms = moveSet(moves);

      expect(ms.has('7,0')).toBe(true);
      expect(moves).toHaveLength(1);
    });

    it('blocked forward by own piece — has no moves', () => {
      let board = emptyBoard();
      board = place(board, 7, 4, redSoldier);
      board = place(board, 6, 4, redChariot); // own piece blocking forward

      const moves = getRawMoves(board, { row: 7, col: 4 });
      expect(moves).toHaveLength(0);
    });

    it('can capture forward into enemy piece', () => {
      let board = emptyBoard();
      board = place(board, 7, 4, redSoldier);
      board = place(board, 6, 4, blackChariot); // enemy piece forward

      const moves = getRawMoves(board, { row: 7, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('6,4')).toBe(true);
      expect(moves).toHaveLength(1);
    });
  });

  // ──────────────────────────────────────────────────────────────
  // Red Soldier — AFTER crossing the river (enemy side, rows 0–4)
  // ──────────────────────────────────────────────────────────────

  describe('Red soldier after crossing the river', () => {
    it('at row 4 (just crossed) — forward + sideways', () => {
      let board = emptyBoard();
      board = place(board, 4, 4, redSoldier);

      const moves = getRawMoves(board, { row: 4, col: 4 });
      const ms = moveSet(moves);

      // Forward
      expect(ms.has('3,4')).toBe(true);
      // Sideways
      expect(ms.has('4,3')).toBe(true);
      expect(ms.has('4,5')).toBe(true);
      expect(moves).toHaveLength(3);
    });

    it('at row 2 (deep in enemy territory) — forward + sideways', () => {
      let board = emptyBoard();
      board = place(board, 2, 0, redSoldier);

      const moves = getRawMoves(board, { row: 2, col: 0 });
      const ms = moveSet(moves);

      // Forward
      expect(ms.has('1,0')).toBe(true);
      // Right (no left — col 0 is board edge)
      expect(ms.has('2,1')).toBe(true);
      expect(moves).toHaveLength(2);
    });

    it('at row 0 (opponent back rank) — sideways only (no forward, out of bounds)', () => {
      let board = emptyBoard();
      board = place(board, 0, 4, redSoldier);

      const moves = getRawMoves(board, { row: 0, col: 4 });
      const ms = moveSet(moves);

      // No forward (row -1 is out of bounds)
      expect(ms.has('-1,4')).toBe(false);
      // Sideways
      expect(ms.has('0,3')).toBe(true);
      expect(ms.has('0,5')).toBe(true);
      expect(moves).toHaveLength(2);
    });

    it('can capture sideways', () => {
      let board = emptyBoard();
      board = place(board, 3, 3, redSoldier);
      board = place(board, 3, 2, blackChariot); // enemy to the left

      const moves = getRawMoves(board, { row: 3, col: 3 });
      const ms = moveSet(moves);

      // Forward
      expect(ms.has('2,3')).toBe(true);
      // Left — capture
      expect(ms.has('3,2')).toBe(true);
      // Right — empty
      expect(ms.has('3,4')).toBe(true);
      expect(moves).toHaveLength(3);
    });

    it('all sideways blocked by own pieces — only forward available', () => {
      let board = emptyBoard();
      board = place(board, 4, 4, redSoldier);
      board = place(board, 4, 3, redChariot); // own piece left
      board = place(board, 4, 5, redChariot); // own piece right

      const moves = getRawMoves(board, { row: 4, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('3,4')).toBe(true);
      expect(moves).toHaveLength(1);
    });

    it('forward blocked by own piece, but sideways open', () => {
      let board = emptyBoard();
      board = place(board, 3, 4, redSoldier);
      board = place(board, 2, 4, redChariot); // own piece blocking forward

      const moves = getRawMoves(board, { row: 3, col: 4 });
      const ms = moveSet(moves);

      // Forward blocked
      expect(ms.has('2,4')).toBe(false);
      // Sideways open
      expect(ms.has('3,3')).toBe(true);
      expect(ms.has('3,5')).toBe(true);
      expect(moves).toHaveLength(2);
    });
  });

  // ──────────────────────────────────────────────────────────────
  // Black Soldier — BEFORE crossing the river (home side, rows 0–3)
  // ──────────────────────────────────────────────────────────────

  describe('Black soldier before crossing the river', () => {
    it('at starting position (row 3) — can only move forward', () => {
      let board = emptyBoard();
      board = place(board, 3, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 3, col: 4 });
      const ms = moveSet(moves);

      // Forward for black = downward (row 4)
      expect(ms.has('4,4')).toBe(true);
      expect(moves).toHaveLength(1);

      // No sideways
      expect(ms.has('3,3')).toBe(false);
      expect(ms.has('3,5')).toBe(false);
    });

    it('at row 2 — can only move forward', () => {
      let board = emptyBoard();
      board = place(board, 2, 2, blackSoldier);

      const moves = getRawMoves(board, { row: 2, col: 2 });
      const ms = moveSet(moves);

      expect(ms.has('3,2')).toBe(true);
      expect(moves).toHaveLength(1);
    });

    it('at row 1 (just before river) — can only move forward', () => {
      let board = emptyBoard();
      board = place(board, 1, 0, blackSoldier);

      const moves = getRawMoves(board, { row: 1, col: 0 });
      const ms = moveSet(moves);

      expect(ms.has('2,0')).toBe(true);
      expect(moves).toHaveLength(1);
    });

    it('blocked forward by own piece — has no moves', () => {
      let board = emptyBoard();
      board = place(board, 2, 4, blackSoldier);
      board = place(board, 3, 4, blackChariot); // own piece blocking forward

      const moves = getRawMoves(board, { row: 2, col: 4 });
      expect(moves).toHaveLength(0);
    });

    it('can capture forward into enemy piece', () => {
      let board = emptyBoard();
      board = place(board, 2, 4, blackSoldier);
      board = place(board, 3, 4, redChariot); // enemy piece forward

      const moves = getRawMoves(board, { row: 2, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('3,4')).toBe(true);
      expect(moves).toHaveLength(1);
    });
  });

  // ──────────────────────────────────────────────────────────────
  // Black Soldier — AFTER crossing the river (enemy side, rows 5–9)
  // ──────────────────────────────────────────────────────────────

  describe('Black soldier after crossing the river', () => {
    it('at row 5 (just crossed) — forward + sideways', () => {
      let board = emptyBoard();
      board = place(board, 5, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 5, col: 4 });
      const ms = moveSet(moves);

      // Forward for black = downward (row 6)
      expect(ms.has('6,4')).toBe(true);
      // Sideways
      expect(ms.has('5,3')).toBe(true);
      expect(ms.has('5,5')).toBe(true);
      expect(moves).toHaveLength(3);
    });

    it('at row 8 (deep in enemy territory) — forward + sideways', () => {
      let board = emptyBoard();
      board = place(board, 8, 8, blackSoldier);

      const moves = getRawMoves(board, { row: 8, col: 8 });
      const ms = moveSet(moves);

      // Forward
      expect(ms.has('9,8')).toBe(true);
      // Left (no right — col 8 is board edge)
      expect(ms.has('8,7')).toBe(true);
      expect(moves).toHaveLength(2);
    });

    it('at row 9 (opponent back rank) — sideways only (no forward, out of bounds)', () => {
      let board = emptyBoard();
      board = place(board, 9, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 9, col: 4 });
      const ms = moveSet(moves);

      // No forward (row 10 is out of bounds)
      expect(ms.has('10,4')).toBe(false);
      // Sideways
      expect(ms.has('9,3')).toBe(true);
      expect(ms.has('9,5')).toBe(true);
      expect(moves).toHaveLength(2);
    });

    it('can capture sideways', () => {
      let board = emptyBoard();
      board = place(board, 6, 3, blackSoldier);
      board = place(board, 6, 4, redChariot); // enemy to the right

      const moves = getRawMoves(board, { row: 6, col: 3 });
      const ms = moveSet(moves);

      // Forward
      expect(ms.has('7,3')).toBe(true);
      // Left — empty
      expect(ms.has('6,2')).toBe(true);
      // Right — capture
      expect(ms.has('6,4')).toBe(true);
      expect(moves).toHaveLength(3);
    });

    it('all sideways blocked by own pieces — only forward available', () => {
      let board = emptyBoard();
      board = place(board, 5, 4, blackSoldier);
      board = place(board, 5, 3, blackChariot); // own piece left
      board = place(board, 5, 5, blackChariot); // own piece right

      const moves = getRawMoves(board, { row: 5, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('6,4')).toBe(true);
      expect(moves).toHaveLength(1);
    });
  });

  // ──────────────────────────────────────────────────────────────
  // Edge / Boundary Cases
  // ──────────────────────────────────────────────────────────────

  describe('Edge and boundary cases', () => {
    it('Red soldier at col 0 after river — no left move (board edge)', () => {
      let board = emptyBoard();
      board = place(board, 3, 0, redSoldier);

      const moves = getRawMoves(board, { row: 3, col: 0 });
      const ms = moveSet(moves);

      // Forward
      expect(ms.has('2,0')).toBe(true);
      // Right only (col -1 is out of bounds)
      expect(ms.has('3,1')).toBe(true);
      expect(moves).toHaveLength(2);
    });

    it('Red soldier at col 8 after river — no right move (board edge)', () => {
      let board = emptyBoard();
      board = place(board, 3, 8, redSoldier);

      const moves = getRawMoves(board, { row: 3, col: 8 });
      const ms = moveSet(moves);

      // Forward
      expect(ms.has('2,8')).toBe(true);
      // Left only (col 9 is out of bounds)
      expect(ms.has('3,7')).toBe(true);
      expect(moves).toHaveLength(2);
    });

    it('Red soldier at river boundary row 5 — forward only (has not crossed)', () => {
      let board = emptyBoard();
      board = place(board, 5, 4, redSoldier);

      const moves = getRawMoves(board, { row: 5, col: 4 });
      const ms = moveSet(moves);

      // Row 5 is on red's side of the river — not crossed yet
      expect(ms.has('4,4')).toBe(true); // forward
      expect(ms.has('5,3')).toBe(false); // no sideways
      expect(ms.has('5,5')).toBe(false);
      expect(moves).toHaveLength(1);
    });

    it('Black soldier at river boundary row 4 — forward only (has not crossed)', () => {
      let board = emptyBoard();
      board = place(board, 4, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 4, col: 4 });
      const ms = moveSet(moves);

      // Row 4 is on black's side of the river — not crossed yet
      expect(ms.has('5,4')).toBe(true); // forward
      expect(ms.has('4,3')).toBe(false); // no sideways
      expect(ms.has('4,5')).toBe(false);
      expect(moves).toHaveLength(1);
    });

    it('soldiers on the initial board have exactly one forward move each', () => {
      const board = createInitialBoard();

      // Red soldiers at row 6, even columns — should have exactly 1 forward move each
      for (let c = 0; c <= 8; c += 2) {
        const moves = getRawMoves(board, { row: 6, col: c });
        const ms = moveSet(moves);
        expect(ms.has('5,c')).toBe(false); // placeholder — check explicitly below
        expect(moves).toHaveLength(1);
        expect(moves[0]).toEqual({ row: 5, col: c });
      }

      // Black soldiers at row 3, even columns — should have exactly 1 forward move each
      for (let c = 0; c <= 8; c += 2) {
        const moves = getRawMoves(board, { row: 3, col: c });
        expect(moves).toHaveLength(1);
        expect(moves[0]).toEqual({ row: 4, col: c });
      }
    });
  });
});
