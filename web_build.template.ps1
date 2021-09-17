$env:Path += ";C:\Program Files\LLVM\bin"
$env:AR="llvm-ar"
$env:CFLAGS="-ID:\Projects\SourceMomentumTools\miniquad-render\wasmincl"
cargo build --release --target wasm32-unknown-unknown