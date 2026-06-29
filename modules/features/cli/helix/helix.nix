{ self, inputs, ... }: {
  flake.nixosModules.helix = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myHelix
    ];
  };


  perSystem = { pkgs, lib, ... }:
    let
      hxTomlDir = pkgs.writeTextDir "share/hx/config.toml" ''
        ${builtins.readFile ./config.toml}
      '';

      myHelix = pkgs.symlinkJoin {
        name = "hx";

        paths = [ pkgs.helix ];

        buildInputs = [ pkgs.makeWrapper ];

        postBuild = ''
          wrapProgram $out/bin/hx \
           --add-flags "--config ${hxTomlDir}/share/hx/config.toml"
        '';
      };
    in {
    packages.myHelix = myHelix;
  };
}
