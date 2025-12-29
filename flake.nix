{
  description = "Development environment for the Som programming language";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable."1.90.0".default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            cargo-llvm-cov
            just
            cargo-edit
            cargo-watch
            cargo-deny
            wasm-bindgen-cli
            llvm
            ripgrep
            fd
            bat
            bun
          ];

          shellHook = ''
            export PATH="$PATH:$HOME/.cargo/bin"

            # Install committer if not already installed
            if ! command -v committer &> /dev/null; then
              echo "Installing committer..."
              cargo install committer
            fi

            # Create 'c' alias as wrapper script
            mkdir -p .direnv/bin
            echo '#!/usr/bin/env bash' > .direnv/bin/c
            echo 'exec committer "$@"' >> .direnv/bin/c
            chmod +x .direnv/bin/c
            export PATH="$PWD/.direnv/bin:$PATH"
          '';

          RUST_BACKTRACE = "1";
          RUST_LOG = "debug";
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
