{ rustPlatform
, lib
, ...
}:

let
  cargoManifest = lib.importTOML ../Cargo.toml;
in

rustPlatform.buildRustPackage {
  pname = cargoManifest.package.name;
  version = cargoManifest.package.version;

  src = with lib.fileset; toSource {
    root = ../.;
    fileset = unions [
      ../.cargo
      ../src
      ../Cargo.lock
      ../Cargo.toml
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  meta = with lib; {
    inherit (cargoManifest.package) description homepage;
    license = licenses.gpl3Only;
    mainProgram = "wolly";
    platforms = platforms.all;
  };
}
