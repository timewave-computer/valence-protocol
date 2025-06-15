{ lib
, buildGoModule
, fetchCrate
, patchelf
, stdenv
, go
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

    nativeBuildInputs = [ go ] ++ lib.optionals stdenv.isLinux [ patchelf ];

    sourceRoot = "${src.name}/${pname}";

    ldflags = [ "-w" ];

    buildPhase = ''
      runHook preBuild

      # Build shared library with appropriate extension for the platform
      ${if stdenv.isDarwin then 
        "go build -buildmode=c-shared -ldflags -w -o ${pname}.dylib main.go" 
      else 
        "go build -buildmode=c-shared -ldflags -w -o ${pname}.so main.go"
      }

      mkdir libwasmvm
      # Copy wasmvm libraries (try both extensions as wasmvm provides different formats)
      cp $(go env GOPATH)/pkg/mod/github.com/\!cosm\!wasm/wasmvm/v2@*/internal/api/*.so libwasmvm/ 2>/dev/null || true
      ${lib.optionalString stdenv.isDarwin ''
        cp $(go env GOPATH)/pkg/mod/github.com/\!cosm\!wasm/wasmvm/v2@*/internal/api/*.dylib libwasmvm/ 2>/dev/null || true
      ''}

      runHook postBuild
    '';

    postInstall = ''
      mkdir -p $out/lib $out/include $out/src

      mv libwasmvm $out/lib/wasmvm

      # Move the shared library with appropriate extension
      if [ -f "${pname}.dylib" ]; then
        mv ${pname}.dylib $out/lib/
        # Create .so symlink for compatibility with test tube build scripts
        ln -s ${pname}.dylib $out/lib/${pname}.so
      elif [ -f "${pname}.so" ]; then
        mv ${pname}.so $out/lib/
        # Create .dylib symlink for consistency
        ln -s ${pname}.so $out/lib/${pname}.dylib
      fi
      mv ${pname}.h $out/include/

      # Copy Go source files for test tube build scripts
      cp -r . $out/src/

      ${lib.optionalString stdenv.isLinux ''
        # Remove reference to libwasmvm in Go workspace in RPATH and replace with libwasmvm in $out/lib/wasmvm
        RPATH=$(patchelf --print-rpath $out/lib/${pname}.so | sed 's/^.*wasmvm[^:]*://'):$out/lib/wasmvm
        patchelf --set-rpath $RPATH $out/lib/${pname}.so
      ''}

      chmod +x $out/lib/* $out/lib/wasmvm/* || true
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
        if std::path::Path::new("${package}/lib/${pname}.dylib").exists() { \
          Command::new("cp") \
            .arg("-f") \
            .arg("${package}/lib/${pname}.dylib") \
            .arg(out_dir.join("${pname}.dylib")) \
            .spawn().unwrap().wait().unwrap(); \
        } else { \
          Command::new("cp") \
            .arg("-f") \
            .arg("${package}/lib/${pname}.so") \
            .arg(out_dir.join("${pname}.so")) \
            .spawn().unwrap().wait().unwrap(); \
        }' \
          build.rs

    '';
  };
in
package 