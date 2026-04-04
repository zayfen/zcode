// src/physics/PhysicsWorld.tsx
import React from 'react';
import { Physics } from '@react-three/cannon';
import {
  GRAVITY,
  BALL_BALL_FRICTION,
  BALL_BALL_RESTITUTION,
  BALL_FELT_FRICTION,
  BALL_FELT_RESTITUTION,
  BALL_CUSHION_FRICTION,
  BALL_CUSHION_RESTITUTION,
  SOLVER_ITERATIONS,
} from '../constants/physics';

interface Props {
  children: React.ReactNode;
}

export default function PhysicsWorld({ children }: Props) {
  return (
    <Physics
      gravity={[0, GRAVITY, 0]}
      iterations={SOLVER_ITERATIONS}
      tolerance={0.0001}
      defaultContactMaterial={{
        friction: BALL_BALL_FRICTION,
        restitution: BALL_BALL_RESTITUTION,
        contactEquationStiffness: 1e8,
        contactEquationRelaxation: 3,
      }}
    >
      {children}
    </Physics>
  );
}
