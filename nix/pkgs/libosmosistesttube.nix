{ lib
, buildGoModule
, fetchFromGitHub
, patchelf
}:
let
  src = fetchFromGitHub {
    owner = "osmosis-labs";
    repo = "test-tube";
    rev = "a68e939";
    hash = "sha256-eU+wZRZ+qQehigT8HYF61leW0s2+yw2qvZve4FXLRZ8=";
    sparseCheckout = [ "packages/osmosis-test-tube/libosmosistesttube/" ];
  };
in
buildGoModule {
  inherit src;
  pname = "libosmosistesttube";
  version = "unstable-2024-10-22";
  sourceRoot = "${src.name}/packages/osmosis-test-tube/libosmosistesttube/";

  ldflags = [ "-w" ];

  buildPhase = ''
    runHook preBuild

    go build -buildmode=c-shared -ldflags -w -o libosmosistesttube.so main.go

    mkdir libwasmvm
    cp $GOPATH/pkg/mod/github.com/\!cosm\!wasm/wasmvm/v2@v2.1.2/internal/api/*.so libwasmvm/

    runHook postBuild
  '';

  postInstall = ''
    mkdir -p $out/lib/

    mv libwasmvm $out/lib/wasmvm
    chmod +x $out/lib/wasmvm/*

    mv libosmosistesttube.so $out/lib/
    # Remove reference to libwasmvm in GOPATH in RPATH and replace with libwasmvm in $out/lib/wasmvm
    RPATH=$(patchelf --print-rpath $out/lib/libosmosistesttube.so | sed 's/^.*wasmvm[^:]*://'):$out/lib/wasmvm
    patchelf --set-rpath $RPATH $out/lib/libosmosistesttube.so
  '';


  proxyVendor = true; # Ensures dependencies are vendored correctly.
  vendorHash = "sha256-T+tf6G63BOOEn9s89mRPtDqC24JsG05TPpPQdcEhDBE=";
}
