// Debug script to test which moves in the fullGame sequence are valid
import { makeMove, createInitialBoardState, getLegalMoves } from './src/engine/index';

const moves = [
  // Move 1 — Red left chariot forward 1
  { from: { row: 9, col: 0 }, to: { row: 8, col: 0 }, label: 'Move 1: Red left chariot forward 1' },
  // Move 2 — Black left knight L-shape
  { from: { row: 0, col: 1 }, to: { row: 2, col: 2 }, label: 'Move 2: Black left knight L-shape' },
  // Move 3 — Red left cannon slides right 3
  { from: { row: 7, col: 1 }, to: { row: 7, col: 4 }, label: 'Move 3: Red left cannon slides right 3' },
  // Move 4 — Black right cannon slides left 3
  { from: { row: 2, col: 7 }, to: { row: 2, col: 4 }, label: 'Move 4: Black right cannon slides left 3' },
  // Move 5 — Red right knight L-shape
  { from: { row: 9, col: 7 }, to: { row: 7, col: 6 }, label: 'Move 5: Red right knight L-shape' },
  // Move 6 — Black right chariot forward 1
  { from: { row: 0, col: 8 }, to: { row: 1, col: 8 }, label: 'Move 6: Black right chariot forward 1' },
  // Move 7 — Red center soldier forward 1
  { from: { row: 6, col: 4 }, to: { row: 5, col: 4 }, label: 'Move 7: Red center soldier forward 1' },
  // Move 8 — Black center soldier forward 1
  { from: { row: 3, col: 4 }, to: { row: 4, col: 4 }, label: 'Move 8: Black center soldier forward 1' },
  // Move 9 — Red left chariot forward 3
  { from: { row: 8, col: 0 }, to: { row: 5, col: 0 }, label: 'Move 9: Red left chariot forward 3' },
  // Move 10 — Black right knight L-shape
  { from: { row: 0, col: 7 }, to: { row: 2, col: 6 }, label: 'Move 10: Black right knight L-shape' },
];

let state = createInitialBoardState();

for (let i = 0; i < moves.length; i++) {
  const { from, to, label } = moves[i];
  console.log(`\n--- ${label} ---`);
  console.log(`Current player: ${state.currentPlayer}`);
  
  const piece = state.board[from.row]?.[from.col];
  console.log(`Piece at (${from.row},${from.col}): ${piece ? `${piece.player} ${piece.type}` : 'null'}`);
  
  // Get legal moves for this piece
  const legal = getLegalMoves(state.board, from);
  console.log(`Legal moves from (${from.row},${from.col}): ${JSON.stringify(legal)}`);
  
  const result = makeMove(state, { from, to });
  console.log(`Move valid: ${result.valid}`);
  
  if (!result.valid) {
    console.log(`STOPPING: Move ${i + 1} is invalid!`);
    break;
  }
  
  state = result.newState!;
}
