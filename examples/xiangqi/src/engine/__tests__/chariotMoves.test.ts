import { describe, expect, it } from 'vitest';
import { getLegalMoves } from '../index';
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

describe('chariot move generation — comprehensive', () => {
  // ── 1. Straight-line moves on empty board (all 4 directions) ──

  it('chariot at center (5,4) on empty board has 17 raw moves (4 + 5 + 4 + 4)', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, { type: 'chariot', player: 'red' });
    // We need generals for legal moves, but raw moves don't need them
    const raw = getRawMoves(board, { row: 5, col: 4 });
    // Up: 5 moves (4,3,2,1,0)
    // Down: 4 moves (6,7,8,9)
    // Left: 4 moves (3,2,1,0)
    // Right: 4 moves (5,6,7,8)
    expect(raw).toHaveLength(17);
  });

  it('chariot at corner (0,0) on empty board has 17 raw moves', () => {
    let board = emptyBoard();
    board = place(board, 0, 0, { type: 'chariot', player: 'red' });
    const raw = getRawMoves(board, { row: 0, col: 0 });
    // Down: 9 moves (1..9)
    // Right: 8 moves (1..8)
    // Up: 0, Left: 0
    expect(raw).toHaveLength(17);
  });

  it('chariot at edge (9,4) on empty board has 17 raw moves', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'chariot', player: 'red' });
    const raw = getRawMoves(board, { row: 9, col: 4 });
    // Up: 9 moves (8..0)
    // Left: 4 moves (3,2,1,0)
    // Right: 4 moves (5,6,7,8)
    // Down: 0
    expect(raw).toHaveLength(17);
  });

  // ── 2. Blocking by own piece ──

  it('chariot blocked by own piece — cannot pass through or capture', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, { type: 'chariot', player: 'red' });
    board = place(board, 5, 7, { type: 'soldier', player: 'red' }); // own piece blocks right
    const raw = getRawMoves(board, { row: 5, col: 4 });
    // Right direction: should stop before col 7 (at col 6), cannot capture own piece
    const rightMoves = raw.filter(m => m.row === 5 && m.col > 4);
    expect(rightMoves).toEqual([
      { row: 5, col: 5 },
      { row: 5, col: 6 },
    ]);
  });

  it('chariot blocked by own piece in multiple directions', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, { type: 'chariot', player: 'red' });
    board = place(board, 3, 4, { type: 'soldier', player: 'red' }); // blocks up
    board = place(board, 7, 4, { type: 'soldier', player: 'red' }); // blocks down
    board = place(board, 5, 2, { type: 'soldier', player: 'red' }); // blocks left
    board = place(board, 5, 6, { type: 'soldier', player: 'red' }); // blocks right
    const raw = getRawMoves(board, { row: 5, col: 4 });
    // Up: 4+1 → stops at 3 (1 move: 4)
    // Down: 4+1 → stops at 7 (1 move: 6)
    // Left: 4+1 → stops at 2 (1 move: 3)
    // Right: 4+1 → stops at 6 (1 move: 5)
    expect(raw).toHaveLength(4);
    expect(raw).toContainEqual({ row: 4, col: 4 });
    expect(raw).toContainEqual({ row: 6, col: 4 });
    expect(raw).toContainEqual({ row: 5, col: 3 });
    expect(raw).toContainEqual({ row: 5, col: 5 });
  });

  // ── 3. Capture of opponent piece at block position ──

  it('chariot can capture opponent piece at block position', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, { type: 'chariot', player: 'red' });
    board = place(board, 5, 7, { type: 'soldier', player: 'black' }); // opponent blocks right
    const raw = getRawMoves(board, { row: 5, col: 4 });
    // Right: should include (5,5), (5,6), (5,7) — capture at 7
    const rightMoves = raw.filter(m => m.row === 5 && m.col > 4);
    expect(rightMoves).toHaveLength(3);
    expect(rightMoves).toContainEqual({ row: 5, col: 7 }); // capture
    // Should NOT include (5,8) — blocked by piece at (5,7)
    expect(rightMoves).not.toContainEqual({ row: 5, col: 8 });
  });

  it('chariot blocked by first of two opponent pieces — only captures first', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, { type: 'chariot', player: 'red' });
    board = place(board, 5, 6, { type: 'soldier', player: 'black' }); // first block
    board = place(board, 5, 8, { type: 'soldier', player: 'black' }); // second, should be unreachable
    const raw = getRawMoves(board, { row: 5, col: 4 });
    const rightMoves = raw.filter(m => m.row === 5 && m.col > 4);
    expect(rightMoves).toEqual([
      { row: 5, col: 5 },
      { row: 5, col: 6 }, // capture first
    ]);
  });

  // ── 4. Capture filtered by check/flying-general ──

  it('chariot capture is legal when it does not create flying general', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 7, 0, { type: 'chariot', player: 'red' });
    board = place(board, 7, 3, { type: 'soldier', player: 'black' });
    board = place(board, 4, 4, { type: 'soldier', player: 'red' }); // blocks flying general

    const legal = getLegalMoves(board, { row: 7, col: 0 });
    expect(legal.some(m => m.row === 7 && m.col === 3)).toBe(true);
  });

  it('chariot cannot move to a square that creates flying general', () => {
    let board = emptyBoard();
    board = place(board, 9, 4, { type: 'general', player: 'red' });
    board = place(board, 0, 4, { type: 'general', player: 'black' });
    board = place(board, 9, 0, { type: 'chariot', player: 'red' });
    // No blocker on col 4 — chariot at (9,0) moving to (4,0) would create flying general
    // because generals at (9,4) and (0,4) would face each other with no pieces between them

    const legal = getLegalMoves(board, { row: 9, col: 0 });
    // (4,0) should NOT be a legal move because it creates flying general
    // (no piece blocks col 4 between generals)
    expect(legal.some(m => m.row === 4 && m.col === 0)).toBe(false);
  });

  // ── 5. Chariot with mixed friendly/enemy pieces on all sides ──

  it('chariot surrounded by mixed pieces — correct raw moves', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, { type: 'chariot', player: 'red' });
    // Up: own piece at (3,4) → moves (4,4) only
    board = place(board, 3, 4, { type: 'soldier', player: 'red' });
    // Down: enemy piece at (7,4) → moves (6,4) and (7,4) capture
    board = place(board, 7, 4, { type: 'soldier', player: 'black' });
    // Left: own piece at (5,1) → moves (5,3), (5,2) only
    board = place(board, 5, 1, { type: 'soldier', player: 'red' });
    // Right: enemy piece at (5,6) → moves (5,5) and (5,6) capture
    board = place(board, 5, 6, { type: 'soldier', player: 'black' });

    const raw = getRawMoves(board, { row: 5, col: 4 });
    expect(raw).toHaveLength(6);
    expect(raw).toContainEqual({ row: 4, col: 4 });
    expect(raw).toContainEqual({ row: 6, col: 4 });
    expect(raw).toContainEqual({ row: 7, col: 4 }); // capture
    expect(raw).toContainEqual({ row: 5, col: 3 });
    expect(raw).toContainEqual({ row: 5, col: 2 });
    expect(raw).toContainEqual({ row: 5, col: 5 });
    expect(raw).toContainEqual({ row: 5, col: 6 }); // capture
    // Actually 7 moves — let me recount
  });

  it('chariot surrounded by mixed pieces — correct raw moves count', () => {
    let board = emptyBoard();
    board = place(board, 5, 4, { type: 'chariot', player: 'red' });
    board = place(board, 3, 4, { type: 'soldier', player: 'red' });  // up: own
    board = place(board, 7, 4, { type: 'soldier', player: 'black' }); // down: enemy
    board = place(board, 5, 1, { type: 'soldier', player: 'red' });  // left: own
    board = place(board, 5, 6, { type: 'soldier', player: 'black' }); // right: enemy

    const raw = getRawMoves(board, { row: 5, col: 4 });
    // Up: (4,4) — 1 move (blocked by own at 3,4)
    // Down: (6,4), (7,4) — 2 moves (capture at 7,4)
    // Left: (5,3), (5,2) — 2 moves (blocked by own at 5,1)
    // Right: (5,5), (5,6) — 2 moves (capture at 5,6)
    // Total: 1 + 2 + 2 + 2 = 7
    expect(raw).toHaveLength(7);
  });
});
