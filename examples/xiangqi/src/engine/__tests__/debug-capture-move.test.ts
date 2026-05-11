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

function p(type: Piece['type'], player: Piece['player']): Piece {
  return { type, player };
}

function setupBoardWithBlocker(): Board {
  // A board where red chariot (8,0) and black chariot (8,8) are on the same
  // row. A red soldier on (8,4) blocks the flying-general path between
  // the generals at (9,4) and (0,4).
  let board = emptyBoard();
  board = place(board, 9, 4, { type: 'general', player: 'red' });
  board = place(board, 0, 4, { type: 'general', player: 'black' });
  board = place(board, 8, 0, { type: 'chariot', player: 'red' });
  board = place(board, 8, 8, { type: 'chariot', player: 'black' });
  // Blocker on col 4 between generals to prevent flying general
  board = place(board, 8, 4, { type: 'soldier', player: 'red' });
  return board;
}

describe('debug chariot capture', () => {
  it('black chariot on (8,8) can capture red chariot on (8,0)', () => {
    const board = setupBoardWithBlocker();

    // Black chariot at (8,8): legal moves should include (8,0) capture
    const legalMoves = getLegalMoves(board, { row: 8, col: 8 });
    console.log(
      'Legal moves for black chariot at (8,8):',
      legalMoves.map((m) => `${m.row},${m.col}`),
    );

    const hasCapture = legalMoves.some(
      (m) => m.row === 8 && m.col === 0,
    );
    expect(hasCapture).toBe(true);
  });

  it('after capturing, captured piece is removed from board', () => {
    const board = setupBoardWithBlocker();

    const newBoard = applyMove(board, {
      from: { row: 8, col: 8 },
      to: { row: 8, col: 0 },
    });

    // Black chariot should now be at (8,0)
    expect(newBoard[8]?.[0]?.type).toBe('chariot');
    expect(newBoard[8]?.[0]?.player).toBe('black');

    // (8,8) should be empty
    expect(newBoard[8]?.[8]).toBeNull();
  });

  it('replicate full move sequence: red (9,0)→(8,0), black (0,8)→(1,8), red (8,0)→(8,1), black (1,8)→(4,8), red (8,1)→(4,1) [all valid]', () => {
    // Build a custom board mimicking the game state after 4 moves.
    // After 4 moves the initial board has soldiers on cols 0,2,4,6,8 at rows 3 and 6.
    //
    // Remaining pieces relevant to the sequence:
    //   Red: general (9,4), chariot now at (8,1), soldiers still at (6,0),(6,2),(6,4),(6,6),(6,8), etc.
    //   Black: general (0,4), chariot now at (4,8), soldiers at (3,0),(3,2),(3,4),(3,6),(3,8), etc.
    //
    // The black chariot at (4,8) wants to capture the red chariot at (4,1) after move 5.
    // BUT col 4 has soldiers at (3,4) [black] and (6,4) [red], which block flying general!
    // So move 5 (red chariot (8,1)→(4,1)) IS legal.
    // Then move 6 (black chariot (4,8)→(4,1)) captures the red chariot — also legal.

    let board = emptyBoard();

    // Red pieces (remaining after 4 moves)
    board = place(board, 9, 4, p('general', 'red'));
    board = place(board, 9, 3, p('advisor', 'red'));
    board = place(board, 9, 5, p('advisor', 'red'));
    board = place(board, 9, 2, p('elephant', 'red'));
    board = place(board, 9, 6, p('elephant', 'red'));
    board = place(board, 9, 1, p('knight', 'red'));
    board = place(board, 9, 7, p('knight', 'red'));
    board = place(board, 9, 8, p('chariot', 'red'));
    board = place(board, 7, 1, p('cannon', 'red'));
    board = place(board, 7, 7, p('cannon', 'red'));
    board = place(board, 6, 0, p('soldier', 'red'));
    board = place(board, 6, 2, p('soldier', 'red'));
    board = place(board, 6, 4, p('soldier', 'red'));
    board = place(board, 6, 6, p('soldier', 'red'));
    board = place(board, 6, 8, p('soldier', 'red'));

    // Black pieces (remaining after 4 moves)
    board = place(board, 0, 4, p('general', 'black'));
    board = place(board, 0, 3, p('advisor', 'black'));
    board = place(board, 0, 5, p('advisor', 'black'));
    board = place(board, 0, 2, p('elephant', 'black'));
    board = place(board, 0, 6, p('elephant', 'black'));
    board = place(board, 0, 1, p('knight', 'black'));
    board = place(board, 0, 7, p('knight', 'black'));
    board = place(board, 0, 0, p('chariot', 'black'));
    board = place(board, 2, 1, p('cannon', 'black'));
    board = place(board, 2, 7, p('cannon', 'black'));
    board = place(board, 3, 0, p('soldier', 'black'));
    board = place(board, 3, 2, p('soldier', 'black'));
    board = place(board, 3, 4, p('soldier', 'black'));
    board = place(board, 3, 6, p('soldier', 'black'));
    board = place(board, 3, 8, p('soldier', 'black'));

    // Place moved chariots
    board = place(board, 8, 1, p('chariot', 'red'));   // after moves 1 and 3
    board = place(board, 4, 8, p('chariot', 'black'));  // after move 4

    // Move 5: Red chariot (8,1) → (4,1) — should be legal
    // Col 4 is blocked by soldiers at (3,4) and (6,4) — NO flying general
    const redLegal = getLegalMoves(board, { row: 8, col: 1 });
    const canRedMoveTo41 = redLegal.some(m => m.row === 4 && m.col === 1);
    console.log('Red chariot legal moves from (8,1):', redLegal.map(m => `${m.row},${m.col}`));
    console.log('Flying general on this board?', isFlyingGeneral(board));
    expect(canRedMoveTo41).toBe(true);

    // Apply move 5
    const boardAfterMove5 = applyMove(board, { from: { row: 8, col: 1 }, to: { row: 4, col: 1 } });
    expect(boardAfterMove5[4]?.[1]?.type).toBe('chariot');
    expect(boardAfterMove5[4]?.[1]?.player).toBe('red');

    // Move 6: Black chariot (4,8) → (4,1) captures red chariot — should be legal
    const blackLegal = getLegalMoves(boardAfterMove5, { row: 4, col: 8 });
    console.log('Black chariot legal moves from (4,8):', blackLegal.map(m => `${m.row},${m.col}`));
    const canBlackCapture = blackLegal.some(m => m.row === 4 && m.col === 1);
    expect(canBlackCapture).toBe(true);
  });
});
