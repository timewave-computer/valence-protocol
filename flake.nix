{
  description = "Valence Protocol Account Factory Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain with WASM support for CosmWasm
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Node.js for JavaScript tooling
        nodejs = pkgs.nodejs_20;

        # Try to use foundry or fall back to manual installation
        foundryPkg = if pkgs ? foundry then pkgs.foundry 
                     else if pkgs ? foundry-bin then pkgs.foundry-bin
                     else null;

        # Try to get crate2nix, fallback to empty list if not available
        crate2nixPkg = if pkgs ? crate2nix then [ pkgs.crate2nix ] else [];

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # === Rust Development ===
            rustToolchain
            cargo-watch
            cargo-edit
            cargo-audit
            cargo-outdated
            
            # === Crate2nix for Rust dependency management ===
            nix-prefetch-git
            
            # === CosmWasm Development ===
            wasm-pack
            binaryen # For wasm-opt
            wabt     # WebAssembly Binary Toolkit
            
            # === Solidity Development ===
            nodejs
            nodePackages.npm
            nodePackages.yarn
            
            # === ZK Development ===
            # SP1 and ZK tooling
            cmake
            pkg-config
            openssl
            
            # === Cross-chain Testing ===
            docker
            docker-compose
            
            # === General Development Tools ===
            git
            jq
            curl
            wget
            gnused
            findutils
            coreutils
            
            # === Protocol Buffers (for blockchain interactions) ===
            protobuf
            protoc-gen-rust
            
            # === Database and Storage (for local testing) ===
            sqlite
            
            # === Networking Tools (for testing) ===
            nettools
            
            # === Build Tools ===
            gnumake
            gcc
            
            # === Documentation ===
            mdbook
            
            # === Blockchain Testing ===
            # Note: Local blockchain nodes would be added here
            # For now, we'll use Docker containers
            
          ] ++ crate2nixPkg ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # macOS specific dependencies
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            pkgs.libiconv
          ] ++ pkgs.lib.optionals (foundryPkg != null) [
            foundryPkg
          ];

          # Environment variables
          shellHook = ''
            echo "🚀 Valence Protocol Account Factory Development Environment"
            echo ""
            echo "Available tools:"
            echo "  📦 Rust ${rustToolchain.version} (with WASM target)"
            if command -v crate2nix >/dev/null 2>&1; then
              echo "  🔧 crate2nix for Rust dependency management"
            else
              echo "  🔧 crate2nix: Install with 'cargo install crate2nix'"
            fi
            if command -v forge >/dev/null 2>&1; then
              echo "  ⚡ Foundry (forge, cast, anvil, chisel)"
            else
              echo "  ⚡ Foundry: Installing via foundryup..."
            fi
            echo "  🌐 Node.js ${nodejs.version}"
            echo "  🔒 ZK Development tools (cmake, openssl)"
            echo "  🐳 Docker & Docker Compose"
            echo "  📚 Documentation tools (mdbook)"
            echo ""
            echo "Quick start commands:"
            echo "  cargo check                 # Check Rust code"
            if command -v crate2nix >/dev/null 2>&1; then
              echo "  crate2nix generate          # Generate Nix expressions from Cargo.lock"
            fi
            echo "  forge build                 # Build Solidity contracts (if foundry installed)"
            echo "  forge test                  # Test Solidity contracts (if foundry installed)"
            echo "  cargo test                  # Test Rust code"
            echo ""
            
            # Set up environment variables
            export RUST_BACKTRACE=1
            export CARGO_NET_GIT_FETCH_WITH_CLI=true
            
            # Foundry/Ethereum tooling
            export FOUNDRY_PROFILE=default
            
            # Add local bin to PATH for any project-specific tools
            export PATH="$PWD/scripts:$HOME/.foundry/bin:$HOME/.cargo/bin:$PATH"
            
            # ZK Development
            export SP1_DEV=1
            
            # CosmWasm optimization
            export DOCKER_PLATFORM=linux/amd64
            
            # Ensure we're using the right Node.js
            export NODE_PATH="${nodejs}/lib/node_modules"
            
            # Install foundry if not available
            if ! command -v forge >/dev/null 2>&1; then
              echo "📦 Installing Foundry..."
              if command -v curl >/dev/null 2>&1; then
                curl -L https://foundry.paradigm.xyz | bash 2>/dev/null || true
                if [ -f "$HOME/.foundry/bin/foundryup" ]; then
                  "$HOME/.foundry/bin/foundryup" 2>/dev/null || true
                fi
              fi
            fi
            
            # Install crate2nix if not available
            if ! command -v crate2nix >/dev/null 2>&1; then
              echo "📦 Installing crate2nix..."
              if command -v cargo >/dev/null 2>&1; then
                cargo install crate2nix 2>/dev/null || echo "⚠️  Failed to install crate2nix via cargo"
              fi
            fi
            
            # Install WASM target
            if command -v rustup >/dev/null 2>&1; then
              rustup target add wasm32-unknown-unknown 2>/dev/null || true
            fi
            
            echo "Environment configured! ✨"
          '';

          # Additional environment variables for different platforms
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          PROTOC = "${pkgs.protobuf}/bin/protoc";
          
          # For M1 Macs, ensure proper architecture
          DOCKER_DEFAULT_PLATFORM = "linux/amd64";
          
          # Rust-specific
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          
          # OpenSSL for native dependencies
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };

        # Additional shells for specific tasks
        devShells.minimal = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            git
            jq
          ] ++ crate2nixPkg;
          shellHook = ''
            echo "🔧 Minimal Valence development environment"
            export PATH="$HOME/.foundry/bin:$HOME/.cargo/bin:$PATH"
          '';
        };

        devShells.testing = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            docker
            docker-compose
            nodejs
            jq
            curl
          ] ++ crate2nixPkg;
          shellHook = ''
            echo "🧪 Testing-focused Valence environment"
            echo "Includes blockchain testing tools"
            export PATH="$HOME/.foundry/bin:$HOME/.cargo/bin:$PATH"
          '';
        };

        devShells.zk = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cmake
            pkg-config
            openssl
            protobuf
            llvmPackages.clang
            llvmPackages.libclang
          ] ++ crate2nixPkg;
          shellHook = ''
            echo "🔐 ZK Development environment"
            echo "Optimized for ZK coprocessor development"
          '';
          
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          CC = "${pkgs.clang}/bin/clang";
          CXX = "${pkgs.clang}/bin/clang++";
        };

        # Helper scripts
        apps = {
          # Build all contracts
          build-all = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "build-all" ''
              echo "🔨 Building all contracts..."
              echo "Building Rust/CosmWasm contracts..."
              cargo build --release
              if command -v forge >/dev/null 2>&1; then
                echo "Building Solidity contracts..."
                cd solidity && forge build
              else
                echo "⚠️  Foundry not available, skipping Solidity builds"
              fi
              echo "✅ All available contracts built successfully!"
            '';
          };

          # Run all tests
          test-all = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "test-all" ''
              echo "🧪 Running all tests..."
              echo "Testing Rust/CosmWasm contracts..."
              cargo test
              if command -v forge >/dev/null 2>&1; then
                echo "Testing Solidity contracts..."
                cd solidity && forge test
              else
                echo "⚠️  Foundry not available, skipping Solidity tests"
              fi
              echo "✅ All available tests completed!"
            '';
          };

          # Generate crate2nix expressions
          generate-nix = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "generate-nix" ''
              echo "🔧 Generating Nix expressions from Cargo.lock..."
              if ! command -v crate2nix >/dev/null 2>&1; then
                echo "❌ crate2nix not found. Install with: cargo install crate2nix"
                exit 1
              fi
              
              if [ ! -f "Cargo.lock" ]; then
                echo "⚠️  Cargo.lock not found. Running cargo build first..."
                cargo build
              fi
              
              echo "Generating Cargo.nix..."
              crate2nix generate
              
              if [ -f "Cargo.nix" ]; then
                echo "✅ Cargo.nix generated successfully!"
                echo "You can now use 'nix-build -A rootCrate.build' to build with Nix"
              else
                echo "❌ Failed to generate Cargo.nix"
                exit 1
              fi
            '';
          };

          # Build with crate2nix
          build-nix = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "build-nix" ''
              echo "🔨 Building with crate2nix..."
              if ! command -v crate2nix >/dev/null 2>&1; then
                echo "❌ crate2nix not found. Install with: cargo install crate2nix"
                exit 1
              fi
              
              if [ ! -f "Cargo.nix" ]; then
                echo "⚠️  Cargo.nix not found. Generating first..."
                crate2nix generate
              fi
              
              echo "Building with Nix..."
              nix-build -A rootCrate.build
              
              echo "✅ Build completed with crate2nix!"
            '';
          };

          # Setup development environment
          dev-setup = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "dev-setup" ''
              echo "🚀 Setting up Valence development environment..."
              
              # Install Foundry
              if ! command -v forge >/dev/null 2>&1; then
                echo "Installing Foundry..."
                if command -v curl >/dev/null 2>&1; then
                  curl -L https://foundry.paradigm.xyz | bash
                  if [ -f "$HOME/.foundry/bin/foundryup" ]; then
                    "$HOME/.foundry/bin/foundryup"
                  fi
                fi
              else
                echo "✅ Foundry already installed"
              fi
              
              # Install Rust WASM target
              if command -v rustup >/dev/null 2>&1; then
                echo "Adding WASM target..."
                rustup target add wasm32-unknown-unknown
              else
                echo "⚠️  rustup not available, using pre-configured Rust from Nix"
              fi
              
              # Install crate2nix
              if ! command -v crate2nix >/dev/null 2>&1; then
                echo "Installing crate2nix..."
                if command -v cargo >/dev/null 2>&1; then
                  cargo install crate2nix
                else
                  echo "⚠️  cargo not available, cannot install crate2nix"
                fi
              else
                echo "✅ crate2nix already installed"
              fi
              
              # Generate initial Cargo.nix if it doesn't exist
              if [ ! -f "Cargo.nix" ] && [ -f "Cargo.toml" ] && command -v crate2nix >/dev/null 2>&1; then
                echo "Generating initial Cargo.nix..."
                cargo build 2>/dev/null || true  # Ensure Cargo.lock exists
                crate2nix generate 2>/dev/null || echo "⚠️  Could not generate Cargo.nix (this is optional)"
              fi
              
              # Setup git hooks (if .git exists)
              if [ -d ".git" ]; then
                echo "Setting up git hooks..."
                # Add pre-commit hooks here if needed
              fi
              
              echo "✅ Development environment setup complete!"
              echo ""
              echo "To use Foundry, make sure $HOME/.foundry/bin is in your PATH"
              echo "Or restart your shell to pick up the new PATH"
              echo ""
              echo "crate2nix commands:"
              echo "  nix run .#generate-nix       # Generate Cargo.nix from Cargo.lock"
              echo "  nix run .#build-nix          # Build using crate2nix"
            '';
          };
        };

        # Formatting and linting
        formatter = pkgs.nixpkgs-fmt;
      });
} 