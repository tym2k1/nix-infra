{ self, inputs, ... }: {
  flake.nixosModules.lf = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myFish
    ];
  };

perSystem = { pkgs, self', ... }: let
    zellijFishCompletion = pkgs.runCommand "zellij-fish-completion" {
    nativeBuildInputs = [ self'.packages.myZellij ];
  } ''
    mkdir -p $out
    ${self'.packages.myZellij}/bin/zellij setup --generate-completion fish \
      > $out/completions.fish
  '';

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

    # '!!' binding
    function bind_bang
        switch (commandline -t)[-1]
            case "!"
                commandline -t -- $history[1]
                commandline -f repaint
            case "*"
                commandline -i !
        end
    end

    # '!$' binding
    function bind_dollar
        switch (commandline -t)[-1]
            case "!"
                commandline -f backward-delete-char history-token-search-backward
            case "*"
                commandline -i '$'
        end
    end

    function fish_user_key_bindings
        bind ! bind_bang
        bind '$' bind_dollar
    end

    # In case anyone does the cursed uninmaginable of setting locale to smth else than english
    set -gx LANG en_US.UTF-8
    set -gx LC_MESSAGES en_US.UTF-8
    set -e LC_ALL                 # make sure nothing overrides these

    source ${zellijFishCompletion}/completions.fish
  '';
in {
  packages.myFish = pkgs.symlinkJoin {
    name = "fish";

    paths = [ pkgs.fish ];

    buildInputs = [ pkgs.makeWrapper ];

    postBuild = ''
      wrapProgram $out/bin/fish \
        --set LOCALE_ARCHIVE ${pkgs.glibcLocales}/lib/locale/locale-archive \
        --add-flags "--init-command 'source ${fishConfig}'"
    '';
  };
};
}
