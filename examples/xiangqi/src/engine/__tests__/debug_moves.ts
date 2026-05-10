import { createInitialBoardState, getLegalMoves, makeMove } from '../index.js';

let state = createInitialBoardState();
const moves = [
  {from:{row:9,col:0},to:{row:8,col:0}},
  {from:{row:0,col:1},to:{row:2,col:2}},
  {from:{row:7,col:1},to:{row:7,col:4}},
  {from:{row:2,col:7},to:{row:2,col:4}},
  {from:{row:9,col:7},to:{row:7,col:6}},
  {from:{row:0,col:8},to:{row:1,col:8}},
  {from:{row:6,col:4},to:{row:5,col:4}},
  {from:{row:3,col:4},to:{row:4,col:4}},
];
for (const m of moves) {
  const r = makeMove(state, m);
  state = r.newState;
}

for (let r = 0; r < 10; r++) {
  for (let c = 0; c < 9; c++) {
    const p = state.board[r][c];
    if (p && p.player === 'red') {
      const legal = getLegalMoves(state.board, {row:r,col:c});
      if (legal.length > 0) {
        console.log(`Red ${p.type} at (${r},${c}) -> ${JSON.stringify(legal)}`);
      }
    }
  }
}
