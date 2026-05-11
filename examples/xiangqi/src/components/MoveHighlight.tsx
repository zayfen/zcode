import type { CSSProperties } from 'react';

// ── Props ──

export interface MoveHighlightProps {
  /** Whether this square is a legal move target. */
  readonly isLegal: boolean;
  /** Whether this square contains a piece (opponent piece that can be captured). */
  readonly hasPiece: boolean;
  /** The cell size in pixels (used to scale the indicator). */
  readonly cellSize: number;
}

// ── Component ──

export function MoveHighlight({ isLegal, hasPiece, cellSize }: MoveHighlightProps) {
  if (!isLegal) return null;

  const shared: CSSProperties = {
    position: 'absolute',
    top: '50%',
    left: '50%',
    transform: 'translate(-50%, -50%)',
    borderRadius: '50%',
    pointerEvents: 'none',
    /* Smooth appearance */
    transition: 'opacity 150ms ease, transform 150ms ease',
  };

  // Empty legal-move square → soft green dot with glow
  if (!hasPiece) {
    return (
      <div
        style={{
          ...shared,
          width: cellSize * 0.22,
          height: cellSize * 0.22,
          background: 'radial-gradient(circle, rgba(0, 160, 0, 0.45) 0%, rgba(0, 128, 0, 0.25) 100%)',
          boxShadow: '0 0 6px 1px rgba(0, 160, 0, 0.15)',
        }}
      />
    );
  }

  // Capture target → ring with subtle glow
  return (
    <div
      style={{
        ...shared,
        width: cellSize * 0.88,
        height: cellSize * 0.88,
        border: `${Math.max(3, cellSize * 0.04)}px solid rgba(0, 140, 0, 0.55)`,
        background: 'transparent',
        boxShadow: '0 0 8px 2px rgba(0, 140, 0, 0.12), inset 0 0 8px 2px rgba(0, 140, 0, 0.06)',
      }}
    />
  );
}
