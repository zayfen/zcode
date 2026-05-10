// Debug script: test new 10-move sequence
import { makeMove, createInitialBoardState, getLegalMoves } from './src/engine/index';
import type { BoardState, Move } from './src/engine/types';

// New 10-move sequence avoiding flying general conflict
const moves: Array<{ from: { row: number; col: number }; to: { row: number; col: number }; label: string }> = [
  { from: { row: 9, col: 0 }, to: { row: 8, col: 0 }, label: 'Red left chariot forward 1' },
  { from: { row: 0, col: 1 }, to: { row: 2, col: 2 }, label: 'Black left knight forward' },
  { from: { row: 7, col: 1 }, to: { row: 7, col: 4 }, label: 'Red left cannon slides right 3' },
  { from: { row: 2, col: 7 }, to: { row: 2, col: 4 }, label: 'Black right cannon slides left 3' },
  { from: { row: 9, col: 7 }, to: { row: 7, col: 6 }, label: 'Red right knight forward' },
  { from: { row: 0, col: 8 }, to: { row: 1, col: 8 }, label: 'Black right chariot forward 1' },
  { from: { row: 6, col: 4 }, to: { row: 5, col: 4 }, label: 'Red center soldier forward 1' },
  { from: { row: 3, col: 2 }, to: { row: 4, col: 2 }, label: 'Black soldier(3,2) forward 1' },
  { from: { row: 8, col: 0 }, to: { row: 5, col: 0 }, label: 'Red left chariot forward 3' },
  { from: { row: 0, col: 7 }, to: { row: 2, col: 6 }, label: 'Black right knight forward' },
];

const expectedPlayer = ['red', 'black', 'red', 'black', 'red', 'black', 'red', 'black', 'red', 'black'];

let state: BoardState = createInitialBoardState();
console.log(`Initial player: ${state.currentPlayer}`);
let allPassed = true;

for (let i = 0; i < moves.length; i++) {
  const { from, to, label } = moves[i]!;
  const movingPlayer = expectedPlayer[i]!;
  const move: Move = { from, to };

  console.log(`\n--- Move ${i + 1}: ${label} (${movingPlayer}) ---`);
  console.log(`  Current player: ${state.currentPlayer}`);

  const piece = state.board[from.row]?.[from.col];
  console.log(`  Piece at (${from.row},${from.col}): ${piece ? `${piece.player} ${piece.type}` : 'null'}`);

  if (piece) {
    const legal = getLegalMoves(state.board, from);
    console.log(`  Legal moves from (${from.row},${from.col}): [${legal.map(p => `(${p.row},${p.col})`).join(', ')}]`);
    const isInLegal = legal.some(p => p.row === to.row && p.col === to.col);
    console.log(`  Target (${to.row},${to.col}) in legal moves? ${isInLegal}`);
  }

  const result = makeMove(state, move);
  console.log(`  Result: valid=${result.valid}`);

  if (!result.valid) {
    console.log(`  *** MOVE FAILED ***`);
    allPassed = false;
    break;
  }

  state = result.newState!;
}

if (allPassed) {
  console.log('\n\nAll 10 moves passed successfully!');
  console.log(`Final current player: ${state.currentPlayer}`);

  // Print the board
  console.log('\nFinal board:');
  for (let r = 0; r < 10; r++) {
    const row: string[] = [];
    for (let c = 0; c < 9; c++) {
      const p = state.board[r]?.[c];
      if (!p) row.push('·');
      else {
        const chars: Record<string, string> = {
          'red-general': '帅', 'red-advisor': '仕', 'red-elephant': '相',
          'red-knight': '马', 'red-chariot': '车', 'red-cannon': '炮', 'red-soldier': '兵',
          'black-general': '将', 'black-advisor': '士', 'black-elephant': '象',
          'black-knight': '馬', 'black-chariot': '車', 'black-cannon': '砲', 'black-soldier': '卒',
        };
        row.push(chars[`${p.player}-${p.type}`] || '?');
      }
    }
    console.log(`  Row ${r}: ${row.join(' ')}`);
  }
}
