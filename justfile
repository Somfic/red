# Build WASM
wasm:
    cargo build --release -p rendering --target wasm32-unknown-unknown
    wasm-bindgen --out-dir web/src/lib/wasm --target web target/wasm32-unknown-unknown/release/rendering.wasm

# Build WASM (debug, faster compile)
wasm-dev:
    cargo build -p rendering --target wasm32-unknown-unknown
    wasm-bindgen --out-dir web/src/lib/wasm --target web target/wasm32-unknown-unknown/debug/rendering.wasm

# Start dev server (rebuild wasm first)
dev: wasm-dev
    cd web && bun run dev

# Watch Rust changes and rebuild WASM
watch:
    cargo watch -w simulation -w rendering -s "just wasm-dev"

# Run dev server + watch in parallel (needs two terminals, or use &)
dev-watch:
    just watch &
    just dev

# Production build
build: wasm
    cd web && bun install && bun run build

# Clean all artifacts
clean:
    cargo clean
    rm -rf web/src/lib/wasm
    rm -rf web/dist
