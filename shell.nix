let
  nixpkgsSrc = builtins.fetchGit {
    url = "https://github.com/NixOS/nixpkgs";
    ref = "nixos-24.05";
    rev = "3c80acabe4eef35d8662733c7e058907fa33a65d";
  };
  pkgs = import nixpkgsSrc {};
in

with pkgs;

mkShell {
  name = "faasten-dev";
  buildInputs = [ rustup rustfmt protobuf pkg-config openssl squashfs-tools-ng foreman python3Packages.protobuf ];
}
