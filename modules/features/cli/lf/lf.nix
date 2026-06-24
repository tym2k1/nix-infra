{ self, inputs, ... }: {
  flake.nixosModules.lf = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myLf
    ];
  };

  perSystem = { pkgs, lib, ... }: let

    lfrc = pkgs.writeText "lfrc" ''
        set autoquit
        set icons
        set ignorecase
        set info "size"
        set number
        set period 5
        set preview
        set scrolloff 10
        set sixel

        set drawbox true
        set roundbox true

        map <backspace2> set hidden!
        map e editor-open
        map E editor-open-select
        cmd editor-open $$EDITOR $(echo "$f" | tr '\n' ' ' | xargs -d ' ' -I {} sh -c 'if [ ! -d "{}" ]; then echo "{}"; fi')
        cmd editor-open-select $$EDITOR $(echo "$fx" | tr '\n' ' ' | xargs -d ' ' -I {} sh -c 'if [ ! -d "{}" ]; then echo "{}"; fi' | tr '\n' ' ')

        map d
        map dd cut
        map p
        map pp paste

        map w

        map y
        map yn yank-name
        map yp yank-path
        map yy copy
        cmd yank-name &${pkgs.wl-clipboard}/bin/wl-copy "$(echo $fx)"
        cmd yank-path &${pkgs.wl-clipboard}/bin/wl-copy "$(echo $fx | while read -r line; do basename "$line"; done)"

        map Q quit
        map q quit-and-cd

        cmd quit-and-cd &{{
          pwd > $LF_CD_FILE
          lf -remote "send $id quit"
        }}

        map do dragon-drop
        cmd dragon-drop &{{
          &${pkgs.dragon-drop}/bin/dragon-drop -a -x -T $(echo $fx)
        }}

        cmd recol %{{
            if [ $lf_width -le 60 ]; then
                lf -remote "send $id :set preview false; set ratios 1"
            elif [ $lf_width -le 120 ]; then
                lf -remote "send $id :set preview true; set ratios 1:1"
            else
                lf -remote "send $id :set preview true; set ratios 1:2:3"
            fi
        }}
        cmd on-focus-lost %{{
          lf -remote "send $id recol"
        }}
        cmd on-focus-gained %{{
          lf -remote "send $id recol"
        }}
        cmd on-redraw %{{
          lf -remote "send $id recol"
        }}

        set previewer ${pkgs.ctpv}/bin/ctpv

        set cleaner ${pkgs.ctpv}/bin/ctpvclear
        set cursorpreviewfmt "\033[7m"
        &${pkgs.ctpv}/bin/ctpv -s $id
        &${pkgs.ctpv}/bin/ctpvquit $id
    '';

    lfWithConfig = pkgs.symlinkJoin {
      name = "lf";

      paths = [ pkgs.lf ];

      buildInputs = [ pkgs.makeWrapper ];

      postBuild = ''
        mkdir -p $out/share/lf

        ln -s ${lfrc} $out/share/lf/lfrc

        wrapProgram $out/bin/lf \
          --add-flags "-config $out/share/lf/lfrc" \
          --set LF_ICONS "${lib.escapeShellArg (builtins.readFile ./icons)}"
      '';
    };

  in {
    packages.myLf = lfWithConfig;
  };
}
