import { useAimStore } from '../store/aimStore';
import { useGameStore } from '../store/gameStore';

export default function PowerMeter() {
  const power = useAimStore((s) => s.power);
  const phase = useGameStore((s) => s.phase);

  if (phase !== 'POWER') return null;

  // Color gradient: green -> yellow -> red
  let color = '#22c55e';
  if (power > 0.66) color = '#ef4444';
  else if (power > 0.33) color = '#eab308';

  return (
    <div
      style={{
        position: 'fixed',
        bottom: '40px',
        left: '50%',
        transform: 'translateX(-50%)',
        width: '300px',
        height: '20px',
        background: 'rgba(0, 0, 0, 0.5)',
        borderRadius: '10px',
        overflow: 'hidden',
        border: '2px solid rgba(255, 255, 255, 0.3)',
      }}
    >
      <div
        style={{
          width: `${power * 100}%`,
          height: '100%',
          background: color,
          borderRadius: '10px',
          transition: 'width 0.05s linear',
        }}
      />
      <span style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%,-50%)', color: 'white', fontWeight: 'bold', fontSize: '12px' }}>{Math.round(power * 100)}%</span>
    </div>
  );
}
