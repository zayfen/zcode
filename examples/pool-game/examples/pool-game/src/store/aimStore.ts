// src/store/aimStore.ts
import { create } from 'zustand';
import type { Vec3 } from '../types';

interface AimState {
  direction: Vec3;
  power: number;
  isCharging: boolean;
  cameraMode: 'orbit' | 'topdown';
  setDirection: (dir: Vec3) => void;
  setPower: (power: number) => void;
  setIsCharging: (charging: boolean) => void;
  toggleCamera: () => void;
  resetShot: () => void;
}

export const useAimStore = create<AimState>((set) => ({
  direction: { x: 0, y: 0, z: -1 },
  power: 0,
  isCharging: false,
  cameraMode: 'orbit',

  setDirection: (dir) => set({ direction: dir }),
  setPower: (power) => set({ power: Math.min(1, Math.max(0, power)) }),
  setIsCharging: (charging) => set({ isCharging: charging }),
  toggleCamera: () =>
    set((s) => ({
      cameraMode: s.cameraMode === 'orbit' ? 'topdown' : 'orbit',
    })),
  resetShot: () => set({ power: 0, isCharging: false }),
}));
