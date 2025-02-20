{
  inputs = {
    # Flake orchestration
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell.url = "github:numtide/devshell";

    # Rust
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    foundry.url = "github:shazow/foundry.nix";
  };

  outputs = inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.devshell.flakeModule ];
      systems = [ "x86_64-linux" "xd8_64-darwin" ];
      perSystem = { lib, pkgs, system, config, ... }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.rust-overlay.overlays.default
            inputs.foundry.overlay
            (final: prev: config.packages // {
              workspaceRoot = ./.;
              inherit (inputs) crane;
            })
          ];
        };
        imports = [
          ./nix/devshell.nix
        ];
        packages = {
          cosmwasm-contracts = pkgs.callPackage ./nix/cosmwasm-contracts.nix { };
          solidity-contracts = pkgs.callPackage ./nix/solidity-contracts.nix { };
          local-ic = pkgs.callPackage ./nix/local-ic.nix {
            localICStartScriptPath = ./scripts/start-local-ic.sh;
          };
          libosmosistesttube = pkgs.callPackage ./nix/libosmosistesttube.nix { };
          libntrntesttube = pkgs.callPackage ./nix/libntrntesttube.nix { };
        };
      };
    };
}
