/**
 * Physics constants for the 8-ball pool simulation.
 *
 * T05 — Physics tuning parameters used by cannon-es / custom solver.
 */
import type { Vec3Tuple } from '../types';

// ---------------------------------------------------------------------------
// Gravity
// ---------------------------------------------------------------------------
export const GRAVITY: Vec3Tuple = [0, -9.81, 0];

// ---------------------------------------------------------------------------
// Ball properties
// ---------------------------------------------------------------------------
/** Standard pool ball mass in kilograms */
export const BALL_MASS = 0.17;

// ---------------------------------------------------------------------------
// Contact materials – friction & restitution pairs
// ---------------------------------------------------------------------------

/** Ball vs ball */
export const BALL_BALL_FRICTION = 0.05;
export const BALL_BALL_RESTITUTION = 0.95;

/** Ball vs felt (table bed) */
export const BALL_FELT_FRICTION = 0.4;
export const BALL_FELT_RESTITUTION = 0.1;

/** Ball vs cushion (rail) */
export const BALL_CUSHION_FRICTION = 0.1;
export const BALL_CUSHION_RESTITUTION = 0.7;

// ---------------------------------------------------------------------------
// Damping (applied per physics step)
// ---------------------------------------------------------------------------
export const LINEAR_DAMPING = 0.3;
export const ANGULAR_DAMPING = 0.5;

// ---------------------------------------------------------------------------
// Settle detection — when all balls have come to rest
// ---------------------------------------------------------------------------
/** Linear velocity magnitude below which a ball is considered "still" (m/s) */
export const SETTLE_LINEAR_THRESHOLD = 0.001;
/** Angular velocity magnitude below which a ball is considered "still" (rad/s) */
export const SETTLE_ANGULAR_THRESHOLD = 0.01;
/** Consecutive frames below threshold required to declare settled */
export const SETTLE_FRAMES = 10;

// ---------------------------------------------------------------------------
// Shot / impulse limits
// ---------------------------------------------------------------------------
/** Maximum impulse applied for a full-power shot (N·s) */
export const MAX_IMPULSE = 8;

// ---------------------------------------------------------------------------
// Solver quality
// ---------------------------------------------------------------------------
/** Number of constraint-solver iterations per physics step */
export const SOLVER_ITERATIONS = 10;

// ---------------------------------------------------------------------------
// Power / charge
// ---------------------------------------------------------------------------
/** Rate at which power charges from 0→1 (per second). Full charge in ~2s. */
export const POWER_CHARGE_RATE = 0.5;
