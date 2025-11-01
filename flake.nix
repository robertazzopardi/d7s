{
  description = "d7s - Database client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Use Rust stable toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rustfmt" "clippy" "rust-src" ];
        };

        # System dependencies needed for the project
        buildInputs = with pkgs; [
          # DBus for Linux secret-service support (keyring crate)
          dbus
          # OpenSSL for secret-service encryption (if needed)
          openssl
          # pkg-config for finding libraries
          pkg-config
          # SQLite for rusqlite crate
          sqlite
          # PostgreSQL client libraries for tokio-postgres
          postgresql.lib
          # Clang for building the Rust project
          clang
          # Mold for faster builds
          mold
        ];

        # Libraries needed at runtime
        runtimeLibs = with pkgs; [
          sqlite
          openssl
          dbus
          postgresql.lib
        ];

        # Native build inputs (rustToolchain already includes cargo, rustfmt, clippy)
        nativeBuildInputs = with pkgs; [
          rustToolchain
          rust-analyzer
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          
          # Make libraries available at runtime
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath runtimeLibs}";

          shellHook = ''
            echo "d7s development environment"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
          '';

          # Set environment variables for Rust crates that need them
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          
          # For DBus secret-service on Linux
          PKG_CONFIG_PATH = "${pkgs.dbus.lib}/lib/pkgconfig:${pkgs.openssl.dev}/lib/pkgconfig";
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "d7s";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit buildInputs nativeBuildInputs;

          # Don't run tests during build (optional, remove if you want tests)
          doCheck = false;
        };

        # Formatter configuration
        formatter = pkgs.nixpkgs-fmt;
      });
}

