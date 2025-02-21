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
    ];
    packages = with pkgs; [
      rustc
      rust-analyzer
      cargo
      clang
      foundry-bin
      taplo
      toml-cli
    ];
    commands = [
      {
        name = "local-ic";
        package = config.packages.local-ic;
      }
    ];
  };
}
