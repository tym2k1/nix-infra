{ self, inputs, ... }: {
  flake.nixosModules.kakoune = { pkgs, ... }: {
    environment.shellInit = ''
        EDITOR=kak
    '';
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myKakoune
      pkgs.wl-clipboard
    ];
  };


  perSystem = { pkgs, lib, ... }:
    let
      kakrcDir = pkgs.writeTextDir "share/kak/kakrc" ''
        ${builtins.readFile ./kakrc}
        # use system clipboard
        # hook global RegisterModified '"' %{ nop %sh{
        #   printf %s "$kak_main_reg_dquote" | ${pkgs.wl-clipboard}/bin/wl-copy > /dev/null 2>&1 &
        # }}

      '';

      colorsDir = pkgs.writeTextDir "share/kak/colors/base16-custom.kak" ''
        ${builtins.readFile ./base16.kak}
      '';

    plugins = with pkgs.kakounePlugins; [
      kakboard
    ];

      kakConfig = pkgs.symlinkJoin {
        name = "kak-config";
        paths = [ kakrcDir colorsDir ] ++ plugins;
      };

      myKakoune = pkgs.symlinkJoin {
        name = "my-kakoune";

        paths = [ pkgs.kakoune ];

        buildInputs = [ pkgs.makeWrapper ];

        postBuild = ''
          wrapProgram $out/bin/kak \
            --set KAKOUNE_CONFIG_DIR ${kakConfig}/share/kak
        '';
      };
    in {
    packages.myKakoune = myKakoune;
  };
}
