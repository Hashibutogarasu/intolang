{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
  };

  outputs =
    { self, nixpkgs }:
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
        in
        {
          default = pkgs.mkShell {
            packages = [
              llvm
              pkgs.libxml2
              pkgs.zlib
              pkgs.pkg-config
            ];

            LLVM_SYS_231_PREFIX = "${llvm.dev}";

            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
              pkgs.zlib
              pkgs.libxml2
              pkgs.stdenv.cc.cc.lib
            ];
          };
        }
      );
    };
}
