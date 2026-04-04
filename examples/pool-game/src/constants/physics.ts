// Physics constants for the pool game

export const BALL_MASS = 0.17; // kg (standard billiard ball)

// Contact materials
export const BALL_BALL_FRICTION = 0.05;
export const BALL_BALL_RESTITUTION = 0.95;

export const BALL_FELT_FRICTION = 0.4;
export const BALL_FELT_RESTITUTION = 0.1;

export const BALL_CUSHION_FRICTION = 0.1;
export const BALL_CUSHION_RESTITUTION = 0.7;

// Damping
export const LINEAR_DAMPING = 0.4;
export const ANGULAR_DAMPING = 0.4;

// Gravity
export const GRAVITY: [number, number, number] = [0, -9.81, 0];

// Physics solver
export const SOLVER_ITERATIONS = 10;
export const BROADPHASE = 'SAPBroadphase' as const;

// Settle detection thresholds
export const SETTLE_LINEAR_THRESHOLD = 0.001;
export const SETTLE_ANGULAR_THRESHOLD = 0.01;
export const SETTLE_FRAMES = 10; // Consecutive frames below threshold

// Shot
export const MAX_IMPULSE = 7; // Maximum impulse force
export const POWER_CHARGE_DURATION = 2; // Seconds to reach full power

// Ball
export const BALL_SLEEP_SPEED_LIMIT = 0.1;
export const BALL_SLEEP_TIME_LIMIT = 1;
