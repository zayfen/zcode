import { createInitialBoardState, makeMove, getLegalMoves } from './src/engine/index.js';

const s = createInitialBoardState();
console.log('Initial player:', s.currentPlayer);
console.log('Board[0][8]:', JSON.stringify(s.board[0][8]));
console.log('Board[0][7]:', JSON.stringify(s.board[0][7]));

// Try black chariot move (should be invalid since it's red's turn)
const r0 = makeMove(s, {from:{row:0,col:8},to:{row:1,col:8}});
console.log('Black chariot 0,8->1,8 valid:', r0.valid, '(expected false)');

// Red chariot move
const r1 = makeMove(s, {from:{row:9,col:0},to:{row:8,col:0}});
console.log('Red chariot 9,0->8,0 valid:', r1.valid);
if (r1.valid) {
  console.log('new player:', r1.newState.currentPlayer);

  // Now black chariot
  const r2 = makeMove(r1.newState, {from:{row:0,col:8},to:{row:1,col:8}});
  console.log('Black chariot 0,8->1,8 valid:', r2.valid);
  if (r2.valid) {
    console.log('new player:', r2.newState.currentPlayer);
    
    // Red chariot (8,0) -> (8,1)
    const r3 = makeMove(r2.newState, {from:{row:8,col:0},to:{row:8,col:1}});
    console.log('Red chariot 8,0->8,1 valid:', r3.valid);
    if (r3.valid) {
      console.log('new player:', r3.newState.currentPlayer);
      console.log('Board[8][1]:', JSON.stringify(r3.newState.board[8][1]));

      // Black chariot (1,8) -> (4,8)
      const r4 = makeMove(r3.newState, {from:{row:1,col:8},to:{row:4,col:8}});
      console.log('Black chariot 1,8->4,8 valid:', r4.valid);
      if (r4.valid) {
        console.log('Board[4][8]:', JSON.stringify(r4.newState.board[4][8]));
        console.log('new player:', r4.newState.currentPlayer);
        
        // Red chariot (8,1) -> (4,1)
        const r5 = makeMove(r4.newState, {from:{row:8,col:1},to:{row:4,col:1}});
        console.log('Red chariot 8,1->4,1 valid:', r5.valid);
        if (r5.valid) {
          console.log('Board[4][1]:', JSON.stringify(r5.newState.board[4][1]));
          console.log('new player:', r5.newState.currentPlayer);
          
          // Black chariot (4,8) -> (4,1) captures red chariot
          const r6 = makeMove(r5.newState, {from:{row:4,col:8},to:{row:4,col:1}});
          console.log('Black chariot 4,8->4,1 valid:', r6.valid, 'captured:', JSON.stringify(r6.captured));
          if (r6.valid) {
            console.log('Board[4][1]:', JSON.stringify(r6.newState.board[4][1]));
          }
        }
      }
    }
  }
}
