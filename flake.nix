{
  description = "Wealthfolio - Portfolio tracking application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        libraries = with pkgs; [
          webkitgtk_4_1
          gtk3
          cairo
          gdk-pixbuf
          glib
          dbus
          openssl
          librsvg
        ];

        packages = with pkgs; [
          # Rust toolchain
          rust-bin.stable.latest.default

          # Build dependencies
          pkg-config
          openssl.dev

          # Tauri dependencies
          cargo-tauri
          webkitgtk_4_1
          gtk3
          cairo
          gdk-pixbuf
          glib
          dbus
          librsvg

          # Frontend dependencies
          nodejs_22
          pnpm

          # Additional utilities
          git
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = packages;

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"

            echo "🦀 Rust development environment loaded"
            echo "📦 pnpm version: $(pnpm --version)"
            echo "🔧 Node version: $(node --version)"
            echo "🔐 OpenSSL: ${pkgs.openssl.dev}"

            # Check if node_modules exists, if not, suggest running pnpm install
            if [ ! -d "node_modules" ]; then
              echo ""
              echo "⚠️  node_modules not found. Run: pnpm install"
            fi
          '';
        };
      }
    );
}
