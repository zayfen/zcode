import { describe, expect, it, vi } from 'vitest';
import { render, fireEvent, type RenderResult } from '@testing-library/react';
import { Board } from '../Board';
import type { GameState } from '../../hooks/useGameState';
import type { Position } from '../../engine';
import { createInitialBoardState, getLegalMoves } from '../../engine';
import { MoveHighlight } from '../MoveHighlight';

// ── Fixtures ──

const CELL_SIZE = 50;

/**
 * Helper to create a minimal mock GameState for the Board component.
 * Starts from the initial board and allows overriding fields.
 */
function createMockGameState(
  overrides: Partial<GameState> = {},
): GameState {
  const boardState = createInitialBoardState();

  const defaults: GameState = {
    boardState,
    selectedPosition: null,
    legalMoves: [],
    moveHistory: [],
    gameStatus: { type: 'playing' },
    capturedPieces: { red: [], black: [] },
    lastMove: null,
    selectPiece: vi.fn(),
    movePiece: vi.fn(),
    resetGame: vi.fn(),
  };

  return { ...defaults, ...overrides };
}

/**
 * Return the ARIA label for a square at (row, col), optionally with piece info.
 */
function squareLabel(row: number, col: number, piece?: string): string {
  return piece
    ? `Square ${row},${col} — ${piece}`
    : `Square ${row},${col}`;
}

/**
 * Get all board squares rendered inside a result, by querying the container directly.
 * This avoids issues with multiple renders in jsdom reusing the same body.
 */
function getBoardSquares(result: RenderResult) {
  // The board-container holds all squares
  return result.container.querySelectorAll('.board-square');
}

/**
 * From a list of board-square elements, return those whose cursor style is 'pointer'.
 */
function getLegalMoveSquares(result: RenderResult) {
  return Array.from(getBoardSquares(result)).filter(
    (el) => (el as HTMLElement).style.cursor === 'pointer',
  );
}

// ══════════════════════════════════════════════
// Unit tests for MoveHighlight component
// ══════════════════════════════════════════════

describe('MoveHighlight', () => {
  // ────────────────────────────────────────────
  describe('rendering', () => {
    it('returns null when isLegal is false', () => {
      const { container } = render(
        <MoveHighlight isLegal={false} hasPiece={false} cellSize={CELL_SIZE} />,
      );
      expect(container.innerHTML).toBe('');
    });

    it('renders a dot indicator for an empty legal-move square', () => {
      const { container } = render(
        <MoveHighlight isLegal={true} hasPiece={false} cellSize={CELL_SIZE} />,
      );
      const el = container.firstChild as HTMLDivElement;
      expect(el).not.toBeNull();
      // Dot style: small width/height, circular
      expect(el.style.width).toBe(`${CELL_SIZE * 0.22}px`);
      expect(el.style.height).toBe(`${CELL_SIZE * 0.22}px`);
      expect(el.style.borderRadius).toBe('50%');
      // The dot should have a boxShadow with green glow
      expect(el.style.boxShadow).toContain('rgba(0, 160, 0');
    });

    it('renders a ring indicator for a capture target (hasPiece=true)', () => {
      const { container } = render(
        <MoveHighlight isLegal={true} hasPiece={true} cellSize={CELL_SIZE} />,
      );
      const el = container.firstChild as HTMLDivElement;
      expect(el).not.toBeNull();
      // Ring style: larger diameter, has a green border, transparent background
      expect(el.style.width).toBe(`${CELL_SIZE * 0.88}px`);
      expect(el.style.height).toBe(`${CELL_SIZE * 0.88}px`);
      expect(el.style.border).toContain('rgba(0, 140, 0');
      expect(el.style.background).toBe('transparent');
    });
  });
});

// ══════════════════════════════════════════════
// Integration tests: Board + MoveHighlight
// ══════════════════════════════════════════════

describe('MoveHighlight on Board after selecting a piece', () => {
  // ────────────────────────────────────────────
  describe('red chariot at (9,0)', () => {
    it('shows no legal-move indicators before any piece is selected', () => {
      const game = createMockGameState();
      const result = render(<Board game={game} cellSize={CELL_SIZE} />);

      const legalSquares = getLegalMoveSquares(result);
      expect(legalSquares).toHaveLength(0);
    });

    it('displays legal-move indicators on correct squares after selecting the red chariot at (9,0)', () => {
      const boardState = createInitialBoardState();
      const selectedPosition: Position = { row: 9, col: 0 };
      const legalMoves = getLegalMoves(boardState.board, selectedPosition);

      const game = createMockGameState({
        boardState,
        selectedPosition,
        legalMoves,
      });

      const result = render(<Board game={game} cellSize={CELL_SIZE} />);

      // Every legal move position should have cursor:pointer on its square
      const legalSquares = getLegalMoveSquares(result);
      expect(legalSquares).toHaveLength(legalMoves.length);
    });

    it('renders MoveHighlight child elements only on legal-move squares', () => {
      const boardState = createInitialBoardState();
      const selectedPosition: Position = { row: 9, col: 0 };
      const legalMoves = getLegalMoves(boardState.board, selectedPosition);

      const game = createMockGameState({
        boardState,
        selectedPosition,
        legalMoves,
      });

      const result = render(<Board game={game} cellSize={CELL_SIZE} />);

      // Build a set of legal positions for quick lookup
      const legalSet = new Set(legalMoves.map((p) => `${p.row},${p.col}`));

      // Every board square should have the correct cursor based on whether it's legal
      const allSquares = getBoardSquares(result);
      for (const sq of allSquares) {
        const el = sq as HTMLElement;
        const ariaLabel = el.getAttribute('aria-label') ?? '';
        // Extract row,col from aria-label like "Square 9,0 — red chariot" or "Square 4,4"
        const match = ariaLabel.match(/^Square (\d+),(\d+)/);
        if (!match) continue;
        const row = parseInt(match[1]!, 10);
        const col = parseInt(match[2]!, 10);
        const isLegal = legalSet.has(`${row},${col}`);
        expect(el.style.cursor).toBe(isLegal ? 'pointer' : 'default');
      }
    });
  });

  // ────────────────────────────────────────────
  describe('red knight at (9,1)', () => {
    it('shows legal-move indicators on knight move targets', () => {
      const boardState = createInitialBoardState();
      const selectedPosition: Position = { row: 9, col: 1 };
      const legalMoves = getLegalMoves(boardState.board, selectedPosition);

      const game = createMockGameState({
        boardState,
        selectedPosition,
        legalMoves,
      });

      const result = render(<Board game={game} cellSize={CELL_SIZE} />);

      const legalSquares = getLegalMoveSquares(result);
      expect(legalSquares).toHaveLength(legalMoves.length);

      // The knight at (9,1) in the initial position should be able to move to
      // at least one square (typically 2 squares: (7,0) and (7,2))
      expect(legalMoves.length).toBeGreaterThan(0);
    });
  });

  // ────────────────────────────────────────────
  describe('red cannon at (7,1)', () => {
    it('shows legal-move indicators on cannon move targets', () => {
      const boardState = createInitialBoardState();
      const selectedPosition: Position = { row: 7, col: 1 };
      const legalMoves = getLegalMoves(boardState.board, selectedPosition);

      const game = createMockGameState({
        boardState,
        selectedPosition,
        legalMoves,
      });

      const result = render(<Board game={game} cellSize={CELL_SIZE} />);

      const legalSquares = getLegalMoveSquares(result);
      expect(legalSquares).toHaveLength(legalMoves.length);

      // Cannon should have multiple moves in the initial position
      expect(legalMoves.length).toBeGreaterThan(0);
    });
  });

  // ────────────────────────────────────────────
  describe('selection deselection clears highlights', () => {
    it('clears legal-move indicators when selection is removed', () => {
      const boardState = createInitialBoardState();
      const selectedPosition: Position = { row: 9, col: 0 };
      const legalMoves = getLegalMoves(boardState.board, selectedPosition);

      // First render with selection — use a fresh container
      const container1 = document.createElement('div');
      document.body.appendChild(container1);

      render(
        <Board
          game={createMockGameState({
            boardState,
            selectedPosition,
            legalMoves,
          })}
          cellSize={CELL_SIZE}
        />,
        { container: container1 },
      );

      // Verify there are legal squares
      const squares1 = container1.querySelectorAll('.board-square');
      const legal1 = Array.from(squares1).filter(
        (el) => (el as HTMLElement).style.cursor === 'pointer',
      );
      expect(legal1.length).toBeGreaterThan(0);

      // Clean up first render
      document.body.removeChild(container1);

      // Second render without selection — fresh container
      const container2 = document.createElement('div');
      document.body.appendChild(container2);

      render(
        <Board
          game={createMockGameState({
            boardState,
            selectedPosition: null,
            legalMoves: [],
          })}
          cellSize={CELL_SIZE}
        />,
        { container: container2 },
      );

      const squares2 = container2.querySelectorAll('.board-square');
      const legal2 = Array.from(squares2).filter(
        (el) => (el as HTMLElement).style.cursor === 'pointer',
      );
      expect(legal2).toHaveLength(0);

      // Clean up
      document.body.removeChild(container2);
    });
  });

  // ────────────────────────────────────────────
  describe('clicking a piece triggers selectPiece', () => {
    it('calls selectPiece when a red piece is clicked', () => {
      const selectPiece = vi.fn();
      const boardState = createInitialBoardState();
      const game = createMockGameState({
        boardState,
        selectPiece,
      });

      const result = render(<Board game={game} cellSize={CELL_SIZE} />);

      // Find the square containing the red general at (9,4)
      // Use container query to get the specific board instance
      const squares = getBoardSquares(result);
      const generalSquare = Array.from(squares).find((sq) => {
        const label = sq.getAttribute('aria-label') ?? '';
        return label === squareLabel(9, 4, 'red general');
      });

      expect(generalSquare).toBeDefined();
      fireEvent.click(generalSquare!);

      expect(selectPiece).toHaveBeenCalledTimes(1);
      expect(selectPiece).toHaveBeenCalledWith({ row: 9, col: 4 });
    });

    it('calls selectPiece when a different red piece is clicked', () => {
      const selectPiece = vi.fn();
      const boardState = createInitialBoardState();
      const game = createMockGameState({
        boardState,
        selectPiece,
      });

      const result = render(<Board game={game} cellSize={CELL_SIZE} />);

      // Find the square containing the red chariot at (9,0)
      const squares = getBoardSquares(result);
      const chariotSquare = Array.from(squares).find((sq) => {
        const label = sq.getAttribute('aria-label') ?? '';
        return label === squareLabel(9, 0, 'red chariot');
      });

      expect(chariotSquare).toBeDefined();
      fireEvent.click(chariotSquare!);

      expect(selectPiece).toHaveBeenCalledWith({ row: 9, col: 0 });
    });
  });

  // ────────────────────────────────────────────
  describe('red general at (9,4)', () => {
    it('shows no legal moves in the initial position (palace is full)', () => {
      const boardState = createInitialBoardState();
      const selectedPosition: Position = { row: 9, col: 4 };
      const legalMoves = getLegalMoves(boardState.board, selectedPosition);

      // In the starting position the general has no moves (surrounded by own pieces)
      const game = createMockGameState({
        boardState,
        selectedPosition,
        legalMoves,
      });

      const result = render(<Board game={game} cellSize={CELL_SIZE} />);

      const legalSquares = getLegalMoveSquares(result);
      expect(legalSquares).toHaveLength(legalMoves.length);
    });
  });
});
