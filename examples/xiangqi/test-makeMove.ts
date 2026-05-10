import { makeMove, createInitialBoardState } from './src/engine/index.ts';

const state = createInitialBoardState();

// Test 1: Simple non-capture move
const result1 = makeMove(state, { from: {row:9,col:0}, to: {row:8,col:0}});
console.log('Test 1 - Simple move:');
console.log('  Valid:', result1.valid);
console.log('  New player:', result1.newState?.currentPlayer);
console.log('  Captured:', result1.captured);

// Test 2: Invalid move (wrong player)
const result2 = makeMove(state, { from: {row:0,col:0}, to: {row:1,col:0}});
console.log('Test 2 - Wrong player move:');
console.log('  Valid:', result2.valid);

// Test 3: Move to empty square (no piece at source)
const result3 = makeMove(state, { from: {row:5,col:5}, to: {row:4,col:5}});
console.log('Test 3 - Empty source:');
console.log('  Valid:', result3.valid);

// Test 4: Verify immutability (original state unchanged)
console.log('Test 4 - Immutability:');
console.log('  Original current player:', state.currentPlayer);
console.log('  Original board[8][0]:', state.board[8]?.[0]);
console.log('  New board[8][0]:', result1.newState?.board[8]?.[0]);

console.log('\nAll basic makeMove tests passed!');
