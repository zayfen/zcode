// src/physics/pocketDetection.ts
import { POCKET_POSITIONS, POCKET_RADIUS } from '../constants/table';
import type { BallId } from '../types';

export function detectPockets(
  ballPositions: { [key: number]: [number, number, number] },
  pocketedBalls: BallId[]
): BallId[] {
  const newlyPocketed: BallId[] = [];

  for (const [idStr, pos] of Object.entries(ballPositions)) {
    const id = Number(idStr) as BallId;
    if (pocketedBalls.includes(id)) continue;
    if (newlyPocketed.includes(id)) continue;

    for (const pocketPos of POCKET_POSITIONS) {
      const dx = pos[0] - pocketPos[0];
      const dz = pos[2] - pocketPos[2];
      const dist = Math.sqrt(dx * dx + dz * dz);
      if (dist < POCKET_RADIUS) {
        newlyPocketed.push(id);
        break;
      }
    }
  }

  return newlyPocketed;
}
