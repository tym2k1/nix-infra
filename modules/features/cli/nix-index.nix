{ inputs, ... }:

{
  perSystem = { pkgs, system, ... }:
    let
      nixIndexPkgs = pkgs.extend inputs.nix-index-database.overlays.nix-index;
    in {
      packages.comma = nixIndexPkgs.callPackage
        "${inputs.nix-index-database}/comma-wrapper.nix"
        {};
    };

  flake.nixosModules.nix-index = { ... }: {
    imports = [
      inputs.nix-index-database.nixosModules.default
    ];
  };
}
