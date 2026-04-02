import type { BallId, Vec3Tuple } from '../types';
import { POCKET_POSITIONS, POCKET_RADIUS } from '../constants/table';
import { distanceSq } from '../utils/vector';

/** Pre-computed squared pocket radius threshold to avoid repeated multiplication */
const POCKET_RADIUS_SQ = POCKET_RADIUS * POCKET_RADIUS;

/**
 * Check if any balls are within pocket zones
 * Returns list of ball IDs that should be pocketed
 */
export function detectPocketedBalls(
  ballPositions: Record<number, Vec3Tuple>,
  alreadyPocketed: Set<number>
): BallId[] {
  const pocketed: BallId[] = [];

  for (const [idStr, pos] of Object.entries(ballPositions)) {
    const id = Number(idStr) as BallId;
    if (alreadyPocketed.has(id)) continue;

    for (const pocketPos of POCKET_POSITIONS) {
      if (distanceSq(pos, pocketPos) < POCKET_RADIUS_SQ) {
        pocketed.push(id);
        break;
      }
    }
  }

  return pocketed;
}

/**
 * Check if a single position is within any pocket zone
 */
export function isInPocket(pos: Vec3Tuple): boolean {
  for (const pocketPos of POCKET_POSITIONS) {
    if (distanceSq(pos, pocketPos) < POCKET_RADIUS_SQ) return true;
  }
  return false;
}
