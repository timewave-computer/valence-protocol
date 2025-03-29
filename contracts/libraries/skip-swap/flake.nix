{
  description = "Skip Swap library development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Common environment variables for running tests
        commonEnvVars = {
          RUST_BACKTRACE = "1";
          RUST_LOG = "debug";
          MACOSX_DEPLOYMENT_TARGET = pkgs.lib.optionalString pkgs.stdenv.isDarwin "11.0";
        };

        # Base project path - hardcoded for simplicity
        projectRoot = "/Users/hxrts/projects/timewave/valence-protocol";

        # Helper function to run cargo test with proper environment variables
        runTest = pkgs.writeShellScriptBin "run-test" ''
          set -euo pipefail

          # Set environment variables
          export RUST_BACKTRACE=${commonEnvVars.RUST_BACKTRACE}
          export RUST_LOG=${commonEnvVars.RUST_LOG}
          export MACOSX_DEPLOYMENT_TARGET=${commonEnvVars.MACOSX_DEPLOYMENT_TARGET}

          # Arguments: $1 = directory, $2 = name
          dir=$1
          name=$2
          
          echo "========================================================="
          echo "  Running tests for $name"
          echo "========================================================="
          
          if [ -d "$dir" ]; then
            cd "$dir"
            cargo test
            echo "Tests for $name completed successfully!"
            echo ""
          else
            echo "Error: Directory $dir does not exist"
            exit 1
          fi
        '';

      in {
        # Add packages
        packages = {
          # Run tests for the whole Valence ecosystem
          run-tests = pkgs.writeShellScriptBin "run-tests" ''
            set -euo pipefail
            
            # Using hardcoded project root
            PROJECT_ROOT="${projectRoot}"
            
            echo "Project root: $PROJECT_ROOT"
            
            # Run skip-swap-valence tests
            ${runTest}/bin/run-test "$PROJECT_ROOT/contracts/libraries/skip-swap/skip-swap-valence" "skip-swap-valence"
            
            # Run authorization-utils tests
            ${runTest}/bin/run-test "$PROJECT_ROOT/packages/authorization-utils" "valence-authorization-utils"
            
            # Run library-utils tests
            ${runTest}/bin/run-test "$PROJECT_ROOT/packages/library-utils" "valence-library-utils"
            
            echo "All tests completed successfully!"
          '';
          
          # Run just the skip-swap-valence tests
          run-skip-swap-tests = pkgs.writeShellScriptBin "run-skip-swap-tests" ''
            set -euo pipefail
            
            # Using hardcoded skip-swap-valence path
            SKIP_SWAP_VALENCE_DIR="${projectRoot}/contracts/libraries/skip-swap/skip-swap-valence"
            
            if [ ! -d "$SKIP_SWAP_VALENCE_DIR" ]; then
              echo "Error: Could not find skip-swap-valence directory at $SKIP_SWAP_VALENCE_DIR"
              exit 1
            fi
            
            echo "Skip-swap-valence directory: $SKIP_SWAP_VALENCE_DIR"
            
            # Run skip-swap-valence tests
            ${runTest}/bin/run-test "$SKIP_SWAP_VALENCE_DIR" "skip-swap-valence"
            
            echo "Skip-swap tests completed successfully!"
          '';
          
          default = self.packages.${system}.run-tests;
        };

        # Add checks for CI
        checks = {
          skip-swap-tests = pkgs.stdenv.mkDerivation {
            name = "skip-swap-tests";
            src = self;
            buildInputs = [ rustToolchain ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];
            buildPhase = ''
              cd skip-swap-valence
              ${pkgs.lib.concatStringsSep " " (pkgs.lib.mapAttrsToList (name: value: "${name}=${value}") commonEnvVars)} cargo test --verbose
            '';
            installPhase = "touch $out";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            libiconv

            # Development tools
            cargo-watch
            cargo-edit
            cargo-expand
            
            # Include our test runners
            self.packages.${system}.run-tests
            self.packages.${system}.run-skip-swap-tests
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          RUST_BACKTRACE = 1;
          RUST_LOG = "debug";
          
          # Explicitly set MACOSX_DEPLOYMENT_TARGET for macOS
          MACOSX_DEPLOYMENT_TARGET = pkgs.lib.optionalString pkgs.stdenv.isDarwin "11.0";
        };
      }
    );
} 