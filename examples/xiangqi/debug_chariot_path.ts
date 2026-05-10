// Debug script to test chariot path from (8,0)
import { createInitialBoardState, getLegalMoves, makeMove } from './src/engine/index';

let state = createInitialBoardState();

// Replay moves 1-8 to reach the position
const moves = [
  { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } },
  { from: { row: 0, col: 1 }, to: { row: 2, col: 2 } },
  { from: { row: 7, col: 1 }, to: { row: 7, col: 4 } },
  { from: { row: 2, col: 7 }, to: { row: 2, col: 4 } },
  { from: { row: 9, col: 7 }, to: { row: 7, col: 6 } },
  { from: { row: 0, col: 8 }, to: { row: 1, col: 8 } },
  { from: { row: 6, col: 4 }, to: { row: 5, col: 4 } },
  { from: { row: 3, col: 4 }, to: { row: 4, col: 4 } },
];

for (const m of moves) {
  const result = makeMove(state, m);
  if (!result.valid) {
    console.log(`Move ${JSON.stringify(m)} is invalid!`);
    process.exit(1);
  }
  state = result.newState!;
}

// Print the board state
console.log("Board after 8 moves:");
for (let r = 0; r < 10; r++) {
  let line = `Row ${r}: `;
  for (let c = 0; c < 9; c++) {
    const p = state.board[r][c];
    if (p) {
      line += `${p.player[0]}-${p.type.padEnd(8)} `;
    } else {
      line += "·--------- ";
    }
  }
  console.log(line);
}

// Check what's at (8,0) and what's in the way
console.log("\nPiece at (8,0):", state.board[8][0]);
console.log("Piece at (7,0):", state.board[7][0]);
console.log("Piece at (6,0):", state.board[6][0]);
console.log("Piece at (5,0):", state.board[5][0]);
console.log("Piece at (4,0):", state.board[4][0]);

// Try legal moves from (8,0)
const legal = getLegalMoves(state.board, { row: 8, col: 0 });
console.log("\nLegal moves from (8,0):", JSON.stringify(legal));

// Try legal moves from (8,0) moving downward (decreasing row = "forward" for red)
// Wait, row 0 = black back rank, row 9 = red back rank
// So for red chariot at (8,0), forward = toward row 0 = decreasing row
// Legal: (7,0), (9,0) - these are row 7 and row 9
// But NOT (6,0) or (5,0) - something must be blocking

// Let's check if chariot can move to (7,0)
console.log("\nTrying to move chariot (8,0) -> (7,0):");
let result1 = makeMove(state, { from: { row: 8, col: 0 }, to: { row: 7, col: 0 } });
console.log("Valid:", result1.valid);

// Check column 0 pieces
console.log("\nColumn 0 contents:");
for (let r = 0; r < 10; r++) {
  const p = state.board[r][0];
  console.log(`  (${r},0): ${p ? `${p.player} ${p.type}` : 'empty'}`);
}
