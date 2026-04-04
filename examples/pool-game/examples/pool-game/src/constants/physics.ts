// src/constants/physics.ts
// Physics constants for the pool game simulation

export const GRAVITY = -9.81;

export const BALL_MASS = 0.17; // kg

// Friction coefficients
export const BALL_BALL_FRICTION = 0.05;
export const BALL_FELT_FRICTION = 0.4;
export const BALL_CUSHION_FRICTION = 0.1;

// Restitution (bounciness)
export const BALL_BALL_RESTITUTION = 0.95;
export const BALL_FELT_RESTITUTION = 0.1;
export const BALL_CUSHION_RESTITUTION = 0.7;

// Damping
export const LINEAR_DAMPING = 0.4;
export const ANGULAR_DAMPING = 0.4;

// Simulation
export const SOLVER_ITERATIONS = 10;
export const SETTLE_LINEAR_THRESHOLD = 0.001;
export const SETTLE_ANGULAR_THRESHOLD = 0.01;
export const SETTLE_DEBOUNCE_FRAMES = 10;

// Shot
export const MAX_IMPULSE = 8;
export const POWER_CHARGE_DURATION = 2000; // ms to reach full power

// Physics step
export const PHYSICS_STEP = 1 / 60;
export const PHYSICS_MAX_SUB_STEPS = 3;
