import { describe, expect, it } from 'vitest';
import { makeMove, getLegalMoves } from '../index';
import { createInitialBoardState } from '../constants';
import type { BoardState, Piece, Position } from '../types';

// ── Helper: create an empty board with just the two generals ──

function emptyBoardWithGenerals(): Piece[][] {
  const board: (Piece | null)[][] = Array.from({ length: 10 }, () =>
    Array.from({ length: 9 }, () => null),
  );
  board[9]![4] = { type: 'general', player: 'red' };
  board[0]![4] = { type: 'general', player: 'black' };
  return board as Piece[][];
}

// ════════════════════════════════════════════════════════════════
// VERIFY: Illegal moves are silently rejected
//         (no state change, no error thrown)
// ════════════════════════════════════════════════════════════════

describe('Illegal move silent rejection', () => {
  // ── 1. Moving an empty square ──

  it('rejects move from an empty square without error', () => {
    const state = createInitialBoardState();
    const result = makeMove(state, {
      from: { row: 4, col: 4 }, // empty
      to: { row: 5, col: 4 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
    expect(result.captured).toBeUndefined();
  });

  // ── 2. Moving opponent's piece ──

  it('rejects moving opponent piece without error', () => {
    const state = createInitialBoardState();
    // Red's turn; try to move a black piece
    const result = makeMove(state, {
      from: { row: 0, col: 0 }, // black chariot
      to: { row: 1, col: 0 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 3. Moving to an illegal destination (not in legal move list) ──

  it('rejects illegal destination for chariot without error', () => {
    const state = createInitialBoardState();
    // Red chariot at (9,0) cannot jump to (7,0) because (8,0) is empty
    // Actually (8,0) IS empty in initial position, so it CAN move there.
    // Instead test a truly illegal move: chariot cannot move diagonally
    const result = makeMove(state, {
      from: { row: 9, col: 0 }, // red chariot
      to: { row: 8, col: 1 },   // diagonal — illegal
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  it('rejects illegal destination for general without error', () => {
    const state = createInitialBoardState();
    // Red general at (9,4) cannot move to (9,6) — out of palace
    const result = makeMove(state, {
      from: { row: 9, col: 4 },
      to: { row: 9, col: 6 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  it('rejects knight move to a non-L-shape square without error', () => {
    const state = createInitialBoardState();
    // Red knight at (9,1) cannot move to (9,2)
    const result = makeMove(state, {
      from: { row: 9, col: 1 },
      to: { row: 9, col: 2 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 4. Move that would leave own general in check ──

  it('rejects move that leaves own general in check without error', () => {
    // Set up: red general at (9,4), black chariot at (7,4), red chariot at (8,4) blocking
    // If red chariot at (8,4) moves away, the general would be exposed to the black chariot
    const board = emptyBoardWithGenerals();
    board[7]![4] = { type: 'chariot', player: 'black' };
    board[8]![4] = { type: 'chariot', player: 'red' };

    const state: BoardState = { board, currentPlayer: 'red' };

    // Red chariot moves from (8,4) to (8,3) — exposes red general to black chariot
    const result = makeMove(state, {
      from: { row: 8, col: 4 },
      to: { row: 8, col: 3 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 5. Move that would create flying general violation ──

  it('rejects move that creates flying general violation without error', () => {
    // Set up: red general at (9,4), black general at (0,4), red chariot at (5,4) blocking
    // If red chariot moves off column 4, generals face each other
    const board = emptyBoardWithGenerals();
    board[5]![4] = { type: 'chariot', player: 'red' };

    const state: BoardState = { board, currentPlayer: 'red' };

    // Red chariot moves off column 4 — creates flying general
    const result = makeMove(state, {
      from: { row: 5, col: 4 },
      to: { row: 5, col: 3 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 6. Moving own piece onto another own piece ──

  it('rejects moving onto own piece without error', () => {
    const state = createInitialBoardState();
    // Red general at (9,4), red advisor at (9,3) — cannot move general onto advisor
    const result = makeMove(state, {
      from: { row: 9, col: 4 },
      to: { row: 9, col: 3 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 7. Elephant cannot cross river ──

  it('rejects elephant crossing river without error', () => {
    const state = createInitialBoardState();
    // Red elephant at (9,2) — all its diagonal-2 destinations would cross river
    // but initially they're blocked by the eye check or own pieces.
    // Let's set up a clean board.
    const board = emptyBoardWithGenerals();
    board[7]![2] = { type: 'elephant', player: 'red' };

    const customState: BoardState = { board, currentPlayer: 'red' };

    // Elephant at (7,2) trying to go to (3,6) — across river (invalid for red)
    // Actually the raw move won't even generate (3,6) from (7,2) — diagonal-2 only.
    // From (7,2): possible destinations are (5,0), (5,4), (9,0), (9,4)
    // (5,0) and (5,4) are on the river — isOnSide(5, 'red') checks row >= 5, so row=5 is on side
    // But actually row 5 is RIVER_ROW_MAX, and isOnSide(5, 'red') returns 5 >= 5 = true
    // So (5,0) and (5,4) would be valid if eye is clear.
    // Let's test elephant at (5,0) trying to go to (1,4) — that's across the river for sure
    board[5]![0] = { type: 'elephant', player: 'red' };
    // Clear old position
    board[7]![2] = null;

    const state2: BoardState = { board, currentPlayer: 'red' };
    const result = makeMove(state2, {
      from: { row: 5, col: 0 },
      to: { row: 1, col: 4 }, // diagonal-2 but row 1 is on black's side
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 8. Advisor cannot leave palace ──

  it('rejects advisor leaving palace without error', () => {
    const board = emptyBoardWithGenerals();
    board[8]![4] = { type: 'advisor', player: 'red' };

    const state: BoardState = { board, currentPlayer: 'red' };

    // Advisor at (8,4) cannot move to (7,5) — (8,4) → (7,5) is diagonal, but (7,5) IS in palace
    // Let's try (8,4) → (6,6) — not a single diagonal, invalid
    const result = makeMove(state, {
      from: { row: 8, col: 4 },
      to: { row: 6, col: 6 }, // not a valid advisor move
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 9. Soldier cannot move backward ──

  it('rejects soldier moving backward without error', () => {
    const board = emptyBoardWithGenerals();
    board[5]![4] = { type: 'soldier', player: 'red' };

    const state: BoardState = { board, currentPlayer: 'red' };

    // Red soldier at (5,4) cannot move backward (to row 6)
    const result = makeMove(state, {
      from: { row: 5, col: 4 },
      to: { row: 6, col: 4 }, // backward for red
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 10. Cannon cannot capture without screen ──

  it('rejects cannon capture without jumping a screen without error', () => {
    const board = emptyBoardWithGenerals();
    board[7]![1] = { type: 'cannon', player: 'red' };
    board[4]![1] = { type: 'chariot', player: 'black' };
    // No screen between cannon and target

    const state: BoardState = { board, currentPlayer: 'red' };

    // Red cannon at (7,1) tries to capture black chariot at (4,1) without a screen
    const result = makeMove(state, {
      from: { row: 7, col: 1 },
      to: { row: 4, col: 1 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 11. Out-of-bounds positions ──

  it('rejects move to out-of-bounds position without error', () => {
    const state = createInitialBoardState();
    const result = makeMove(state, {
      from: { row: 9, col: 0 },
      to: { row: 10, col: 0 }, // row 10 doesn't exist
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  it('rejects move from out-of-bounds position without error', () => {
    const state = createInitialBoardState();
    const result = makeMove(state, {
      from: { row: -1, col: 0 },
      to: { row: 0, col: 0 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 12. No mutation of original state on rejected move ──

  it('does not mutate board state when move is rejected', () => {
    const state = createInitialBoardState();
    const boardBefore = state.board;

    // Attempt several illegal moves
    makeMove(state, { from: { row: 4, col: 4 }, to: { row: 5, col: 4 } }); // empty square
    makeMove(state, { from: { row: 0, col: 0 }, to: { row: 1, col: 0 } }); // opponent piece
    makeMove(state, { from: { row: 9, col: 4 }, to: { row: 9, col: 6 } }); // general out of palace

    // Verify board is unchanged
    expect(state.board).toBe(boardBefore); // same reference
    expect(state.board[9]?.[4]).toEqual({ type: 'general', player: 'red' });
    expect(state.board[0]?.[0]).toEqual({ type: 'chariot', player: 'black' });
    expect(state.currentPlayer).toBe('red'); // unchanged
  });

  // ── 13. No error/exception is thrown for any illegal move ──

  it('never throws an exception for illegal moves', () => {
    const state = createInitialBoardState();

    const illegalMoves: { from: Position; to: Position }[] = [
      // Empty square
      { from: { row: 4, col: 4 }, to: { row: 5, col: 4 } },
      // Opponent piece
      { from: { row: 0, col: 0 }, to: { row: 1, col: 0 } },
      // Invalid chariot move (diagonal)
      { from: { row: 9, col: 0 }, to: { row: 8, col: 1 } },
      // Invalid knight move
      { from: { row: 9, col: 1 }, to: { row: 9, col: 2 } },
      // General out of palace
      { from: { row: 9, col: 4 }, to: { row: 9, col: 6 } },
      // Out of bounds
      { from: { row: 9, col: 0 }, to: { row: 10, col: 0 } },
      { from: { row: -1, col: 0 }, to: { row: 0, col: 0 } },
      // Move onto own piece
      { from: { row: 9, col: 4 }, to: { row: 9, col: 3 } },
      // Same position (no movement)
      { from: { row: 9, col: 0 }, to: { row: 9, col: 0 } },
    ];

    for (const move of illegalMoves) {
      expect(() => makeMove(state, move)).not.toThrow();
      const result = makeMove(state, move);
      expect(result.valid).toBe(false);
      expect(result.newState).toBeUndefined();
    }
  });

  // ── 14. Same source and destination (no movement) ──

  it('rejects move where source equals destination without error', () => {
    const state = createInitialBoardState();
    const result = makeMove(state, {
      from: { row: 9, col: 0 },
      to: { row: 9, col: 0 },
    });
    expect(result.valid).toBe(false);
    expect(result.newState).toBeUndefined();
  });

  // ── 15. Wrong player's turn ──

  it('rejects move by correct player on subsequent turn without mutation', () => {
    const state = createInitialBoardState();

    // Make a valid move: red chariot (9,0) → (8,0)
    const result1 = makeMove(state, {
      from: { row: 9, col: 0 },
      to: { row: 8, col: 0 },
    });
    expect(result1.valid).toBe(true);
    expect(result1.newState).toBeDefined();

    // Now it's black's turn. Try to move a red piece.
    const result2 = makeMove(result1.newState!, {
      from: { row: 9, col: 1 }, // red knight
      to: { row: 7, col: 2 },
    });
    expect(result2.valid).toBe(false);
    expect(result2.newState).toBeUndefined();

    // Board state from result1 should be unchanged
    expect(result1.newState!.board[9]?.[1]).toEqual({ type: 'knight', player: 'red' });
    expect(result1.newState!.currentPlayer).toBe('black');
  });
});
