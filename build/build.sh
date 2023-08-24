#!/bin/bash
cargo build --release --target wasm32-unknown-unknown
rm -rf Minesweeper
wasm-bindgen \
    --no-typescript \
    --out-dir minesweeper \
    --target web target/wasm32-unknown-unknown/release/minesweeper.wasm
cp build/index.html Minesweeper/
cat build/append.txt >> Minesweeper/minesweeper.js

mkdir Minesweeper/assets
cp assets/minesweeper_tiles.png Minesweeper/assets/
cp assets/padding.png Minesweeper/assets/

/Applications/Firefox.app/Contents/MacOS/firefox -new-tab http://\[::\]:8004/minesweeper
python3 -m http.server 8004 
