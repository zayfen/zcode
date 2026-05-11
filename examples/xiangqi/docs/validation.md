# Validation

## Quality Gates
- [ ] `npm run build` (or `npx tsc --noEmit`) — 0 TypeScript errors
- [ ] `npx vitest run` — 0 test failures
- [ ] `npx eslint src/` — 0 errors (warnings acceptable if documented)
- [ ] `npx prettier --check "src/**/*.{ts,tsx,css}"` — clean formatting

## Acceptance Validation
- [ ] All PRD Acceptance Criteria in `docs/prd/001-feature.md` are satisfied
- [ ] Manual smoke test completed:
  - [ ] Board renders correctly with all 32 pieces in starting position
  - [ ] Clicking a piece highlights all legal moves
  - [ ] Moving a piece produces smooth animation
  - [ ] Illegal moves are prevented (no state change)
  - [ ] Turn alternates correctly between Red and Black
  - [ ] Check is detected and indicated
  - [ ] Checkmate ends the game with winner announcement
  - [ ] "New Game" resets the board
- [ ] Cross-browser check: passes on Chrome, Firefox, and Edge (latest stable)
- [ ] No console errors or warnings during normal gameplay
