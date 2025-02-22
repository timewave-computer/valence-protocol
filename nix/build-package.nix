{ lib
, craneLib
, cargoVendorDir
, cargoDeps
, pname
, cargoArgs ? ""
, drvArgs ? { }
}:
craneLib.buildPackage ({
  inherit cargoVendorDir;
  pname = "pname";

  src = craneLib.cleanCargoSource ../.;
  cargoArtifacts = cargoDeps;

  doCheck = false;

  cargoExtraArgs = "-p ${pname} ${cargoArgs}";

} // drvArgs)
