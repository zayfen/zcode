// Debug: understand why red chariot at (8,0) can't go to (5,0)
import { makeMove, createInitialBoardState, getLegalMoves, isInCheck, isFlyingGeneral } from './src/engine/index';
import type { BoardState, Move, Board } from './src/engine/types';

function applyMove(board: Board, move: Move): Board {
  const newBoard: (import('./src/engine/types').Piece | null)[][] = board.map((row) => [...row]);
  const piece = newBoard[move.from.row]?.[move.from.col];
  newBoard[move.from.row]![move.from.col] = null;
  newBoard[move.to.row]![move.to.col] = piece ?? null;
  return newBoard;
}

// Reproduce state after 8 moves
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
  if (!result.valid) {
    console.log('Failed early!');
    process.exit(1);
  }
  state = result.newState!;
}

console.log('State after 8 moves:');
console.log('Current player:', state.currentPlayer);

// Print the board
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

// Check flying general status
console.log('\nFlying general?', isFlyingGeneral(state.board));

// Simulate chariot moving to (5,0) manually
const testMove: Move = { from: { row: 8, col: 0 }, to: { row: 5, col: 0 } };
const testBoard = applyMove(state.board, testMove);

console.log('\nAfter hypothetical move (8,0)->(5,0):');
for (let r = 0; r < 10; r++) {
  const row: string[] = [];
  for (let c = 0; c < 9; c++) {
    const p = testBoard[r]?.[c];
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

console.log('\nFlying general after move?', isFlyingGeneral(testBoard));
console.log('Red in check after move?', isInCheck(testBoard, 'red'));

// Check what's on column 4 between rows 0-9
console.log('\nColumn 4 contents (from top to bottom):');
for (let r = 0; r < 10; r++) {
  const p = state.board[r]?.[4];
  console.log(`  Row ${r}: ${p ? `${p.player} ${p.type}` : 'empty'}`);
}
