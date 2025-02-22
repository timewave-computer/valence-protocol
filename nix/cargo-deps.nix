{ lib
, craneLib
, cargoVendorDir
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

    OPENSSL_NO_VENDOR = 1;
    OPENSSL_LIB_DIR = lib.makeLibraryPath [ openssl ];
    OPENSSL_DIR = lib.getDev openssl;
    LIBCLANG_PATH = lib.makeLibraryPath [ libclang ];

    nativeBuildInputs = [
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

  passthru = { inherit commonArgs; };

})
