# Validation: 3D 8-Ball Pool Game

## Quality Gates

The following gates must pass before the pool game is considered complete.

### Gate 1: Build & Launch

| Check | Command | Expected |
|-------|---------|----------|
| Production build | `npm run build` | Completes with exit code 0, no errors |
| Dev server | `npm run dev` | Starts without errors |
| Console errors | Open in browser, check DevTools | Zero errors or warnings |
| Bundle size | Check `dist/` output | Under 2MB total (gzipped) |

### Gate 2: Rendering

| Check | Method | Expected |
|-------|--------|----------|
| All 16 balls render | Visual inspection | Cue ball + 15 numbered balls visible on table |
| Ball positions | Visual inspection | Triangle rack at foot spot; cue ball at head spot |
| Ball colors | Visual inspection | Each ball has correct color per standard billiards |
| Ball numbers | Visual inspection | Numbers visible and correctly placed on each ball |
| Table renders | Visual inspection | Green felt, brown rails, 6 pockets, cushions visible |
| Lighting | Visual inspection | Even illumination, no harsh shadows or dark areas |

### Gate 3: Physics

| Check | Method | Expected |
|-------|--------|----------|
| Ball collisions | Shoot cue ball into rack | Balls scatter with realistic elastic collisions |
| Friction deceleration | Roll a ball and watch | Balls decelerate and come to rest naturally |
| Cushion rebounds | Shoot ball at cushion | Ball bounces off at correct reflection angle |
| Pocket detection | Shoot ball into pocket | Ball disappears when entering pocket zone |
| No tunneling | Shoot ball at high power | Balls never pass through each other or cushions |
| Settle detection | After shot | System correctly detects when all balls stop |
| Physics stability | 60-second idle | Balls at rest remain at rest (no jitter) |

### Gate 4: Aiming & Controls

| Check | Method | Expected |
|-------|--------|----------|
| Aim line | Move mouse | Dashed line follows mouse direction from cue ball |
| Cue stick | Move mouse | Cue stick rotates to follow aim direction |
| Power meter | Hold mouse button | Bar fills from 0% to 100% over ~2 seconds |
| Shoot | Release mouse button | Cue ball receives impulse in aim direction |
| Power scaling | Shoot at different powers | Higher power = greater ball speed |
| Camera orbit | Click and drag | Camera orbits around table |
| Top-down view | Press T key | Camera switches to top-down orthographic view |
| Ball-in-hand | After foul | Click on table to place cue ball |

### Gate 5: Game Logic

| Check | Method | Expected |
|-------|--------|----------|
| Turn alternation | Complete a shot | Turn passes to other player |
| Group assignment | First player pockets a solid | That player assigned solids, other gets stripes |
| Foul: scratch | Pocket cue ball | Foul called, opponent gets ball-in-hand |
| Foul: wrong ball | Hit stripe when assigned solids | Foul called, opponent gets ball-in-hand |
| Foul: no contact | Miss all balls | Foul called, opponent gets ball-in-hand |
| Win condition | Pocket 8-ball after clearing group | Player wins, game over modal shown |
| Loss: early 8-ball | Pocket 8-ball before clearing group | Player loses, game over modal shown |
| Loss: scratch on 8 | Scratch while shooting 8-ball | Player loses |
| Play again | Click "Play Again" | Game resets to initial state |
| Undo | Press U key | Previous shot state restored |

### Gate 6: Performance

| Check | Method | Expected |
|-------|--------|----------|
| Frame rate | Chrome DevTools Performance tab | Consistent 60fps during gameplay |
| Memory | Chrome DevTools Memory tab | No memory leaks over 50 shots |
| Physics step | DevTools console timing | Under 2ms per physics step |

### Gate 7: Cross-Browser

| Browser | Status |
|---------|--------|
| Chrome (latest) | Must pass all gates |
| Firefox (latest) | Must pass Gates 1–5 |
| Safari (latest) | Must pass Gates 1–5 |

## Validation Procedure

1. **Automated checks**: Run `npm run build` — must pass
2. **Smoke test**: `npm run dev`, open in browser, click through a basic shot
3. **Full playthrough**: Complete an entire game from break to win/loss
4. **Edge case testing**: Trigger each foul type, pocket 8-ball early, scratch on break
5. **Performance profiling**: Open DevTools, play 10 shots, check frame timing

## Validation Commands

```bash
# Gate 1: Build
cd examples/pool-game
npm install
npm run build

# Gate 1: Launch
npm run dev
# Open http://localhost:5173 in browser

# Gate 6: Performance
# Open Chrome DevTools → Performance → Record → Play 10 shots → Check frame rate
```
