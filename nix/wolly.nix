{ rustToolchain
, makeRustPlatform
, lib
, ...
}:

with builtins;
with lib;

let
  cargoManifest = fromTOML (readFile ../Cargo.toml);

  rustPlatform = makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };
in

rustPlatform.buildRustPackage {
  pname = cargoManifest.package.name;
  version = cargoManifest.package.version;

  src = with lib.fileset; toSource {
    root = ../.;
    fileset = unions [
      ../src
      ../Cargo.lock
      ../Cargo.toml
    ];
  };

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "miniarg-0.4.0" = "sha256-UIH38oGo6pUc6lN9JrhHsTjvmUoubxSw3+9+1vVyYSc=";
    };
  };

  meta = with lib; {
    inherit (cargoManifest.package) description homepage;
    license = licenses.gpl3Only;
    mainProgram = "wolly";
    platforms = platforms.all;
  };
}
