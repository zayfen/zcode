import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import { Piece } from '../Piece';
import type { Piece as PieceData } from '../../engine/types';

// ── Fixtures ──

const redGeneral: PieceData = { type: 'general', player: 'red' };
const blackSoldier: PieceData = { type: 'soldier', player: 'black' };

// ── Tests ──

describe('Piece', () => {
  // ────────────────────────────────────────────
  describe('rendering', () => {
    afterEach(cleanup);

    it('renders the correct Chinese character for a red general', () => {
      render(<Piece piece={redGeneral} size={48} />);
      // No onClick → no role="button", query by text content
      expect(screen.getByText('帥')).toBeTruthy();
    });

    it('renders the correct Chinese character for a black soldier', () => {
      render(<Piece piece={blackSoldier} size={48} />);
      expect(screen.getByText('卒')).toBeTruthy();
    });

    it('sets correct aria-label when onClick is provided', () => {
      render(<Piece piece={redGeneral} size={48} onClick={vi.fn()} />);
      const el = screen.getByRole('button');
      expect(el.getAttribute('aria-label')).toBe('red general');
    });

    it('sets aria-label for a black soldier when onClick is provided', () => {
      render(<Piece piece={blackSoldier} size={48} onClick={vi.fn()} />);
      const el = screen.getByRole('button');
      expect(el.getAttribute('aria-label')).toBe('black soldier');
    });

    it('does not set role=button when onClick is omitted', () => {
      render(<Piece piece={redGeneral} size={48} />);
      // No role="button" → querying by role finds nothing
      expect(screen.queryByRole('button')).toBeNull();
    });

    it('has className "piece"', () => {
      render(<Piece piece={redGeneral} size={48} onClick={vi.fn()} />);
      const el = screen.getByRole('button');
      expect(el.classList.contains('piece')).toBe(true);
    });

    it('applies the given size as width and height', () => {
      render(<Piece piece={redGeneral} size={48} onClick={vi.fn()} />);
      const el = screen.getByRole('button');
      expect(el.style.width).toBe('48px');
      expect(el.style.height).toBe('48px');
    });

    it('applies pointer cursor when onClick is provided', () => {
      render(<Piece piece={redGeneral} size={48} onClick={vi.fn()} />);
      const el = screen.getByRole('button');
      expect(el.style.cursor).toBe('pointer');
    });

    it('applies default cursor when onClick is omitted', () => {
      render(<Piece piece={redGeneral} size={48} />);
      // Query by text since there's no role="button"
      const el = screen.getByText('帥');
      expect(el.style.cursor).toBe('default');
    });

    it('sets tabIndex=0 when onClick is provided', () => {
      render(<Piece piece={redGeneral} size={48} onClick={vi.fn()} />);
      const el = screen.getByRole('button');
      expect(el.tabIndex).toBe(0);
    });

    it('does not set tabIndex when onClick is omitted', () => {
      render(<Piece piece={redGeneral} size={48} />);
      const el = screen.getByText('帥');
      // jsdom returns -1 when tabIndex attribute is absent
      expect(el.getAttribute('tabindex')).toBeNull();
    });
  });

  // ────────────────────────────────────────────
  describe('click interaction', () => {
    afterEach(cleanup);

    it('invokes onClick when the piece div is clicked', () => {
      const onClick = vi.fn();
      render(<Piece piece={redGeneral} size={48} onClick={onClick} />);
      fireEvent.click(screen.getByRole('button'));
      expect(onClick).toHaveBeenCalledTimes(1);
    });

    it('does not crash when onClick is not provided and piece is clicked', () => {
      render(<Piece piece={redGeneral} size={48} />);
      expect(() => {
        fireEvent.click(screen.getByText('帥'));
      }).not.toThrow();
    });
  });

  // ────────────────────────────────────────────
  describe('keyboard interaction', () => {
    afterEach(cleanup);

    it('invokes onClick on Enter key', () => {
      const onClick = vi.fn();
      render(<Piece piece={redGeneral} size={48} onClick={onClick} />);
      fireEvent.keyDown(screen.getByRole('button'), { key: 'Enter' });
      expect(onClick).toHaveBeenCalledTimes(1);
    });

    it('invokes onClick on Space key', () => {
      const onClick = vi.fn();
      render(<Piece piece={redGeneral} size={48} onClick={onClick} />);
      fireEvent.keyDown(screen.getByRole('button'), { key: ' ' });
      expect(onClick).toHaveBeenCalledTimes(1);
    });

    it('does NOT invoke onClick on other keys', () => {
      const onClick = vi.fn();
      render(<Piece piece={redGeneral} size={48} onClick={onClick} />);
      fireEvent.keyDown(screen.getByRole('button'), { key: 'Tab' });
      expect(onClick).not.toHaveBeenCalled();
    });

    it('no key handler when onClick is undefined — keydown does not crash', () => {
      render(<Piece piece={redGeneral} size={48} />);
      // No role="button" but we can still fire keyDown on the element
      const el = screen.getByText('帥');
      expect(() => {
        fireEvent.keyDown(el, { key: 'Enter' });
      }).not.toThrow();
    });
  });

  // ────────────────────────────────────────────
  describe('selected state', () => {
    afterEach(cleanup);

    it('isSelected=true applies the selected box-shadow', () => {
      render(
        <Piece piece={redGeneral} size={48} isSelected={true} onClick={vi.fn()} />,
      );
      const el = screen.getByRole('button');
      expect(el.style.boxShadow).toContain('var(--piece-shadow-selected)');
      expect(el.style.boxShadow).not.toContain('var(--piece-shadow-default)');
    });

    it('isSelected=false applies the default box-shadow', () => {
      render(
        <Piece piece={redGeneral} size={48} isSelected={false} onClick={vi.fn()} />,
      );
      const el = screen.getByRole('button');
      expect(el.style.boxShadow).toContain('var(--piece-shadow-default)');
      expect(el.style.boxShadow).not.toContain('var(--piece-shadow-selected)');
    });
  });
});
