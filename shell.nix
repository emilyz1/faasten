{ pkgs ? import <nixpkgs> {}, withFaasten ? false }:

with pkgs;

mkShell {
  name = "faasten-dev";
  buildInputs = [
    rustup rustfmt protobuf pkg-config openssl squashfs-tools-ng foreman python3Packages.protobuf lkl
  ] ++ (if withFaasten then [ (callPackage ./. {}) ] else []);
}
