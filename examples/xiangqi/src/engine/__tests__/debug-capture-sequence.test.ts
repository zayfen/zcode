import { describe, expect, it } from 'vitest';
import type { Board, Piece } from '../../engine/types';
import { createInitialBoardState, getLegalMoves, makeMove } from '../../engine';

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

describe('debug chariot capture sequence', () => {
  it('verifies chariot straight-line capture on a custom board', () => {
    // Build a custom board: red chariot at (5,8), black chariot at (5,0)
    // Generals on col 4 with a blocker between them
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 4, 4, { type: 'soldier', player: 'red' }); // blocker on col 4
    board = place(board, 5, 8, { type: 'chariot', player: 'black' });
    board = place(board, 5, 0, { type: 'chariot', player: 'red' });

    // Black chariot at (5,8) should be able to capture red chariot at (5,0)
    const legalMoves = getLegalMoves(board, { row: 5, col: 8 });
    console.log('Legal moves for black chariot at (5,8):', legalMoves.map(p => `${p.row},${p.col}`));

    const hasCapture = legalMoves.some(p => p.row === 5 && p.col === 0);
    expect(hasCapture).toBe(true);

    const moveResult = makeMove(
      { board, currentPlayer: 'black' },
      { from: { row: 5, col: 8 }, to: { row: 5, col: 0 } },
    );
    expect(moveResult.valid).toBe(true);
    expect(moveResult.captured).toBeDefined();
    expect(moveResult.captured!.type).toBe('chariot');
    expect(moveResult.captured!.player).toBe('red');
  });
});
