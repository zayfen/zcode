import { createInitialBoardState, makeMove, getLegalMoves } from './src/engine';

// Reproduce steps 1-3
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

console.log('s3.currentPlayer:', s3.currentPlayer);

// Try makeMove for step 4
const m4 = makeMove(s3, { from: { row: 1, col: 8 }, to: { row: 4, col: 8 } });
console.log('Step 4 valid:', m4.valid);

// Let's check what makeMove sees
const piece = s3.board[1]?.[8];
console.log('Piece at (1,8):', piece);
console.log('Piece player:', piece?.player);
console.log('Current player:', s3.currentPlayer);
console.log('Match:', piece?.player === s3.currentPlayer);

// Check legal moves
const legal = getLegalMoves(s3.board, { row: 1, col: 8 });
console.log('Legal moves from (1,8):', legal.map(p => `${p.row},${p.col}`));
const hasTarget = legal.some(p => p.row === 4 && p.col === 8);
console.log('Has (4,8):', hasTarget);
