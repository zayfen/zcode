/**
 * 8-Ball Pool Rules Engine — Phase 5 (T15)
 *
 * Evaluates shot results according to standard 8-ball rules:
 * - Foul detection (scratch, no ball hit, wrong ball first, no rail contact)
 * - Group assignment on first legal pocket
 * - Win/loss conditions (8-ball pocketed legally after clearing group = win,
 *   8-ball early or scratch on 8 = loss)
 */

import type { BallId, Player, BallGroup, FoulType, Vec3Tuple } from '../types';
import { getBallGroup, EIGHT, SOLIDS, STRIPES } from '../types';
import {
  PLAYING_BOUNDS,
  HEAD_STRING_Z,
} from '../constants/table';
// ─────────────────────────────────────────────────────────────────────────────

/** Complete result returned by the rules evaluation */
export interface EvaluationResult {
  /** Whether a foul was committed and what type */
  foul: FoulType;
  /** Which player takes the next turn */
  nextPlayer: Player;
  /** Whether the game should end */
  gameOver: boolean;
  /** Winner (only set when gameOver is true) */
  winner: Player | null;
  /** Group assignment (set on first legal pocket when groups not yet assigned) */
  assignGroup: { player: Player; group: BallGroup } | null;
  /** Whether the incoming player gets ball-in-hand */
  ballInHand: boolean;
  /** Whether the current player keeps their turn */
  playerContinues: boolean;
}

/** Input data for shot evaluation */
export interface ShotEvaluationInput {
  currentPlayer: Player;
  playerGroups: Record<Player, BallGroup>;
  pocketedThisShot: BallId[];
  firstContact: BallId | null;
  /** Whether any ball hit a cushion after first contact */
  railContact: boolean;
  /** All balls pocketed in the game so far (before this shot) */
  allPocketedBalls: BallId[];
  /** Whether this is the break shot */
  isBreak: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/** Get the opponent player number */
function opponentOf(player: Player): Player {
  return (player === 1 ? 2 : 1) as Player;
}

/**
 * Check whether every ball of the given group has been pocketed
 * (excluding the 8-ball, which is handled separately).
 */
function isGroupCleared(
  group: BallGroup,
  allPocketedBalls: BallId[]
): boolean {
  if (group === null) return false;

  const groupBalls = group === 'solids' ? SOLIDS : STRIPES;
  return groupBalls.every((id) => allPocketedBalls.includes(id));
}

/**
 * Determine whether a pocketed ball is "legal" for the given player:
 * - Before group assignment: any numbered ball (1-7, 9-15) is legal
 * - After group assignment: balls of the player's own group are legal
 */
export function isLegalPocket(
  ballId: BallId,
  playerGroup: BallGroup
): boolean {
  if (ballId === 0 || ballId === EIGHT) return false; // cue and 8-ball handled separately
  if (playerGroup === null) return true; // before assignment, any numbered ball is fine
  return getBallGroup(ballId) === playerGroup;
}

// ─────────────────────────────────────────────────────────────────────────────
// Main evaluation function
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Evaluate the result of a shot according to standard 8-ball rules.
 *
 * Foul types detected:
 * - SCRATCH: cue ball (0) pocketed
 * - NO_BALL_HIT: cue ball did not contact any other ball
 * - WRONG_BALL_FIRST: first ball contacted is not of the player's group
 *   (only applicable after groups are assigned)
 * - NO_RAIL_CONTACT: after legal contact, no ball (including cue) hit a rail
 *   cushion and no ball was pocketed
 *
 * Group assignment:
 * - On the first shot where a player legally pockets a ball (no foul) and
 *   groups have not yet been assigned, the player receives the group of that
 *   first legally pocketed ball.
 *
 * Win/loss:
 * - Win: player pockets the 8-ball legally after clearing all their group balls
 * - Loss: player pockets the 8-ball before clearing their group
 * - Loss: player scratches (pockets cue ball) on the same shot as pocketing 8-ball
 * - Loss: player pockets the 8-ball on the break (some rule variants allow this;
 *   we treat it as a re-rack by not ending the game)
 */
export function evaluateShotResult(input: ShotEvaluationInput): EvaluationResult {
  const {
    currentPlayer,
    playerGroups,
    pocketedThisShot,
    firstContact,
    railContact,
    allPocketedBalls,
    isBreak,
  } = input;

  const myGroup = playerGroups[currentPlayer];
  const opponent: Player = opponentOf(currentPlayer);

  // ── Track foul ──────────────────────────────────────────────────────────
  let foul: FoulType = null;
  let ballInHand = false;

  // 1. SCRATCH: cue ball pocketed
  const cueBallPocketed = pocketedThisShot.includes(0 as BallId);

  if (cueBallPocketed) {
    foul = 'SCRATCH';
    ballInHand = true;
  }

  // 2. NO_BALL_HIT: cue ball didn't contact any object ball
  if (!cueBallPocketed && firstContact === null) {
    foul = 'NO_BALL_HIT';
    ballInHand = true;
  }

  // 3. WRONG_BALL_FIRST: first contact is not the player's assigned group
  //    (only checked after groups are assigned and a ball was actually hit)
  if (
    !cueBallPocketed &&
    firstContact !== null &&
    myGroup !== null &&
    foul === null
  ) {
    const contactedGroup = getBallGroup(firstContact);
    // Contacting the 8-ball first is only legal when the player's group is cleared
    if (firstContact === EIGHT) {
      if (!isGroupCleared(myGroup, allPocketedBalls)) {
        foul = 'WRONG_BALL_FIRST';
        ballInHand = true;
      }
    } else if (contactedGroup !== myGroup) {
      foul = 'WRONG_BALL_FIRST';
      ballInHand = true;
    }
  }

  // 4. NO_RAIL_CONTACT: after legal contact, if no ball was pocketed and
  //    no ball (including the cue ball) contacted a cushion, it's a foul.
  //    On the break, at least 4 balls must hit a rail (simplified: we
  //    check railContact for normal shots).
  if (
    foul === null &&
    firstContact !== null &&
    !cueBallPocketed &&
    pocketedThisShot.length === 0 &&
    !railContact
  ) {
    foul = 'NO_RAIL_CONTACT';
    ballInHand = true;
  }

  // ── 8-ball pocketed? ───────────────────────────────────────────────────
  const eightBallPocketed = pocketedThisShot.includes(EIGHT);

  if (eightBallPocketed) {
    // On the break, pocketing the 8-ball does NOT end the game in our variant.
    // The breaker may choose to re-rack or spot the 8-ball.
    // For simplicity, we treat break 8-ball pocket as a re-rack signal.
    if (isBreak) {
      // Don't end the game; treat as special case
      return {
        foul: foul,
        nextPlayer: foul ? opponent : currentPlayer,
        gameOver: false,
        winner: null,
        assignGroup: null,
        ballInHand: foul !== null,
        playerContinues: false,
      };
    }

    // Determine effective group (may be assigned this shot)
    const effectiveGroup = myGroup; // groups are assigned AFTER evaluation for this case

    // Win condition: all group balls cleared AND no foul AND 8-ball pocketed
    if (
      foul === null &&
      effectiveGroup !== null &&
      isGroupCleared(effectiveGroup, allPocketedBalls)
    ) {
      return {
        foul: null,
        nextPlayer: currentPlayer,
        gameOver: true,
        winner: currentPlayer,
        assignGroup: null,
        ballInHand: false,
        playerContinues: false,
      };
    }

    // Loss: 8-ball pocketed with a foul, or before clearing group
    return {
      foul: foul ?? 'EIGHT_EARLY',
      nextPlayer: opponent,
      gameOver: true,
      winner: opponent,
      assignGroup: null,
      ballInHand: false,
      playerContinues: false,
    };
  }

  // ── Group assignment (first legal pocket when groups not assigned) ──────
  let assignGroup: { player: Player; group: BallGroup } | null = null;

  if (myGroup === null && foul === null) {
    // Find the first non-cue, non-8 ball pocketed this shot
    const legalPockets = pocketedThisShot.filter(
      (id) => id !== 0 && id !== EIGHT
    );

    if (legalPockets.length > 0) {
      const firstPocketed = legalPockets[0];
      const group = getBallGroup(firstPocketed);
      if (group !== null) {
        assignGroup = { player: currentPlayer, group };
      }
    }
  }

  // ── Determine if player continues ──────────────────────────────────────
  // Player continues if:
  // - No foul was committed
  // - At least one ball of their group was legally pocketed (or any numbered
  //   ball before group assignment)
  let playerContinues = false;

  if (foul === null && pocketedThisShot.length > 0) {
    // Filter out cue ball (0) — already handled by foul check
    const nonCuePockets = pocketedThisShot.filter((id) => id !== 0);

    if (myGroup === null) {
      // Before assignment: any numbered ball pocketed = continue
      playerContinues = nonCuePockets.length > 0;
    } else {
      // After assignment: must pocket at least one of own group
      playerContinues = nonCuePockets.some(
        (id) => getBallGroup(id) === myGroup
      );
    }
  }

  // ── Build result ───────────────────────────────────────────────────────
  return {
    foul,
    nextPlayer: playerContinues ? currentPlayer : opponent,
    gameOver: false,
    winner: null,
    assignGroup,
    ballInHand,
    playerContinues,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience wrapper (backward-compatible with existing evaluateShot signature)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Evaluate a shot using positional parameters.
 *
 * @deprecated Prefer {@link evaluateShotResult} with the full input object.
 */
export function evaluateShot(
  currentPlayer: Player,
  playerGroups: Record<Player, BallGroup>,
  pocketedThisShot: BallId[],
  firstContact: BallId | null,
  _cueBallPosition: Vec3Tuple,
  isBreak: boolean,
  allPocketedBalls?: BallId[],
  railContact?: boolean
): EvaluationResult {
  return evaluateShotResult({
    currentPlayer,
    playerGroups,
    pocketedThisShot,
    firstContact,
    railContact: railContact ?? true, // assume rail contact if not tracked
    allPocketedBalls: allPocketedBalls ?? pocketedThisShot,
    isBreak,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Table bounds utilities
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Check if a position is within the playable table bounds.
 * Used for ball-in-hand placement validation.
 */
export function isWithinBounds(pos: Vec3Tuple): boolean {
  return (
    pos[0] > PLAYING_BOUNDS.minX &&
    pos[0] < PLAYING_BOUNDS.maxX &&
    pos[2] > PLAYING_BOUNDS.minZ &&
    pos[2] < PLAYING_BOUNDS.maxZ
  );
}

/**
 * Check if a position is behind the head string (for break placement).
 */
export function isBehindHeadString(pos: Vec3Tuple): boolean {
  return pos[2] < HEAD_STRING_Z;
}

/**
 * Validate a ball-in-hand placement position.
 * Returns the clamped position within table bounds, or null if invalid.
 */
export function validateBallPlacement(
  pos: Vec3Tuple,
  existingBallPositions: Record<number, Vec3Tuple>,
  pocketedBallIds: Set<number>
): Vec3Tuple | null {
  // Must be within playing bounds
  if (!isWithinBounds(pos)) return null;

  // Must not overlap with any other active ball
  const MIN_DISTANCE = 0.057; // 2 * BALL_RADIUS
  for (const [idStr, ballPos] of Object.entries(existingBallPositions)) {
    const id = Number(idStr);
    if (pocketedBallIds.has(id)) continue; // skip pocketed balls
    if (id === 0) continue; // skip self

    const dx = pos[0] - ballPos[0];
    const dz = pos[2] - ballPos[2];
    const dist = Math.sqrt(dx * dx + dz * dz);
    if (dist < MIN_DISTANCE) return null;
  }

  return [
    Math.max(PLAYING_BOUNDS.minX, Math.min(PLAYING_BOUNDS.maxX, pos[0])),
    pos[1],
    Math.max(PLAYING_BOUNDS.minZ, Math.min(PLAYING_BOUNDS.maxZ, pos[2])),
  ];
}
