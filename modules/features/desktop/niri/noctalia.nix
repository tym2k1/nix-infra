{ self, inputs, ... }: {
  flake.nixosModules.noctalia = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myNoctalia
    ];
  };
  perSystem = { pkgs, lib, ... }:
    let
      # wallpaperDirectory = "/home/user/Pictures/Wallpapers";
      config = pkgs.writeText "noctalia-config.toml" (
        # lib.replaceStrings
        #   [ "@wallpaper-directory@" ]
        #   [ wallpaperDirectory ]
          (builtins.readFile ./noctalia-config.toml)
      );
    in
    {
      packages.myNoctalia = pkgs.symlinkJoin {
        name = "noctalia";

        paths = [ pkgs.noctalia ];

        nativeBuildInputs = [ pkgs.makeWrapper ];

        postBuild = ''
            mkdir -p $out/custom-config/noctalia
            cp ${config} $out/custom-config/noctalia/noctalia-config.toml

            wrapProgram $out/bin/noctalia \
              --set NOCTALIA_CONFIG_HOME "$out/custom-config/noctalia"
        '';
      };
    };
}
