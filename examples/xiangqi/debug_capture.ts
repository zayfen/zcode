import { createInitialBoardState, getLegalMoves, makeMove } from './src/engine';

let state = createInitialBoardState();
let r;

// 1. Red soldier (6,4) → (5,4)
r = makeMove(state, { from: { row: 6, col: 4 }, to: { row: 5, col: 4 } });
state = r.newState!;

// 2. Black soldier (3,4) → (4,4)
r = makeMove(state, { from: { row: 3, col: 4 }, to: { row: 4, col: 4 } });
state = r.newState!;

// Check soldier at (4,4) legal moves
console.log('Piece at (4,4):', state.board[4][4]);
console.log('Piece at (5,4):', state.board[5][4]);
const legal44 = getLegalMoves(state.board, { row: 4, col: 4 });
console.log('Legal moves for black soldier at (4,4):', JSON.stringify(legal44));
// Black soldier hasn't crossed the river, so can only go forward to (5,4)
// But (5,4) has a red soldier - that should be a capture move!

// Let's check raw soldier move logic
// Black soldier at row 4. Forward = +1 (toward red). Next row = 5.
// RIVER_ROW_MAX = 5. hasCrossedRiver for black: row >= RIVER_ROW_MAX => 4 >= 5 => false
// So no sideways moves. But forward to (5,4) should still be valid since it's an enemy piece.
// Wait, let me check the actual soldier logic more carefully...
console.log('\n--- Debug soldier moves ---');
console.log('For black soldier at (4,4):');
console.log('  forward = 1 (black moves toward higher rows)');
console.log('  fwdRow = 5');
console.log('  target at (5,4):', state.board[5][4]);
console.log('  target.player:', state.board[5][4]?.player, 'vs current player: black');
console.log('  target.player !== player:', state.board[5][4]?.player !== 'black');

// The soldier move code:
// const target = getPiece(board, { row: fwdRow, col: from.col });
// if (target === null || target.player !== player) {
//   moves.push({ row: fwdRow, col: from.col });
// }
// So it should add (5,4) since target.player ('red') !== player ('black')

// But it's RED's turn now, not black's!
console.log('\nCurrent player:', state.currentPlayer);
// After move 2 (black), it's RED's turn. So we can't move black soldier.

// Let's do it properly:
// 3. Red move (anything)
r = makeMove(state, { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } });
console.log('3. Red chariot forward:', r.valid);
state = r.newState!;

// 4. Black soldier (4,4) → (5,4) -- captures red soldier
r = makeMove(state, { from: { row: 4, col: 4 }, to: { row: 5, col: 4 } });
console.log('4. Black soldier captures red soldier:', r.valid, 'captured:', r.captured);
if (r.valid) {
  state = r.newState!;
  console.log('Piece at (5,4):', state.board[5][4]);
  console.log('SUCCESS!');
}
