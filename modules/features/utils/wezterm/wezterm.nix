{ self, inputs, ... }: {
  flake.nixosModules.helix = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myWezterm
    ];
  };

   perSystem = { pkgs, lib, ... }:
       let
         weztermConfig = pkgs.writeTextDir "share/wezterm/wezterm.lua" ''
           ${builtins.readFile ./wezterm.lua}
         '';

         myWezterm = pkgs.symlinkJoin {
           name = "wezterm";

           paths = [ pkgs.wezterm ];

           buildInputs = [ pkgs.makeWrapper ];

           postBuild = ''
             wrapProgram $out/bin/wezterm \
              --add-flags "--config-file ${weztermConfig}/share/wezterm/wezterm.lua"
           '';
         };
       in {
       packages.myWezterm = myWezterm;
     };

}
