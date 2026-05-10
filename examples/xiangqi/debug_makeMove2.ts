import { createInitialBoardState, makeMove, getLegalMoves } from './src/engine';
import { getRawMoves } from './src/engine/moveValidation';

// Reproduce steps 1-3
const state = createInitialBoardState();
const m1 = makeMove(state, { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } });
const s1 = m1.newState!;

const m2 = makeMove(s1, { from: { row: 0, col: 8 }, to: { row: 1, col: 8 } });
const s2 = m2.newState!;

const m3 = makeMove(s2, { from: { row: 9, col: 1 }, to: { row: 7, col: 2 } });
const s3 = m3.newState!;

console.log('s3.currentPlayer:', s3.currentPlayer);

// Check raw moves - we know (4,8) is NOT in raw moves
const raw = getRawMoves(s3.board, { row: 1, col: 8 });
console.log('Raw moves from (1,8):', raw.map(p => `${p.row},${p.col}`));
console.log('Has (4,8) in raw:', raw.some(p => p.row === 4 && p.col === 8));

// Check what's blocking on column 8 between row 1 and row 4
console.log('\nColumn 8 contents in s3:');
for (let r = 0; r < 10; r++) {
  const p = s3.board[r]?.[8];
  console.log(`  row ${r}: ${p ? p.player + ' ' + p.type : 'empty'}`);
}

// The chariot is at (1,8). Moving down: (2,8), (3,8), (4,8)
// Check each:
console.log('\nChecking path from (1,8) downward:');
console.log('(2,8):', JSON.stringify(s3.board[2]?.[8]));
console.log('(3,8):', JSON.stringify(s3.board[3]?.[8]));
console.log('(4,8):', JSON.stringify(s3.board[4]?.[8]));
console.log('(5,8):', JSON.stringify(s3.board[5]?.[8]));
console.log('(6,8):', JSON.stringify(s3.board[6]?.[8]));
