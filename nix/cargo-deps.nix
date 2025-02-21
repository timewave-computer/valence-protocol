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
craneLib.buildDepsOnly {
  inherit cargoVendorDir;
  src = craneLib.cleanCargoSource ../.;
  pname = "valence";
  strictDeps = true;
  doCheck = false;

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


}
