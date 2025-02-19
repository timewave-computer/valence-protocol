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
      perSystem = { pkgs, system, config, ... }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.rust-overlay.overlays.default
          ];
        };
        imports = [
          ./nix/devshell.nix
        ];
        packages.local-ic = pkgs.callPackage ./nix/pkgs/local-ic.nix { inherit self; };
        packages.libosmosistesttube = pkgs.callPackage ./nix/pkgs/libosmosistesttube.nix { };
        packages.libntrntesttube = pkgs.callPackage ./nix/pkgs/libntrntesttube.nix {
          inherit (config.packages) libosmosistesttube;
        };
        packages.valence-protocol = pkgs.callPackage ./nix/pkgs/valence-protocol.nix {
          inherit (inputs)
            self
            crane;
          inherit (config.packages) libosmosistesttube libntrntesttube;
        };
      };
    };
}
