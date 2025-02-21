{ lib
, craneLib
, cargoVendorDir
, cargoDeps
, openssl
, libclang
}:
craneLib.cargoNextest (cargoDeps.commonArgs // {
  src = craneLib.cleanCargoSource ../.;
  pname = "valence";

  strictDeps = true;
  cargoArtifacts = cargoDeps;
  inherit cargoVendorDir;
})
