{ self, inputs, ... }: {
  flake.nixosModules.starship = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myStarship
    ];
  };

  perSystem = { pkgs, lib, ... }:
    let
      starshipTomlDir = pkgs.writeTextDir "share/starship/config.toml" ''
        ${builtins.readFile ./config.toml}
      '';

      myStarship = pkgs.symlinkJoin {
        name = "starship";

        paths = [ pkgs.starship ];

        buildInputs = [ pkgs.makeWrapper ];

        postBuild = ''
          wrapProgram $out/bin/starship \
           --set STARSHIP_CONFIG "${starshipTomlDir}/share/starship/config.toml"
        '';
      };
    in {
    packages.myStarship = myStarship;
  };
}
