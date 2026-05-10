import { describe, expect, it } from 'vitest';
import { isSquareAttackedBy, isInCheck, findGeneralPosition } from '../index';
import { getRawMoves } from '../moveValidation';
import type { Board, Piece } from '../types';

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

describe('isSquareAttackedBy', () => {
  // ── Chariot attacks ──
  it('detects chariot attacking along a row', () => {
    let board = emptyBoard();
    board = place(board, 5, 0, { type: 'chariot', player: 'black' });
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Black chariot at (5,0) attacks (5,5) along row 5
    expect(isSquareAttackedBy(board, { row: 5, col: 5 }, 'black')).toBe(true);
    // Red does not attack that square
    expect(isSquareAttackedBy(board, { row: 5, col: 5 }, 'red')).toBe(false);
  });

  it('detects chariot attacking along a column', () => {
    let board = emptyBoard();
    board = place(board, 0, 3, { type: 'chariot', player: 'red' });
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    expect(isSquareAttackedBy(board, { row: 5, col: 3 }, 'red')).toBe(true);
  });

  it('chariot does NOT attack through a blocking piece', () => {
    let board = emptyBoard();
    board = place(board, 0, 0, { type: 'chariot', player: 'red' });
    board = place(board, 3, 0, { type: 'soldier', player: 'black' }); // blocker
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Red chariot at (0,0) cannot attack (5,0) — blocked by black soldier at (3,0)
    expect(isSquareAttackedBy(board, { row: 5, col: 0 }, 'red')).toBe(false);
    // But can attack the blocker itself
    expect(isSquareAttackedBy(board, { row: 3, col: 0 }, 'red')).toBe(true);
  });

  // ── Cannon attacks ──
  it('detects cannon capturing over a screen', () => {
    let board = emptyBoard();
    board = place(board, 2, 1, { type: 'cannon', player: 'black' });
    board = place(board, 4, 1, { type: 'soldier', player: 'red' }); // screen
    board = place(board, 6, 1, { type: 'chariot', player: 'red' }); // capture target behind screen
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Black cannon at (2,1) with screen at (4,1) attacks (6,1) — captures red chariot
    expect(isSquareAttackedBy(board, { row: 6, col: 1 }, 'black')).toBe(true);
    // (7,1) is behind the capture target — cannon cannot attack past the chariot
    expect(isSquareAttackedBy(board, { row: 7, col: 1 }, 'black')).toBe(false);
  });

  // ── Knight attacks ──
  it('detects knight attacking in L-shape', () => {
    let board = emptyBoard();
    board = place(board, 5, 5, { type: 'knight', player: 'red' });
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Knight at (5,5) attacks (4,3), (4,7), (3,4), (3,6), (6,3), (6,7), (7,4), (7,6)
    expect(isSquareAttackedBy(board, { row: 4, col: 3 }, 'red')).toBe(true);
    expect(isSquareAttackedBy(board, { row: 3, col: 6 }, 'red')).toBe(true);
    expect(isSquareAttackedBy(board, { row: 7, col: 4 }, 'red')).toBe(true);
  });

  it('knight does NOT attack when leg is blocked (蹩马腿)', () => {
    let board = emptyBoard();
    board = place(board, 5, 5, { type: 'knight', player: 'red' });
    // Block the leg for the (3,4) move — leg is at (4,5)
    board = place(board, 4, 5, { type: 'soldier', player: 'black' });
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Knight at (5,5) cannot reach (3,4) — leg at (4,5) blocked
    expect(isSquareAttackedBy(board, { row: 3, col: 4 }, 'red')).toBe(false);
    // But still can reach (3,6) — leg at (4,6) not blocked
    expect(isSquareAttackedBy(board, { row: 4, col: 7 }, 'red')).toBe(true);
  });

  // ── General attacks ──
  it('detects general attacking adjacent square in palace', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Red general at (9,4) attacks (8,4) and (9,3) and (9,5)
    expect(isSquareAttackedBy(board, { row: 8, col: 4 }, 'red')).toBe(true);
    expect(isSquareAttackedBy(board, { row: 9, col: 3 }, 'red')).toBe(true);
    expect(isSquareAttackedBy(board, { row: 9, col: 5 }, 'red')).toBe(true);
    // Does not attack two squares away
    expect(isSquareAttackedBy(board, { row: 7, col: 4 }, 'red')).toBe(false);
  });

  // ── Advisor attacks ──
  it('detects advisor attacking diagonally in palace', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 9, 3, { type: 'advisor', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Red advisor at (9,3) attacks (8,4)
    expect(isSquareAttackedBy(board, { row: 8, col: 4 }, 'red')).toBe(true);
  });

  // ── Elephant attacks ──
  it('detects elephant attacking diagonally (田) with clear eye', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 7, 2, { type: 'elephant', player: 'red' });

    // Red elephant at (7,2):
    //   Diagonal-2 destinations: (5,0), (5,4), (9,0), (9,4)
    //   River check: all row >= 5 for red → all valid
    //   Eye at (6,1), (6,3), (8,1), (8,3) — all clear on empty board
    //   (9,4) has red general → own piece, not a valid move target
    //   Valid targets: (5,0), (5,4), (9,0)
    expect(isSquareAttackedBy(board, { row: 5, col: 0 }, 'red')).toBe(true);
    expect(isSquareAttackedBy(board, { row: 5, col: 4 }, 'red')).toBe(true);
    expect(isSquareAttackedBy(board, { row: 9, col: 0 }, 'red')).toBe(true);
    // (9,4) is blocked by own general
    expect(isSquareAttackedBy(board, { row: 9, col: 4 }, 'red')).toBe(false);
  });

  // ── Soldier attacks ──
  it('detects soldier attacking forward', () => {
    let board = emptyBoard();
    board = place(board, 6, 4, { type: 'soldier', player: 'red' });
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Red soldier at (6,4) attacks (5,4) forward
    expect(isSquareAttackedBy(board, { row: 5, col: 4 }, 'red')).toBe(true);
    // Does NOT attack sideways — hasn't crossed river
    expect(isSquareAttackedBy(board, { row: 6, col: 3 }, 'red')).toBe(false);
  });

  it('detects soldier attacking sideways after crossing river', () => {
    let board = emptyBoard();
    board = place(board, 4, 4, { type: 'soldier', player: 'red' }); // crossed river
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });

    // Red soldier at (4,4) — crossed river — attacks (3,4), (4,3), (4,5)
    expect(isSquareAttackedBy(board, { row: 3, col: 4 }, 'red')).toBe(true);
    expect(isSquareAttackedBy(board, { row: 4, col: 3 }, 'red')).toBe(true);
    expect(isSquareAttackedBy(board, { row: 4, col: 5 }, 'red')).toBe(true);
  });

  // ── No attacker found ──
  it('returns false when no attacker piece can reach the target', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 0, 0, { type: 'chariot', player: 'red' });

    // Red chariot at (0,0) cannot reach (5,5)
    expect(isSquareAttackedBy(board, { row: 5, col: 5 }, 'red')).toBe(false);
    // No black pieces attack (5,5)
    expect(isSquareAttackedBy(board, { row: 5, col: 5 }, 'black')).toBe(false);
  });

  // ── Integration with check detection ──
  it('correctly identifies that a square adjacent to general is attacked by enemy chariot', () => {
    let board = emptyBoard();
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 1, 4, { type: 'chariot', player: 'red' }); // attacks (0,4)

    expect(isSquareAttackedBy(board, { row: 0, col: 4 }, 'red')).toBe(true);
    expect(isInCheck(board, 'black')).toBe(true);
  });
});
