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

        rustToolchain = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        version = "3.5.2";

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

        # ─── Web mode: static frontend bundle (`dist`) ───────────────────────
        # Mirrors the Dockerfile's frontend stage: `BUILD_TARGET=web` +
        # `pnpm --filter frontend... build`, whose vite outDir is the repo-root
        # `dist/`. Built without Wealthfolio Connect (the Connect keys are
        # optional `option_env!`/runtime envs, omitted for self-host).
        wealthfolio-frontend = pkgs.stdenv.mkDerivation (finalAttrs: {
          pname = "wealthfolio-frontend";
          inherit version;
          src = ./.;

          pnpmDeps = pkgs.pnpm_9.fetchDeps {
            inherit (finalAttrs) pname version src;
            fetcherVersion = 2;
            # Resolve in the nixos-config context:
            #   nix build '.#nixosConfigurations.nixos.pkgs.wealthfolio-frontend.pnpmDeps'
            hash = "sha256-hBPy9Z8ULSEobF9GPoyPGtANf1hFn4j4gp0fXfDdGV4=";
          };

          nativeBuildInputs = [ pkgs.nodejs_22 pkgs.pnpm_9 pkgs.pnpm_9.configHook ];

          env = {
            CI = "1";
            BUILD_TARGET = "web";
          };

          buildPhase = ''
            runHook preBuild
            pnpm --filter frontend... build
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            cp -r dist $out
            runHook postInstall
          '';
        });

        # ─── Web mode: Axum backend binary (apps/server) ─────────────────────
        # Mirrors the Dockerfile's backend stage: builds only the
        # `wealthfolio-server` workspace member. Native crypto deps need cmake +
        # perl + nasm (aws-lc-rs via jsonwebtoken, ring via reqwest/rustls);
        # openssl + sqlite are system-linked (OPENSSL_NO_VENDOR).
        wealthfolio-server = rustPlatform.buildRustPackage {
          pname = "wealthfolio-server";
          inherit version;
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              # tauri plugin git dep — only used by apps/tauri (not built here),
              # but cargoLock vendors the whole lockfile so it needs a hash.
              "tauri-plugin-barcode-scanner-2.4.5" =
                "sha256-xuyC/um19uifPNCLUymVTnaPQ8flaSRmK2j3OyFhTbs=";
            };
          };

          buildAndTestSubdir = "apps/server";

          # Enable MCP-server tools in the Assistant (apps/server `mcp` ->
          # wealthfolio-ai `mcp` -> rig `rmcp`). Drop this line to disable.
          buildFeatures = [ "assistant-mcp" ];

          nativeBuildInputs = [ pkgs.pkg-config pkgs.cmake pkgs.perl pkgs.nasm ];
          buildInputs = [ pkgs.openssl pkgs.sqlite ];

          dontUseCmakeConfigure = true;
          env.OPENSSL_NO_VENDOR = "1";

          doCheck = false;

          meta.mainProgram = "wealthfolio-server";
        };
      in
      {
        packages = {
          default = wealthfolio-server;
          server = wealthfolio-server;
          frontend = wealthfolio-frontend;
        };

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
    ) // {
      overlays.default = final: prev: {
        wealthfolio = self.packages.${final.system}.server;
        wealthfolio-frontend = self.packages.${final.system}.frontend;
      };
    };
}
