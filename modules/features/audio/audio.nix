{ self, inputs, ... }: {
  flake.nixosModules.audio = { pkgs, lib, ... }: {
    services.pipewire = {
      enable = true;
      alsa.enable = true;
      pulse.enable = true;
      jack.enable = true;
    };
    environment.systemPackages = with pkgs; [
        qpwgraph
      ];
  };
}
