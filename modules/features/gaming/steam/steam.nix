{ self, inputs, ... }: {
  flake.nixosModules.steam = { pkgs, lib, ... }: {
    programs.steam = {
      enable = true;
      gamescopeSession = {
        enable = true;
      };
    };
    programs.gamemode.enable = true;
    environment.systemPackages = with pkgs; [
      mangohud
      protonup-ng
    ];
    environment.sessionVariables = {
      STEAM_EXTRA_COMPAT_TOOLS_PATHS =
        "\${HOME}/.steam/root/compatibilitytools.d";
    };
  };
}
