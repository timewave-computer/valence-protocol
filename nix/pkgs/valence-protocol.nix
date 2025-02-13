{ self
, lib
, pkgs
, crane
, fenix
, stdenv
, openssl
, go
}:
let
  craneLib = crane.mkLib pkgs;
  src = craneLib.cleanCargoSource self;

  # Common arguments can be set here to avoid repeating them later
  commonArgs = {
    pname = "valence-protocol";
    inherit src;
    strictDeps = true;

    OPENSSL_NO_VENDOR = 1;
    OPENSSL_LIB_DIR = "${lib.getLib openssl}/lib";
    OPENSSL_DIR="${lib.getDev openssl}";

    nativeBuildInputs = [ go ];

    buildInputs = [
      # Add additional build inputs here
    ] ++ lib.optionals stdenv.isDarwin [
      # Additional darwin specific inputs can be set here
      pkgs.libiconv
    ];

    # Additional environment variables can be set directly
    # MY_CUSTOM_VAR = "some value";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

in
cargoArtifacts
# craneLib.buildPackage (commonArgs // {
#   inherit cargoArtifacts;
# })
