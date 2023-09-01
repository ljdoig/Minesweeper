#!/bin/zsh
cargo build --release --target wasm32-unknown-unknown

rm minesweeper/minesweeper*
wasm-bindgen \
    --no-typescript \
    --out-dir minesweeper \
    --target web target/wasm32-unknown-unknown/release/minesweeper.wasm
cat build/append.txt >> Minesweeper/minesweeper.js

python3 -m http.server 8080
