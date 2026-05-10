// Replicate the debug-capture-sequence test scenario to verify
import { createInitialBoardState, getLegalMoves, makeMove, getRawMoves } from './src/engine/index.ts';

const state = createInitialBoardState();

const m1 = makeMove(state, { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } });
console.log('Step 1 valid:', m1.valid);
const s1 = m1.newState!;

const m2 = makeMove(s1, { from: { row: 0, col: 8 }, to: { row: 1, col: 8 } });
console.log('Step 2 valid:', m2.valid);
const s2 = m2.newState!;

const m3 = makeMove(s2, { from: { row: 9, col: 1 }, to: { row: 7, col: 2 } });
console.log('Step 3 valid:', m3.valid);
const s3 = m3.newState!;

// Print column 8 contents after step 3
console.log('\nColumn 8 contents after step 3:');
for (let r = 0; r < 10; r++) {
  const p = s3.board[r][8];
  if (p) console.log(`  (${r},8): ${p.player} ${p.type}`);
}

// Check raw moves for chariot at (1,8)
const rawMoves = getRawMoves(s3.board, { row: 1, col: 8 });
console.log('\nRaw moves for chariot at (1,8):', rawMoves.map(m => `(${m.row},${m.col})`).join(', '));
console.log('(4,8) in raw moves?', rawMoves.some(m => m.row === 4 && m.col === 8));

// Check legal moves for chariot at (1,8)
const legalMoves = getLegalMoves(s3.board, { row: 1, col: 8 });
console.log('\nLegal moves for chariot at (1,8):', legalMoves.map(m => `(${m.row},${m.col})`).join(', '));
console.log('(4,8) in legal moves?', legalMoves.some(m => m.row === 4 && m.col === 8));

// Try step 4
const m4 = makeMove(s3, { from: { row: 1, col: 8 }, to: { row: 4, col: 8 } });
console.log('\nStep 4 valid:', m4.valid);
