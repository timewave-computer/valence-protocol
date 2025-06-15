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
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Base osmosis test tube package
        libosmosistesttube = pkgs.callPackage ./nix/libosmosistesttube.nix {
          inherit (pkgs) lib buildGoModule fetchCrate patchelf;
        };

        # Neutron test tube package override
        libntrntesttube = libosmosistesttube.override {
          pname = "libntrntesttube";
          version = "5.0.1";
          crateName = "neutron-test-tube";
          srcHash = "sha256-gJ1exKm3TSfOHM9ooaONLOrtvjKaBI15RcDu5MkawnA=";
          vendorHash = "sha256-gway1f2vjzMxPK4Ua38DjNEKtUrzlBBRYY4Q+SNZK/M=";
        };

        # Soldeer package manager for Solidity 
        # Note: On first build, Nix will provide the correct hashes to replace the placeholder values below
        soldeer = pkgs.rustPlatform.buildRustPackage rec {
          pname = "soldeer";
          version = "0.5.4";

          src = pkgs.fetchFromGitHub {
            owner = "mario-eth";
            repo = "soldeer";
            rev = "v${version}";
            sha256 = "sha256-1J2zb3X9QNx5AmHCFdjtNIoJmK5T0ebbGzhLL+vgK40=";
          };

          cargoHash = "sha256-54PE/d7wgprmf0/ZHVxV9DHEycXMcVCbbSVIMUfWEIg=";

          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            pkgs.darwin.apple_sdk.frameworks.Security
          ];

          doCheck = false; # Skip tests to avoid network dependency

          meta = with pkgs.lib; {
            description = "Solidity Package Manager written in Rust";
            homepage = "https://github.com/mario-eth/soldeer";
            license = licenses.mit;
            platforms = platforms.all;
          };
        };

        # Development shell dependencies
        shellPackages = with pkgs; [
          # Essential development tools
          rustToolchain
          pkg-config
          openssl
          protobuf
          cacert
          curl
          git
          gnumake

          # CosmWasm tools
          just
          foundry-bin
          binaryen
          soldeer

          # Test tube dependencies
          libosmosistesttube
          libntrntesttube
          go  # Required for test tube Go build scripts

          # Build dependencies
          clang
          llvm
          lld

          # macOS specific
        ] ++ lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          pkgs.darwin.apple_sdk.frameworks.Security
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = shellPackages;

          # Environment variables
          shellHook = ''
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            export RUST_BACKTRACE=1
            export RUST_LOG=debug
            export VALENCE_DEV_MODE=1

            # Test tube library paths
            export ${if pkgs.stdenv.isDarwin then "DYLD_LIBRARY_PATH" else "LD_LIBRARY_PATH"}="${libosmosistesttube}/lib:${libntrntesttube}/lib:''${${if pkgs.stdenv.isDarwin then "DYLD_LIBRARY_PATH" else "LD_LIBRARY_PATH"}:-}"

            # Create local directories with complete Go source from Nix packages
            if [ ! -d "./libntrntesttube" ]; then
              echo "Setting up neutron test tube Go environment..."
              cp -r "${libntrntesttube}/src" ./libntrntesttube
              chmod -R u+w ./libntrntesttube
              # Also create artifacts directory with prebuilt libraries for fallback
              mkdir -p ./libntrntesttube/artifacts
              ln -sf "${libntrntesttube}/lib/libntrntesttube".* ./libntrntesttube/artifacts/
              ln -sf "${libntrntesttube}/include/libntrntesttube.h" ./libntrntesttube/artifacts/
            fi
            if [ ! -d "./libosmosistesttube" ]; then  
              echo "Setting up osmosis test tube Go environment..."
              cp -r "${libosmosistesttube}/src" ./libosmosistesttube
              chmod -R u+w ./libosmosistesttube
              # Also create artifacts directory with prebuilt libraries for fallback  
              mkdir -p ./libosmosistesttube/artifacts
              ln -sf "${libosmosistesttube}/lib/libosmosistesttube".* ./libosmosistesttube/artifacts/
              ln -sf "${libosmosistesttube}/include/libosmosistesttube.h" ./libosmosistesttube/artifacts/
            fi
            
            # Set library paths for test tubes  
            export LIBOSMOSISTESTTUBE_LIB_DIR="${libosmosistesttube}/lib"
            export LIBNTRNTESTTUBE_LIB_DIR="${libntrntesttube}/lib"
            export LIBOSMOSISTESTTUBE_INCLUDE_DIR="${libosmosistesttube}/include" 
            export LIBNTRNTESTTUBE_INCLUDE_DIR="${libntrntesttube}/include"
            
            # Configure environment to skip test tube Go builds when prebuilt libs exist
            export OSMOSIS_TUBE_DEV=0
            
            # Set up directories for prebuilt test tube integration 
            mkdir -p .cargo

            echo "Valence Protocol Development Environment"

            echo "Available tools:"
            echo "  • Rust $(rustc --version)"
            echo "  • Cargo $(cargo --version)"
            echo "  • Forge $(forge --version | head -1)"
            echo "  • Just $(just --version)"
            echo "  • Soldeer $(soldeer --version 2>/dev/null || echo 'v0.5.4 (may need hash update on first build)')"

            echo ""
            echo "Project structure:"
            echo "  • contracts/ - CosmWasm smart contracts"
            echo "  • solidity/ - Solidity smart contracts"
            echo "  • zk/ - Zero-knowledge components"

            echo "  • packages/ - Shared Rust packages"
            echo ""
            echo "Quick start:"
            echo "  • cargo check - Check Rust code"
            echo "  • just build - Build all contracts"
            echo "  • cd solidity && soldeer install && forge build - Build Solidity contracts"

            echo ""
            echo "Environment ready!"
            echo "Account Factory development environment loaded!"
            echo "Use 'nix develop .#minimal' for a lightweight shell"
            echo "Use 'nix develop .#testing' for testing-focused shell"
            echo "Use 'nix develop .#zk' for ZK development shell"
          '';
        };

        # Additional development shells
        devShells.minimal = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            just
          ];
        };

        devShells.testing = pkgs.mkShell {
          buildInputs = shellPackages;
          shellHook = ''
            export RUST_BACKTRACE=1
            export RUST_LOG=info
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            # Test tube library paths
            export ${if pkgs.stdenv.isDarwin then "DYLD_LIBRARY_PATH" else "LD_LIBRARY_PATH"}="${libosmosistesttube}/lib:${libntrntesttube}/lib:''${${if pkgs.stdenv.isDarwin then "DYLD_LIBRARY_PATH" else "LD_LIBRARY_PATH"}:-}"
            echo "Testing environment loaded with test tube support"
          '';
        };

        devShells.zk = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            just
            binaryen
          ];
          shellHook = ''
            echo "ZK development environment loaded"
            export RUST_BACKTRACE=1
          '';
        };
      });
} 