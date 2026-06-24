{ self, inputs, ... }: {
  flake.nixosModules.nix-index = { ... }: {
    imports = [
      inputs.nix-index-database.nixosModules.default
    ];
  };
}
