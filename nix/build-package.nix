{ lib
, craneLib
, cargoVendorDir
, cargoDeps
, pname
, cargoArgs ? ""
, drvArgs ? { }
}:
craneLib.buildPackage (cargoDeps.commonArgs // {
  inherit pname cargoVendorDir;

  src = craneLib.cleanCargoSource ../.;
  cargoArtifacts = cargoDeps;

  doCheck = false;

  cargoExtraArgs = "-p ${pname} ${cargoArgs}";

} // drvArgs)
