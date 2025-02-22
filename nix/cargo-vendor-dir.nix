{ craneLib
, libosmosistesttube
, libntrntesttube
}:
craneLib.vendorCargoDeps {
  src = craneLib.cleanCargoSource ../.;

  overrideVendorCargoPackage = p: drv:
    if p.name == "osmosis-test-tube" then
      drv.overrideAttrs (_: {
        preInstall = libosmosistesttube.fixCargoBuildScript;
      })
    else if p.name == "neutron-test-tube" then
      drv.overrideAttrs (_: {
        preInstall = libntrntesttube.fixCargoBuildScript;
      })
    else if p.name == "injective-protobuf" then
      # injective-protobuf custom build script is unnecessary
      # and tries to write in vendor dir which is a read-only filesystem
      drv.overrideAttrs (_: {
        preInstall = ''
          rm build.rs
        '';
      })
    else
      drv
    ;
}
