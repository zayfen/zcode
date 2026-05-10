// Try different alternatives for move 9 and 10
import { createInitialBoardState, getLegalMoves, makeMove, getGameStatus } from './src/engine/index';

let state = createInitialBoardState();

// Moves 1-8 (same as before)
const moves1to8 = [
  { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } },
  { from: { row: 0, col: 1 }, to: { row: 2, col: 2 } },
  { from: { row: 7, col: 1 }, to: { row: 7, col: 4 } },
  { from: { row: 2, col: 7 }, to: { row: 2, col: 4 } },
  { from: { row: 9, col: 7 }, to: { row: 7, col: 6 } },
  { from: { row: 0, col: 8 }, to: { row: 1, col: 8 } },
  { from: { row: 6, col: 4 }, to: { row: 5, col: 4 } },
  { from: { row: 3, col: 4 }, to: { row: 4, col: 4 } },
];

for (const m of moves1to8) {
  const result = makeMove(state, m);
  state = result.newState!;
}

// Move 9: Red left chariot forward 1 (only option)
let result9 = makeMove(state, { from: { row: 8, col: 0 }, to: { row: 7, col: 0 } });
console.log("Move 9 (chariot 8,0 -> 7,0): valid =", result9.valid);
state = result9.newState!;

// Move 10 options for black
console.log("\nBlack pieces and their legal moves:");
for (let r = 0; r < 10; r++) {
  for (let c = 0; c < 9; c++) {
    const p = state.board[r][c];
    if (p && p.player === 'black') {
      const legal = getLegalMoves(state.board, { row: r, col: c });
      if (legal.length > 0) {
        console.log(`  ${p.type} at (${r},${c}): ${JSON.stringify(legal)}`);
      }
    }
  }
}

// Try move 10: Black right knight L-shape
let result10 = makeMove(state, { from: { row: 0, col: 7 }, to: { row: 2, col: 6 } });
console.log("\nMove 10 (knight 0,7 -> 2,6): valid =", result10.valid);

if (result10.valid) {
  state = result10.newState!;
  
  console.log("\nFinal board:");
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
  
  console.log("\nCurrent player:", state.currentPlayer);
  console.log("Game status:", getGameStatus(state));
  
  // Count pieces
  let count = 0;
  for (const row of state.board) {
    for (const cell of row) {
      if (cell) count++;
    }
  }
  console.log("Total pieces:", count);
}
