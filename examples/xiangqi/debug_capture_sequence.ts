import { createInitialBoardState, makeMove, getLegalMoves } from './src/engine/index.js';

const state = createInitialBoardState();

// Move 1: Red chariot (9,0) → (8,0)
let r = makeMove(state, { from: { row: 9, col: 0 }, to: { row: 8, col: 0 } });
console.log("Move 1 valid:", r.valid, "captured:", r.captured);
const s1 = r.newState!;

// Move 2: Black chariot (0,8) → (1,8)
console.log("Piece at (0,8):", JSON.stringify(s1.board[0][8]));
r = makeMove(s1, { from: { row: 0, col: 8 }, to: { row: 1, col: 8 } });
console.log("Move 2 valid:", r.valid, "captured:", r.captured);
const s2 = r.newState!;

// Move 3: Red chariot (8,0) → (8,1)
console.log("Piece at (8,0):", JSON.stringify(s2.board[8][0]));
r = makeMove(s2, { from: { row: 8, col: 0 }, to: { row: 8, col: 1 } });
console.log("Move 3 valid:", r.valid, "captured:", r.captured);
const s3 = r.newState!;

// Move 4: Black chariot (1,8) → (4,8)
console.log("Piece at (1,8):", JSON.stringify(s3.board[1][8]));
r = makeMove(s3, { from: { row: 1, col: 8 }, to: { row: 4, col: 8 } });
console.log("Move 4 valid:", r.valid, "captured:", r.captured);
const s4 = r.newState!;

// Move 5: Red chariot (8,1) → (4,1)
console.log("Piece at (8,1):", JSON.stringify(s4.board[8][1]));
const legal5 = getLegalMoves(s4.board, { row: 8, col: 1 });
console.log("Red chariot legal moves from (8,1):", legal5.map(p => `(${p.row},${p.col})`).join(", "));
r = makeMove(s4, { from: { row: 8, col: 1 }, to: { row: 4, col: 1 } });
console.log("Move 5 valid:", r.valid, "captured:", r.captured);
const s5 = r.newState!;

// Move 6: Black chariot (4,8) → (4,1) — captures red chariot
console.log("Piece at (4,8):", JSON.stringify(s5.board[4][8]));
console.log("Piece at (4,1):", JSON.stringify(s5.board[4][1]));
const legal6 = getLegalMoves(s5.board, { row: 4, col: 8 });
console.log("Black chariot legal moves from (4,8):", legal6.map(p => `(${p.row},${p.col})`).join(", "));
r = makeMove(s5, { from: { row: 4, col: 8 }, to: { row: 4, col: 1 } });
console.log("Move 6 valid:", r.valid, "captured:", r.captured);
const s6 = r.newState!;

// Move 7: Red knight (9,1) → (7,2)
console.log("Piece at (9,1):", JSON.stringify(s6.board[9][1]));
r = makeMove(s6, { from: { row: 9, col: 1 }, to: { row: 7, col: 2 } });
console.log("Move 7 valid:", r.valid, "captured:", r.captured);
const s7 = r.newState!;

// Move 8: Black chariot (4,1) → (4,2) captures red knight
console.log("Piece at (4,1):", JSON.stringify(s7.board[4][1]));
console.log("Piece at (4,2):", JSON.stringify(s7.board[4][2]));
const legal8 = getLegalMoves(s7.board, { row: 4, col: 1 });
console.log("Black chariot legal moves from (4,1):", legal8.map(p => `(${p.row},${p.col})`).join(", "));
r = makeMove(s7, { from: { row: 4, col: 1 }, to: { row: 4, col: 2 } });
console.log("Move 8 valid:", r.valid, "captured:", r.captured);
