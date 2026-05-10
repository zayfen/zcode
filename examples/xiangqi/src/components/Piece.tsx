import type { Piece as PieceData } from '../engine';
import { getPieceChar } from '../engine/constants';

// ── Props ──

export interface PieceProps {
  /** The piece data (type + player). */
  readonly piece: PieceData;
  /** Pixel diameter for the circle. */
  readonly size: number;
  /** Whether the piece is currently selected (toggles glow shadow). */
  readonly isSelected?: boolean;
  /** Click handler passed from parent. */
  readonly onClick?: () => void;
  /** Custom box-shadow override (e.g. elevated shadow during animation flight). */
  readonly shadowOverride?: string;
}

// ── Helpers ──

function getBackground(piece: PieceData): string {
  if (piece.player === 'red') {
    return `radial-gradient(
      circle at 35% 28%,
      rgba(255,248,235,0.95) 0%,
      var(--piece-wood-red-light) 15%,
      var(--piece-wood-red-mid) 40%,
      #c9a05c 70%,
      var(--piece-wood-red-dark) 100%
    )`;
  }
  return `radial-gradient(
    circle at 35% 28%,
    rgba(240,235,220,0.85) 0%,
    var(--piece-wood-black-light) 15%,
    var(--piece-wood-black-mid) 40%,
    #9a8a6c 70%,
    var(--piece-wood-black-dark) 100%
  )`;
}

function getBorder(piece: PieceData): string {
  const color =
    piece.player === 'red'
      ? 'var(--piece-border-red)'
      : 'var(--piece-border-black)';
  return `2px solid ${color}`;
}

function getColor(piece: PieceData): string {
  return piece.player === 'red'
    ? 'var(--piece-text-red)'
    : 'var(--piece-text-black)';
}

function getBoxShadow(isSelected: boolean): string {
  const inset =
    'inset 0 2px 4px rgba(255,255,255,0.5), inset 0 -3px 6px rgba(0,0,0,0.20)';

  if (isSelected) {
    return `var(--piece-shadow-selected), ${inset}`;
  }
  return `var(--piece-shadow-default), ${inset}`;
}

// ── Component ──

export function Piece({ piece, size, isSelected = false, onClick, shadowOverride }: PieceProps) {
  return (
    <div
      className="piece"
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      aria-label={`${piece.player} ${piece.type}`}
      onClick={onClick}
      onKeyDown={
        onClick
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onClick();
              }
            }
          : undefined
      }
      style={{
        width: size,
        height: size,
        background: getBackground(piece),
        border: getBorder(piece),
        color: getColor(piece),
        fontSize: size * 0.50,
        fontFamily: "'Ma Shan Zheng', 'Noto Serif SC', STKaiti, KaiTi, serif",
        fontWeight: 700,
        lineHeight: 1,
        letterSpacing: 'normal',
        /* Multi-layer text shadow for depth & readability */
        textShadow: [
          '0 1px 1px rgba(255,255,255,0.35)',
          '0 -1px 1px rgba(0,0,0,0.12)',
          '0 0 2px rgba(0,0,0,0.08)',
        ].join(', '),
        boxShadow: shadowOverride ?? getBoxShadow(isSelected),
        cursor: onClick ? 'pointer' : 'default',
        /* z-index must be higher than ::before overlay (1) */
        position: 'relative',
        zIndex: 2,
      }}
    >
      {getPieceChar(piece.player, piece.type)}
    </div>
  );
}
