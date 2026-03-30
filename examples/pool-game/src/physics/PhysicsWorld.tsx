import { Physics } from '@react-three/cannon';
import type { ReactNode } from 'react';
import {
  GRAVITY,
  BALL_FELT_FRICTION,
  BALL_FELT_RESTITUTION,
  SOLVER_ITERATIONS,
} from '../constants/physics';

interface PhysicsWorldProps {
  children: ReactNode;
}

export default function PhysicsWorld({ children }: PhysicsWorldProps) {
  return (
    <Physics
      gravity={GRAVITY}
      iterations={SOLVER_ITERATIONS}
      defaultContactMaterial={{
        friction: BALL_FELT_FRICTION,
        restitution: BALL_FELT_RESTITUTION,
      }}
    >
      {children}
    </Physics>
  );
}
