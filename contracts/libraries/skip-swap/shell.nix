(import (
  fetchTarball {
    url = "https://github.com/edolstra/flake-compat/archive/master.tar.gz";
    sha256 = "0m4gyjxj0ms0in633rkg2qwrpkiasfb6da3xdkdd8zx2ijcr55jy";
  }
) {
  src = ./.;
}).shellNix 