{ craneLib
, cargoVendorDir
, cargoDeps
, name ? "valence-program-manager"
}:
craneLib.buildPackage {
  inherit cargoVendorDir;

  src = craneLib.cleanCargoSource ../.;
  cargoArtifacts = cargoDeps;

  pname = "${name}-schema";
  strictDeps = true;
  doCheck = false;

  cargoExtraArgs = "-p ${name} --bin schema";

  meta.mainProgram = "schema";
}
