import { describe, it, expect } from 'vitest';
import type { Board, BoardState, Piece } from '../types';
import { isCheckmate, isStalemate, getGameStatus } from '../index';

// ── Helpers ──

/** Create an empty 10×9 board. */
function emptyBoard(): Board {
  return Array.from({ length: 10 }, () =>
    Array.from({ length: 9 }, () => null),
  );
}

/** Place a piece on the board, returning a new board. */
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

/** Shorthand factories */
const redGeneral: Piece = { type: 'general', player: 'red' };
const blackGeneral: Piece = { type: 'general', player: 'black' };
const redChariot: Piece = { type: 'chariot', player: 'red' };
const _blackChariot: Piece = { type: 'chariot', player: 'black' };
const redKnight: Piece = { type: 'knight', player: 'red' };
const redAdvisor: Piece = { type: 'advisor', player: 'red' };
const _redSoldier: Piece = { type: 'soldier', player: 'red' };

// ── isCheckmate ──

describe('isCheckmate', () => {
  it('returns true when the player is in check and has no legal moves', () => {
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);
    board = place(board, 0, 2, redChariot);  // controls row 0 left of general
    board = place(board, 0, 6, redChariot);  // controls row 0 right of general
    board = place(board, 1, 4, redChariot);  // direct check from below
    board = place(board, 9, 4, redGeneral);

    expect(isCheckmate(board, 'black')).toBe(true);
  });

  it('returns false when the player is in check but can escape', () => {
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);
    board = place(board, 1, 4, redChariot); // check
    board = place(board, 9, 4, redGeneral);

    expect(isCheckmate(board, 'black')).toBe(false);
  });

  it('returns false when the player is not in check', () => {
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);
    board = place(board, 9, 4, redGeneral);
    board = place(board, 9, 0, redChariot);

    expect(isCheckmate(board, 'black')).toBe(false);
    expect(isCheckmate(board, 'red')).toBe(false);
  });

  it('returns false when not in check but no legal moves (that is stalemate, not checkmate)', () => {
    let board = emptyBoard();
    board = place(board, 0, 3, blackGeneral);
    board = place(board, 9, 4, redGeneral);

    // Black general can move to (0,4) — not in check, has moves
    expect(isCheckmate(board, 'black')).toBe(false);
  });
});

// ── Known Classical Checkmate Patterns (经典杀法) ──

describe('known checkmate patterns (经典杀法)', () => {
  it('detects 钓鱼马 (Angler\'s Horse) checkmate', () => {
    // 钓鱼马: a red knight at (2,3) checks the black general at (0,4).
    // The knight attacks (0,4) via L-move { dr: -2, dc: +1 }, blocking leg at (1,3) (empty).
    // It also attacks (1,4) via L-move { dr: -1, dc: +1 }, preventing forward escape.
    // A red advisor at (0,2) blocks the (0,3) diagonal escape (out of palace anyway).
    // A red chariot at (0,6) controls the entire row 0, covering (0,5).
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);   // 将 at palace center
    board = place(board, 0, 2, redAdvisor);      // blocks (0,3) escape
    board = place(board, 2, 3, redKnight);       // 钓鱼马: checks (0,4), attacks (1,4)
    board = place(board, 0, 6, redChariot);      // controls row 0 → covers (0,5)
    board = place(board, 9, 4, redGeneral);      // red king (no flying general — knight at (2,3) is not on col 4)

    expect(isCheckmate(board, 'black')).toBe(true);
  });

  it('detects 双车错 (Double-Chariot Stagger) checkmate', () => {
    // 双车错: two chariots staggered on row and column deliver checkmate.
    // Chariot at (0,3) controls the entire row 0 (covering (0,5) escape)
    //   and the entire column 3 (covering escape/capture to (0,3)).
    // Chariot at (1,4) checks the general at (0,4) along column 4.
    // Chariot at (1,3) covers column 3 (blocks general from moving to col 3)
    //   and row 1 (blocks forward escape to (1,3) and (1,5)).
    // Together the three chariots leave the general with no escape.
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);   // 将 at palace center
    board = place(board, 0, 3, redChariot);      // controls row 0 + col 3
    board = place(board, 1, 4, redChariot);      // checks along col 4, controls row 1
    board = place(board, 1, 3, redChariot);      // covers col 3 + row 1 (ensures no escape)
    board = place(board, 9, 4, redGeneral);      // red king in palace

    expect(isCheckmate(board, 'black')).toBe(true);
    expect(isStalemate(board, 'black')).toBe(false);
  });
});

// ── isStalemate ──

describe('isStalemate', () => {
  it('returns true when the player is NOT in check and has no legal moves', () => {
    let board = emptyBoard();
    board = place(board, 0, 3, blackGeneral);
    board = place(board, 1, 4, redChariot);
    board = place(board, 9, 4, redGeneral);

    expect(isStalemate(board, 'black')).toBe(true);
  });

  it('returns false when the player has legal moves available', () => {
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);
    board = place(board, 9, 4, redGeneral);

    expect(isStalemate(board, 'black')).toBe(false);
  });

  it('returns false when the player is in check (that is checkmate, not stalemate)', () => {
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);
    board = place(board, 0, 2, redChariot);
    board = place(board, 0, 6, redChariot);
    board = place(board, 1, 4, redChariot); // check
    board = place(board, 9, 4, redGeneral);

    expect(isStalemate(board, 'black')).toBe(false);
  });
});

// ── getGameStatus ──

describe('getGameStatus', () => {
  it('returns { type: "playing" } for a normal game position', () => {
    const board = emptyBoard();
    const state: BoardState = {
      board: place(place(board, 0, 4, blackGeneral), 9, 4, redGeneral),
      currentPlayer: 'red',
    };

    const status = getGameStatus(state);
    expect(status).toEqual({ type: 'playing' });
  });

  it('returns { type: "checkmate", winner } when currentPlayer is checkmated', () => {
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);
    board = place(board, 0, 2, redChariot);
    board = place(board, 0, 6, redChariot);
    board = place(board, 1, 4, redChariot); // check
    board = place(board, 9, 4, redGeneral);

    const state: BoardState = {
      board,
      currentPlayer: 'black',
    };

    const status = getGameStatus(state);
    expect(status.type).toBe('checkmate');
    if (status.type === 'checkmate') {
      expect(status.winner).toBe('red');
    }
  });

  it('returns { type: "stalemate", loser } when currentPlayer is stalemated', () => {
    let board = emptyBoard();
    board = place(board, 0, 3, blackGeneral);
    board = place(board, 1, 4, redChariot);
    board = place(board, 9, 4, redGeneral);

    const state: BoardState = {
      board,
      currentPlayer: 'black',
    };

    const status = getGameStatus(state);
    expect(status.type).toBe('stalemate');
    if (status.type === 'stalemate') {
      expect(status.loser).toBe('black');
    }
  });

  it('returns { type: "playing" } at the initial board position', async () => {
    const { createInitialBoardState } = await import('../index');
    const state = createInitialBoardState();

    const status = getGameStatus(state);
    expect(status).toEqual({ type: 'playing' });
  });
});
