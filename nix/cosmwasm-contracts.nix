{ lib
, craneLib
, stdenv
, coreutils
, findutils
, openssl
, libclang
, clang
, llvm
, lld
, binaryen
, libosmosistesttube
, libntrntesttube
, buildWasmBindgenCli
, fetchCrate
, rustPlatform
, libiconv
, cargoVendorDir
, contractNames
}:
let
  src = craneLib.cleanCargoSource ../.;

  commonArgs = {
    pname = "valence-cosmwasm-contracts";
    inherit src cargoVendorDir;
    strictDeps = true;
    doCheck = false;

    # OPENSSL_NO_VENDOR = 1;
    # OPENSSL_LIB_DIR = lib.makeLibraryPath [ openssl ];
    # OPENSSL_DIR = lib.getDev openssl;
    # LIBCLANG_PATH = lib.makeLibraryPath [ libclang ];
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";

    cargoExtraArgs = "-p ${lib.concatStringsSep " -p " contractNames} --lib --locked";

    # buildInputs = [
    # ] ++ lib.optionals stdenv.isDarwin [
    #   # Additional darwin specific inputs can be set here
    #   libiconv
    # ];

    nativeBuildInputs = [
      coreutils
      findutils
      # clang
      # llvm
      lld
      binaryen # for wasm-opt
    ];

    # Additional environment variables can be set directly
    # MY_CUSTOM_VAR = "some value";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

in
craneLib.buildPackage (commonArgs // {
  inherit cargoArtifacts;
  passthru = { inherit cargoArtifacts; };
  # Based on CosmWasm optimizer optimize.sh
  postInstall = ''
    for WASM in $out/lib/*.wasm; do
      [ -e "$WASM" ] || continue # https://superuser.com/a/519493

      OUT_FILENAME=$(basename "$WASM")
      echo "Optimizing $OUT_FILENAME ..."
      # --signext-lowering is needed to support blockchains runnning CosmWasm < 1.3. It can be removed eventually
      wasm-opt -Os --signext-lowering "$WASM" -o "$out/$OUT_FILENAME"
    done

    rm -rf $out/lib

    echo "Post-processing artifacts..."
    sha256sum -- $out/*.wasm | tee $out/checksums.txt
  '';
})
