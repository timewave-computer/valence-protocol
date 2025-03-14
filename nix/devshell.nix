{ config, lib, pkgs, ... }:
{
  devshells.default = {
    env = [
      {
        name = "OPENSSL_DIR";
        value = "${lib.getDev pkgs.openssl}";
      }
      {
        name = "OPENSSL_LIB_DIR";
        value = "${lib.getLib pkgs.openssl}/lib";
      }
      {
        name = "LIBCLANG_PATH";
        value = lib.makeLibraryPath [ pkgs.libclang ];
      }

    ];
    packages = with pkgs; [
      rustc
      rust-analyzer
      cargo
      clang
      foundry-bin
      taplo
      toml-cli
      lld
    ];
    commands = [
      {
        name = "local-ic";
        package = config.packages.local-ic;
      }
    ];
  };
}
