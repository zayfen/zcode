import { create } from 'zustand'
import * as THREE from 'three'

interface AimState {
  direction: THREE.Vector3
  power: number
  isCharging: boolean
  setDirection: (dir: THREE.Vector3) => void
  setPower: (power: number) => void
  setIsCharging: (charging: boolean) => void
  reset: () => void
}

export const useAimStore = create<AimState>((set) => ({
  direction: new THREE.Vector3(0, 0, -1),
  power: 0,
  isCharging: false,
  setDirection: (dir) => set({ direction: dir }),
  setPower: (power) => set({ power: Math.min(1, Math.max(0, power)) }),
  setIsCharging: (charging) => set({ isCharging: charging }),
  reset: () => set({ direction: new THREE.Vector3(0, 0, -1), power: 0, isCharging: false }),
}))
