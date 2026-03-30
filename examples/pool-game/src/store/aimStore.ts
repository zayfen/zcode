import { create } from 'zustand';
import type { Vec3Tuple } from '../types';

// ---------------------------------------------------------------------------
// Aim store — Phase 6 (T17)
//
// Tracks the player's aim direction, shot power, charging state, and camera
// mode.  Updated every frame by the useAim hook (mouse movement) and the
// usePower hook (charging animation).
// ---------------------------------------------------------------------------

interface AimStoreState {
  /** Normalised aim direction on the XZ plane */
  direction: Vec3Tuple;
  /** Shot power clamped to [0, 1] */
  power: number;
  /** True while the player is holding the mouse button to charge power */
  isCharging: boolean;
  /** Camera mode toggle */
  cameraMode: 'orbit' | 'topdown';
}

interface AimStoreActions {
  setDirection: (dir: Vec3Tuple) => void;
  setPower: (power: number) => void;
  /** Alias: set isCharging to true */
  setIsCharging: (charging: boolean) => void;
  /** Begin charging from 0 */
  startCharging: () => void;
  /** Stop charging (power stays at current value) */
  stopCharging: () => void;
  /** Reset power and charging for next shot */
  resetPower: () => void;
  toggleCameraMode: () => void;
  setCameraMode: (mode: 'orbit' | 'topdown') => void;
}

export const useAimStore = create<AimStoreState & AimStoreActions>((set) => ({
  direction: [0, 0, 1],
  power: 0,
  isCharging: false,
  cameraMode: 'orbit',

  setDirection: (dir: Vec3Tuple) => set({ direction: dir }),
  setPower: (power: number) => set({ power: Math.min(1, Math.max(0, power)) }),
  setIsCharging: (charging: boolean) => set({ isCharging: charging }),
  startCharging: () => set({ isCharging: true, power: 0 }),
  stopCharging: () => set({ isCharging: false }),
  resetPower: () => set({ power: 0, isCharging: false }),
  toggleCameraMode: () =>
    set((s) => ({ cameraMode: s.cameraMode === 'orbit' ? 'topdown' : 'orbit' })),
  setCameraMode: (mode) => set({ cameraMode: mode }),
}));
