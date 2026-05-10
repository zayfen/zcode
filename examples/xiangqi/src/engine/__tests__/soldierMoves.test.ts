import { describe, it, expect } from 'vitest';
import type { Board, Piece, Position } from '../types';
import { getRawMoves } from '../moveValidation';

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

/** Red soldier factory */
const redSoldier: Piece = { type: 'soldier', player: 'red' };
/** Black soldier factory */
const blackSoldier: Piece = { type: 'soldier', player: 'black' };
/** Red chariot (blocking/capture target) */
const redChariot: Piece = { type: 'chariot', player: 'red' };
/** Black chariot (blocking/capture target) */
const blackChariot: Piece = { type: 'chariot', player: 'black' };

/** Shorthand to get moves as a set of "row,col" strings */
function moveSet(moves: Position[]): Set<string> {
  return new Set(moves.map((m) => `${m.row},${m.col}`));
}

// ── Tests ──

describe('soldierMoves', () => {
  // ── Red soldier (兵) — forward is UP (row decreasing) ──

  describe('red soldier before river', () => {
    it('can only move forward (up) before crossing river', () => {
      // Red soldier at (6,4) — starting position, on red's side (row 6 >= 5)
      let board = emptyBoard();
      board = place(board, 6, 4, redSoldier);

      const moves = getRawMoves(board, { row: 6, col: 4 });
      const ms = moveSet(moves);

      // Only forward (up) to (5,4)
      expect(ms.has('5,4')).toBe(true);
      // No sideways
      expect(ms.has('6,3')).toBe(false);
      expect(ms.has('6,5')).toBe(false);
      // No backward
      expect(ms.has('7,4')).toBe(false);
      // Exactly 1 move
      expect(moves).toHaveLength(1);
    });

    it('cannot move backward from any position before river', () => {
      let board = emptyBoard();
      board = place(board, 6, 0, redSoldier); // edge position

      const moves = getRawMoves(board, { row: 6, col: 0 });
      const ms = moveSet(moves);

      expect(ms.has('5,0')).toBe(true);  // forward
      expect(ms.has('7,0')).toBe(false); // backward — forbidden
      expect(moves).toHaveLength(1);
    });
  });

  describe('red soldier after crossing river', () => {
    it('can move forward and sideways after crossing river (at river boundary)', () => {
      // Red soldier at (4,4) — has crossed river (row 4 <= 4)
      let board = emptyBoard();
      board = place(board, 4, 4, redSoldier);

      const moves = getRawMoves(board, { row: 4, col: 4 });
      const ms = moveSet(moves);

      // Forward (up)
      expect(ms.has('3,4')).toBe(true);
      // Sideways
      expect(ms.has('4,3')).toBe(true);
      expect(ms.has('4,5')).toBe(true);
      // No backward
      expect(ms.has('5,4')).toBe(false);
      // 3 moves total
      expect(moves).toHaveLength(3);
    });

    it('can move forward and sideways deep in enemy territory', () => {
      // Red soldier at (1,4) — deep in black territory
      let board = emptyBoard();
      board = place(board, 1, 4, redSoldier);

      const moves = getRawMoves(board, { row: 1, col: 4 });
      const ms = moveSet(moves);

      // Forward (up)
      expect(ms.has('0,4')).toBe(true);
      // Sideways
      expect(ms.has('1,3')).toBe(true);
      expect(ms.has('1,5')).toBe(true);
      // No backward
      expect(ms.has('2,4')).toBe(false);
      expect(moves).toHaveLength(3);
    });

    it('at left edge can only move forward and right', () => {
      let board = emptyBoard();
      board = place(board, 3, 0, redSoldier);

      const moves = getRawMoves(board, { row: 3, col: 0 });
      const ms = moveSet(moves);

      expect(ms.has('2,0')).toBe(true);  // forward
      expect(ms.has('3,1')).toBe(true);  // right
      expect(ms.has('3,-1')).toBe(false); // left — out of bounds
      expect(moves).toHaveLength(2);
    });

    it('at right edge can only move forward and left', () => {
      let board = emptyBoard();
      board = place(board, 3, 8, redSoldier);

      const moves = getRawMoves(board, { row: 3, col: 8 });
      const ms = moveSet(moves);

      expect(ms.has('2,8')).toBe(true);  // forward
      expect(ms.has('3,7')).toBe(true);  // left
      expect(moves).toHaveLength(2);
    });

    it('at row 0 cannot move forward (top of board) but can move sideways', () => {
      let board = emptyBoard();
      board = place(board, 0, 4, redSoldier);

      const moves = getRawMoves(board, { row: 0, col: 4 });
      const ms = moveSet(moves);

      // Forward would be row -1 — out of bounds
      expect(ms.has('-1,4')).toBe(false);
      // Sideways
      expect(ms.has('0,3')).toBe(true);
      expect(ms.has('0,5')).toBe(true);
      expect(moves).toHaveLength(2);
    });
  });

  // ── Black soldier (卒) — forward is DOWN (row increasing) ──

  describe('black soldier before river', () => {
    it('can only move forward (down) before crossing river', () => {
      // Black soldier at (3,4) — starting position, on black's side (row 3 <= 4)
      let board = emptyBoard();
      board = place(board, 3, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 3, col: 4 });
      const ms = moveSet(moves);

      // Only forward (down)
      expect(ms.has('4,4')).toBe(true);
      // No sideways
      expect(ms.has('3,3')).toBe(false);
      expect(ms.has('3,5')).toBe(false);
      // No backward
      expect(ms.has('2,4')).toBe(false);
      expect(moves).toHaveLength(1);
    });

    it('cannot move backward from any position before river', () => {
      let board = emptyBoard();
      board = place(board, 3, 0, blackSoldier);

      const moves = getRawMoves(board, { row: 3, col: 0 });
      const ms = moveSet(moves);

      expect(ms.has('4,0')).toBe(true);  // forward
      expect(ms.has('2,0')).toBe(false); // backward — forbidden
      expect(moves).toHaveLength(1);
    });
  });

  describe('black soldier after crossing river', () => {
    it('can move forward and sideways after crossing river (at river boundary)', () => {
      // Black soldier at (5,4) — has crossed river (row 5 >= 5)
      let board = emptyBoard();
      board = place(board, 5, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 5, col: 4 });
      const ms = moveSet(moves);

      // Forward (down)
      expect(ms.has('6,4')).toBe(true);
      // Sideways
      expect(ms.has('5,3')).toBe(true);
      expect(ms.has('5,5')).toBe(true);
      // No backward
      expect(ms.has('4,4')).toBe(false);
      expect(moves).toHaveLength(3);
    });

    it('can move forward and sideways deep in enemy territory', () => {
      // Black soldier at (8,4) — deep in red territory
      let board = emptyBoard();
      board = place(board, 8, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 8, col: 4 });
      const ms = moveSet(moves);

      // Forward (down)
      expect(ms.has('9,4')).toBe(true);
      // Sideways
      expect(ms.has('8,3')).toBe(true);
      expect(ms.has('8,5')).toBe(true);
      // No backward
      expect(ms.has('7,4')).toBe(false);
      expect(moves).toHaveLength(3);
    });

    it('at row 9 cannot move forward (bottom of board) but can move sideways', () => {
      let board = emptyBoard();
      board = place(board, 9, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 9, col: 4 });
      const ms = moveSet(moves);

      // Forward would be row 10 — out of bounds
      expect(ms.has('10,4')).toBe(false);
      // Sideways
      expect(ms.has('9,3')).toBe(true);
      expect(ms.has('9,5')).toBe(true);
      expect(moves).toHaveLength(2);
    });
  });

  // ── Capture tests ──

  describe('soldier captures', () => {
    it('red soldier can capture opponent piece forward', () => {
      let board = emptyBoard();
      board = place(board, 6, 4, redSoldier);
      board = place(board, 5, 4, blackChariot); // opponent ahead

      const moves = getRawMoves(board, { row: 6, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('5,4')).toBe(true); // can capture
      expect(moves).toHaveLength(1);
    });

    it('red soldier cannot capture own piece forward', () => {
      let board = emptyBoard();
      board = place(board, 6, 4, redSoldier);
      board = place(board, 5, 4, redChariot); // own piece ahead

      const moves = getRawMoves(board, { row: 6, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('5,4')).toBe(false); // blocked by own piece
      expect(moves).toHaveLength(0);
    });

    it('red soldier can capture opponent piece sideways after crossing river', () => {
      let board = emptyBoard();
      board = place(board, 4, 4, redSoldier); // crossed river
      board = place(board, 4, 3, blackChariot); // opponent to the left
      board = place(board, 4, 5, blackChariot); // opponent to the right

      const moves = getRawMoves(board, { row: 4, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('3,4')).toBe(true); // forward
      expect(ms.has('4,3')).toBe(true); // capture left
      expect(ms.has('4,5')).toBe(true); // capture right
      expect(moves).toHaveLength(3);
    });

    it('red soldier cannot capture own piece sideways', () => {
      let board = emptyBoard();
      board = place(board, 4, 4, redSoldier); // crossed river
      board = place(board, 4, 3, redChariot); // own piece to the left
      board = place(board, 4, 5, redChariot); // own piece to the right

      const moves = getRawMoves(board, { row: 4, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('3,4')).toBe(true);  // forward still works
      expect(ms.has('4,3')).toBe(false); // blocked by own piece
      expect(ms.has('4,5')).toBe(false); // blocked by own piece
      expect(moves).toHaveLength(1);
    });

    it('black soldier can capture opponent piece forward', () => {
      let board = emptyBoard();
      board = place(board, 3, 4, blackSoldier);
      board = place(board, 4, 4, redChariot); // opponent ahead

      const moves = getRawMoves(board, { row: 3, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('4,4')).toBe(true); // can capture
      expect(moves).toHaveLength(1);
    });

    it('black soldier can capture opponent piece sideways after crossing river', () => {
      let board = emptyBoard();
      board = place(board, 5, 4, blackSoldier); // crossed river
      board = place(board, 5, 3, redChariot); // opponent to the left
      board = place(board, 5, 5, redChariot); // opponent to the right

      const moves = getRawMoves(board, { row: 5, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('6,4')).toBe(true); // forward
      expect(ms.has('5,3')).toBe(true); // capture left
      expect(ms.has('5,5')).toBe(true); // capture right
      expect(moves).toHaveLength(3);
    });
  });

  // ── River boundary edge cases ──

  describe('river boundary edge cases', () => {
    it('red soldier at row 5 (on own side, just before river) moves only forward', () => {
      // Row 5 is on red's side (5 >= 5 is true for isOnSide with red)
      let board = emptyBoard();
      board = place(board, 5, 4, redSoldier);

      const moves = getRawMoves(board, { row: 5, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('4,4')).toBe(true);  // forward
      expect(ms.has('5,3')).toBe(false); // sideways — not crossed river
      expect(ms.has('5,5')).toBe(false); // sideways — not crossed river
      expect(moves).toHaveLength(1);
    });

    it('red soldier at row 4 (just crossed river) gets sideways moves', () => {
      // Row 4 is on black's side for red (4 <= 4)
      let board = emptyBoard();
      board = place(board, 4, 4, redSoldier);

      const moves = getRawMoves(board, { row: 4, col: 4 });
      expect(moves).toHaveLength(3); // forward + 2 sideways
    });

    it('black soldier at row 4 (on own side, just before river) moves only forward', () => {
      // Row 4 is on black's side (4 <= 4 is true for isOnSide with black)
      let board = emptyBoard();
      board = place(board, 4, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 4, col: 4 });
      const ms = moveSet(moves);

      expect(ms.has('5,4')).toBe(true);  // forward
      expect(ms.has('4,3')).toBe(false); // sideways — not crossed river
      expect(ms.has('4,5')).toBe(false); // sideways — not crossed river
      expect(moves).toHaveLength(1);
    });

    it('black soldier at row 5 (just crossed river) gets sideways moves', () => {
      // Row 5 is on red's side for black (5 >= 5)
      let board = emptyBoard();
      board = place(board, 5, 4, blackSoldier);

      const moves = getRawMoves(board, { row: 5, col: 4 });
      expect(moves).toHaveLength(3); // forward + 2 sideways
    });
  });

  // ── Never backward ──

  describe('never moves backward', () => {
    it('red soldier never generates backward moves in any position', () => {
      for (let row = 0; row <= 9; row++) {
        let board = emptyBoard();
        board = place(board, row, 4, redSoldier);

        const moves = getRawMoves(board, { row, col: 4 });
        const ms = moveSet(moves);

        // Backward for red = increasing row
        expect(ms.has(`${row + 1},4`)).toBe(false);
      }
    });

    it('black soldier never generates backward moves in any position', () => {
      for (let row = 0; row <= 9; row++) {
        let board = emptyBoard();
        board = place(board, row, 4, blackSoldier);

        const moves = getRawMoves(board, { row, col: 4 });
        const ms = moveSet(moves);

        // Backward for black = decreasing row
        expect(ms.has(`${row - 1},4`)).toBe(false);
      }
    });
  });
});
