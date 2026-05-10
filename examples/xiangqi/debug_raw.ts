// Debug: test rawMoves for chariot at (8,0) after 8 moves
import { makeMove, createInitialBoardState, getLegalMoves } from './src/engine/index';
import { getRawMoves } from './src/engine/moveValidation';
import type { BoardState } from './src/engine/types';

const moves: Array<{ from: { row: number; col: number }; to: { row: number; col: number } }> = [
  { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } },
  { from: { row: 0, col: 1 }, to: { row: 2, col: 2 } },
  { from: { row: 7, col: 1 }, to: { row: 7, col: 4 } },
  { from: { row: 2, col: 7 }, to: { row: 2, col: 4 } },
  { from: { row: 9, col: 7 }, to: { row: 7, col: 6 } },
  { from: { row: 0, col: 8 }, to: { row: 1, col: 8 } },
  { from: { row: 6, col: 4 }, to: { row: 5, col: 4 } },
  { from: { row: 3, col: 2 }, to: { row: 4, col: 2 } },
];

let state: BoardState = createInitialBoardState();
for (const m of moves) {
  const result = makeMove(state, m);
  if (!result.valid) { console.log('Failed early!'); process.exit(1); }
  state = result.newState!;
}

const from = { row: 8, col: 0 };
console.log('Raw moves for chariot at (8,0):');
const raw = getRawMoves(state.board, from);
console.log(raw.map(p => `(${p.row},${p.col})`).join(', '));

console.log('\nLegal moves for chariot at (8,0):');
const legal = getLegalMoves(state.board, from);
console.log(legal.map(p => `(${p.row},${p.col})`).join(', '));

// What piece is at each position along column 0?
console.log('\nColumn 0 contents (row 0-9):');
for (let r = 0; r < 10; r++) {
  const p = state.board[r]?.[0];
  console.log(`  Row ${r}: ${p ? `${p.player} ${p.type}` : 'empty'}`);
}
