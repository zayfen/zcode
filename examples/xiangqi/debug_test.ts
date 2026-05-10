import { createInitialBoardState } from './src/engine/constants';
import { makeMove, getLegalMoves } from './src/engine/index';

let state = createInitialBoardState();

// Move 1: Red chariot (9,0) → (8,0)
let result = makeMove(state, { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } });
console.log('Move 1 valid:', result.valid);
state = result.newState!;

// Move 2: Black chariot (0,8) → (1,8)
result = makeMove(state, { from: { row: 0, col: 8 }, to: { row: 1, col: 8 } });
console.log('Move 2 valid:', result.valid);
state = result.newState!;

// Move 3: Red chariot (8,0) → (8,1)
result = makeMove(state, { from: { row: 8, col: 0 }, to: { row: 8, col: 1 } });
console.log('Move 3 valid:', result.valid);
state = result.newState!;

// Move 4: Black chariot (1,8) → (4,8) 
console.log('Column 8 contents:');
for (let r = 0; r < 10; r++) {
  const p = state.board[r]?.[8];
  if (p) console.log(`  row ${r}: ${p.player} ${p.type}`);
}

const legal4 = getLegalMoves(state.board, { row: 1, col: 8 });
console.log('Black chariot (1,8) legal moves:', legal4.map(p => `${p.row},${p.col}`));

result = makeMove(state, { from: { row: 1, col: 8 }, to: { row: 4, col: 8 } });
console.log('Move 4 valid:', result.valid);
if (result.valid) {
  state = result.newState!;
} else {
  console.log('Move 4 FAILED - cannot continue');
  process.exit(1);
}

// Move 5: Red chariot (8,1) → (4,1)
console.log('\nColumn 1 contents:');
for (let r = 0; r < 10; r++) {
  const p = state.board[r]?.[1];
  if (p) console.log(`  row ${r}: ${p.player} ${p.type}`);
}

const legal5 = getLegalMoves(state.board, { row: 8, col: 1 });
console.log('Red chariot (8,1) legal moves:', legal5.map(p => `${p.row},${p.col}`));

result = makeMove(state, { from: { row: 8, col: 1 }, to: { row: 4, col: 1 } });
console.log('Move 5 valid:', result.valid);
if (!result.valid) {
  console.log('Move 5 FAILED!');
  
  // Check general positions
  console.log('General positions:');
  for (let r = 0; r < 10; r++) {
    for (let c = 0; c < 9; c++) {
      const p = state.board[r]?.[c];
      if (p && p.type === 'general') {
        console.log(`  ${p.player} general at (${r},${c})`);
      }
    }
  }
  
  // Test applying the move manually
  const { applyMove, isFlyingGeneral, isInCheck } = await import('./src/engine/specialRules');
  const testBoard = applyMove(state.board, { from: { row: 8, col: 1 }, to: { row: 4, col: 1 } });
  console.log('Flying general after move 5:', isFlyingGeneral(testBoard));
  console.log('Red in check after move 5:', isInCheck(testBoard, 'red'));
  
  // Print column 4 after move
  console.log('Column 4 contents after move:');
  for (let r = 0; r < 10; r++) {
    const p = testBoard[r]?.[4];
    if (p) console.log(`  row ${r}: ${p.player} ${p.type}`);
  }
  
  // Column 8 after move
  console.log('Column 8 contents:');
  for (let r = 0; r < 10; r++) {
    const p = testBoard[r]?.[8];
    if (p) console.log(`  row ${r}: ${p.player} ${p.type}`);
  }
}
