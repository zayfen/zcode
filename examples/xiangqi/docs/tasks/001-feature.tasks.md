# Tasks: 中国象棋 (Xiangqi) Web Game

## Implementation Tasks

### Phase 1 — Engine Foundation
- [ ] Define core types in `engine/types.ts`: `PieceType`, `Player` (Red/Black), `Position` (row, col), `Piece`, `BoardState`, `Move`, `MoveResult`.
- [ ] Define initial board layout and constants in `engine/constants.ts`: starting positions for all 32 pieces, palace coordinate ranges, river row index, board dimensions (9 cols × 10 rows).
- [ ] Implement `createInitialBoard()` returning the default `BoardState`.

### Phase 2 — Move Validation (Per Piece)
- [ ] Implement chariot (车) move generation: straight-line moves, blocked by first piece encountered; can capture opponent piece at block position.
- [ ] Implement knight (马) move generation: L-shape offsets; check for 蹩马腿 (blocking piece on orthogonal adjacent square).
- [ ] Implement elephant (象) move generation: diagonal-2-square (田) pattern; check for 塞象眼 (blocking piece at center of 田); enforce river boundary (cannot cross).
- [ ] Implement advisor (士) move generation: one-step diagonal within palace (九宫) bounds.
- [ ] Implement general (将/帅) move generation: one-step orthogonal within palace bounds.
- [ ] Implement cannon (炮) move generation: straight-line moves like chariot for non-capture; capture requires exactly one screen (炮架) between cannon and target.
- [ ] Implement soldier (兵/卒) move generation: forward-only before river; forward + left/right after crossing river; never backward.

### Phase 3 — Check & Special Rules
- [ ] Implement `isSquareAttackedBy(board, position, attacker)` — determines if any piece of `attacker` can move to `position`.
- [ ] Implement `isInCheck(board, player)` — returns true if `player`'s general is under attack.
- [ ] Implement flying-general rule (将帅照面): detect when both generals share the same column with no pieces between them; treat as illegal state.
- [ ] Implement `getLegalMoves(board, position)` — generate raw moves for piece at `position`, then filter out any move that would leave own general in check or violate flying-general rule.

### Phase 4 — Game State & Win Detection
- [ ] Implement `makeMove(board, move)` — returns new `BoardState` with move applied (immutable); returns `MoveResult` indicating success/failure.
- [ ] Implement `isCheckmate(board, player)` — player is in check AND has zero legal moves.
- [ ] Implement `isStalemate(board, player)` — player is NOT in check but has zero legal moves (loss in Xiangqi).
- [ ] Create `useGameState` hook: manages `BoardState`, current player, selected piece, move history; exposes `selectPiece()`, `movePiece()`, `resetGame()`.

### Phase 5 — Board & Piece Rendering
- [ ] Create `Board` component: renders 9×10 grid with SVG/CSS; draws river (楚河汉界) text and decorative border.
- [ ] Create `Piece` component: renders a single piece with Chinese character, wooden-texture gradient, `box-shadow` for 3D effect; accepts click handler.
- [ ] Create `MoveHighlight` component: renders highlighted dots/rings on legal target squares.
- [ ] Apply board background styling: wood-grain CSS gradient or SVG pattern.
- [ ] Apply piece styling: radial gradient for dome effect, drop shadow, Chinese calligraphy font (e.g., Ma Shan Zheng or system serif).

### Phase 6 — Interaction & Animation
- [ ] Implement click-to-select flow: clicking own piece sets `selectedPiece` and triggers legal-move highlight.
- [ ] Implement click-to-move flow: clicking a highlighted square calls `makeMove()` and clears selection.
- [ ] Implement click-to-deselect: clicking own piece again or clicking an empty non-legal square clears selection.
- [ ] Implement `useAnimation` hook: on move, animate piece from source to destination square using CSS `transition: transform` (≥ 200 ms, ease-out).
- [ ] Implement turn indicator UI showing current player color (Red / Black).

### Phase 7 — Game Completion & Reset
- [ ] Implement `GameOverModal` component: displays on checkmate/stalemate with winner announcement and "Play Again" button.
- [ ] Implement `ResetButton` component: resets `useGameState` to initial board.
- [ ] Wire up game-over detection in `useGameState`: after each move, check `isCheckmate` / `isStalemate`.

### Phase 8 — Polish & Integration
- [ ] Responsive layout: ensure board scales properly on common desktop viewports (1024 px–1920 px width).
- [ ] Add captured-pieces display area showing taken pieces grouped by side.
- [ ] Final visual review: shadows, textures, font rendering, animation smoothness.
- [ ] Cross-browser smoke test: Chrome, Firefox, Edge (latest stable).

## Test Tasks

### Engine Unit Tests
- [ ] `moveValidation.test.ts` — test move generation for every piece type on an empty board and with blocking pieces.
- [ ] `knightBlocking.test.ts` — verify 蹩马腿 blocks each of the 8 possible L-moves.
- [ ] `elephantBlocking.test.ts` — verify 塞象眼 and river-crossing prevention.
- [ ] `advisorGeneralPalace.test.ts` — verify palace boundary enforcement for advisor and general.
- [ ] `cannonCapture.test.ts` — verify cannon moves without capture (no screen), with capture (one screen), and failure case (two screens).
- [ ] `soldierRiverCrossing.test.ts` — verify soldier behavior before and after crossing the river.
- [ ] `flyingGeneral.test.ts` — verify that moves creating a direct general face-off are rejected.
- [ ] `checkDetection.test.ts` — verify check detection from multiple piece types.
- [ ] `checkmateDetection.test.ts` — test at least 2 known checkmate positions (e.g., "钓鱼马" and "双车错").
- [ ] `legalMoveFilter.test.ts` — verify that moves leaving own king in check are filtered out.

### Component Tests
- [ ] `Board.test.tsx` — board renders 90 squares + 32 pieces in initial position.
- [ ] `Piece.test.tsx` — clicking a piece triggers selection; visual selected state appears.
- [ ] `MoveHighlight.test.tsx` — after selecting a piece, legal-move indicators appear on correct squares.
- [ ] `GameOverModal.test.tsx` — modal appears on checkmate; "Play Again" resets the board.

### Integration Tests
- [ ] Play a full short game (e.g., 10-move scripted sequence) and verify final board state.
- [ ] Verify that illegal moves are silently rejected (no state change, no error).
