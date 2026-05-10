import { createInitialBoardState, makeMove, getLegalMoves, isInCheck, isFlyingGeneral } from './src/engine';
import { applyMove } from './src/engine/specialRules';
import { getRawMoves } from './src/engine/moveValidation';

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

// Now try step 4: Black chariot (1,8) → (4,8)
console.log('\n--- Before Step 4 (s3 board, currentPlayer:', s3.currentPlayer, ') ---');
console.log('Piece at (1,8):', JSON.stringify(s3.board[1]?.[8]));

const raw4 = getRawMoves(s3.board, { row: 1, col: 8 });
console.log('Raw moves from (1,8):', raw4.map(p => `${p.row},${p.col}`));

const legal4 = getLegalMoves(s3.board, { row: 1, col: 8 });
console.log('Legal moves from (1,8):', legal4.map(p => `${p.row},${p.col}`));

// Try the move and check what happens
const testMove = { from: { row: 1, col: 8 }, to: { row: 4, col: 8 } };
const newBoard = applyMove(s3.board, testMove);
console.log('\nAfter hypothetical move (1,8)→(4,8):');
console.log('isFlyingGeneral(newBoard):', isFlyingGeneral(newBoard));
console.log('isInCheck(newBoard, black):', isInCheck(newBoard, 'black'));
console.log('isInCheck(newBoard, red):', isInCheck(newBoard, 'red'));

// Print all pieces on col 4
console.log('\nAll pieces on col 4 (generals column):');
for (let r = 0; r < 10; r++) {
  const p = newBoard[r]?.[4];
  console.log(`  row ${r}: ${p ? p.player + ' ' + p.type : 'empty'}`);
}

// Print col 8 too
console.log('\nAll pieces on col 8:');
for (let r = 0; r < 10; r++) {
  const p = newBoard[r]?.[8];
  console.log(`  row ${r}: ${p ? p.player + ' ' + p.type : 'empty'}`);
}

// Check each raw move individually
console.log('\n--- Checking each raw move for legality ---');
for (const to of raw4) {
  const testB = applyMove(s3.board, { from: { row: 1, col: 8 }, to });
  const fg = isFlyingGeneral(testB);
  const ic = isInCheck(testB, 'black');
  const legal = !fg && !ic;
  console.log(`  (1,8)→(${to.row},${to.col}): flyingGeneral=${fg}, inCheck=${ic}, legal=${legal}`);
}
