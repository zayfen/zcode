# Coding Spec

## Tech Stack
- **Language**: TypeScript (strict mode)
- **UI Framework**: React 18+ with functional components and hooks
- **Styling**: CSS3 with CSS Modules (or Tailwind CSS) for board aesthetics, textures, and shadows
- **Rendering**: SVG-based board and pieces (preferred for crisp scaling + CSS animation support), with optional Canvas fallback
- **Build Tool**: Vite
- **Package Manager**: npm or pnpm
- **Testing**: Vitest (unit) + React Testing Library (component)

## File Structure
```
src/
├── engine/                  # Pure-logic Xiangqi rules engine (no UI dependency)
│   ├── types.ts             # Piece, Position, Board, Player, Move types
│   ├── constants.ts         # Initial board layout, palace bounds, river row
│   ├── moveValidation.ts    # Per-piece legal-move generators
│   ├── checkDetection.ts    # Check / checkmate / stalemate detection
│   ├── specialRules.ts      # Flying-general rule, king-in-check after move filter
│   └── index.ts             # Public API: createGame(), getLegalMoves(), makeMove()
├── components/              # React UI components
│   ├── Board.tsx             # Main board grid renderer
│   ├── Piece.tsx             # Individual piece with 3D wooden styling
│   ├── MoveHighlight.tsx     # Legal-move dot/overlay on target squares
│   ├── TurnIndicator.tsx     # Shows current player (Red/Black)
│   ├── GameOverModal.tsx     # Checkmate / draw announcement
│   └── ResetButton.tsx       # New Game trigger
├── hooks/
│   ├── useGameState.ts       # Core game state: board, turn, selected piece, history
│   └── useAnimation.ts       # Piece-move animation controller
├── styles/
│   ├── board.css             # Board grid, river, textures, background
│   ├── pieces.css            # Piece appearance: gradients, shadows, fonts
│   └── animations.css        # Keyframes for piece movement transitions
├── App.tsx                   # Root component assembling Board + UI chrome
└── main.tsx                  # Entry point (Vite bootstrap)
```

## Conventions

### Naming
- Files: `camelCase.ts` / `camelCase.tsx`.
- React components: `PascalCase` named exports.
- Types / Interfaces: `PascalCase`, prefixed with descriptive nouns (e.g., `BoardState`, `MoveResult`).
- Functions: `camelCase`; boolean-returning functions prefixed with `is`/`has`/`can` (e.g., `isInCheck()`, `canMove()`).

### Engine Purity
- The `engine/` directory must contain **zero** React or DOM imports. It is a pure TypeScript library.
- All engine functions are stateless: they accept a `BoardState` and return new data without mutation.
- `makeMove()` returns a new `BoardState` rather than mutating the old one (immutable pattern).

### Error Handling
- Engine functions throw descriptive errors only for programmer errors (e.g., invalid piece type).
- User-facing illegal moves are handled by returning an empty legal-move set or a `{ valid: false }` result — never by throwing.
- UI components use optional chaining and null guards for rendering safety.

### Testing
- Every exported function in `engine/` must have a corresponding unit test file: `engine/__tests__/moveValidation.test.ts`, etc.
- Tests must cover:
  - All piece move patterns (straight, diagonal, L-shape).
  - Blocking rules (蹩马腿, 塞象眼).
  - River constraints (象 not crossing, 兵 behavior change).
  - Palace constraints (士, 将/帅).
  - Cannon capture mechanics (exactly one screen).
  - Flying-general detection.
  - Check and checkmate scenarios (at least 2 endgame positions).
- React component tests verify: piece click → highlight appears; move click → piece animates; game-over modal appears.

### Styling
- Board background uses a CSS gradient or SVG pattern simulating wood grain.
- Pieces use `box-shadow` (or SVG filters) for a raised/3D appearance and Chinese calligraphy font for characters.
- All colors, sizes, and fonts are defined as CSS custom properties for easy theming.
- Animations use CSS `transition` (preferred) or `requestAnimationFrame` for move animation; duration ≥ 200 ms, `ease-out`.

### Code Organization
- One React component per file.
- Max file length: 250 lines (excluding tests). If exceeded, extract sub-components or utility functions.
- No circular imports: `engine` ← `hooks` ← `components` ← `App`. Never reverse.
