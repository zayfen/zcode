import { StrictMode } from 'react'
import Scene from './components/Scene'
import Table from './components/Table'
import Balls from './components/Balls'
import CueStick from './components/CueStick'
import AimLine from './components/AimLine'
import GameUI from './components/GameUI'
import GameController from './components/GameController'

export default function App() {
  return (
    <>
      <Scene>
        <Table />
        <Balls />
        <CueStick />
        <AimLine />
        <GameController />
      </Scene>
      <GameUI />
    </>
  )
}
