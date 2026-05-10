import type { Piece } from '../engine';
import { getPieceChar } from '../engine/constants';

// ── Types ──

interface CapturedPiecesProps {
  readonly capturedPieces: {
    readonly red: readonly Piece[];
    readonly black: readonly Piece[];
  };
}

// ── Piece-value sort order ──

const PIECE_ORDER: Record<Piece['type'], number> = {
  general: 0,
  chariot: 1,
  knight: 2,
  cannon: 3,
  advisor: 4,
  elephant: 5,
  soldier: 6,
};

function sortPieces(pieces: readonly Piece[]): Piece[] {
  return [...pieces].sort((a, b) => PIECE_ORDER[a.type] - PIECE_ORDER[b.type]);
}

// ── Helpers ──

function getBackground(piece: Piece): string {
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

function getBorder(piece: Piece): string {
  const color =
    piece.player === 'red'
      ? 'var(--piece-border-red)'
      : 'var(--piece-border-black)';
  return `1.5px solid ${color}`;
}

function getColor(piece: Piece): string {
  return piece.player === 'red'
    ? 'var(--piece-text-red)'
    : 'var(--piece-text-black)';
}

// ── Sub-component: single mini piece ──

function CapturedPiece({ piece }: { readonly piece: Piece }) {
  return (
    <div
      className="captured-piece"
      title={`${piece.player} ${piece.type}`}
      style={{
        width: 28,
        height: 28,
        borderRadius: '50%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: getBackground(piece),
        border: getBorder(piece),
        color: getColor(piece),
        fontSize: 14,
        fontFamily: "'Ma Shan Zheng', 'Noto Serif SC', serif",
        fontWeight: 700,
        lineHeight: 1,
        letterSpacing: 'normal',
        textShadow: [
          '0 1px 1px rgba(255,255,255,0.35)',
          '0 -1px 1px rgba(0,0,0,0.12)',
        ].join(', '),
        boxShadow:
          'inset 0 1px 2px rgba(255,255,255,0.5), inset 0 -1px 3px rgba(0,0,0,0.18), 2px 2px 5px rgba(0,0,0,0.25)',
        userSelect: 'none',
        flexShrink: 0,
        position: 'relative',
      }}
    >
      {getPieceChar(piece.player, piece.type)}
    </div>
  );
}

// ── Sub-component: one row (one side's captures) ──

function CapturedRow({
  label,
  pieces,
  labelColor,
}: {
  readonly label: string;
  readonly pieces: readonly Piece[];
  readonly labelColor: string;
}) {
  if (pieces.length === 0) {
    return null;
  }

  const sorted = sortPieces(pieces);

  return (
    <div
      className="captured-row"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        flexWrap: 'wrap',
      }}
    >
      <span
        style={{
          fontSize: '0.85rem',
          fontWeight: 600,
          color: labelColor,
          fontFamily: "'Noto Serif SC', serif",
          whiteSpace: 'nowrap',
          minWidth: 56,
        }}
      >
        {label}
      </span>
      {sorted.map((piece, i) => (
        <CapturedPiece key={`${piece.type}-${piece.player}-${i}`} piece={piece} />
      ))}
    </div>
  );
}

// ── Main component ──

export function CapturedPieces({ capturedPieces }: CapturedPiecesProps) {
  const { red, black } = capturedPieces;
  const hasCaptures = red.length > 0 || black.length > 0;

  if (!hasCaptures) {
    return null;
  }

  return (
    <div
      className="captured-pieces"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        padding: '10px 16px',
        margin: '6px 0',
        background: 'linear-gradient(180deg, #fdf6e3 0%, #f5e6c8 100%)',
        border: '1px solid #d4a76a',
        borderRadius: 8,
        boxShadow: '0 2px 6px rgba(0,0,0,0.08)',
        minWidth: 200,
      }}
    >
      {/* Red's captures → these are black pieces taken by red */}
      <CapturedRow
        label="红方得子"
        pieces={red}
        labelColor="var(--piece-text-red)"
      />
      {/* Black's captures → these are red pieces taken by black */}
      <CapturedRow
        label="黑方得子"
        pieces={black}
        labelColor="var(--piece-text-black)"
      />
    </div>
  );
}
