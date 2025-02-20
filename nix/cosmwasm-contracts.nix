{ lib
, pkgs
, crane
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
}:
let
  craneLib = (crane.mkLib pkgs);

  src = craneLib.cleanCargoSource ../.;

  cargoTOML = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  contracts = builtins.attrNames (lib.filterAttrs (name: value:
    lib.hasPrefix "valence" name && lib.hasPrefix "contracts" value.path
  ) cargoTOML.workspace.dependencies);

  commonBaseArgs = {
    pname = "valence-cosmwasm-contracts";
    inherit src;
    strictDeps = true;
    doCheck = false;

    OPENSSL_NO_VENDOR = 1;
    OPENSSL_LIB_DIR = lib.makeLibraryPath [ openssl ];
    OPENSSL_DIR = lib.getDev openssl;
    LIBCLANG_PATH = lib.makeLibraryPath [ libclang ];
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";

    cargoExtraArgs = "-p ${lib.concatStringsSep " -p " contracts} --lib --locked";

    buildInputs = [
      libosmosistesttube
      libntrntesttube
      # Add additional build inputs here
    ] ++ lib.optionals stdenv.isDarwin [
      # Additional darwin specific inputs can be set here
      libiconv
    ];

    nativeBuildInputs = [
      coreutils
      findutils
      clang
      llvm
      lld
      binaryen # for wasm-opt
    ];

    # Additional environment variables can be set directly
    # MY_CUSTOM_VAR = "some value";
  };

  cargoVendorDir = craneLib.vendorCargoDeps (commonBaseArgs // {
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
  });

  commonArgs = commonBaseArgs // { inherit cargoVendorDir; };

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
