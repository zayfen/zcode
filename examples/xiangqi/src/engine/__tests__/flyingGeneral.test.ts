import { describe, it, expect } from 'vitest';
import type { Board, Piece, Position } from '../types';
import { isFlyingGeneral } from '../index';
import { getLegalMoves } from '../specialRules';
import { makeMove } from '../index';
import { createInitialBoard } from '../constants';

// ── Helpers ──

/** Create an empty 10×9 board. */
function emptyBoard(): Board {
  return Array.from({ length: 10 }, () =>
    Array.from({ length: 9 }, () => null),
  );
}

/** Place a piece on the board and return a new board. */
function place(board: Board, row: number, col: number, piece: Piece): Board {
  const b = board.map((r) => r.map((c) => c));
  b[row]![col] = piece;
  return b;
}

/** Convert moves to a Set of "row,col" strings for easy membership assertions. */
function moveSet(moves: Position[]): Set<string> {
  return new Set(moves.map((m) => `${m.row},${m.col}`));
}

// ── Piece factories ──

const redGeneral: Piece = { type: 'general', player: 'red' };
const blackGeneral: Piece = { type: 'general', player: 'black' };
const redChariot: Piece = { type: 'chariot', player: 'red' };
const redSoldier: Piece = { type: 'soldier', player: 'red' };
const blackSoldier: Piece = { type: 'soldier', player: 'black' };
const redAdvisor: Piece = { type: 'advisor', player: 'red' };
const redKnight: Piece = { type: 'knight', player: 'red' };
const _blackKnight: Piece = { type: 'knight', player: 'black' };

// ────────────────────────────────────────────────────────────────────
// 1. isFlyingGeneral
// ────────────────────────────────────────────────────────────────────

describe('isFlyingGeneral', () => {
  it('returns false when generals are on different columns', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 0, 3, blackGeneral); // different column

    expect(isFlyingGeneral(board)).toBe(false);
  });

  it('returns false when generals are on the same column but pieces block', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 0, 4, blackGeneral);
    board = place(board, 5, 4, redSoldier); // blocker between them

    expect(isFlyingGeneral(board)).toBe(false);
  });

  it('returns true when generals are on the same column with nothing between', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 0, 4, blackGeneral);

    expect(isFlyingGeneral(board)).toBe(true);
  });

  it('returns false when only one piece blocks, even at edge', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 1, 4, blackGeneral);
    board = place(board, 5, 4, redSoldier); // blocker between rows 1 and 9

    expect(isFlyingGeneral(board)).toBe(false);
  });

  it('returns true even with pieces on other columns', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 0, 4, blackGeneral);
    // Pieces on other columns — should not affect column 4
    board = place(board, 5, 3, redSoldier);
    board = place(board, 3, 5, blackSoldier);
    board = place(board, 7, 0, redKnight);

    expect(isFlyingGeneral(board)).toBe(true);
  });
});

// ────────────────────────────────────────────────────────────────────
// 2. getLegalMoves — flying general filtering
// ────────────────────────────────────────────────────────────────────

describe('getLegalMoves — flying general filtering', () => {
  it('chariot staying on column 4 does not create flying general — move is legal', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 5, 4, redChariot); // blocker on col 4
    board = place(board, 0, 4, blackGeneral);
    // Additional blocker so chariot moving down doesn't create flying general
    board = place(board, 3, 4, redKnight);

    const moves = getLegalMoves(board, { row: 5, col: 4 });
    const ms = moveSet(moves);

    // Moving down stays on column 4 — still blocking
    expect(ms.has('6,4')).toBe(true);
    expect(ms.has('7,4')).toBe(true);
    expect(ms.has('8,4')).toBe(true);
    // Moving up to (4,4) is open
    expect(ms.has('4,4')).toBe(true);
  });

  it('chariot moving off column 4 would create flying general — move is rejected', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 5, 4, redChariot); // sole blocker on col 4
    board = place(board, 0, 4, blackGeneral);
    // The chariot is the ONLY piece between the two generals.
    // Moving it off column 4 leaves them facing each other.

    const moves = getLegalMoves(board, { row: 5, col: 4 });
    const ms = moveSet(moves);

    // Moving laterally off column 4 leaves generals facing each other
    expect(ms.has('5,3')).toBe(false);
    expect(ms.has('5,5')).toBe(false);
  });

  it('soldier on column 4 moving sideways exposes generals — move is rejected', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 6, 4, redSoldier); // sole blocker, has crossed river
    board = place(board, 0, 4, blackGeneral);

    const moves = getLegalMoves(board, { row: 6, col: 4 });
    const ms = moveSet(moves);

    // Forward (up for red) stays on column — still blocking
    expect(ms.has('5,4')).toBe(true);
    // Sideways would vacate column 4 — flying general
    expect(ms.has('6,3')).toBe(false);
    expect(ms.has('6,5')).toBe(false);
  });

  it('advisor that is the sole blocker on column 4 cannot vacate column 4', () => {
    // Red general at (9,4), Red advisor at (8,4) — advisor is sole blocker on col 4.
    // Black general at (0,4).
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 8, 4, redAdvisor); // sole blocker on col 4
    board = place(board, 0, 4, blackGeneral);

    const moves = getLegalMoves(board, { row: 8, col: 4 });
    const ms = moveSet(moves);

    // All advisor moves vacate col 4 → all create flying general → all rejected
    expect(ms.has('7,3')).toBe(false);
    expect(ms.has('7,5')).toBe(false);
    expect(ms.has('9,3')).toBe(false);
    expect(ms.has('9,5')).toBe(false);

    // No legal moves for this advisor
    expect(moves).toHaveLength(0);
  });

  it('general move that would put both generals on the same column with nothing between is rejected', () => {
    // Red general at (9,3), Black general at (0,5).
    // Moving to (9,4): red on col 4, black on col 5 → different columns → legal.
    let board = emptyBoard();
    board = place(board, 9, 3, redGeneral);
    board = place(board, 0, 5, blackGeneral);

    const moves = getLegalMoves(board, { row: 9, col: 3 });
    const ms = moveSet(moves);

    // (9,4): red at col 4, black at col 5 → different columns → legal
    expect(ms.has('9,4')).toBe(true);
    // (8,3): red at col 3, black at col 5 → different columns → legal
    expect(ms.has('8,3')).toBe(true);
  });

  it('general lateral move toward opponent general column with nothing between is rejected', () => {
    // Red general at (9,4), Black general at (0,5).
    // Moving to (9,5) puts both on col 5 → flying general → rejected.
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 0, 5, blackGeneral);

    const moves = getLegalMoves(board, { row: 9, col: 4 });
    const ms = moveSet(moves);

    // (9,5) → same column as black general, no blocker → flying general
    expect(ms.has('9,5')).toBe(false);
    // (9,3) → different column → legal
    expect(ms.has('9,3')).toBe(true);
    // (8,4) → different column from black general → legal
    expect(ms.has('8,4')).toBe(true);
  });

  it('chariot moving off column is legal when another piece still blocks', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 5, 4, redChariot);
    board = place(board, 3, 4, redSoldier); // additional blocker on col 4
    board = place(board, 0, 4, blackGeneral);

    const moves = getLegalMoves(board, { row: 5, col: 4 });
    const ms = moveSet(moves);

    // Chariot moving sideways is fine — soldier still blocks
    expect(ms.has('5,3')).toBe(true);
    expect(ms.has('5,5')).toBe(true);
  });
});

// ────────────────────────────────────────────────────────────────────
// 3. makeMove — flying general rejection
// ────────────────────────────────────────────────────────────────────

describe('makeMove — flying general rejection', () => {
  it('makeMove returns valid:false when chariot move creates flying general', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 5, 4, redChariot); // sole blocker on col 4
    board = place(board, 0, 4, blackGeneral);
    // The chariot is the ONLY piece between the two generals.

    const state = { board, currentPlayer: 'red' as const };

    const result = makeMove(state, {
      from: { row: 5, col: 4 },
      to: { row: 5, col: 3 },
    });

    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  it('makeMove returns valid:true when move does NOT create flying general', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 5, 4, redChariot); // blocker on col 4
    board = place(board, 3, 4, redSoldier); // extra blocker
    board = place(board, 0, 4, blackGeneral);

    const state = { board, currentPlayer: 'red' as const };

    const result = makeMove(state, {
      from: { row: 5, col: 4 },
      to: { row: 6, col: 4 }, // stays on column 4
    });

    expect(result.valid).toBe(true);
    expect(result.newState).toBeDefined();
  });

  it('makeMove returns valid:true for chariot moving sideways when another piece blocks column', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 5, 4, redChariot);
    board = place(board, 3, 4, redSoldier); // additional blocker on col 4
    board = place(board, 0, 4, blackGeneral);

    const state = { board, currentPlayer: 'red' as const };

    const result = makeMove(state, {
      from: { row: 5, col: 4 },
      to: { row: 5, col: 3 },
    });

    expect(result.valid).toBe(true);
    expect(result.newState).toBeDefined();
  });

  it('makeMove returns valid:false for soldier moving sideways off blocking column', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 6, 4, redSoldier); // sole blocker, has crossed river
    board = place(board, 0, 4, blackGeneral);

    const state = { board, currentPlayer: 'red' as const };

    const result = makeMove(state, {
      from: { row: 6, col: 4 },
      to: { row: 6, col: 3 }, // sideways — exposes generals
    });

    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  it('makeMove returns valid:false for general moving to face opponent general', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, redGeneral);
    board = place(board, 0, 5, blackGeneral);

    const state = { board, currentPlayer: 'red' as const };

    const result = makeMove(state, {
      from: { row: 9, col: 4 },
      to: { row: 9, col: 5 }, // creates flying general on col 5
    });

    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });
});

// ────────────────────────────────────────────────────────────────────
// 4. Initial board — no flying general
// ────────────────────────────────────────────────────────────────────

describe('initial board — no flying general', () => {
  it('initial board does not have flying general', () => {
    const board = createInitialBoard();
    // Both generals at col 4, but soldiers at (3,4) and (6,4) block
    expect(isFlyingGeneral(board)).toBe(false);
  });
});
