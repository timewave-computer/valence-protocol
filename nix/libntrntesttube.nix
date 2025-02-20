{ lib, libosmosistesttube }:
libosmosistesttube.override {
  pname = "libntrntesttube";
  version = "5.0.1";
  crateName = "neutron-test-tube";
  srcHash = "sha256-gJ1exKm3TSfOHM9ooaONLOrtvjKaBI15RcDu5MkawnA=";
  vendorHash = "sha256-gway1f2vjzMxPK4Ua38DjNEKtUrzlBBRYY4Q+SNZK/M=";
}
