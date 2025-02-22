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
              craneLib = inputs.crane.mkLib pkgs;
              inherit (inputs) crane;
              cargoVendorDir = final.callPackage ./nix/cargo-vendor-dir.nix { };
              cargoDeps = final.callPackage ./nix/cargo-deps.nix { };
              contractNames = let
                cargoTOML = builtins.fromTOML (builtins.readFile ./Cargo.toml);
              in
                builtins.attrNames (lib.filterAttrs (name: value:
                  lib.hasPrefix "valence" name && lib.hasPrefix "contracts" value.path
                ) cargoTOML.workspace.dependencies);
              buildValencePackage = pkgs.callPackage ./nix/build-package.nix;
            })
          ];
        };
        imports = [
          ./nix/devshell.nix
        ];
        packages = {
          valence-cosmwasm-contracts = pkgs.callPackage ./nix/cosmwasm-contracts.nix { };
          valence-solidity-contracts = pkgs.callPackage ./nix/solidity-contracts.nix { };
          valence-program-examples = pkgs.buildValencePackage {
            pname = "valence-program-examples";
            cargoArgs = "--bins";
          };
          valence-e2e = pkgs.buildValencePackage {
            pname = "valence-e2e";
            cargoArgs = "--examples";
          };
          local-ic = pkgs.callPackage ./nix/local-ic.nix {
            localICStartScriptPath = ./scripts/start-local-ic.sh;
          };
          libosmosistesttube = pkgs.callPackage ./nix/libosmosistesttube.nix { };
          libntrntesttube = pkgs.callPackage ./nix/libntrntesttube.nix { };
        };
        checks = {
          nextest = pkgs.craneLib.cargoNextest (pkgs.cargoDeps.commonArgs // {
            pname = "valence";
            cargoArtifacts = pkgs.cargoDeps;
          });
          clippy = pkgs.craneLib.cargoClippy (pkgs.cargoDeps.commonArgs // {
            pname = "valence";
            cargoArtifacts = pkgs.cargoDeps;
            cargoClippyExtraArgs = "--all-targets --verbose -- --deny warnings";
          });
        };
        apps = lib.listToAttrs (lib.map (pname: {
          name = "${pname}-schema";
          # flake-parts requires derivation to be in program attribute
          value.program = pkgs.buildValencePackage {
            pname = pname;
            cargoArgs = "--bin schema";
            drvArgs.meta.mainProgram = "schema";
          };
        }) (pkgs.contractNames ++ [ "valence-program-manager" ]));
      };
    };
}
