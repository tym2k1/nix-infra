{ self, inputs, ... }: {
  flake.nixosModules.waydroid = { ... }: {
    virtualisation.waydroid.enable = true;
  };
}
