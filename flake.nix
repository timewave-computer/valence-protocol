{
  inputs = {
    # Flake orchestration
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell.url = "github:numtide/devshell";

    # Rust
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

  };

  outputs = inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.devshell.flakeModule ];
      systems = [ "x86_64-linux" "xd8_64-darwin" ];
      perSystem = { pkgs, ... }: {
        imports = [
          ./nix/devshell.nix
        ];
        packages.local-ic = pkgs.callPackage ./nix/pkgs/local-ic.nix { inherit self; };
        packages.libosmosistesttube = pkgs.callPackage ./nix/pkgs/libosmosistesttube.nix { };
        packages.valence-protocol = pkgs.callPackage ./nix/pkgs/valence-protocol.nix {
          inherit (inputs)
            self
            crane
            fenix;
        };
      };
    };
}
