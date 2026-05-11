# Review Checklist

## Code Quality
- [ ] No `any` types in TypeScript (strict mode enforced)
- [ ] No `// @ts-ignore` or `// @ts-expect-error` without a linked TODO ticket
- [ ] No unused imports or variables
- [ ] No console.log in production code (debug-only is acceptable behind a flag)
- [ ] Functions are concise and single-purpose (≤ 50 lines)
- [ ] No magic numbers — board dimensions, durations, and offsets are named constants

## Engine Purity
- [ ] `engine/` has zero React or DOM imports
- [ ] All engine functions are pure (no side effects, no mutation of input state)
- [ ] `makeMove()` returns a new `BoardState` — input state is never mutated
- [ ] No circular dependencies: `engine` ← `hooks` ← `components` ← `App`

## Architecture
- [ ] Follows the file structure defined in `docs/specs/coding.spec.md`
- [ ] Move validation is fully encapsulated in `engine/moveValidation.ts`
- [ ] Check / checkmate detection is in `engine/checkDetection.ts`
- [ ] Game state management is isolated in `hooks/useGameState.ts`
- [ ] No business logic leaked into React components (components only render + delegate)

## Testing
- [ ] Every exported engine function has a corresponding unit test
- [ ] All piece move types have dedicated test coverage (车, 马, 象, 士, 将/帅, 炮, 兵/卒)
- [ ] Special rules tested: 蹩马腿, 塞象眼, river crossing, palace bounds, flying general
- [ ] At least 2 checkmate positions are tested
- [ ] Component tests cover: piece selection, move highlight, move execution, game-over modal
- [ ] `vitest run` — 0 failures

## Styling & Visuals
- [ ] Board has wood-grain texture background (CSS gradient or SVG)
- [ ] Pieces have 3D wooden appearance (radial gradient + box-shadow)
- [ ] Chinese characters rendered with appropriate calligraphy font
- [ ] Move animation duration ≥ 200 ms with ease-out timing
- [ ] Legal-move highlights are clearly visible and do not obscure pieces
- [ ] Turn indicator is unambiguous (Red / Black)

## Accessibility & UX
- [ ] All interactive elements are keyboard-navigable (tab + enter)
- [ ] Piece elements have `aria-label` describing the piece (e.g., "Red Chariot at row 0 col 0")
- [ ] Game-over announcement is accessible (aria-live region or modal focus trap)
- [ ] Color is not the sole indicator of side (pieces also labeled with characters)

## Acceptance Validation
- [ ] All PRD Acceptance Criteria in `docs/prd/001-feature.md` are met
- [ ] Manual smoke test: play a complete game end-to-end
- [ ] No regression on previous features (if applicable)
