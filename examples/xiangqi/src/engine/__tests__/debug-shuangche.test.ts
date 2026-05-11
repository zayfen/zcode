import { describe, it, expect } from 'vitest';
import type { Board, Piece } from '../types';
import { isCheckmate, isStalemate, getLegalMoves, isInCheck, hasAnyLegalMoves } from '../index';

function emptyBoard(): Board {
  return Array.from({ length: 10 }, () =>
    Array.from({ length: 9 }, () => null),
  );
}

function place(board: Board, row: number, col: number, piece: Piece): Board {
  const b = board.map((r) => r.map((c) => c));
  b[row]![col] = piece;
  return b;
}

const redGeneral: Piece = { type: 'general', player: 'red' };
const blackGeneral: Piece = { type: 'general', player: 'black' };
const redChariot: Piece = { type: 'chariot', player: 'red' };
const redKnight: Piece = { type: 'knight', player: 'red' };
const redAdvisor: Piece = { type: 'advisor', player: 'red' };

describe('双车错 debug', () => {
  it('debug position', () => {
    let board = emptyBoard();
    board = place(board, 0, 4, blackGeneral);
    board = place(board, 0, 3, redChariot);
    board = place(board, 1, 4, redChariot);
    board = place(board, 9, 3, redGeneral);

    // Debug checks
    const inCheck = isInCheck(board, 'black');
    const legalMoves = getLegalMoves(board, { row: 0, col: 4 });
    console.log('isInCheck:', inCheck);
    console.log('legalMoves:', JSON.stringify(legalMoves));
    console.log('hasAnyLegalMoves:', legalMoves.length > 0);
    
    expect(inCheck).toBe(true);
  });
});
