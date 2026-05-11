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

/** Red cannon factory */
const redCannon: Piece = { type: 'cannon', player: 'red' };
/** Black cannon factory */
const blackCannon: Piece = { type: 'cannon', player: 'black' };
/** Red soldier (used as screen) */
const redSoldier: Piece = { type: 'soldier', player: 'red' };
/** Black soldier (used as screen / capture target) */
const blackSoldier: Piece = { type: 'soldier', player: 'black' };
/** Red chariot (used as blocking piece) */
const redChariot: Piece = { type: 'chariot', player: 'red' };
/** Black chariot (used as capture target) */
const blackChariot: Piece = { type: 'chariot', player: 'black' };

/** Shorthand to get moves as a set of "row,col" strings for easy comparison */
function moveSet(moves: Position[]): Set<string> {
  return new Set(moves.map((m) => `${m.row},${m.col}`));
}

// ── Tests ──

describe('cannonMoves', () => {
  it('moves like chariot on empty board (non-capture, straight lines)', () => {
    let board = emptyBoard();
    board = place(board, 4, 4, redCannon);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Up from (4,4) → rows 0-3
    expect(ms.has('3,4')).toBe(true);
    expect(ms.has('2,4')).toBe(true);
    expect(ms.has('1,4')).toBe(true);
    expect(ms.has('0,4')).toBe(true);

    // Down from (4,4) → rows 5-9
    expect(ms.has('5,4')).toBe(true);
    expect(ms.has('6,4')).toBe(true);
    expect(ms.has('7,4')).toBe(true);
    expect(ms.has('8,4')).toBe(true);
    expect(ms.has('9,4')).toBe(true);

    // Left from (4,4) → cols 0-3
    expect(ms.has('4,3')).toBe(true);
    expect(ms.has('4,2')).toBe(true);
    expect(ms.has('4,1')).toBe(true);
    expect(ms.has('4,0')).toBe(true);

    // Right from (4,4) → cols 5-8
    expect(ms.has('4,5')).toBe(true);
    expect(ms.has('4,6')).toBe(true);
    expect(ms.has('4,7')).toBe(true);
    expect(ms.has('4,8')).toBe(true);

    // Total: 4 + 5 + 4 + 4 = 17 moves
    expect(moves).toHaveLength(17);
  });

  it('is blocked by pieces on the same line (cannot jump for non-capture)', () => {
    let board = emptyBoard();
    board = place(board, 4, 4, redCannon);
    // Own piece directly above at (2,4)
    board = place(board, 2, 4, redSoldier);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Up: (3,4) is reachable, but (2,4) blocks and is own piece → no capture
    expect(ms.has('3,4')).toBe(true);
    expect(ms.has('2,4')).toBe(false);
    expect(ms.has('1,4')).toBe(false);
    expect(ms.has('0,4')).toBe(false);

    // Down, left, right are all open
    expect(ms.has('5,4')).toBe(true);
    expect(ms.has('4,3')).toBe(true);
    expect(ms.has('4,5')).toBe(true);

    // Total: 1 + 5 + 4 + 4 = 14
    expect(moves).toHaveLength(14);
  });

  it('captures opponent piece by jumping over exactly one screen', () => {
    let board = emptyBoard();
    board = place(board, 4, 4, redCannon);
    // Screen (any piece) at (2,4)
    board = place(board, 3, 4, redSoldier);
    // Opponent piece behind screen at (1,4)
    board = place(board, 1, 4, blackChariot);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Up: (3,4) is blocked (screen, own piece), but cannon jumps over it
    // (2,4) is empty → skip
    // (1,4) has opponent → capture!
    expect(ms.has('3,4')).toBe(false); // screen square, not a move
    expect(ms.has('2,4')).toBe(false); // empty square after screen, cannot land here
    expect(ms.has('1,4')).toBe(true);  // capture over screen!
    expect(ms.has('0,4')).toBe(false); // blocked after capture target

    // Down, left, right are open
    expect(ms.has('5,4')).toBe(true);
    expect(ms.has('4,3')).toBe(true);
    expect(ms.has('4,5')).toBe(true);

    // Total: 1 (capture) + 5 + 4 + 4 = 14
    expect(moves).toHaveLength(14);
  });

  it('cannot capture if there is no screen between cannon and target', () => {
    let board = emptyBoard();
    board = place(board, 4, 4, redCannon);
    // Opponent piece directly above at (2,4) with no screen
    board = place(board, 2, 4, blackChariot);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Up: (3,4) is reachable, (2,4) blocks — it's opponent but no screen → no capture
    expect(ms.has('3,4')).toBe(true);
    expect(ms.has('2,4')).toBe(false); // blocks like chariot, but cannot capture without screen
    expect(ms.has('1,4')).toBe(false);

    // Total: 1 + 5 + 4 + 4 = 14
    expect(moves).toHaveLength(14);
  });

  it('cannot capture if there are two or more screens between cannon and target', () => {
    let board = emptyBoard();
    board = place(board, 4, 4, redCannon);
    // Two pieces (screens) at (3,4) and (2,4)
    board = place(board, 3, 4, redSoldier);
    board = place(board, 2, 4, blackSoldier);
    // Opponent target at (0,4)
    board = place(board, 0, 4, blackChariot);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // First screen at (3,4), second "screen" at (2,4) which is also a piece
    // After the first screen, the next piece encountered is (2,4) — black soldier
    // That IS an opponent piece behind exactly one screen, so it IS capturable
    expect(ms.has('2,4')).toBe(true); // capture blackSoldier over redSoldier screen

    // (0,4) is behind TWO screens → cannot capture
    expect(ms.has('0,4')).toBe(false);

    // (1,4) is empty behind the first screen but before the second piece → not a move
    expect(ms.has('1,4')).toBe(false);

    // Down, left, right are open
    expect(moves.length).toBeGreaterThanOrEqual(14);
  });

  it('stops after first piece behind screen (cannot reach further pieces)', () => {
    let board = emptyBoard();
    board = place(board, 0, 0, redCannon);
    // Screen at (0,3)
    board = place(board, 0, 3, redSoldier);
    // First piece behind screen at (0,5) — opponent
    board = place(board, 0, 5, blackChariot);
    // Another piece further at (0,7) — opponent
    board = place(board, 0, 7, blackSoldier);

    const moves = getRawMoves(board, { row: 0, col: 0 });
    const ms = moveSet(moves);

    // Right: (0,1), (0,2) are open; (0,3) is screen; (0,4) empty skip; (0,5) capture; stop
    expect(ms.has('0,1')).toBe(true);
    expect(ms.has('0,2')).toBe(true);
    expect(ms.has('0,3')).toBe(false); // screen, not a move
    expect(ms.has('0,4')).toBe(false); // empty behind screen, cannot land
    expect(ms.has('0,5')).toBe(true);  // capture over screen
    expect(ms.has('0,6')).toBe(false); // blocked
    expect(ms.has('0,7')).toBe(false); // blocked by piece at (0,5)
    expect(ms.has('0,8')).toBe(false); // blocked

    // Down: rows 1-9 open
    for (let r = 1; r <= 9; r++) {
      expect(ms.has(`${r},0`)).toBe(true);
    }

    // Total: 2 (right) + 1 (capture) + 9 (down) = 12
    expect(moves).toHaveLength(12);
  });

  it('does not capture own piece behind screen', () => {
    let board = emptyBoard();
    board = place(board, 5, 5, redCannon);
    // Screen at (3,5) — own piece
    board = place(board, 3, 5, redSoldier);
    // Own piece behind screen at (1,5)
    board = place(board, 1, 5, redChariot);

    const moves = getRawMoves(board, { row: 5, col: 5 });
    const ms = moveSet(moves);

    // Up: (4,5) open, (3,5) screen, (2,5) empty skip, (1,5) own piece → cannot capture, stop
    expect(ms.has('4,5')).toBe(true);
    expect(ms.has('3,5')).toBe(false); // screen
    expect(ms.has('2,5')).toBe(false); // empty behind screen
    expect(ms.has('1,5')).toBe(false); // own piece, no capture
    expect(ms.has('0,5')).toBe(false); // blocked

    // Down: rows 6-9 open
    expect(ms.has('6,5')).toBe(true);
    expect(ms.has('7,5')).toBe(true);
    expect(ms.has('8,5')).toBe(true);
    expect(ms.has('9,5')).toBe(true);

    // Left and right open
    expect(moves.length).toBeGreaterThan(0);
  });

  it('works for black cannon moving downward', () => {
    let board = emptyBoard();
    board = place(board, 2, 7, blackCannon); // black cannon at starting position
    // Screen at (5,7)
    board = place(board, 5, 7, redSoldier);
    // Red piece behind screen at (8,7)
    board = place(board, 8, 7, redChariot);

    const moves = getRawMoves(board, { row: 2, col: 7 });
    const ms = moveSet(moves);

    // Down: (3,7) open, (4,7) open, (5,7) screen, (6,7) empty skip, (7,7) empty skip, (8,7) capture
    expect(ms.has('3,7')).toBe(true);
    expect(ms.has('4,7')).toBe(true);
    expect(ms.has('5,7')).toBe(false); // screen
    expect(ms.has('6,7')).toBe(false); // empty behind screen
    expect(ms.has('7,7')).toBe(false); // empty behind screen
    expect(ms.has('8,7')).toBe(true);  // capture
    expect(ms.has('9,7')).toBe(false); // blocked

    // Up: (1,7), (0,7) open
    expect(ms.has('1,7')).toBe(true);
    expect(ms.has('0,7')).toBe(true);

    // Left: cols 0-6 open
    for (let c = 0; c <= 6; c++) {
      expect(ms.has(`2,${c}`)).toBe(true);
    }

    // Right: (2,8) open
    expect(ms.has('2,8')).toBe(true);
  });

  it('handles cannon in corner with all four directions', () => {
    let board = emptyBoard();
    board = place(board, 0, 0, redCannon);

    const moves = getRawMoves(board, { row: 0, col: 0 });
    const ms = moveSet(moves);

    // Down: rows 1-9
    for (let r = 1; r <= 9; r++) {
      expect(ms.has(`${r},0`)).toBe(true);
    }
    // Right: cols 1-8
    for (let c = 1; c <= 8; c++) {
      expect(ms.has(`0,${c}`)).toBe(true);
    }
    // Up and left: none (out of bounds)

    // Total: 9 + 8 = 17
    expect(moves).toHaveLength(17);
  });

  it('cannon captures opponent piece that is the screen itself is NOT allowed', () => {
    let board = emptyBoard();
    board = place(board, 4, 4, redCannon);
    // Opponent piece directly adjacent at (4,5) — no screen before it
    board = place(board, 4, 5, blackSoldier);

    const moves = getRawMoves(board, { row: 4, col: 4 });
    const ms = moveSet(moves);

    // Right: (4,5) is the first piece encountered — acts as screen/piece, not capturable without a screen
    expect(ms.has('4,5')).toBe(false);
    expect(ms.has('4,6')).toBe(false); // nothing behind the "screen"
  });

  it('cannon at initial board position has correct moves', () => {
    // Set up the actual initial position for the red cannon at (7,1)
    let board = emptyBoard();
    board = place(board, 7, 1, redCannon);
    // Black soldiers at row 3, cols 0,2,4,6,8
    for (let c = 0; c <= 8; c += 2) {
      board = place(board, 3, c, blackSoldier);
    }
    // Red soldiers at row 6, cols 0,2,4,6,8
    for (let c = 0; c <= 8; c += 2) {
      board = place(board, 6, c, redSoldier);
    }

    const moves = getRawMoves(board, { row: 7, col: 1 });
    const ms = moveSet(moves);

    // Up: (6,1) empty (no red soldier at col 1) → open
    expect(ms.has('6,1')).toBe(true);
    // (5,1) empty → open
    expect(ms.has('5,1')).toBe(true);
    // (4,1) empty → open
    expect(ms.has('4,1')).toBe(true);
    // (3,1) empty → open (black soldiers are at even cols only)
    expect(ms.has('3,1')).toBe(true);
    // (2,1) empty → open (but wait, no piece at col 1 on row 2 either... in initial board there's no cannon at (2,1))
    // Actually in our setup only the soldiers are placed, so let's just check open path
    expect(ms.has('2,1')).toBe(true);
    expect(ms.has('1,1')).toBe(true);
    expect(ms.has('0,1')).toBe(true);

    // Down: (8,1) empty → open
    expect(ms.has('8,1')).toBe(true);
    // (9,1) empty → open
    expect(ms.has('9,1')).toBe(true);

    // Left: (7,0) empty → open
    expect(ms.has('7,0')).toBe(true);

    // Right: (7,2) empty → open
    expect(ms.has('7,2')).toBe(true);
    // (7,3) empty → open
    expect(ms.has('7,3')).toBe(true);
    // ... all the way to (7,8)
    expect(ms.has('7,8')).toBe(true);
  });
});
