{ lib
, craneLib
, cargoVendorDir
, pkg-config
, coreutils
, findutils
, openssl
, libclang
, clang
, llvm
, lld
}:
let
  commonArgs = {
    src = craneLib.cleanCargoSource ../.;
    inherit cargoVendorDir;

    LIBCLANG_PATH = lib.makeLibraryPath [ libclang ];

    buildInputs = [
      openssl
    ];

    nativeBuildInputs = [
      pkg-config
      coreutils
      findutils
      clang
      llvm
      lld
    ];
  };
in
craneLib.buildDepsOnly (commonArgs // {
  pname = "valence";
  strictDeps = true;
  doCheck = false;

  cargoExtraArgs = "--workspace --all-targets";

  passthru = { inherit commonArgs; };

})
