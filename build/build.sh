#!/bin/zsh
cargo build --release --target wasm32-unknown-unknown

rm minesweeper/minesweeper*
wasm-bindgen \
    --no-typescript \
    --out-dir minesweeper \
    --target web target/wasm32-unknown-unknown/release/minesweeper.wasm
cat build/append.txt >> Minesweeper/minesweeper.js


/Applications/Firefox.app/Contents/MacOS/firefox -new-tab http://\[::\]:8004/minesweeper
python3 -m http.server 8004 
