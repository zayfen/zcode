import { describe, expect, it } from 'vitest';
import { makeMove, getLegalMoves, isInCheck } from '../index';
import { createInitialBoardState } from '../constants';

describe('makeMove engine test', () => {
  it('rejects moving an empty square', () => {
    const state = createInitialBoardState();
    const result = makeMove(state, { from: { row: 4, col: 4 }, to: { row: 5, col: 4 } });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  it('rejects moving opponent piece', () => {
    const state = createInitialBoardState();
    // Red's turn, try to move black piece
    const result = makeMove(state, { from: { row: 0, col: 0 }, to: { row: 1, col: 0 } });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  it('rejects illegal destination', () => {
    const state = createInitialBoardState();
    // Red knight at (9,1) cannot move to (9,2) — that's not an L-shape
    const result = makeMove(state, { from: { row: 9, col: 1 }, to: { row: 9, col: 2 } });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  it('applies a valid chariot move and switches player', () => {
    const state = createInitialBoardState();
    const result = makeMove(state, { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } });
    expect(result.valid).toBe(true);
    expect(result.newState).toBeDefined();
    expect(result.newState!.currentPlayer).toBe('black');
    expect(result.newState!.board[8][0]).toEqual({ type: 'chariot', player: 'red' });
    expect(result.newState!.board[9][0]).toBeNull();
    expect(result.captured).toBeUndefined();
  });

  it('applies a valid knight move', () => {
    const state = createInitialBoardState();
    // Red knight at (9,1) → (7,2) — valid L-shape
    const result = makeMove(state, { from: { row: 9, col: 1 }, to: { row: 7, col: 2 } });
    expect(result.valid).toBe(true);
    expect(result.newState!.board[7][2]).toEqual({ type: 'knight', player: 'red' });
    expect(result.newState!.board[9][1]).toBeNull();
  });

  it('returns captured piece when capturing', () => {
    // Set up a board where a capture doesn't create a flying-general violation.
    // Custom board: red chariot and black soldier on column 4 (same as generals).
    const board: (import('../types').Piece | null)[][] = Array.from({ length: 10 }, () =>
      Array.from({ length: 9 }, () => null),
    );
    // Red general at (9,4)
    board[9]![4] = { type: 'general', player: 'red' };
    // Red chariot at (7,4) — on same column as generals
    board[7]![4] = { type: 'chariot', player: 'red' };
    // Black general at (0,4)
    board[0]![4] = { type: 'general', player: 'black' };
    // Black soldier at (6,4) — capture target on same column
    board[6]![4] = { type: 'soldier', player: 'black' };

    const state: import('../types').BoardState = {
      board,
      currentPlayer: 'red',
    };

    // Red chariot captures black soldier on the same column
    const result = makeMove(state, { from: { row: 7, col: 4 }, to: { row: 6, col: 4 } });
    expect(result.valid).toBe(true);
    expect(result.captured).toEqual({ type: 'soldier', player: 'black' });
    expect(result.newState!.board[6][4]).toEqual({ type: 'chariot', player: 'red' });
    expect(result.newState!.board[7][4]).toBeNull();
  });

  it('original board is not mutated after move', () => {
    const state = createInitialBoardState();
    const originalBoard = state.board;
    // Make a move
    const result = makeMove(state, { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } });
    expect(result.valid).toBe(true);
    // Original board should not be changed
    expect(originalBoard[8][0]).toBeNull();
    expect(originalBoard[9][0]).toEqual({ type: 'chariot', player: 'red' });
  });

  it('cannot move general out of palace', () => {
    const state = createInitialBoardState();
    // Red general at (9,4) — cannot move to (9,6) (out of palace)
    const result = makeMove(state, { from: { row: 9, col: 4 }, to: { row: 9, col: 6 } });
    expect(result.valid).toBe(false);
  });

  it('chariot can move multiple squares in straight line', () => {
    // Custom board: red chariot + both generals, with a blocker on col 4 to prevent flying general
    const board: (import('../types').Piece | null)[][] = Array.from({ length: 10 }, () =>
      Array.from({ length: 9 }, () => null),
    );
    board[9]![4] = { type: 'general', player: 'red' };
    board[0]![4] = { type: 'general', player: 'black' };
    board[9]![0] = { type: 'chariot', player: 'red' };
    board[4]![4] = { type: 'soldier', player: 'red' }; // blocker on col 4

    const state: import('../types').BoardState = {
      board,
      currentPlayer: 'red',
    };

    // Red chariot moves from (9,0) up to (4,0) — 5 squares
    const result = makeMove(state, { from: { row: 9, col: 0 }, to: { row: 4, col: 0 } });
    expect(result.valid).toBe(true);
    expect(result.newState!.board[4][0]).toEqual({ type: 'chariot', player: 'red' });
    expect(result.newState!.board[9][0]).toBeNull();
  });
});
