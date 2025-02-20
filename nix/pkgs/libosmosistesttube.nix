{ lib
, buildGoModule
, fetchCrate
, patchelf
, pname ? "libosmosistesttube"
, version ? "26.0.1"
, crateName ? "osmosis-test-tube"
, srcHash ? "sha256-yt7fv9kDVA5kJkrDWnzjEkKzIOvl+nvafq2Z8d79Rw8="
, vendorHash ? "sha256-T+tf6G63BOOEn9s89mRPtDqC24JsG05TPpPQdcEhDBE="
}:
let
  src = fetchCrate {
    pname = crateName;
    inherit version;
    hash = srcHash;
  };
  # create package inside let so that passthru can depend on final result
  package = buildGoModule {
    inherit pname version src vendorHash;

    sourceRoot = "${src.name}/${pname}";

    ldflags = [ "-w" ];

    buildPhase = ''
      runHook preBuild

      go build -buildmode=c-shared -ldflags -w -o ${pname}.so main.go

      mkdir libwasmvm
      cp $GOPATH/pkg/mod/github.com/\!cosm\!wasm/wasmvm/v2@*/internal/api/*.so libwasmvm/

      runHook postBuild
    '';

    postInstall = ''
      mkdir -p $out/lib $out/include

      mv libwasmvm $out/lib/wasmvm

      mv ${pname}.so $out/lib/
      mv ${pname}.h $out/include/

      # Remove reference to libwasmvm in GOPATH in RPATH and replace with libwasmvm in $out/lib/wasmvm
      RPATH=$(patchelf --print-rpath $out/lib/${pname}.so | sed 's/^.*wasmvm[^:]*://'):$out/lib/wasmvm
      patchelf --set-rpath $RPATH $out/lib/${pname}.so

      chmod +x $out/lib/* $out/lib/wasmvm/*
    '';

    proxyVendor = true; # Ensures dependencies are vendored correctly.

    passthru.fixCargoBuildScript = ''
      sed -i '/let out_dir =.*/a \
        Command::new("cp") \
          .arg("-f") \
          .arg("${package}/include/${pname}.h") \
          .arg(out_dir.join("${pname}.h")) \
          .spawn().unwrap().wait().unwrap();' \
          build.rs

        sed -i '/let out_dir =.*/a \
        Command::new("cp") \
          .arg("-f") \
          .arg("${package}/lib/${pname}.so") \
          .arg(out_dir.join("${pname}.so")) \
          .spawn().unwrap().wait().unwrap();' \
          build.rs

    '';
  };
in
package
