{
  description = "Valence Protocol development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    
    # Include skip-swap as a subflake with relative path
    skip-swap = {
      url = "git+file:.?dir=contracts/libraries/skip-swap";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, skip-swap, ... }:
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

      in {
        # Use the run-tests package from skip-swap flake
        packages = {
          run-tests = skip-swap.packages.${system}.run-tests;
          run-skip-swap-tests = skip-swap.packages.${system}.run-skip-swap-tests;
          default = self.packages.${system}.run-tests;
        };

        # Add checks to run tests
        checks = {
          # Skip-swap tests
          skip-swap-tests = skip-swap.checks.${system}.skip-swap-tests;
        };

        devShells = {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustToolchain
              pkg-config
              openssl
              libiconv

              # Development tools
              cargo-watch
              cargo-edit
              cargo-expand
              cargo-generate
              
              # Add the test runners
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

          # Include the skip-swap shell as well
          skipSwap = skip-swap.devShells.${system}.default;
        };
      }
    );
} 