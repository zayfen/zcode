// src/App.tsx
import React from 'react';
import Scene from './components/Scene';
import GameInner from './components/GameInner';
import GameUI from './components/GameUI';
import PowerMeter from './components/PowerMeter';

export default function App() {
  return (
    <div style={{ width: '100vw', height: '100vh', position: 'relative' }}>
      <Scene>
        <GameInner />
      </Scene>
      <GameUI />
      <PowerMeter />
    </div>
  );
}
