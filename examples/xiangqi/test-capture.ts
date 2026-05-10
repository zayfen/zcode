import { makeMove, createInitialBoardState, getLegalMoves } from './src/engine/index.ts';

const state = createInitialBoardState();

// Simulate the capture sequence from the failing test

// Move 1: Red chariot (9,0) → (8,0)
const s1 = makeMove(state, { from: {row:9,col:0}, to: {row:8,col:0}});
console.log('Move 1 valid:', s1.valid, 'player:', s1.newState?.currentPlayer);

// Move 2: Black chariot (0,8) → (1,8)
const s2 = makeMove(s1.newState!, { from: {row:0,col:8}, to: {row:1,col:8}});
console.log('Move 2 valid:', s2.valid, 'player:', s2.newState?.currentPlayer);

// Move 3: Red chariot (8,0) → (7,0)
const s3 = makeMove(s2.newState!, { from: {row:8,col:0}, to: {row:7,col:0}});
console.log('Move 3 valid:', s3.valid, 'player:', s3.newState?.currentPlayer);

// Move 4: Black chariot (1,8) → (1,0) (move along row 1)
// Check legal moves first
const legal4 = getLegalMoves(s3.newState!.board, {row:1, col:8});
const hasMove10 = legal4.some(m => m.row === 1 && m.col === 0);
console.log('Move 4 legal moves for (1,8):', legal4.map(m => `${m.row},${m.col}`));
console.log('Has (1,0):', hasMove10);

const s4 = makeMove(s3.newState!, { from: {row:1,col:8}, to: {row:1,col:0}});
console.log('Move 4 valid:', s4.valid, 'player:', s4.newState?.currentPlayer);

// Move 5: Red chariot (9,8) → (8,8)
const s5 = makeMove(s4.newState!, { from: {row:9,col:8}, to: {row:8,col:8}});
console.log('Move 5 valid:', s5.valid, 'player:', s5.newState?.currentPlayer);

// Move 6: Black chariot (1,0) → (7,0) — captures red chariot
// Check legal moves first
const legal6 = getLegalMoves(s5.newState!.board, {row:1, col:0});
const hasMove70 = legal6.some(m => m.row === 7 && m.col === 0);
console.log('Move 6 legal moves for (1,0):', legal6.map(m => `${m.row},${m.col}`));
console.log('Has (7,0):', hasMove70);

// Check column 0 contents
console.log('Column 0 contents in s5:');
for (let r = 0; r <= 9; r++) {
  const p = s5.newState!.board[r]?.[0];
  if (p) console.log(`  (${r},0): ${p.type} ${p.player}`);
}

const s6 = makeMove(s5.newState!, { from: {row:1,col:0}, to: {row:7,col:0}});
console.log('Move 6 valid:', s6.valid, 'player:', s6.newState?.currentPlayer);
console.log('Captured:', s6.captured);
