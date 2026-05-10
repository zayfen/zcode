import { describe, expect, it } from 'vitest';
import { getLegalMoves, isInCheck, isFlyingGeneral } from '../index';
import { applyMove } from '../specialRules';
import { getRawMoves } from '../moveValidation';
import type { Board, Piece } from '../types';

function emptyBoard(): Board {
  return Array.from({ length: 10 }, () =>
    Array.from({ length: 9 }, () => null),
  );
}

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

describe('debug chariot move from (9,0) to (4,0)', () => {
  it('raw moves from (9,0) on custom board include (4,0)', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 9, 0, { type: 'chariot', player: 'red' });

    const rawMoves = getRawMoves(board, { row: 9, col: 0 });
    console.log('Raw moves from (9,0):', rawMoves.map(m => `${m.row},${m.col}`));
    const hasTarget = rawMoves.some(m => m.row === 4 && m.col === 0);
    expect(hasTarget).toBe(true);
  });

  it('legal moves from (9,0) on custom board include (4,0)', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 9, 0, { type: 'chariot', player: 'red' });
    // Blocker on col 4 to prevent flying general violation
    board = place(board, 4, 4, { type: 'soldier', player: 'red' });

    const legalMoves = getLegalMoves(board, { row: 9, col: 0 });
    console.log('Legal moves from (9,0):', legalMoves.map(m => `${m.row},${m.col}`));
    const hasTarget = legalMoves.some(m => m.row === 4 && m.col === 0);
    expect(hasTarget).toBe(true);
  });

  it('after moving chariot to (4,0), board IS in flying general (chariot not blocking col 4)', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 9, 0, { type: 'chariot', player: 'red' });

    const newBoard = applyMove(board, { from: { row: 9, col: 0 }, to: { row: 4, col: 0 } });
    console.log('Red general still at (9,4):', newBoard[9][4]);
    console.log('Black general still at (0,4):', newBoard[0][4]);
    console.log('Chariot now at (4,0):', newBoard[4][0]);
    console.log('(9,0) should be null:', newBoard[9][0]);

    const check = isInCheck(newBoard, 'red');
    console.log('Red in check after move:', check);

    const flying = isFlyingGeneral(newBoard);
    console.log('Flying general after move:', flying);

    // Chariot moved to (4,0) which is NOT on col 4, so generals face each other
    expect(flying).toBe(true);
    // Since flying general is a violation, red would be in check (from the black general)
    expect(check).toBe(true);
  });

  it('raw moves from (7,0) on custom board with capture target at (7,3)', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 7, 0, { type: 'chariot', player: 'red' });
    board = place(board, 7, 3, { type: 'soldier', player: 'black' });

    const rawMoves = getRawMoves(board, { row: 7, col: 0 });
    console.log('Raw moves from (7,0):', rawMoves.map(m => `${m.row},${m.col}`));
    const hasTarget = rawMoves.some(m => m.row === 7 && m.col === 3);
    expect(hasTarget).toBe(true);
  });

  it('legal moves from (7,0) on custom board with capture target at (7,3) — has blocker on col 4', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 7, 0, { type: 'chariot', player: 'red' });
    board = place(board, 7, 3, { type: 'soldier', player: 'black' });
    // Need a blocker on col 4 between generals, otherwise flying general violation
    board = place(board, 4, 4, { type: 'soldier', player: 'red' });

    const legalMoves = getLegalMoves(board, { row: 7, col: 0 });
    console.log('Legal moves from (7,0):', legalMoves.map(m => `${m.row},${m.col}`));
    const hasTarget = legalMoves.some(m => m.row === 7 && m.col === 3);
    expect(hasTarget).toBe(true);
  });
});
