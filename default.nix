{}:

let
  nixpkgsSrc = builtins.fetchGit {
    url = "https://github.com/NixOS/nixpkgs";
    ref = "nixos-24.05";
    rev = "3c80acabe4eef35d8662733c7e058907fa33a65d";
  };
  pkgs = import nixpkgsSrc {};
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = "faasten";
  version = "0.1.0";

  buildType = "release";

  src = builtins.filterSource
      (path: type: !(type == "directory" && baseNameOf path == "target"))
          ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "arch-0.1.0" = "sha256-yyRIuYBz0B6oQw5G9piZ9y/0yghxmZrtGgWQOEHhwus=";
      "kvm-bindings-0.1.1" = "sha256-gqFUe8cFKcmS3uoFEf4wlMSQidXMR11pSU5tDqBDa9k=";
      "labeled-0.1.0" = "sha256-IWZhzD+NAZ+Mnh3Jzrt0wEne9BLLtPzrZR8v/gdXRNo=";
    };
  };

  nativeBuildInputs = [ pkgs.perl pkgs.gcc10 pkgs.openssl pkgs.pkg-config pkgs.protobuf pkgs.unzip pkgs.cmake ];
  buildInputs = [ pkgs.openssl ];

  meta = {
    description = "A user-centric function-as-a-service platform";
    homepage = "https://github.com/faasten/faasten";
  };
}
