import { describe, expect, it } from 'vitest';
import {
  makeMove,
  getGameStatus,
  createInitialBoardState,
} from '../index';
import type { Board, BoardState, Move, Piece, PieceType, Player } from '../types';

// ── Helpers ──

/** Assert that a cell contains a specific piece type and player. */
function expectPiece(
  board: Board,
  row: number,
  col: number,
  type: PieceType,
  player: Player,
): void {
  const p = board[row]?.[col] ?? null;
  expect(p).not.toBeNull();
  expect(p!.type).toBe(type);
  expect(p!.player).toBe(player);
}

/** Assert that a cell is empty. */
function expectEmpty(board: Board, row: number, col: number): void {
  expect(board[row]?.[col] ?? null).toBeNull();
}

/** Count all pieces on the board. */
function countPieces(board: Board): number {
  let count = 0;
  for (const row of board) {
    for (const cell of row) {
      if (cell) count++;
    }
  }
  return count;
}

// ── Test ──

describe('full 10-move game sequence', () => {
  it('plays 10 moves and verifies final board state', () => {
    // ────────────────────────────────────────────────────────
    //  Scripted 10-half-move sequence (no captures)
    //
    //  Initial position is the standard Xiangqi setup.
    //  Row 0 = black back rank, Row 9 = red back rank.
    // ────────────────────────────────────────────────────────
    const moves: Array<{ from: { row: number; col: number }; to: { row: number; col: number }; label: string }> = [
      // Move 1 — Red left chariot forward 1
      { from: { row: 9, col: 0 }, to: { row: 8, col: 0 }, label: 'Red left chariot forward 1' },
      // Move 2 — Black left knight L-shape
      { from: { row: 0, col: 1 }, to: { row: 2, col: 2 }, label: 'Black left knight L-shape' },
      // Move 3 — Red left cannon slides right 3
      { from: { row: 7, col: 1 }, to: { row: 7, col: 4 }, label: 'Red left cannon slides right 3' },
      // Move 4 — Black right cannon slides left 3
      { from: { row: 2, col: 7 }, to: { row: 2, col: 4 }, label: 'Black right cannon slides left 3' },
      // Move 5 — Red right knight L-shape
      { from: { row: 9, col: 7 }, to: { row: 7, col: 6 }, label: 'Red right knight L-shape' },
      // Move 6 — Black right chariot forward 1
      { from: { row: 0, col: 8 }, to: { row: 1, col: 8 }, label: 'Black right chariot forward 1' },
      // Move 7 — Red center soldier forward 1
      { from: { row: 6, col: 4 }, to: { row: 5, col: 4 }, label: 'Red center soldier forward 1' },
      // Move 8 — Black center soldier forward 1
      { from: { row: 3, col: 4 }, to: { row: 4, col: 4 }, label: 'Black center soldier forward 1' },
      // Move 9 — Red left chariot forward 1 (only to row 7, blocked by soldier at row 6)
      { from: { row: 8, col: 0 }, to: { row: 7, col: 0 }, label: 'Red left chariot forward 1 (to row 7)' },
      // Move 10 — Black right knight L-shape
      { from: { row: 0, col: 7 }, to: { row: 2, col: 6 }, label: 'Black right knight L-shape' },
    ];

    const expectedPlayer: Player[] = [
      'red', 'black', 'red', 'black', 'red',
      'black', 'red', 'black', 'red', 'black',
    ];

    // Create initial game state
    let state: BoardState = createInitialBoardState();
    expect(state.currentPlayer).toBe('red');

    // ── Play each move and verify intermediate state ──
    for (let i = 0; i < moves.length; i++) {
      const { from, to, label } = moves[i];
      const movingPlayer = expectedPlayer[i]!;
      const opponent: Player = movingPlayer === 'red' ? 'black' : 'red';

      const move: Move = { from, to };
      const result = makeMove(state, move);

      // 1. Move must be valid
      expect(result.valid).toBe(true);

      // 2. New state must be defined
      expect(result.newState).toBeDefined();

      // 3. Moved piece must be at the destination
      const movedPiece = result.newState!.board[to.row][to.col];
      expect(movedPiece).not.toBeNull();
      expect(movedPiece!.player).toBe(movingPlayer);

      // 4. Source square must be empty
      expect(result.newState!.board[from.row][from.col]).toBeNull();

      // 5. Turn must have switched
      expect(result.newState!.currentPlayer).toBe(opponent);

      // 6. No captures in this sequence
      expect(result.captured).toBeUndefined();

      // Advance state
      state = result.newState!;
    }

    // ── Final board state verification ──
    const { board, currentPlayer } = state;

    // Turn should be red (black just played move 10)
    expect(currentPlayer).toBe('red');

    // Game should still be in progress
    const status = getGameStatus(state);
    expect(status.type).toBe('playing');

    // Total pieces: no captures occurred, so all 32 remain
    expect(countPieces(board)).toBe(32);

    // ── Black pieces ──

    // Black back rank (row 0)
    expectPiece(board, 0, 0, 'chariot', 'black');
    expectEmpty(board, 0, 1);  // knight moved to (2,2)
    expectPiece(board, 0, 2, 'elephant', 'black');
    expectPiece(board, 0, 3, 'advisor', 'black');
    expectPiece(board, 0, 4, 'general', 'black');
    expectPiece(board, 0, 5, 'advisor', 'black');
    expectPiece(board, 0, 6, 'elephant', 'black');
    expectEmpty(board, 0, 7);  // knight moved to (2,6)
    expectEmpty(board, 0, 8);  // chariot moved to (1,8)

    // Black knight from (0,1) now at (2,2)
    expectPiece(board, 2, 2, 'knight', 'black');
    // Black right cannon from (2,7) now at (2,4)
    expectPiece(board, 2, 4, 'cannon', 'black');
    // Black left cannon still at (2,1)
    expectPiece(board, 2, 1, 'cannon', 'black');
    // Black knight from (0,7) now at (2,6)
    expectPiece(board, 2, 6, 'knight', 'black');

    // Black soldiers (row 3)
    expectPiece(board, 3, 0, 'soldier', 'black');
    expectPiece(board, 3, 2, 'soldier', 'black');
    expectEmpty(board, 3, 4);  // soldier moved to (4,4)
    expectPiece(board, 3, 6, 'soldier', 'black');
    expectPiece(board, 3, 8, 'soldier', 'black');

    // Black center soldier advanced to (4,4)
    expectPiece(board, 4, 4, 'soldier', 'black');

    // Black right chariot advanced to (1,8)
    expectPiece(board, 1, 8, 'chariot', 'black');

    // ── Red pieces ──

    // Red back rank (row 9)
    expectEmpty(board, 9, 0);  // chariot moved out
    expectPiece(board, 9, 1, 'knight', 'red');
    expectPiece(board, 9, 2, 'elephant', 'red');
    expectPiece(board, 9, 3, 'advisor', 'red');
    expectPiece(board, 9, 4, 'general', 'red');
    expectPiece(board, 9, 5, 'advisor', 'red');
    expectPiece(board, 9, 6, 'elephant', 'red');
    expectEmpty(board, 9, 7);  // knight moved to (7,6)
    expectPiece(board, 9, 8, 'chariot', 'red');

    // Red left cannon moved to (7,4)
    expectPiece(board, 7, 4, 'cannon', 'red');
    // Red right cannon still at (7,7)
    expectPiece(board, 7, 7, 'cannon', 'red');
    // Red right knight moved to (7,6)
    expectPiece(board, 7, 6, 'knight', 'red');

    // Red soldiers (row 6) — center moved out
    expectPiece(board, 6, 0, 'soldier', 'red');
    expectPiece(board, 6, 2, 'soldier', 'red');
    expectEmpty(board, 6, 4);  // soldier moved to (5,4)
    expectPiece(board, 6, 6, 'soldier', 'red');
    expectPiece(board, 6, 8, 'soldier', 'red');

    // Red center soldier advanced to (5,4)
    expectPiece(board, 5, 4, 'soldier', 'red');

    // Red left chariot moved to (7,0)
    expectPiece(board, 7, 0, 'chariot', 'red');

    // ── Verify key empty squares that were never occupied ──
    // Row 8 should be empty (red chariot passed through, now at (5,0))
    for (let c = 0; c < 9; c++) {
      expectEmpty(board, 8, c);
    }
    // Row 1: only (1,8) has the black chariot; rest empty
    for (let c = 0; c < 8; c++) {
      expectEmpty(board, 1, c);
    }
    // Row 5: only (5,4) red soldier; rest empty
    for (let c = 0; c < 9; c++) {
      if (c !== 4) {
        expectEmpty(board, 5, c);
      }
    }
    // Row 4: only (4,4) black soldier; rest empty
    for (let c = 0; c < 9; c++) {
      if (c !== 4) {
        expectEmpty(board, 4, c);
      }
    }
  });
});
