import { afterEach, describe, expect, it } from 'vitest';
import { render, cleanup, within } from '@testing-library/react';
import { Board } from '../Board';
import { useGameState } from '../../hooks/useGameState';
import { INITIAL_POSITIONS, ROWS, COLS } from '../../engine/constants';
import type { InitialPlacement } from '../../engine/constants';
import React from 'react';

// ── Helper: renders <Board> with fresh initial game state ──

function BoardWrapper() {
  const game = useGameState();
  // Pass explicit cellSize so the test is deterministic regardless of jsdom viewport
  return <Board game={game} cellSize={50} />;
}

// ── Tests ──

describe('Board', () => {
  afterEach(cleanup);

  // ────────────────────────────────────────────
  describe('initial rendering', () => {
    it('renders 90 clickable squares (10 rows × 9 cols)', () => {
      const { container } = render(<BoardWrapper />);

      // All squares are rendered as div.board-square with role="button"
      const squares = container.querySelectorAll<HTMLDivElement>('.board-square');
      expect(squares.length).toBe(ROWS * COLS); // 10 × 9 = 90

      // Every square has role="button"
      const roleSquares = container.querySelectorAll('[role="button"].board-square');
      expect(roleSquares.length).toBe(90);
    });

    it('renders 32 pieces in the initial position', () => {
      const { container } = render(<BoardWrapper />);

      // Each <Piece> component renders a div with className="piece"
      const pieces = container.querySelectorAll('.piece');
      expect(pieces.length).toBe(32);
    });

    it('renders correct piece metadata at every initial position via aria-label', () => {
      const { container } = render(<BoardWrapper />);

      const squares = container.querySelectorAll<HTMLDivElement>('.board-square');

      // Build a lookup: "r,c" → InitialPlacement
      const lookup = new Map<string, InitialPlacement>();
      for (const ip of INITIAL_POSITIONS) {
        lookup.set(`${ip.row},${ip.col}`, ip);
      }

      for (const sq of squares) {
        const label = sq.getAttribute('aria-label') ?? '';
        // label format: "Square r,c" or "Square r,c — player type"
        const match = label.match(/^Square (\d+),(\d+)(?:\s*—\s*(.+))?$/);
        expect(match).not.toBeNull();
        const r = match![1];
        const c = match![2];
        const pieceInfo = match![3]; // e.g. "red general" or undefined

        const key = `${r},${c}`;
        const expected = lookup.get(key);

        if (expected) {
          // Square should have piece info
          expect(pieceInfo).toBe(`${expected.player} ${expected.type}`);
        } else {
          // Square should be empty
          expect(pieceInfo).toBeUndefined();
        }
      }
    });

    it('renders empty squares without pieces', () => {
      const { container } = render(<BoardWrapper />);

      // Build the set of occupied keys
      const occupied = new Set<string>();
      for (const ip of INITIAL_POSITIONS) {
        occupied.add(`${ip.row},${ip.col}`);
      }

      const squares = container.querySelectorAll<HTMLDivElement>('.board-square');

      for (const sq of squares) {
        const label = sq.getAttribute('aria-label') ?? '';
        const match = label.match(/^Square (\d+),(\d+)/);
        if (!match) continue;
        const key = `${match[1]},${match[2]}`;

        if (!occupied.has(key)) {
          // Empty square: should NOT contain a child with className "piece"
          const pieceEl = within(sq).queryByTestId?.('piece') ?? sq.querySelector('.piece');
          expect(pieceEl).toBeNull();
        }
      }
    });
  });

  // ────────────────────────────────────────────
  describe('square click targets', () => {
    it('each square has role="button" and tabIndex=0', () => {
      const { container } = render(<BoardWrapper />);

      const squares = container.querySelectorAll<HTMLDivElement>('.board-square');
      expect(squares.length).toBe(90);

      for (const sq of squares) {
        expect(sq.getAttribute('role')).toBe('button');
        expect(sq.tabIndex).toBe(0);
      }
    });

    it('each square has a descriptive aria-label', () => {
      const { container } = render(<BoardWrapper />);

      const squares = container.querySelectorAll<HTMLDivElement>('.board-square');

      for (const sq of squares) {
        const label = sq.getAttribute('aria-label');
        expect(label).toBeTruthy();
        // Should start with "Square "
        expect(label).toMatch(/^Square \d+,\d+/);
      }
    });

    it('occupied squares have piece info in aria-label', () => {
      const { container } = render(<BoardWrapper />);

      // Check a few known positions
      const redGeneralSquare = container.querySelector(
        '[aria-label="Square 9,4 — red general"]',
      );
      expect(redGeneralSquare).not.toBeNull();

      const blackGeneralSquare = container.querySelector(
        '[aria-label="Square 0,4 — black general"]',
      );
      expect(blackGeneralSquare).not.toBeNull();

      const redChariotSquare = container.querySelector(
        '[aria-label="Square 9,0 — red chariot"]',
      );
      expect(redChariotSquare).not.toBeNull();

      const blackCannonSquare = container.querySelector(
        '[aria-label="Square 2,1 — black cannon"]',
      );
      expect(blackCannonSquare).not.toBeNull();
    });

    it('empty squares have no piece info in aria-label', () => {
      const { container } = render(<BoardWrapper />);

      // Row 1 is entirely empty in the initial position
      for (let c = 0; c < COLS; c++) {
        const sq = container.querySelector(
          `[aria-label="Square 1,${c}"]`,
        );
        expect(sq).not.toBeNull();
        // Ensure no "—" separator (meaning no piece)
        const label = sq!.getAttribute('aria-label');
        expect(label).not.toContain('—');
      }
    });
  });

  // ────────────────────────────────────────────
  describe('initial position completeness', () => {
    it('every entry in INITIAL_POSITIONS has a corresponding piece on the board', () => {
      const { container } = render(<BoardWrapper />);

      for (const { row, col, player, type } of INITIAL_POSITIONS) {
        const sq = container.querySelector(
          `[aria-label="Square ${row},${col} — ${player} ${type}"]`,
        );
        expect(sq).not.toBeNull();
      }
    });

    it('piece count matches INITIAL_POSITIONS length', () => {
      const { container } = render(<BoardWrapper />);

      const pieces = container.querySelectorAll('.piece');
      expect(pieces.length).toBe(INITIAL_POSITIONS.length);
    });
  });
});
