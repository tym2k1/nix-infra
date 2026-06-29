{ self, inputs, ... }: {
  flake.nixosModules.lf = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myFish
    ];
  };

perSystem = { pkgs, self', ... }: let
  fishConfig = pkgs.writeText "config.fish" ''
    set -gx EDITOR hx
    set -U fish_greeting
    export SHELL=$(command -v fish)
    alias e=$EDITOR

    function lf --wraps="lf" --description="lf - Terminal file manager (changing directory on exit)"
        # `command` is needed in case `lfcd` is aliased to `lf`.
        # Quotes will cause `cd` to not change directory if `lf` prints nothing to stdout due to an error.
        cd "$(command ${self'.packages.myLf}/bin/lf -print-last-dir $argv)"
    end
  '';
in {
  packages.myFish = pkgs.symlinkJoin {
    name = "fish";

    paths = [ pkgs.fish ];

    buildInputs = [ pkgs.makeWrapper ];

    postBuild = ''
      wrapProgram $out/bin/fish \
        --add-flags "--init-command 'source ${fishConfig}'"
    '';
  };
};
}
