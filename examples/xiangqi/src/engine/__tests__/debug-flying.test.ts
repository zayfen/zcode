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

describe('debug flying general', () => {
  it('chariot at (9,0) on board with generals at (9,4)/(0,4) — IS flying general (no blockers on col 4)', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 9, 0, { type: 'chariot', player: 'red' });

    // The chariot is NOT on col 4, so nothing blocks the generals — flying general IS true
    const flying = isFlyingGeneral(board);
    console.log('Flying general with chariot at (9,0):', flying);
    expect(flying).toBe(true);
  });

  it('board with only generals at (9,4) and (0,4) IS flying general', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    const flying = isFlyingGeneral(board);
    console.log('Flying general with only generals:', flying);
    expect(flying).toBe(true);
  });

  it('board with generals at (9,4)/(0,4) and piece at (4,4) is NOT flying general', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 4, 4, { type: 'soldier', player: 'red' });

    const flying = isFlyingGeneral(board);
    console.log('Flying general with blocker at (4,4):', flying);
    expect(flying).toBe(false);
  });

  it('chariot move from (9,0) to (4,0) on board with blocker at (4,4) should be legal', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 9, 0, { type: 'chariot', player: 'red' });
    board = place(board, 4, 4, { type: 'soldier', player: 'red' }); // blocker

    const legalMoves = getLegalMoves(board, { row: 9, col: 0 });
    console.log('Legal moves with blocker:', legalMoves.map(m => `${m.row},${m.col}`));
    const hasTarget = legalMoves.some(m => m.row === 4 && m.col === 0);
    expect(hasTarget).toBe(true);
  });
});
