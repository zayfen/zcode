import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen as _screen, fireEvent, within } from '@testing-library/react';
import { GameOverModal } from '../GameOverModal';
import type { GameStatus } from '../../engine';

// ── Helpers ──

/** Render the modal and also return the container for direct DOM queries. */
function renderModal(gameStatus: GameStatus, onPlayAgain = vi.fn()) {
  return render(
    <GameOverModal gameStatus={gameStatus} onPlayAgain={onPlayAgain} />,
  );
}

/** Query whether the dialog overlay is present in the DOM. */
function queryOverlay(container: HTMLElement) {
  return container.querySelector('[aria-modal="true"]') as HTMLElement | null;
}

// ══════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════

describe('GameOverModal', () => {
  // Ensure the DOM is cleaned up between tests
  afterEach(() => {
    cleanup();
  });

  // ────────────────────────────────────────────
  describe('rendering', () => {
    it('does NOT render when gameStatus is "playing"', () => {
      const { container } = renderModal({ type: 'playing' });
      expect(queryOverlay(container)).toBeNull();
    });

    it('renders with checkmate message when red wins by checkmate', () => {
      const { container } = renderModal({ type: 'checkmate', winner: 'red' });
      const dialog = queryOverlay(container);
      expect(dialog).not.toBeNull();
      expect(within(dialog!).getByText('将死！')).toBeDefined();
      expect(within(dialog!).getByText('红方胜！')).toBeDefined();
    });

    it('renders with checkmate message when black wins by checkmate', () => {
      const { container } = renderModal({ type: 'checkmate', winner: 'black' });
      const dialog = queryOverlay(container);
      expect(dialog).not.toBeNull();
      expect(within(dialog!).getByText('将死！')).toBeDefined();
      expect(within(dialog!).getByText('黑方胜！')).toBeDefined();
    });

    it('renders with stalemate message when a player is stalemated', () => {
      const { container } = renderModal({ type: 'stalemate', loser: 'red' });
      const dialog = queryOverlay(container);
      expect(dialog).not.toBeNull();
      expect(within(dialog!).getByText('困毙！')).toBeDefined();
      expect(within(dialog!).getByText('红方负！')).toBeDefined();
    });

    it('renders with stalemate message for black loser', () => {
      const { container } = renderModal({ type: 'stalemate', loser: 'black' });
      const dialog = queryOverlay(container);
      expect(dialog).not.toBeNull();
      expect(within(dialog!).getByText('困毙！')).toBeDefined();
      expect(within(dialog!).getByText('黑方负！')).toBeDefined();
    });

    it('contains a "Play Again" button', () => {
      const { container } = renderModal({ type: 'checkmate', winner: 'red' });
      const dialog = queryOverlay(container);
      expect(dialog).not.toBeNull();
      const button = within(dialog!).getByRole('button', { name: /play again/i });
      expect(button).toBeDefined();
      expect(button.textContent).toContain('再局');
    });

    it('has role="dialog" and aria-modal="true"', () => {
      const { container } = renderModal({ type: 'checkmate', winner: 'red' });
      const dialog = queryOverlay(container);
      expect(dialog).not.toBeNull();
      expect(dialog!.getAttribute('role')).toBe('dialog');
      expect(dialog!.getAttribute('aria-modal')).toBe('true');
    });

    it('has correct aria-label for checkmate', () => {
      const { container } = renderModal({ type: 'checkmate', winner: 'red' });
      const dialog = queryOverlay(container);
      expect(dialog!.getAttribute('aria-label')).toBe('Game over — checkmate');
    });

    it('has correct aria-label for stalemate', () => {
      const { container } = renderModal({ type: 'stalemate', loser: 'black' });
      const dialog = queryOverlay(container);
      expect(dialog!.getAttribute('aria-label')).toBe('Game over — stalemate');
    });
  });

  // ────────────────────────────────────────────
  describe('Play Again button', () => {
    it('calls onPlayAgain callback when clicked', () => {
      const onPlayAgain = vi.fn();
      const { container } = renderModal({ type: 'checkmate', winner: 'red' }, onPlayAgain);

      const dialog = queryOverlay(container);
      const button = within(dialog!).getByRole('button', { name: /play again/i });
      fireEvent.click(button);

      expect(onPlayAgain).toHaveBeenCalledTimes(1);
    });

    it('calls onPlayAgain for stalemate too', () => {
      const onPlayAgain = vi.fn();
      const { container } = renderModal({ type: 'stalemate', loser: 'black' }, onPlayAgain);

      const dialog = queryOverlay(container);
      const button = within(dialog!).getByRole('button', { name: /play again/i });
      fireEvent.click(button);

      expect(onPlayAgain).toHaveBeenCalledTimes(1);
    });
  });

  // ────────────────────────────────────────────
  describe('integration: modal appears on checkmate, Play Again resets', () => {
    it('modal is hidden during normal play, appears after checkmate, disappears after reset', () => {
      const onPlayAgain = vi.fn();

      // 1. Start with "playing" — no modal
      const { container, rerender } = render(
        <GameOverModal
          gameStatus={{ type: 'playing' }}
          onPlayAgain={onPlayAgain}
        />,
      );

      expect(queryOverlay(container)).toBeNull();

      // 2. Game ends in checkmate — modal appears
      rerender(
        <GameOverModal
          gameStatus={{ type: 'checkmate', winner: 'red' }}
          onPlayAgain={onPlayAgain}
        />,
      );

      const dialog = queryOverlay(container);
      expect(dialog).not.toBeNull();
      expect(within(dialog!).getByText('将死！')).toBeDefined();
      expect(within(dialog!).getByText('红方胜！')).toBeDefined();

      // 3. Click "Play Again"
      const button = within(dialog!).getByRole('button', { name: /play again/i });
      fireEvent.click(button);
      expect(onPlayAgain).toHaveBeenCalledTimes(1);

      // 4. After reset, game goes back to "playing" — modal disappears
      rerender(
        <GameOverModal
          gameStatus={{ type: 'playing' }}
          onPlayAgain={onPlayAgain}
        />,
      );

      expect(queryOverlay(container)).toBeNull();
    });

    it('modal appears after stalemate and disappears after reset', () => {
      const onPlayAgain = vi.fn();

      // Playing — no modal
      const { container, rerender } = render(
        <GameOverModal
          gameStatus={{ type: 'playing' }}
          onPlayAgain={onPlayAgain}
        />,
      );
      expect(queryOverlay(container)).toBeNull();

      // Stalemate — modal appears
      rerender(
        <GameOverModal
          gameStatus={{ type: 'stalemate', loser: 'red' }}
          onPlayAgain={onPlayAgain}
        />,
      );
      const dialog = queryOverlay(container);
      expect(dialog).not.toBeNull();
      expect(within(dialog!).getByText('困毙！')).toBeDefined();

      // Click Play Again
      const button = within(dialog!).getByRole('button', { name: /play again/i });
      fireEvent.click(button);
      expect(onPlayAgain).toHaveBeenCalledTimes(1);

      // Reset to playing — modal gone
      rerender(
        <GameOverModal
          gameStatus={{ type: 'playing' }}
          onPlayAgain={onPlayAgain}
        />,
      );
      expect(queryOverlay(container)).toBeNull();
    });
  });
});
