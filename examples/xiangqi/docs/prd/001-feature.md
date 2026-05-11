# Feature: 中国象棋 (Xiangqi) Web Game

## Goals
- Build a high-quality, visually polished web-based Chinese Chess (Xiangqi) game with realistic wooden-piece aesthetics and a modernized ancient-Chinese visual style.
- Implement the complete, rule-accurate Xiangqi move/validation engine covering all piece types and special rules.
- Provide a smooth, interactive two-player hot-seat (same-device) experience with legal-move highlighting and animated piece movement.
- Maintain a clean, modular codebase with well-separated concerns (rules engine, board rendering, game state).

## Non-Goals
- AI opponent / computer player (out of scope for initial release).
- Online multiplayer or network play.
- Game history replay / PGN export.
- Sound effects or background music.
- Mobile-first responsive layout (desktop-first; mobile is a future enhancement).
- Server-side persistence or user accounts.

## User Stories
- As a player, I want to see a beautiful chessboard with wooden textures and three-dimensional piece styling so that the game feels immersive.
- As a player, I want to click a piece and immediately see all its legal destination squares highlighted so that I can make informed moves.
- As a player, I want pieces to animate smoothly when moved so the transition feels natural and polished.
- As a player, I want to play a complete game of Xiangqi against a friend on the same device, with the game enforcing all official rules (including special rules like "flying general" / 白脸将).
- As a player, I want clear visual feedback for whose turn it is, captured pieces, and game-over state (checkmate / stalemate).
- As a player, I want the game to prevent illegal moves entirely rather than allowing and then undoing them.

## Acceptance Criteria
- [ ] The board renders a 9×10 Xiangqi grid with a "river" (楚河汉界) clearly depicted.
- [ ] All seven piece types per side (将/帅, 士, 象, 马, 车, 炮, 兵/卒) are rendered with Chinese characters, 3D wooden styling, and shadows.
- [ ] Clicking a piece highlights every legal move for that piece given the current board state (including blocking checks).
- [ ] Piece movement is animated with a CSS/Canvas transition (≥ 200 ms, ease-out).
- [ ] Complete move validation implemented for every piece type:
  - [ ] 车 (Chariot): moves in straight lines, blocked by intervening pieces.
  - [ ] 马 (Knight): moves in an L-shape (日); blocked when the orthogonal adjacent square is occupied (蹩马腿).
  - [ ] 象/相 (Elephant): moves diagonally two squares (田); cannot cross the river; blocked when the eye (田心) is occupied (塞象眼).
  - [ ] 士/仕 (Advisor): moves one step diagonally within the palace (九宫).
  - [ ] 将/帅 (General): moves one step orthogonally within the palace.
  - [ ] 炮 (Cannon): moves like a chariot but captures by jumping over exactly one intervening piece (炮架).
  - [ ] 兵/卒 (Soldier): moves one step forward before crossing the river; after crossing, can also move one step horizontally.
- [ ] The "flying general" rule (将帅不能照面) is enforced: the two generals may never face each other on the same file with no pieces between them.
- [ ] The game detects check (将军) and prevents any move that leaves the moving side's general in check.
- [ ] The game detects checkmate (将杀) and declares the opponent the winner.
- [ ] A turn indicator clearly shows which side (Red / Black) is to move.
- [ ] A "New Game" / "Reset" button restarts the board to the initial position.
- [ ] The codebase is separated into at minimum: Rules engine module, Board/Rendering module, Game State controller.
