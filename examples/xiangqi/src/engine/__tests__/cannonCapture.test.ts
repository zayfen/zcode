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

/** Red cannon */
const redCannon: Piece = { type: 'cannon', player: 'red' };
/** Red soldier (used as screen) */
const redSoldier: Piece = { type: 'soldier', player: 'red' };
/** Black soldier (used as screen / capture target) */
const blackSoldier: Piece = { type: 'soldier', player: 'black' };
/** Black chariot (used as capture target) */
const blackChariot: Piece = { type: 'chariot', player: 'black' };

/** Shorthand to get moves as a set of "row,col" strings for easy comparison */
function moveSet(moves: Position[]): Set<string> {
  return new Set(moves.map((m) => `${m.row},${m.col}`));
}

// ── Tests ──

describe('cannonCapture', () => {
  // ────────────────────────────────────────────────────────────────
  // 1. No screen → cannon cannot capture; it moves like a chariot
  // ────────────────────────────────────────────────────────────────
  it('moves without capture when no screen is present', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, redCannon);
    // Opponent piece on the same row with NO piece between
    board = place(board, 5, 7, blackChariot);

    const moves = getRawMoves(board, { row: 5, col: 4 });
    const ms = moveSet(moves);

    // Right: empty squares (5,5) and (5,6) are reachable, but (5,7) blocks
    // and cannot be captured because there is no screen
    expect(ms.has('5,5')).toBe(true);
    expect(ms.has('5,6')).toBe(true);
    expect(ms.has('5,7')).toBe(false); // no screen → cannot capture
    expect(ms.has('5,8')).toBe(false); // blocked by piece at (5,7)

    // Left: cols 0–3 all open
    expect(ms.has('5,3')).toBe(true);
    expect(ms.has('5,2')).toBe(true);
    expect(ms.has('5,1')).toBe(true);
    expect(ms.has('5,0')).toBe(true);

    // Up and down are also open (no obstacles)
    expect(ms.has('4,4')).toBe(true);
    expect(ms.has('6,4')).toBe(true);

    // Total: 2 (right) + 4 (left) + 5 (up: rows 0-4) + 4 (down: rows 6-9) = 15
    expect(moves).toHaveLength(15);
  });

  // ────────────────────────────────────────────────────────────────
  // 2. Exactly one screen → cannon captures the first opponent piece
  //    behind that screen; movement stops after capture
  // ────────────────────────────────────────────────────────────────
  it('captures opponent piece by jumping over exactly one screen', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, redCannon);
    // Screen (own piece) one square to the right
    board = place(board, 5, 5, redSoldier);
    // Opponent piece behind the screen
    board = place(board, 5, 7, blackChariot);

    const moves = getRawMoves(board, { row: 5, col: 4 });
    const ms = moveSet(moves);

    // Right direction:
    // (5,5) is the screen → not a valid landing square
    expect(ms.has('5,5')).toBe(false);
    // (5,6) is empty but behind the screen → cannot land
    expect(ms.has('5,6')).toBe(false);
    // (5,7) has opponent piece behind exactly one screen → capture!
    expect(ms.has('5,7')).toBe(true);
    // (5,8) is beyond the captured piece → blocked, not reachable
    expect(ms.has('5,8')).toBe(false);

    // Left, up, down directions are open (no obstacles)
    expect(ms.has('5,3')).toBe(true);
    expect(ms.has('4,4')).toBe(true);
    expect(ms.has('6,4')).toBe(true);

    // Total: 1 (capture) + 4 (left) + 5 (up) + 4 (down) = 14
    expect(moves).toHaveLength(14);
  });

  // ────────────────────────────────────────────────────────────────
  // 3. Two screens → cannon captures the FIRST opponent piece behind
  //    exactly one screen, but CANNOT reach a piece behind two screens
  // ────────────────────────────────────────────────────────────────
  it('cannot capture when two or more screens are between cannon and target', () => {
    let board = emptyBoard();
    board = place(board, 5, 0, redCannon);
    // First screen (own piece)
    board = place(board, 5, 2, redSoldier);
    // Second screen (opponent piece) — this is the first piece behind one screen
    board = place(board, 5, 4, blackSoldier);
    // Target behind TWO screens
    board = place(board, 5, 6, blackChariot);

    const moves = getRawMoves(board, { row: 5, col: 0 });
    const ms = moveSet(moves);

    // Right direction analysis:
    // (5,1) empty before screen → reachable
    expect(ms.has('5,1')).toBe(true);
    // (5,2) first screen → not landable
    expect(ms.has('5,2')).toBe(false);
    // (5,3) empty between screens → behind screen, cannot land
    expect(ms.has('5,3')).toBe(false);
    // (5,4) second screen — opponent piece behind exactly one screen → capturable!
    expect(ms.has('5,4')).toBe(true);
    // (5,5) empty beyond captured piece → blocked (movement stops after capture)
    expect(ms.has('5,5')).toBe(false);
    // (5,6) target behind TWO screens → cannot be reached
    expect(ms.has('5,6')).toBe(false);
    // (5,7) and beyond → also blocked
    expect(ms.has('5,7')).toBe(false);
    expect(ms.has('5,8')).toBe(false);

    // Left: no squares (cannon at col 0)
    // Up and down are open
    expect(ms.has('4,0')).toBe(true);
    expect(ms.has('6,0')).toBe(true);

    // Total: 1 (5,1) + 1 (capture 5,4) + 5 (up: rows 0-4) + 4 (down: rows 6-9) = 11
    expect(moves).toHaveLength(11);
  });
});
