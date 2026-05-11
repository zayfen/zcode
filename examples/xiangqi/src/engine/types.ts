// ── Core Types for the Xiangqi Rules Engine ──

/** The two players / sides. */
export type Player = 'red' | 'black';

/** The seven piece types in Xiangqi. */
export type PieceType =
  | 'general'    // 将/帅
  | 'advisor'    // 士/仕
  | 'elephant'   // 象/相
  | 'knight'     // 马
  | 'chariot'    // 车
  | 'cannon'     // 炮
  | 'soldier';   // 兵/卒

/** A single piece on the board. */
export interface Piece {
  readonly type: PieceType;
  readonly player: Player;
}

/** Zero-based board coordinate: row 0 = black back rank, row 9 = red back rank. */
export interface Position {
  readonly row: number;
  readonly col: number;
}

/**
 * The board is a 10-row × 9-col grid.
 * `board[row][col]` is `null` for an empty square or a `Piece`.
 */
export type Board = ReadonlyArray<ReadonlyArray<Piece | null>>;

/** Full game state (immutable). */
export interface BoardState {
  readonly board: Board;
  readonly currentPlayer: Player;
}

/** A move from one square to another. */
export interface Move {
  readonly from: Position;
  readonly to: Position;
}

/** Result of attempting a move. */
export interface MoveResult {
  readonly valid: boolean;
  readonly newState?: BoardState;
  readonly captured?: Piece;
}

/** Game is ongoing. */
export interface GameStatusPlaying {
  readonly type: 'playing';
}

/** Game ended by checkmate — the `winner` delivered the fatal move. */
export interface GameStatusCheckmate {
  readonly type: 'checkmate';
  readonly winner: Player;
}

/** Game ended by stalemate — the `loser` has no legal moves (but is not in check). */
export interface GameStatusStalemate {
  readonly type: 'stalemate';
  readonly loser: Player;
}

/** Discriminated union representing the current game status. */
export type GameStatus =
  | GameStatusPlaying
  | GameStatusCheckmate
  | GameStatusStalemate;
