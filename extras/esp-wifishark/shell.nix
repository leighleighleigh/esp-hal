{ pkgs ? import <nixpkgs> {}}:
let 
  rustOverlay = builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz";
  pkgs = import <nixpkgs> {
    overlays = [ (import rustOverlay) ];
  };
  rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
in
pkgs.mkShell rec {
    name = "esp-rs-nix";

    buildInputs = [
        rust
        pkgs.rustup 
        pkgs.rust-analyzer
        pkgs.pkg-config 
        pkgs.stdenv.cc 
        pkgs.bacon 
        pkgs.systemdMinimal 
        pkgs.just 
        pkgs.lunarvim 
        pkgs.inotify-tools
        pkgs.picocom
        pkgs.fzf
        pkgs.zlib
        pkgs.colordiff
    ];

    shellHook = ''
    # custom bashrc stuff
    export PS1_PREFIX="(esp-rs)"
    . ~/.bashrc
    '';
}
