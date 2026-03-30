import Scene from './components/Scene';
import GameUI from './components/GameUI';

/**
 * Root component for the 3D 8-Ball Pool Game.
 *
 * Architecture:
 *   App
 *   ├── Scene (R3F Canvas)
 *   │   ├── PhysicsWorld
 *   │   ├── Table
 *   │   ├── Balls (16x BallBody + Ball mesh)
 *   │   ├── CueStick
 *   │   └── AimLine
 *   └── GameUI (HTML overlay)
 */
function App() {
  return (
    <div
      tabIndex={0}
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        width: '100%',
        height: '100%',
        background: '#1a1a2e',
        overflow: 'hidden',
        outline: 'none',
      }}
    >
      <Scene />
      <GameUI />
    </div>
  );
}

export default App;
