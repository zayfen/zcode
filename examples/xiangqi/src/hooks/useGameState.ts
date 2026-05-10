import { useCallback, useRef, useState } from 'react';
import type { BoardState, GameStatus, Move, Piece, Position } from '../engine';
import {
  createInitialBoardState,
  getGameStatus,
  getLegalMoves,
  makeMove,
} from '../engine';

// ── Captured pieces tracker ──

interface CapturedPieces {
  readonly red: readonly Piece[];
  readonly black: readonly Piece[];
}

// ── Last move tracker ──

export interface LastMoveInfo {
  readonly from: Position;
  readonly to: Position;
  readonly piece: Piece;
}

// ── Hook return type ──

export interface GameState {
  readonly boardState: BoardState;
  readonly selectedPosition: Position | null;
  readonly legalMoves: readonly Position[];
  readonly moveHistory: readonly Move[];
  readonly gameStatus: GameStatus;
  readonly capturedPieces: CapturedPieces;
  /** The most recent move plus the piece that was moved (null at game start). */
  readonly lastMove: LastMoveInfo | null;
  /**
   * Unified click handler for the board.
   *
   * - If no piece is selected and the click is on the current player's piece, selects it.
   * - If a piece is already selected and the click is on a legal-move square, executes the move.
   * - If a piece is already selected and the click is on another of the current player's pieces,
   *   switches the selection.
   * - Otherwise, deselects.
   */
  readonly selectPiece: (pos: Position) => void;
  /** Execute the move to the given position using the currently selected piece, then clear selection. */
  readonly movePiece: (to: Position) => void;
  readonly resetGame: () => void;
}

// ── Initial values ──

const INITIAL_GAME_STATUS: GameStatus = { type: 'playing' };
const INITIAL_CAPTURED: CapturedPieces = {
  red: [] as readonly Piece[],
  black: [] as readonly Piece[],
};

// ── Hook ──

export function useGameState(): GameState {
  const [boardState, setBoardState] = useState<BoardState>(() =>
    createInitialBoardState(),
  );
  const [selectedPosition, setSelectedPosition] = useState<Position | null>(
    null,
  );
  const [legalMoves, setLegalMoves] = useState<readonly Position[]>([]);
  const [moveHistory, setMoveHistory] = useState<readonly Move[]>([]);
  const [gameStatus, setGameStatus] = useState<GameStatus>(INITIAL_GAME_STATUS);
  const [capturedPieces, setCapturedPieces] =
    useState<CapturedPieces>(INITIAL_CAPTURED);
  const [lastMove, setLastMove] = useState<LastMoveInfo | null>(null);

  // ── Mutable refs to always read the latest state without stale closures ──
  const boardStateRef = useRef(boardState);
  boardStateRef.current = boardState;

  const selectedPositionRef = useRef(selectedPosition);
  selectedPositionRef.current = selectedPosition;

  const legalMovesRef = useRef(legalMoves);
  legalMovesRef.current = legalMoves;

  const gameStatusRef = useRef(gameStatus);
  gameStatusRef.current = gameStatus;

  // ── Move execution (reads latest state via refs, applies all updates directly) ──

  const movePiece = useCallback((to: Position) => {
    // Snapshot the latest state via refs
    const prevSelected = selectedPositionRef.current;
    const prevLegal = legalMovesRef.current;
    const currentBoard = boardStateRef.current;

    if (prevSelected === null) return;

    // Verify the target is a legal move
    const isLegal = prevLegal.some(
      (p: Position) => p.row === to.row && p.col === to.col,
    );
    if (!isLegal) return;

    const move: Move = { from: prevSelected, to };
    const result = makeMove(currentBoard, move);

    if (!result.valid || !result.newState) return;

    // ── Apply new board state ──
    setBoardState(result.newState);

    // ── Record move in history ──
    setMoveHistory((prev: readonly Move[]) => [...prev, move]);

    // ── Track captured piece ──
    if (result.captured) {
      const capturingPlayer = currentBoard.currentPlayer;
      setCapturedPieces((prev: CapturedPieces) => ({
        ...prev,
        [capturingPlayer]: [...prev[capturingPlayer as keyof CapturedPieces], result.captured!],
      }));
    }

    // ── Record last move ──
    const movedPiece =
      currentBoard.board[move.from.row]?.[move.from.col];
    if (movedPiece) {
      setLastMove({ from: move.from, to: move.to, piece: movedPiece });
    }

    // ── Clear selection ──
    setSelectedPosition(null);
    setLegalMoves([]);

    // ── GAME-OVER DETECTION ──
    const status = getGameStatus(result.newState);
    setGameStatus(status);
  }, []); // no dependencies — reads latest state via refs

  // ── Piece selection (unified click handler) ──
  //
  // Click-to-select:  clicking own piece sets selectedPiece and triggers legal-move highlight.
  // Click-to-move:    clicking a highlighted square calls movePiece() and clears selection.
  // Click-to-deselect: clicking own piece again or an empty non-legal square clears selection.

  const selectPiece = useCallback(
    (pos: Position) => {
      // Ignore clicks if the game is over
      if (gameStatusRef.current.type !== 'playing') return;

      // If a piece is already selected and the click is on a legal move target,
      // delegate to movePiece (click-to-move flow).
      if (selectedPositionRef.current !== null) {
        const isLegal = legalMovesRef.current.some(
          (p: Position) => p.row === pos.row && p.col === pos.col,
        );
        if (isLegal) {
          movePiece(pos);
          return;
        }
      }

      const board = boardStateRef.current.board;
      const currentPlayer = boardStateRef.current.currentPlayer;

      // If clicking on own piece, select it (or switch selection)
      const piece = board[pos.row]?.[pos.col] ?? null;
      if (piece && piece.player === currentPlayer) {
        // Click-to-deselect: clicking the already-selected piece clears selection
        const alreadySelected =
          selectedPositionRef.current !== null &&
          selectedPositionRef.current.row === pos.row &&
          selectedPositionRef.current.col === pos.col;
        if (alreadySelected) {
          setSelectedPosition(null);
          setLegalMoves([]);
          return;
        }
        // Select it (or switch selection to a different own piece)
        setSelectedPosition(pos);
        setLegalMoves(getLegalMoves(board, pos));
        return;
      }

      // Otherwise, deselect
      setSelectedPosition(null);
      setLegalMoves([]);
    },
    [movePiece], // movePiece is stable (empty deps)
  );

  // ── Reset ──

  const resetGame = useCallback(() => {
    setBoardState(createInitialBoardState());
    setSelectedPosition(null);
    setLegalMoves([]);
    setMoveHistory([]);
    setGameStatus({ type: 'playing' });
    setCapturedPieces({ red: [], black: [] });
    setLastMove(null);
  }, []);

  return {
    boardState,
    selectedPosition,
    legalMoves,
    moveHistory,
    gameStatus,
    capturedPieces,
    lastMove,
    selectPiece,
    movePiece,
    resetGame,
  };
}
