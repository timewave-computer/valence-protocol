{ self
, lib
, pkgs
, crane
, stdenv
, openssl
, libclang
, clang
, llvm
, libosmosistesttube
, libntrntesttube
, buildWasmBindgenCli
, fetchCrate
, rustPlatform
, libiconv
}:
let

  # rustToolchainFor = p: p.rust-bin.stable.latest.default.override {
  #   # Set the build targets supported by the toolchain,
  #   # wasm32-unknown-unknown is required for trunk
  #   targets = [ "wasm32-unknown-unknown" ];
  # };
  craneLib = (crane.mkLib pkgs); #.overrideToolchain rustToolchainFor;

  src = craneLib.cleanCargoSource self;

  # Common arguments can be set here to avoid repeating them later
  commonBaseArgs = {
    pname = "valence-protocol";
    inherit src;
    strictDeps = true;
    doCheck = false;

    # We must force the target, otherwise cargo will attempt to use your native target

    OPENSSL_NO_VENDOR = 1;
    OPENSSL_LIB_DIR = lib.makeLibraryPath [ openssl ];
    OPENSSL_DIR = lib.getDev openssl;
    LIBCLANG_PATH = lib.makeLibraryPath [ libclang ];

    cargoExtraArgs = "--lib --locked";

    buildInputs = [
      libosmosistesttube
      libntrntesttube
      # Add additional build inputs here
    ] ++ lib.optionals stdenv.isDarwin [
      # Additional darwin specific inputs can be set here
      libiconv
    ];

    nativeBuildInputs = [
      clang
      llvm
    ];

    # Additional environment variables can be set directly
    # MY_CUSTOM_VAR = "some value";
  };

  getTestTubeInfo = name: if name == "neutron-test-tube" then
    { package = libntrntesttube; name = "libntrntesttube"; }
    else
    { package = libosmosistesttube; name = "libosmosistesttube"; };

  cargoVendorDir = craneLib.vendorCargoDeps (commonBaseArgs // {
    overrideVendorCargoPackage = p: drv:
      if lib.elem p.name [ "osmosis-test-tube" "neutron-test-tube" ] then
        drv.overrideAttrs (_: {
          preInstall = let info = getTestTubeInfo p.name; in ''
            sed -i '/let out_dir =.*/a \
            Command::new("cp") \
              .arg("-f") \
              .arg("${info.package}/include/${info.name}.h") \
              .arg(out_dir.join("${info.name}.h")) \
              .spawn().unwrap().wait().unwrap();' \
              build.rs

            sed -i '/let out_dir =.*/a \
            Command::new("cp") \
              .arg("-f") \
              .arg("${info.package}/lib/${info.name}.so") \
              .arg(out_dir.join("${info.name}.so")) \
              .spawn().unwrap().wait().unwrap();' \
              build.rs
          '';
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

  wasm-bindgen-cli = buildWasmBindgenCli rec {
    src = fetchCrate {
      pname = "wasm-bindgen-cli";
      version = "0.2.95";
      hash = "sha256-prMIreQeAcbJ8/g3+pMp1Wp9H5u+xLqxRxL+34hICss=";
      # hash = lib.fakeHash;
    };

    cargoDeps = rustPlatform.fetchCargoVendor {
      inherit src;
      inherit (src) pname version;
      hash = "sha256-+h87/onAdpG0Jmeuy0Wd2djLFOAyaE52cAfxDVVcgP8=";
      # hash = lib.fakeHash;
    };
  };

in
craneLib.buildPackage (commonArgs // {
  inherit cargoArtifacts wasm-bindgen-cli;
  CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
})
