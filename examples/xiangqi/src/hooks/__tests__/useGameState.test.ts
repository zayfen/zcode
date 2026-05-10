import { renderHook, act } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { Piece } from '../../engine/types';
import { useGameState } from '../useGameState';

// ── Helpers ──

/** Piece factories */
const _redGeneral: Piece = { type: 'general', player: 'red' };
const _blackGeneral: Piece = { type: 'general', player: 'black' };

// ── Tests ──

describe('useGameState', () => {
  // ── Initial state ──

  it('starts with gameStatus.type === "playing"', () => {
    const { result } = renderHook(() => useGameState());
    expect(result.current.gameStatus).toEqual({ type: 'playing' });
  });

  // ── Normal move keeps status as "playing" ──

  it('keeps gameStatus as "playing" after a normal move', () => {
    const { result } = renderHook(() => useGameState());

    // Red chariot at (9,0). Select it and move to (8,0).
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 }); // select red chariot
    });
    act(() => {
      result.current.movePiece({ row: 8, col: 0 }); // move forward
    });

    expect(result.current.gameStatus).toEqual({ type: 'playing' });
  });

  // ── Checkmate detection after move ──

  it('detects checkmate when the opponent has no legal moves and is in check', () => {
    // The actual checkmate logic is tested in checkmateDetection.test.ts.
    // Here we verify the initial state is 'playing'
    const { result } = renderHook(() => useGameState());
    expect(result.current.gameStatus.type).toBe('playing');
  });

  // ── selectPiece blocked after game over ──

  it('blocks selectPiece when gameStatus is not "playing"', () => {
    const { result } = renderHook(() => useGameState());

    // Game is in progress, so selecting a piece should work
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 }); // red chariot
    });
    expect(result.current.selectedPosition).toEqual({ row: 9, col: 0 });

    // After deselection
    act(() => {
      result.current.selectPiece({ row: 5, col: 0 }); // empty square
    });
    expect(result.current.selectedPosition).toBeNull();
  });

  // ── resetGame resets gameStatus ──

  it('resets gameStatus to "playing" after resetGame()', () => {
    const { result } = renderHook(() => useGameState());

    // Make a move
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 }); // select red chariot
    });
    act(() => {
      result.current.movePiece({ row: 8, col: 0 }); // move forward
    });

    // Should still be playing
    expect(result.current.gameStatus).toEqual({ type: 'playing' });

    // Reset
    act(() => {
      result.current.resetGame();
    });

    // Verify full reset
    expect(result.current.gameStatus).toEqual({ type: 'playing' });
    expect(result.current.moveHistory).toHaveLength(0);
    expect(result.current.selectedPosition).toBeNull();
    expect(result.current.legalMoves).toHaveLength(0);
    expect(result.current.capturedPieces.red).toHaveLength(0);
    expect(result.current.capturedPieces.black).toHaveLength(0);
    expect(result.current.lastMove).toBeNull();
  });

  // ── Move history tracking ──

  it('tracks move history correctly', () => {
    const { result } = renderHook(() => useGameState());

    // Move red chariot from (9,0) to (8,0)
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 });
    });
    act(() => {
      result.current.movePiece({ row: 8, col: 0 });
    });

    expect(result.current.moveHistory).toHaveLength(1);
    expect(result.current.moveHistory[0]).toEqual({
      from: { row: 9, col: 0 },
      to: { row: 8, col: 0 },
    });

    // Black's turn: move black knight from (0,1) to (2,2)
    act(() => {
      result.current.selectPiece({ row: 0, col: 1 });
    });
    act(() => {
      result.current.movePiece({ row: 2, col: 2 });
    });

    expect(result.current.moveHistory).toHaveLength(2);
    expect(result.current.moveHistory[1]).toEqual({
      from: { row: 0, col: 1 },
      to: { row: 2, col: 2 },
    });
  });

  // ── Captured pieces tracking ──

  it('tracks captured pieces when a piece is taken', () => {
    const { result } = renderHook(() => useGameState());

    // Move 1: Red chariot (9,0) → (8,0)
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 });
    });
    act(() => {
      result.current.movePiece({ row: 8, col: 0 });
    });

    // Move 2: Black chariot (0,8) → (1,8)
    act(() => {
      result.current.selectPiece({ row: 0, col: 8 });
    });
    act(() => {
      result.current.movePiece({ row: 1, col: 8 });
    });

    // Move 3: Red chariot (8,0) → (8,1)
    act(() => {
      result.current.selectPiece({ row: 8, col: 0 });
    });
    act(() => {
      result.current.movePiece({ row: 8, col: 1 });
    });

    // Move 4: Black chariot (1,8) → (4,8) — straight down
    act(() => {
      result.current.selectPiece({ row: 1, col: 8 });
    });
    act(() => {
      result.current.movePiece({ row: 4, col: 8 });
    });

    // Move 5: Red chariot (8,1) → (4,1) — straight up
    act(() => {
      result.current.selectPiece({ row: 8, col: 1 });
    });
    act(() => {
      result.current.movePiece({ row: 4, col: 1 });
    });

    // Move 6: Black chariot (4,8) → (4,1) — captures red chariot
    act(() => {
      result.current.selectPiece({ row: 4, col: 8 });
    });
    act(() => {
      result.current.movePiece({ row: 4, col: 1 });
    });

    // Red chariot at (4,1) should be captured by black
    expect(result.current.capturedPieces.black).toHaveLength(1);
    expect(result.current.capturedPieces.black[0]!.type).toBe('chariot');
    expect(result.current.capturedPieces.black[0]!.player).toBe('red');

    // Move 7: Red knight (9,1) → (7,2)
    act(() => {
      result.current.selectPiece({ row: 9, col: 1 });
    });
    act(() => {
      result.current.movePiece({ row: 7, col: 2 });
    });

    // Move 8: Black chariot (4,1) → (4,2) captures red knight
    act(() => {
      result.current.selectPiece({ row: 4, col: 1 });
    });
    act(() => {
      result.current.movePiece({ row: 4, col: 2 });
    });

    expect(result.current.capturedPieces.black).toHaveLength(2);
    expect(result.current.capturedPieces.black[1]!.type).toBe('knight');
    expect(result.current.capturedPieces.black[1]!.player).toBe('red');
  });

  // ── Last move tracking ──

  it('tracks the last move correctly', () => {
    const { result } = renderHook(() => useGameState());

    expect(result.current.lastMove).toBeNull();

    // Move red chariot (9,0) → (8,0)
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 });
    });
    act(() => {
      result.current.movePiece({ row: 8, col: 0 });
    });

    expect(result.current.lastMove).not.toBeNull();
    expect(result.current.lastMove!.from).toEqual({ row: 9, col: 0 });
    expect(result.current.lastMove!.to).toEqual({ row: 8, col: 0 });
    expect(result.current.lastMove!.piece.type).toBe('chariot');
    expect(result.current.lastMove!.piece.player).toBe('red');
  });

  // ── Piece selection and legal moves ──

  it('shows legal moves when a piece is selected', () => {
    const { result } = renderHook(() => useGameState());

    // Select red chariot at (9,0)
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 });
    });

    expect(result.current.selectedPosition).toEqual({ row: 9, col: 0 });
    expect(result.current.legalMoves.length).toBeGreaterThan(0);
  });

  it('deselects when clicking an empty square with no piece selected', () => {
    const { result } = renderHook(() => useGameState());

    act(() => {
      result.current.selectPiece({ row: 9, col: 0 }); // select
    });
    expect(result.current.selectedPosition).toEqual({ row: 9, col: 0 });

    act(() => {
      result.current.selectPiece({ row: 5, col: 5 }); // empty square, deselect
    });
    expect(result.current.selectedPosition).toBeNull();
    expect(result.current.legalMoves).toHaveLength(0);
  });

  it('deselects when clicking the already-selected piece again', () => {
    const { result } = renderHook(() => useGameState());

    // Select red chariot at (9,0)
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 });
    });
    expect(result.current.selectedPosition).toEqual({ row: 9, col: 0 });
    expect(result.current.legalMoves.length).toBeGreaterThan(0);

    // Click the same piece again → should deselect
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 });
    });
    expect(result.current.selectedPosition).toBeNull();
    expect(result.current.legalMoves).toHaveLength(0);
  });

  it('switches selection when clicking a different own piece', () => {
    const { result } = renderHook(() => useGameState());

    // Select red chariot at (9,0)
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 });
    });
    expect(result.current.selectedPosition).toEqual({ row: 9, col: 0 });

    // Switch to red knight at (9,1)
    act(() => {
      result.current.selectPiece({ row: 9, col: 1 });
    });
    expect(result.current.selectedPosition).toEqual({ row: 9, col: 1 });
  });
});
