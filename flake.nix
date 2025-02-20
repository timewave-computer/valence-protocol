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
            (final: prev: config.packages // {
              inherit self;
              inherit (inputs) crane;
            })
          ];
        };
        imports = [
          ./nix/devshell.nix
        ];
        packages = let
          cargoTOML = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          contracts = lib.filterAttrs
            (n: v: lib.hasPrefix "valence" n && lib.hasPrefix "contracts" v.path)
            cargoTOML.workspace.dependencies;
        buildValenceContract = pkgs.callPackage ./nix/pkgs/valence-contract.nix;
        in lib.mapAttrs (pname: value: buildValenceContract {
          inherit pname;
          cargoPackages = [ pname ];
        }) contracts
        // {
          valence-contracts = buildValenceContract {
            pname = "valence-contracts";
            cargoPackages = lib.attrNames contracts;
          };
          local-ic = pkgs.callPackage ./nix/pkgs/local-ic.nix {
            localICStartScriptPath = ./scripts/start-local-ic.sh;
          };
          libosmosistesttube = pkgs.callPackage ./nix/pkgs/libosmosistesttube.nix { };
          libntrntesttube = pkgs.callPackage ./nix/pkgs/libntrntesttube.nix {
            inherit (config.packages) libosmosistesttube;
          };
        };
      };
    };
}
