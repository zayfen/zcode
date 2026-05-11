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

/** Shorthand to get moves as a set of "row,col" strings for easy comparison */
function moveSet(moves: Position[]): Set<string> {
  return new Set(moves.map((m) => `${m.row},${m.col}`));
}

// ── Piece constants ──

const redKnight: Piece = { type: 'knight', player: 'red' };
const blackKnight: Piece = { type: 'knight', player: 'black' };
const redSoldier: Piece = { type: 'soldier', player: 'red' };
const blackSoldier: Piece = { type: 'soldier', player: 'black' };

// ── Tests ──
//
// Knight placed at center (4,4). All 8 L-move destinations are in-bounds:
//   (2,3) (2,5) (3,2) (3,6) (5,2) (5,6) (6,3) (6,5)
//
// KNIGHT_OFFSETS blocking-leg table (from moveValidation.ts):
//   L-move (dr,dc)  →  blocking leg (blockDr, blockDc) from knight position
//   (-2,-1)         →  (-1, 0)   i.e. one step up        → block at (3,4)
//   (-2,+1)         →  (-1, 0)   i.e. one step up        → block at (3,4)
//   (-1,-2)         →  ( 0,-1)   i.e. one step left      → block at (4,3)
//   (-1,+2)         →  ( 0,+1)   i.e. one step right     → block at (4,5)
//   (+1,-2)         →  ( 0,-1)   i.e. one step left      → block at (4,3)
//   (+1,+2)         →  ( 0,+1)   i.e. one step right     → block at (4,5)
//   (+2,-1)         →  (+1, 0)   i.e. one step down      → block at (5,4)
//   (+2,+1)         →  (+1, 0)   i.e. one step down      → block at (5,4)

describe('knightBlocking', () => {
  // All 8 expected destinations from (4,4)
  const allDestinations: Position[] = [
    { row: 2, col: 3 },
    { row: 2, col: 5 },
    { row: 3, col: 2 },
    { row: 3, col: 6 },
    { row: 5, col: 2 },
    { row: 5, col: 6 },
    { row: 6, col: 3 },
    { row: 6, col: 5 },
  ];

  it('knight at center on empty board has all 8 L-moves', () => {
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    expect(moves).toHaveLength(8);
    for (const dest of allDestinations) {
      expect(ms.has(`${dest.row},${dest.col}`)).toBe(true);
    }
  });

  it('blocking leg at (3,4) blocks upward L-moves to (2,3) and (2,5)', () => {
    // Blocker one step up from knight → blocks (-2,-1) and (-2,+1)
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 3, 4, blackSoldier);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // These two destinations should be blocked
    expect(ms.has('2,3')).toBe(false);
    expect(ms.has('2,5')).toBe(false);

    // The other 6 destinations should still be reachable
    const remaining = allDestinations.filter(
      (d) => !(d.row === 2 && (d.col === 3 || d.col === 5)),
    );
    expect(moves).toHaveLength(remaining.length);
    for (const dest of remaining) {
      expect(ms.has(`${dest.row},${dest.col}`)).toBe(true);
    }
  });

  it('blocking leg at (5,4) blocks downward L-moves to (6,3) and (6,5)', () => {
    // Blocker one step down from knight → blocks (+2,-1) and (+2,+1)
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 5, 4, blackSoldier);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // These two destinations should be blocked
    expect(ms.has('6,3')).toBe(false);
    expect(ms.has('6,5')).toBe(false);

    // The other 6 destinations should still be reachable
    const remaining = allDestinations.filter(
      (d) => !(d.row === 6 && (d.col === 3 || d.col === 5)),
    );
    expect(moves).toHaveLength(remaining.length);
    for (const dest of remaining) {
      expect(ms.has(`${dest.row},${dest.col}`)).toBe(true);
    }
  });

  it('blocking leg at (4,3) blocks leftward L-moves to (3,2) and (5,2)', () => {
    // Blocker one step left from knight → blocks (-1,-2) and (+1,-2)
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 4, 3, blackSoldier);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // These two destinations should be blocked
    expect(ms.has('3,2')).toBe(false);
    expect(ms.has('5,2')).toBe(false);

    // The other 6 destinations should still be reachable
    const remaining = allDestinations.filter(
      (d) => !(d.col === 2 && (d.row === 3 || d.row === 5)),
    );
    expect(moves).toHaveLength(remaining.length);
    for (const dest of remaining) {
      expect(ms.has(`${dest.row},${dest.col}`)).toBe(true);
    }
  });

  it('blocking leg at (4,5) blocks rightward L-moves to (3,6) and (5,6)', () => {
    // Blocker one step right from knight → blocks (-1,+2) and (+1,+2)
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 4, 5, blackSoldier);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // These two destinations should be blocked
    expect(ms.has('3,6')).toBe(false);
    expect(ms.has('5,6')).toBe(false);

    // The other 6 destinations should still be reachable
    const remaining = allDestinations.filter(
      (d) => !(d.col === 6 && (d.row === 3 || d.row === 5)),
    );
    expect(moves).toHaveLength(remaining.length);
    for (const dest of remaining) {
      expect(ms.has(`${dest.row},${dest.col}`)).toBe(true);
    }
  });

  it('own piece on blocking square still blocks the move', () => {
    // A red (own) piece at the blocking leg should also block
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 3, 4, redSoldier); // own piece blocking upward leg

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Upward L-moves should be blocked regardless of piece color
    expect(ms.has('2,3')).toBe(false);
    expect(ms.has('2,5')).toBe(false);

    // Remaining 6 still reachable
    expect(moves).toHaveLength(6);
  });

  it('opponent piece on blocking square also blocks the move', () => {
    // A black (opponent) piece at the blocking leg should also block
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 3, 4, blackSoldier); // opponent piece blocking upward leg

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Upward L-moves should be blocked regardless of piece color
    expect(ms.has('2,3')).toBe(false);
    expect(ms.has('2,5')).toBe(false);

    // Remaining 6 still reachable
    expect(moves).toHaveLength(6);
  });

  it('cannot capture at L-destination when leg is blocked', () => {
    // Enemy piece at destination (2,3) BUT blocker at (3,4) prevents reaching it
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 3, 4, blackSoldier); // blocking leg
    board = place(board, 2, 3, blackSoldier); // enemy at destination

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Cannot reach (2,3) to capture because leg is blocked
    expect(ms.has('2,3')).toBe(false);

    // (2,5) also blocked by same leg blocker
    expect(ms.has('2,5')).toBe(false);
  });

  it('can capture at L-destination when leg is clear', () => {
    // Enemy piece at destination (2,3) with NO blocker — knight can capture
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 2, 3, blackSoldier); // enemy at destination, no blocker

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Can reach (2,3) to capture
    expect(ms.has('2,3')).toBe(true);

    // All 8 destinations still present (7 moves + 1 capture)
    expect(moves).toHaveLength(8);
  });

  it('all 4 blocking legs occupied — knight has 0 moves', () => {
    // Surround all 4 adjacent orthogonal squares with pieces
    let board = emptyBoard();
    board = place(board, 4, 4, redKnight);
    board = place(board, 3, 4, blackSoldier); // up
    board = place(board, 5, 4, blackSoldier); // down
    board = place(board, 4, 3, blackSoldier); // left
    board = place(board, 4, 5, blackSoldier); // right

    const moves = getRawMoves(board, { row: 4, col: 4 });

    expect(moves).toHaveLength(0);
  });

  it('black knight at center — same blocking rules apply', () => {
    // Mirror test with black knight
    let board = emptyBoard();
    board = place(board, 4, 4, blackKnight);
    // No blocker — all 8 moves available
    let moves = getRawMoves(board, { row: 4, col: 4 });
    expect(moves).toHaveLength(8);

    // Add blocker at (3,4) — should block upward L-moves
    board = place(board, 3, 4, redSoldier);
    moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    expect(ms.has('2,3')).toBe(false);
    expect(ms.has('2,5')).toBe(false);
    expect(moves).toHaveLength(6);
  });
});
