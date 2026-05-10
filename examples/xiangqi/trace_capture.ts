import { createInitialBoardState, getLegalMoves, makeMove } from './src/engine';

function printBoard(board: ReturnType<typeof createInitialBoardState>["board"], label: string) {
  console.log(`\n=== ${label} ===`);
  for (let r = 0; r < 10; r++) {
    const row: string[] = [];
    for (let c = 0; c < 9; c++) {
      const p = board[r]?.[c];
      if (!p) row.push('·');
      else row.push(p.player === 'red' ? 'R' : 'B');
    }
    console.log(`Row ${r}: ${row.join(' ')}`);
  }
}

const state = createInitialBoardState();

// Move 1: Red chariot (9,0) → (8,0)
let result = makeMove(state, { from: {row:9,col:0}, to: {row:8,col:0} });
console.log("Move 1 valid:", result.valid);

// Move 2: Black chariot (0,8) → (1,8)
result = makeMove(result.newState!, { from: {row:0,col:8}, to: {row:1,col:8} });
console.log("Move 2 valid:", result.valid);

// Move 3: Red chariot (8,0) → (8,1)
result = makeMove(result.newState!, { from: {row:8,col:0}, to: {row:8,col:1} });
console.log("Move 3 valid:", result.valid);

printBoard(result.newState!.board, "After move 3");

// Check legal moves for black chariot at (1,8)
const legal4 = getLegalMoves(result.newState!.board, {row:1,col:8});
console.log("\nLegal moves for black chariot at (1,8):", legal4.map(m => `${m.row},${m.col}`));

// Try Move 4: Black chariot (1,8) → (4,8)
let result4 = makeMove(result.newState!, { from: {row:1,col:8}, to: {row:4,col:8} });
console.log("Move 4 (1,8)→(4,8) valid:", result4.valid);

if (!result4.valid) {
  // Check raw moves
  // Try step by step
  result4 = makeMove(result.newState!, { from: {row:1,col:8}, to: {row:2,col:8} });
  console.log("Move 4a (1,8)→(2,8) valid:", result4.valid);
  result4 = makeMove(result.newState!, { from: {row:1,col:8}, to: {row:3,col:8} });
  console.log("Move 4b (1,8)→(3,8) valid:", result4.valid);
}
