import { useCallback, useRef, useState } from 'react';
import type { Piece, Position } from '../engine';

// ── Animation State ──

export interface AnimationState {
  /** Source square the piece is moving from. */
  readonly from: Position;
  /** Destination square the piece is moving to. */
  readonly to: Position;
  /** The piece being animated. */
  readonly piece: Piece;
  /** Timestamp (ms) when the animation was started. */
  readonly startTime: number;
}

// ── Hook Return Type ──

export interface UseAnimationReturn {
  /** Current animation data (null when idle). */
  readonly animation: AnimationState | null;
  /** Whether an animation is currently in progress. */
  readonly isAnimating: boolean;
  /**
   * The destination position that should be **hidden** in the board
   * rendering while the animation is playing, so the phantom piece and
   * the "real" piece don't overlap.
   */
  readonly hiddenPosition: Position | null;
  /** Start a new move animation from `from` to `to` for the given `piece`. */
  readonly startAnimation: (from: Position, to: Position, piece: Piece) => void;
  /** Clear the current animation (typically called on `transitionend`). */
  readonly clearAnimation: () => void;
  /** Configured animation duration in milliseconds (≥ 200). */
  readonly duration: number;
}

// ── Hook ──

/**
 * Manages piece-move animation state.
 *
 * Usage: The consumer renders an absolutely-positioned "phantom" piece at
 * `animation.from` and applies a CSS `transform: translate(…)` to slide it
 * to `animation.to`.  The CSS `transition` (configured externally via
 * `.piece-animated`) handles the interpolation.  When the transition ends
 * the consumer calls `clearAnimation()`.
 *
 * @param duration Animation duration in ms (default 250, minimum 200).
 */
export function useAnimation(duration = 250): UseAnimationReturn {
  const safeDuration = Math.max(200, duration);

  const [animation, setAnimation] = useState<AnimationState | null>(null);
  const fallbackTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ── Clear ──

  const clearAnimation = useCallback(() => {
    if (fallbackTimerRef.current !== null) {
      clearTimeout(fallbackTimerRef.current);
      fallbackTimerRef.current = null;
    }
    setAnimation(null);
  }, []);

  // ── Start ──

  const startAnimation = useCallback(
    (from: Position, to: Position, piece: Piece) => {
      // Cancel any in-flight animation
      if (fallbackTimerRef.current !== null) {
        clearTimeout(fallbackTimerRef.current);
        fallbackTimerRef.current = null;
      }

      const next: AnimationState = {
        from,
        to,
        piece,
        startTime: Date.now(),
      };

      setAnimation(next);

      // Fallback: if `onTransitionEnd` never fires (e.g. element removed,
      // tab backgrounded, etc.) we auto-clear after duration + 50 ms.
      fallbackTimerRef.current = setTimeout(() => {
        setAnimation(null);
        fallbackTimerRef.current = null;
      }, safeDuration + 50);
    },
    [safeDuration],
  );

  // ── Derived values ──

  const isAnimating = animation !== null;
  const hiddenPosition: Position | null = animation ? animation.to : null;

  return {
    animation,
    isAnimating,
    hiddenPosition,
    startAnimation,
    clearAnimation,
    duration: safeDuration,
  };
}
