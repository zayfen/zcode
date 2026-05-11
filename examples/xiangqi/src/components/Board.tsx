import { useCallback, useLayoutEffect, useRef, useState } from 'react';
import type { Piece as PieceData, Position } from '../engine';
import { COLS, ROWS } from '../engine/constants';
import { useAnimation } from '../hooks/useAnimation';
import { useResponsiveCellSize } from '../hooks/useResponsiveCellSize';
import type { GameState } from '../hooks/useGameState';
import { Piece } from './Piece';
import { MoveHighlight } from './MoveHighlight';
import '../styles/animations.css';
import '../styles/pieces.css';

// ── Props ──

export interface BoardProps {
  readonly game: GameState;
  /** Override the auto-computed cell size (useful for tests). */
  readonly cellSize?: number;
}

// ── Board Component ──

export function Board({ game, cellSize: overrideCellSize }: BoardProps) {
  const responsive = useResponsiveCellSize();
  const cellSize = overrideCellSize ?? responsive;
  const anim = useAnimation(280);
  const phantomRef = useRef<HTMLDivElement>(null);
  const [pendingAnim, setPendingAnim] = useState<{
    from: Position;
    to: Position;
    piece: PieceData;
  } | null>(null);

  const boardWidth = (COLS - 1) * cellSize;
  const boardHeight = (ROWS - 1) * cellSize;

  // ── Handle square click ──

  const handleSquareClick = useCallback(
    (pos: Position) => {
      // If we have a selected piece and the click is on a legal move target,
      // capture the moving piece BEFORE the game state updates, then execute.
      if (game.selectedPosition) {
        const isLegal = game.legalMoves.some(
          (p) => p.row === pos.row && p.col === pos.col,
        );
        if (isLegal) {
          const piece =
            game.boardState.board[game.selectedPosition.row]?.[
              game.selectedPosition.col
            ];
          if (piece) {
            setPendingAnim({
              from: game.selectedPosition,
              to: pos,
              piece,
            });
          }
          // Execute the move (applies board change & clears selection)
          game.movePiece(pos);
          return;
        }
      }

      // Otherwise trigger selection / deselection
      game.selectPiece(pos);
    },
    [game],
  );

  // ── Start animation when pending data is available ──

  useLayoutEffect(() => {
    if (pendingAnim) {
      anim.startAnimation(pendingAnim.from, pendingAnim.to, pendingAnim.piece);
      setPendingAnim(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingAnim]);

  // ── Trigger CSS transition on the phantom element ──

  useLayoutEffect(() => {
    if (anim.animation && phantomRef.current) {
      const { from, to } = anim.animation;
      const dx = (to.col - from.col) * cellSize;
      const dy = (to.row - from.row) * cellSize;

      // Add will-animate class for GPU acceleration
      phantomRef.current.classList.add('will-animate');

      // Force a reflow so the initial position (translate(0,0)) is painted
      // before we set the final transform.
      // eslint-disable-next-line no-void
      void phantomRef.current.offsetHeight;

      phantomRef.current.style.transform = `translate(${dx}px, ${dy}px)`;
    }
    // Reset phantom transform when animation is cleared
    if (!anim.animation && phantomRef.current) {
      phantomRef.current.style.transform = 'translate(0px, 0px)';
      phantomRef.current.classList.remove('will-animate');
    }
  }, [anim.animation, cellSize]);

  // ── Transition end handler ──

  const handleTransitionEnd = useCallback(() => {
    anim.clearAnimation();
  }, [anim]);

  // ── Render ──

  const { board } = game.boardState;

  // Determine which position to hide (the destination of the animated piece)
  const hidePos = anim.hiddenPosition;

  // ── Last-move highlight positions ──
  const lastFrom = game.lastMove?.from ?? null;
  const lastTo = game.lastMove?.to ?? null;

  // ── SVG filter IDs for texture effects ──

  return (
    <div
      className="board-container"
      style={{
        position: 'relative',
        width: boardWidth + cellSize,
        height: boardHeight + cellSize,
        margin: `${Math.max(2, cellSize * 0.1)}px auto`,
        /* Board outer shadow */
        boxShadow: '0 4px 12px rgba(0,0,0,0.15), 0 2px 4px rgba(0,0,0,0.1)',
      }}
    >
      {/* ── Grid background ── */}
      <svg
        className="board-grid"
        width={boardWidth + cellSize}
        height={boardHeight + cellSize}
        style={{ position: 'absolute', top: 0, left: 0 }}
      >
        <defs>
          {/* Wood-grain texture pattern */}
          <pattern
            id="woodgrain"
            patternUnits="userSpaceOnUse"
            width={cellSize * 9}
            height={cellSize * 10}
          >
            {/* Base warm wood colour */}
            <rect
              width={cellSize * 9}
              height={cellSize * 10}
              fill="#f0d9b5"
            />
            {/* Subtle colour variation layer */}
            <rect
              width={cellSize * 9}
              height={cellSize * 10}
              fill="url(#woodVariation)"
              opacity="0.5"
            />
            {/* Diagonal grain lines — layer 1 */}
            <line x1="0" y1="0" x2={cellSize * 9} y2={cellSize * 10}
              stroke="rgba(139,69,19,0.05)" strokeWidth="1.5" />
            <line x1={cellSize * 1.2} y1="0" x2={cellSize * 9} y2={cellSize * 7.8}
              stroke="rgba(139,69,19,0.045)" strokeWidth="1" />
            <line x1="0" y1={cellSize * 2.2} x2={cellSize * 7.5} y2={cellSize * 10}
              stroke="rgba(139,69,19,0.05)" strokeWidth="1.2" />
            <line x1={cellSize * 3} y1="0" x2={cellSize * 9} y2={cellSize * 5.5}
              stroke="rgba(139,69,19,0.04)" strokeWidth="0.8" />
            <line x1="0" y1={cellSize * 5} x2={cellSize * 4} y2={cellSize * 10}
              stroke="rgba(139,69,19,0.045)" strokeWidth="1" />
            <line x1={cellSize * 5} y1="0" x2={cellSize * 9} y2={cellSize * 3.5}
              stroke="rgba(139,69,19,0.04)" strokeWidth="0.8" />
            <line x1="0" y1={cellSize * 7.5} x2={cellSize * 2.5} y2={cellSize * 10}
              stroke="rgba(139,69,19,0.04)" strokeWidth="0.7" />
            {/* Additional diagonal grain for richer texture */}
            <line x1={cellSize * 2} y1="0" x2={cellSize * 9} y2={cellSize * 6.5}
              stroke="rgba(139,69,19,0.035)" strokeWidth="0.6" />
            <line x1="0" y1={cellSize * 3.8} x2={cellSize * 6} y2={cellSize * 10}
              stroke="rgba(139,69,19,0.035)" strokeWidth="0.9" />
            <line x1={cellSize * 7} y1="0" x2={cellSize * 9} y2={cellSize * 2}
              stroke="rgba(139,69,19,0.03)" strokeWidth="0.7" />
            {/* Horizontal grain lines — layer 2 */}
            <line x1="0" y1={cellSize * 0.8} x2={cellSize * 9} y2={cellSize * 0.8}
              stroke="rgba(139,69,19,0.04)" strokeWidth="2" />
            <line x1="0" y1={cellSize * 2.5} x2={cellSize * 9} y2={cellSize * 2.5}
              stroke="rgba(139,69,19,0.035)" strokeWidth="1.5" />
            <line x1="0" y1={cellSize * 4.2} x2={cellSize * 9} y2={cellSize * 4.2}
              stroke="rgba(139,69,19,0.04)" strokeWidth="1.8" />
            <line x1="0" y1={cellSize * 5.8} x2={cellSize * 9} y2={cellSize * 5.8}
              stroke="rgba(139,69,19,0.035)" strokeWidth="1.5" />
            <line x1="0" y1={cellSize * 7.2} x2={cellSize * 9} y2={cellSize * 7.2}
              stroke="rgba(139,69,19,0.04)" strokeWidth="2" />
            <line x1="0" y1={cellSize * 9} x2={cellSize * 9} y2={cellSize * 9}
              stroke="rgba(139,69,19,0.035)" strokeWidth="1" />
          </pattern>

          {/* Subtle radial colour variation across the board */}
          <radialGradient id="woodVariation" cx="50%" cy="50%" r="70%">
            <stop offset="0%" stopColor="#e8c888" stopOpacity="0.3" />
            <stop offset="60%" stopColor="#d4a55a" stopOpacity="0.1" />
            <stop offset="100%" stopColor="#c49040" stopOpacity="0.15" />
          </radialGradient>

          {/* Board border bevel shadow (unused — kept for potential future use) */}
          <filter id="boardBevel" x="-2%" y="-2%" width="104%" height="104%">
            <feDropShadow dx="0" dy="0" stdDeviation="3" floodColor="#8B4513" floodOpacity="0.08" />
          </filter>
        </defs>

        {/* Board background with wood grain */}
        <rect x={0} y={0}
          width={boardWidth + cellSize} height={boardHeight + cellSize}
          fill="url(#woodgrain)" rx="4" ry="4" />

        {/* Subtle colour variation overlay */}
        <rect x={0} y={0}
          width={boardWidth + cellSize} height={boardHeight + cellSize}
          fill="url(#woodVariation)" opacity="0.3" rx="4" ry="4" />

        {/* Board edge / border */}
        <rect x={0} y={0}
          width={boardWidth + cellSize} height={boardHeight + cellSize}
          fill="none" stroke="#8B4513" strokeWidth="2.5" rx="4" ry="4"
          strokeOpacity="0.6" />

        {/* River text: 楚河汉界 */}
        <text
          x={2 * cellSize + cellSize / 2}
          y={4.5 * cellSize + cellSize / 2}
          textAnchor="middle"
          dominantBaseline="central"
          fill="#8B4513"
          opacity="0.4"
          fontSize={cellSize * 0.42}
          fontFamily="'Ma Shan Zheng', 'Noto Serif SC', serif"
          fontWeight={600}
          letterSpacing={cellSize * 0.3}
        >
          楚河
        </text>
        <text
          x={6 * cellSize + cellSize / 2}
          y={4.5 * cellSize + cellSize / 2}
          textAnchor="middle"
          dominantBaseline="central"
          fill="#8B4513"
          opacity="0.4"
          fontSize={cellSize * 0.42}
          fontFamily="'Ma Shan Zheng', 'Noto Serif SC', serif"
          fontWeight={600}
          letterSpacing={cellSize * 0.3}
        >
          汉界
        </text>

        {/* Horizontal lines */}
        {Array.from({ length: ROWS }, (_, r) => (
          <line
            key={`h-${r}`}
            x1={cellSize / 2}
            y1={r * cellSize + cellSize / 2}
            x2={boardWidth + cellSize / 2}
            y2={r * cellSize + cellSize / 2}
            stroke="#8B4513"
            strokeWidth={1}
            strokeOpacity="0.7"
          />
        ))}

        {/* Vertical lines (split by river) */}
        {Array.from({ length: COLS }, (_, c) => {
          const x = c * cellSize + cellSize / 2;
          return (
            <g key={`v-${c}`}>
              {/* Top half */}
              <line x1={x} y1={cellSize / 2} x2={x} y2={4 * cellSize + cellSize / 2}
                stroke="#8B4513" strokeWidth={1} strokeOpacity="0.7" />
              {/* Bottom half */}
              <line x1={x} y1={5 * cellSize + cellSize / 2} x2={x} y2={9 * cellSize + cellSize / 2}
                stroke="#8B4513" strokeWidth={1} strokeOpacity="0.7" />
            </g>
          );
        })}

        {/* Palace diagonals */}
        <line x1={3 * cellSize + cellSize / 2} y1={0 * cellSize + cellSize / 2}
          x2={5 * cellSize + cellSize / 2} y2={2 * cellSize + cellSize / 2}
          stroke="#8B4513" strokeWidth={1} strokeOpacity="0.7" />
        <line x1={5 * cellSize + cellSize / 2} y1={0 * cellSize + cellSize / 2}
          x2={3 * cellSize + cellSize / 2} y2={2 * cellSize + cellSize / 2}
          stroke="#8B4513" strokeWidth={1} strokeOpacity="0.7" />
        <line x1={3 * cellSize + cellSize / 2} y1={7 * cellSize + cellSize / 2}
          x2={5 * cellSize + cellSize / 2} y2={9 * cellSize + cellSize / 2}
          stroke="#8B4513" strokeWidth={1} strokeOpacity="0.7" />
        <line x1={5 * cellSize + cellSize / 2} y1={7 * cellSize + cellSize / 2}
          x2={3 * cellSize + cellSize / 2} y2={9 * cellSize + cellSize / 2}
          stroke="#8B4513" strokeWidth={1} strokeOpacity="0.7" />
      </svg>

      {/* ── Squares / click targets ── */}
      {Array.from({ length: ROWS }, (_, r) =>
        Array.from({ length: COLS }, (_, c) => {
          const pos: Position = { row: r, col: c };
          const piece = board[r]?.[c] ?? null;
          const isSelected =
            game.selectedPosition?.row === r && game.selectedPosition?.col === c;
          const isLegal = game.legalMoves.some(
            (p) => p.row === r && p.col === c,
          );
          const isHidden = hidePos?.row === r && hidePos?.col === c;

          // Last-move highlight: dim yellow background
          const isLastFrom = lastFrom?.row === r && lastFrom?.col === c;
          const isLastTo = lastTo?.row === r && lastTo?.col === c;
          const showLastMove = isLastFrom || isLastTo;

          return (
            <div
              key={`${r}-${c}`}
              className="board-square"
              role="button"
              tabIndex={0}
              aria-label={`Square ${r},${c}${piece ? ` — ${piece.player} ${piece.type}` : ''}`}
              style={{
                position: 'absolute',
                left: c * cellSize,
                top: r * cellSize,
                width: cellSize,
                height: cellSize,
                cursor: isLegal ? 'pointer' : 'default',
                /* Last-move highlight */
                background: showLastMove
                  ? 'radial-gradient(circle, rgba(255,215,0,0.18) 0%, rgba(255,200,0,0.08) 70%, transparent 100%)'
                  : undefined,
                borderRadius: '50%',
              }}
              onClick={() => handleSquareClick(pos)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  handleSquareClick(pos);
                }
              }}
            >
              {/* Piece */}
              {piece && !isHidden && (
                <Piece
                  piece={piece}
                  size={cellSize * 0.85}
                  isSelected={isSelected}
                  onClick={() => handleSquareClick(pos)}
                />
              )}

              {/* Legal move indicator */}
              <MoveHighlight
                isLegal={isLegal}
                hasPiece={!!piece}
                cellSize={cellSize}
              />
            </div>
          );
        }),
      )}

      {/* ── Animated phantom piece ── */}
      {anim.animation && (
        <div
          ref={phantomRef}
          className="piece-animated"
          onTransitionEnd={handleTransitionEnd}
          style={{
            position: 'absolute',
            left: anim.animation.from.col * cellSize,
            top: anim.animation.from.row * cellSize,
            width: cellSize,
            height: cellSize,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transform: 'translate(0px, 0px)',
          }}
        >
          <Piece
            piece={anim.animation.piece}
            size={cellSize * 0.85}
            shadowOverride="0 6px 16px rgba(0,0,0,0.45), 3px 4px 10px rgba(0,0,0,0.35), inset 0 2px 4px rgba(255,255,255,0.5), inset 0 -3px 6px rgba(0,0,0,0.20)"
          />
        </div>
      )}
    </div>
  );
}
