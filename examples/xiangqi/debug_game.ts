import { createInitialBoardState, getLegalMoves, makeMove, getGameStatus } from './src/engine/index';
import type { Board, BoardState, Move, Piece, PieceType, Player } from './src/engine/types';

function printBoard(board: Board): void {
  const chars: Record<string, string> = {
    'red-general': '帥', 'red-advisor': '仕', 'red-elephant': '相',
    'red-knight': '馬', 'red-chariot': '車', 'red-cannon': '炮', 'red-soldier': '兵',
    'black-general': '將', 'black-advisor': '士', 'black-elephant': '象',
    'black-knight': '馬', 'black-chariot': '車', 'black-cannon': '砲', 'black-soldier': '卒',
  };
  for (let r = 0; r < 10; r++) {
    let line = '';
    for (let c = 0; c < 9; c++) {
      const p = board[r][c];
      if (p) {
        line += chars[`${p.player}-${p.type}`] + ' ';
      } else {
        line += '· ';
      }
    }
    console.log(`Row ${r}: ${line}`);
  }
}

const state = createInitialBoardState();
console.log('Initial board:');
printBoard(state.board);
console.log('Current player:', state.currentPlayer);

// Check first move: Red left chariot from (9,0) to (8,0)
console.log('\n--- Move 1: Red left chariot (9,0) -> (8,0) ---');
const legalMoves = getLegalMoves(state.board, { row: 9, col: 0 });
console.log('Legal moves for red chariot at (9,0):', JSON.stringify(legalMoves));

const move1: Move = { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } };
const result1 = makeMove(state, move1);
console.log('Move 1 result:', JSON.stringify(result1, null, 2));

if (!result1.valid) {
  console.log('\nDEBUGGING: Checking piece at (9,0):', state.board[9][0]);
  console.log('DEBUGGING: Checking piece at (8,0):', state.board[8][0]);
  
  // Try another first move
  console.log('\n--- Trying alternative: Red right chariot (9,8) -> (8,8) ---');
  const legalMoves2 = getLegalMoves(state.board, { row: 9, col: 8 });
  console.log('Legal moves for red chariot at (9,8):', JSON.stringify(legalMoves2));
  
  console.log('\n--- Trying: Red center soldier (6,4) -> (5,4) ---');
  const legalMoves3 = getLegalMoves(state.board, { row: 6, col: 4 });
  console.log('Legal moves for red soldier at (6,4):', JSON.stringify(legalMoves3));
  
  console.log('\n--- Trying: Red left cannon (7,1) -> (7,4) ---');
  const legalMoves4 = getLegalMoves(state.board, { row: 7, col: 1 });
  console.log('Legal moves for red cannon at (7,1):', JSON.stringify(legalMoves4));
  
  console.log('\n--- Trying: Red left knight (9,1) -> (7,0) ---');
  const legalMoves5 = getLegalMoves(state.board, { row: 9, col: 1 });
  console.log('Legal moves for red knight at (9,1):', JSON.stringify(legalMoves5));
}

// Try the full game with alternate moves
console.log('\n\n=== Trying a simple 10-move game ===');
let gameState = createInitialBoardState();

const moves: Array<{ from: { row: number; col: number }; to: { row: number; col: number }; label: string }> = [
  // Move 1 — Red center soldier forward 1
  { from: { row: 6, col: 4 }, to: { row: 5, col: 4 }, label: 'Red center soldier forward 1' },
  // Move 2 — Black center soldier forward 1
  { from: { row: 3, col: 4 }, to: { row: 4, col: 4 }, label: 'Black center soldier forward 1' },
  // Move 3 — Red right knight L-shape
  { from: { row: 9, col: 7 }, to: { row: 7, col: 6 }, label: 'Red right knight L-shape' },
  // Move 4 — Black left knight L-shape
  { from: { row: 0, col: 1 }, to: { row: 2, col: 2 }, label: 'Black left knight L-shape' },
  // Move 5 — Red left cannon slides right 3
  { from: { row: 7, col: 1 }, to: { row: 7, col: 4 }, label: 'Red left cannon slides right 3' },
  // Move 6 — Black right cannon slides left 3
  { from: { row: 2, col: 7 }, to: { row: 2, col: 4 }, label: 'Black right cannon slides left 3' },
  // Move 7 — Red left chariot forward 1
  { from: { row: 9, col: 0 }, to: { row: 8, col: 0 }, label: 'Red left chariot forward 1' },
  // Move 8 — Black right chariot forward 1
  { from: { row: 0, col: 8 }, to: { row: 1, col: 8 }, label: 'Black right chariot forward 1' },
  // Move 9 — Red left chariot forward 3
  { from: { row: 8, col: 0 }, to: { row: 5, col: 0 }, label: 'Red left chariot forward 3' },
  // Move 10 — Black right knight L-shape
  { from: { row: 0, col: 7 }, to: { row: 2, col: 6 }, label: 'Black right knight L-shape' },
];

for (let i = 0; i < moves.length; i++) {
  const { from, to, label } = moves[i];
  console.log(`\nMove ${i+1}: ${label}`);
  console.log(`  From (${from.row},${from.col}) to (${to.row},${to.col})`);
  console.log(`  Current player: ${gameState.currentPlayer}`);
  console.log(`  Piece at from: ${JSON.stringify(gameState.board[from.row][from.col])}`);
  
  const move: Move = { from, to };
  const result = makeMove(gameState, move);
  console.log(`  Result valid: ${result.valid}`);
  
  if (!result.valid) {
    // Debug
    const legal = getLegalMoves(gameState.board, from);
    console.log(`  Legal moves from this position: ${JSON.stringify(legal)}`);
    console.log(`  FAILED at move ${i+1}!`);
    break;
  }
  
  gameState = result.newState!;
}

console.log('\n\nFinal board:');
printBoard(gameState.board);
console.log('Current player:', gameState.currentPlayer);
const status = getGameStatus(gameState);
console.log('Game status:', JSON.stringify(status));
