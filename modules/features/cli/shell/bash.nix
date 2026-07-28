{ self, inputs, ... }: {
  flake.nixosModules.bash = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myBash
    ];
  };
perSystem = { pkgs, self', ... }: let
  bashConfig = pkgs.writeText "bashrc" ''
    eval "$(${self'.packages.myStarship}/bin/starship init bash)"
  '';
in {
  packages.myBash = pkgs.symlinkJoin {
    name = "bash";

    paths = [ pkgs.bash ];

    buildInputs = [ pkgs.makeWrapper ];

    postBuild = ''
      wrapProgram $out/bin/bash \
        --add-flags "--rcfile ${bashConfig}"
    '';
  };
};
}
