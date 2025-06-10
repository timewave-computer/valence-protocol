{
  description = "Valence Protocol Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    foundry.url = "github:shazow/foundry.nix/monthly";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, foundry }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) foundry.overlay ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain with specific components
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            cargo-watch
            cargo-edit
            cargo-udeps
            cargo-audit
            cargo-tarpaulin

            # Solidity and Foundry
            foundry-bin

            # Node.js and npm (might be needed for some tools)
            nodejs_20
            nodePackages.npm

            # System dependencies
            pkg-config
            openssl
            curl
            git
            just
            jq

            # CosmWasm dependencies
            cosmwasm-check

            # Additional tools
            protobuf
            clang
            llvm

            # Database tools (might be needed)
            sqlite

            # Docker (for contract optimization)
            docker
            docker-compose
          ] ++ lib.optionals stdenv.isDarwin [
            # macOS specific dependencies
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.CoreServices
            libiconv
          ];

          shellHook = ''
            echo "🦀 Valence Protocol Development Environment"
            echo "📋 Installing soldeer if not present..."
            
            # Install soldeer via cargo if not already installed
            if ! command -v soldeer &> /dev/null; then
              echo "Installing soldeer..."
              cargo install soldeer --force
            fi
            
            echo "📋 Available tools:"
            echo "  • Rust $(rustc --version)"
            echo "  • Cargo $(cargo --version)"
            echo "  • Forge $(forge --version 2>/dev/null || echo 'not available')"
            echo "  • Just $(just --version)"
            echo "  • CosmWasm Check $(cosmwasm-check --version 2>/dev/null || echo 'not available')"
            echo "  • Soldeer $(soldeer --version 2>/dev/null || echo 'installing...')"
            echo ""
            echo "📁 Project structure:"
            echo "  • contracts/ - CosmWasm smart contracts"
            echo "  • solidity/ - Solidity smart contracts"
            echo "  • zk/ - Zero-knowledge components"
            echo "  • packages/ - Shared Rust packages"
            echo ""
            echo "🚀 Quick start:"
            echo "  • cargo check - Check Rust code"
            echo "  • just build - Build all contracts"
            echo "  • cd solidity && soldeer install && forge build - Build Solidity contracts"
            echo ""

            # Set environment variables
            export RUST_LOG=debug
            export RUST_BACKTRACE=1
            
            # Add cargo bin to PATH
            export PATH="$HOME/.cargo/bin:$PATH"
            
            # Set up Foundry
            export FOUNDRY_PROFILE=default
            
            echo "✅ Environment ready!"
          '';

          # Environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.lib.getVersion pkgs.clang}/include";
        };

        # Package outputs
        packages.default = pkgs.hello; # Placeholder

        # Formatter
        formatter = pkgs.nixpkgs-fmt;
      });
} 