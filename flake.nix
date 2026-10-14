{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
  };

  outputs = inputs: {
    packages = builtins.mapAttrs (system: pkgs: {
      llvm = pkgs.llvm;

      default = inputs.self.packages.${system}.llvm;
    }) inputs.nixpkgs.legacyPackages;
  };
}
