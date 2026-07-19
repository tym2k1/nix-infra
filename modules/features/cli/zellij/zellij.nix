{ self, inputs, ... }: {
  flake.nixosModules.zellij = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myZellij
    ];
  };


  perSystem = { pkgs, lib, ... }:
    let
      zellijConfDir = pkgs.writeTextDir "share/zellij/config.kdl" ''
        ${builtins.readFile ./config.kdl}
      '';

      myZellij = pkgs.symlinkJoin {
        name = "zellij";

        paths = [ pkgs.zellij ];

        buildInputs = [ pkgs.makeWrapper ];

        postBuild = ''
          wrapProgram $out/bin/zellij \
           --add-flags "--config ${zellijConfDir}/share/zellij/config.kdl"
        '';
      };
    in {
    packages.myZellij = myZellij;
  };
}
