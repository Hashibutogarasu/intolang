{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
    codex-cli-nix.url = "github:sadjow/codex-cli-nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      codex-cli-nix,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          llvm = pkgs.llvmPackages_23.llvm;
          default = self.packages.${system}.llvm;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          llvm = pkgs.llvmPackages_23.llvm;
          codex = codex-cli-nix.packages.${system}.default;
        in
        {
          llvm = pkgs.mkShell {
            packages = [
              llvm
              pkgs.llvmPackages_23.lld
              pkgs.cargo
              pkgs.rustc
              pkgs.clippy
              pkgs.rustfmt
              pkgs.git
              pkgs.libxml2
              pkgs.zlib
              pkgs.pkg-config
            ];

            LLVM_SYS_231_PREFIX = "${llvm.dev}";

            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
              llvm
              pkgs.rustc
              pkgs.zlib
              pkgs.libxml2
              pkgs.stdenv.cc.cc.lib
            ];

            shellHook = ''
              projectRoot="$(git -C "$PWD" rev-parse --show-toplevel 2>/dev/null || pwd -P)"
              export CARGO_TARGET_DIR="$projectRoot/dist"
            '';
          };

          default = pkgs.mkShell {
            packages = [
              codex
              llvm
              pkgs.llvmPackages_23.lld
              pkgs.cargo
              pkgs.rustc
              pkgs.clippy
              pkgs.rustfmt
              pkgs.git
              pkgs.yt-dlp
              pkgs.ffmpeg
              pkgs.libxml2
              pkgs.zlib
              pkgs.pkg-config
              pkgs.wayland
              pkgs.wayland-protocols
              pkgs.wayland-scanner
              pkgs.weston
              pkgs.vulkan-headers
              pkgs.vulkan-loader
              pkgs.SDL2
            ];

            LLVM_SYS_231_PREFIX = "${llvm.dev}";

            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
              llvm
              pkgs.rustc
              pkgs.zlib
              pkgs.libxml2
              pkgs.stdenv.cc.cc.lib
              pkgs.wayland
              pkgs.vulkan-loader
              pkgs.SDL2
            ];

            shellHook = ''
              projectRoot="$(git -C "$PWD" rev-parse --show-toplevel 2>/dev/null || pwd -P)"
              export CARGO_TARGET_DIR="$projectRoot/dist"
              export CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY=1
              export PATH="$CARGO_TARGET_DIR/debug:$PATH"
              mkdir -p "$CARGO_TARGET_DIR/debug"

              if cargo build --quiet --manifest-path "$projectRoot/Cargo.toml" --bin intolang; then
                if ! ln -sf intolang "$CARGO_TARGET_DIR/debug/into"; then
                  printf '%s\n' 'warning: could not create the `into` command symlink' >&2
                fi
              else
                printf '%s\n' 'warning: could not build the latest Into binary; using the existing binary if available' >&2
                if [ -x "$CARGO_TARGET_DIR/debug/intolang" ]; then
                  ln -sf intolang "$CARGO_TARGET_DIR/debug/into" || true
                elif [ -L "$CARGO_TARGET_DIR/debug/into" ]; then
                  rm "$CARGO_TARGET_DIR/debug/into"
                fi
              fi
              true
            '';
          };
        }
      );
    };
}
