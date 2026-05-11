import { renderHook, act } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useGameState } from '../useGameState';

describe('debug captured pieces', () => {
  it('debug capture path', () => {
    const { result } = renderHook(() => useGameState());

    // Move 1: Red chariot (9,0) → (8,0)
    act(() => {
      result.current.selectPiece({ row: 9, col: 0 });
    });
    act(() => {
      result.current.movePiece({ row: 8, col: 0 });
    });

    console.log('After move 1: player=', result.current.boardState.currentPlayer);
    console.log('Board[8][0]:', JSON.stringify(result.current.boardState.board[8]?.[0]));
    console.log('Board[9][0]:', JSON.stringify(result.current.boardState.board[9]?.[0]));

    // Move 2: Black chariot (0,8) → (1,8)
    act(() => {
      result.current.selectPiece({ row: 0, col: 8 });
    });
    act(() => {
      result.current.movePiece({ row: 1, col: 8 });
    });

    console.log('After move 2: player=', result.current.boardState.currentPlayer);
    console.log('Board[1][8]:', JSON.stringify(result.current.boardState.board[1]?.[8]));

    // Move 3: Red chariot (8,0) → (8,1)
    act(() => {
      result.current.selectPiece({ row: 8, col: 0 });
    });
    act(() => {
      result.current.movePiece({ row: 8, col: 1 });
    });

    console.log('After move 3: player=', result.current.boardState.currentPlayer);
    console.log('Board[8][1]:', JSON.stringify(result.current.boardState.board[8]?.[1]));
    console.log('Board[8][0]:', JSON.stringify(result.current.boardState.board[8]?.[0]));

    // Move 4: Black chariot (1,8) → (4,8) — straight down
    act(() => {
      result.current.selectPiece({ row: 1, col: 8 });
    });
    act(() => {
      result.current.movePiece({ row: 4, col: 8 });
    });

    console.log('After move 4: player=', result.current.boardState.currentPlayer);
    console.log('Board[4][8]:', JSON.stringify(result.current.boardState.board[4]?.[8]));

    // Move 5: Red chariot (8,1) → (4,1) — straight up
    act(() => {
      result.current.selectPiece({ row: 8, col: 1 });
    });
    act(() => {
      result.current.movePiece({ row: 4, col: 1 });
    });

    console.log('After move 5: player=', result.current.boardState.currentPlayer);
    console.log('Board[4][1]:', JSON.stringify(result.current.boardState.board[4]?.[1]));
    console.log('Board[8][1]:', JSON.stringify(result.current.boardState.board[8]?.[1]));

    // Move 6: Black chariot (4,8) → (4,1) — captures red chariot
    act(() => {
      result.current.selectPiece({ row: 4, col: 8 });
    });
    console.log('Legal moves for black chariot at (4,8):',
      result.current.legalMoves.map(m => `${m.row},${m.col}`));

    act(() => {
      result.current.movePiece({ row: 4, col: 1 });
    });

    console.log('After move 6 (capture):');
    console.log('Board[4][1]:', JSON.stringify(result.current.boardState.board[4]?.[1]));
    console.log('Board[4][8]:', JSON.stringify(result.current.boardState.board[4]?.[8]));
    console.log('Captured pieces:', JSON.stringify(result.current.capturedPieces));
    console.log('Current player:', result.current.boardState.currentPlayer);

    expect(result.current.capturedPieces.black).toHaveLength(1);
  });
});
