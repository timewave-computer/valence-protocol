{ lib
, foundry-bin
, solc
, unzip
, stdenv
, symlinkJoin
}:
let
  foundryTOML = builtins.fromTOML (builtins.readFile ../solidity/foundry.toml);
  soldeerLock = builtins.fromTOML (builtins.readFile ../solidity/soldeer.lock);

  dependencyDrvs = lib.map (dep: stdenv.mkDerivation {
    name = "source";
    src = if dep ? url then
      builtins.fetchurl {
        inherit (dep) url;
        name = "source.zip";
        sha256 = dep.checksum;
      }
    else if dep ? git then
      builtins.fetchGit {
        url = dep.git;
        name = "source";
        rev = foundryTOML.dependencies.${dep.name}.rev;
      }
    else throw "Soldeer dependency ${dep.name} does not have url or git attribute";
    dontUnpack = true;
    dontConfigure = true;
    dontBuild = true;
    nativeBuildInputs = [ unzip ];
    installPhase = ''
      mkdir -p $out
      ${if dep ? url then ''
        unzip -qq -d $out/${dep.name}-${dep.version} $src
      '' else ''
        cp -r $src $out/${dep.name}-${dep.version}
      ''}
    '';
  }) soldeerLock.dependencies;

  dependenciesDir = symlinkJoin {
    name = "valence-solidity-contracts-deps";
    paths = dependencyDrvs;
  };
in
stdenv.mkDerivation {
  name = "valence-solidity-contracts";
  src = lib.fileset.toSource {
    root = ../solidity;
    fileset = lib.fileset.unions [
      ../solidity/foundry.toml
      ../solidity/soldeer.lock
      ../solidity/remappings.txt
      ../solidity/src
      ../solidity/test
    ];
  };
  unpackPhase = ''
    cp -r $src source
  '';
  sourceRoot = "source";
  dontConfigure = true;
  nativeBuildInputs = [ foundry-bin solc ];
  buildPhase = ''
    runHook preBuild

    chmod 777 .
    ln -s ${dependenciesDir} dependencies

    export XDG_DATA_HOME=$(mktemp -d)
    mkdir -p $XDG_DATA_HOME/svm/${solc.version}
    ln -s ${solc}/bin/solc $XDG_DATA_HOME/svm/${solc.version}/${solc.name}

    forge build --sizes --offline
  '';

  checkPhase = ''
    forge test -vvv
  '';

  installPhase = ''
    mv out $out
  '';
}
