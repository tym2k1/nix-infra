{ self, inputs, ... }: {
  flake.nixosModules.desktop-utils = { pkgs, lib, ... }: {
    environment.systemPackages = with pkgs; [
        librewolf
        inkscape
        gimp
        legcord
        kicad
        orca-slicer
        zathura
        freecad
        kdePackages.okular
        self.packages.${pkgs.stdenv.hostPlatform.system}.myWezterm
      ];
    };
}
