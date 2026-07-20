{ self, inputs, ... }: {
  flake.nixosModules.lf = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myLf
    ];
  };

  perSystem = { pkgs, lib, ... }: let

    yankMarkdown = pkgs.writeShellScriptBin "yank-markdown" ''
      set -eu

      lang_of() {
        case "$1" in
          *.nix) echo nix ;;
          *.py) echo python ;;
          *.rs) echo rust ;;
          *.js) echo javascript ;;
          *.ts) echo typescript ;;
          *.tsx) echo tsx ;;
          *.jsx) echo jsx ;;
          *.sh) echo bash ;;
          *.zsh) echo zsh ;;
          *.fish) echo fish ;;
          *.json) echo json ;;
          *.yaml|*.yml) echo yaml ;;
          *.toml) echo toml ;;
          *.md) echo markdown ;;
          *.html) echo html ;;
          *.css) echo css ;;
          *.lua) echo lua ;;
          *.go) echo go ;;
          *.c) echo c ;;
          *.cpp|*.cc|*.cxx|*.hpp|*.h) echo cpp ;;
          *.bb|*.bbappend) echo bitbake ;;
          *) echo "" ;;
        esac
      }

      common_prefix() {
        prefix="$1"
        shift
        for path in "$@"; do
          while [ "''${path#"$prefix"}" = "$path" ]; do
            prefix="$(dirname "$prefix")"
            [ "$prefix" = "/" ] && break
          done
        done
        printf '%s\n' "$prefix"
      }

      files=
      for f in "$@"; do
        [ -f "$f" ] || continue
        if [ -z "''${files-}" ]; then
          files="$f"
        else
          files="$files
    $f"
        fi
      done

      [ -n "''${files-}" ] || exit 0

      first="$(printf '%s\n' "$files" | sed -n '1p')"

      git_root=
      if repo="$(${pkgs.git}/bin/git -C "$(dirname "$first")" rev-parse --show-toplevel 2>/dev/null)"; then
        all_in_repo=1
        while IFS= read -r f; do
          this_repo="$(${pkgs.git}/bin/git -C "$(dirname "$f")" rev-parse --show-toplevel 2>/dev/null || true)"
          [ "$this_repo" = "$repo" ] || all_in_repo=0
        done <<EOF
    $files
    EOF
        [ "$all_in_repo" -eq 1 ] && git_root="$repo"
      fi

      if [ -n "$git_root" ]; then
        base="$git_root"
      else
        set -- $(printf '%s\n' "$files")
        base="$(common_prefix "$@")"
      fi

      out="$(
        while IFS= read -r f; do
          lang="$(lang_of "$f")"
          rel="''${f#"$base"/}"
          [ "$f" = "$base" ] && rel="$(basename "$f")"
          printf '## File: `%s`\n\n' "$rel"
          printf '```%s\n' "$lang"
          cat -- "$f"
          printf '\n```\n\n'
        done <<EOF
    $files
    EOF
      )"

      printf '\e]52;c;%s\a' "$(
        printf '%s' "$out" | ${pkgs.coreutils}/bin/base64 | tr -d '\n'
      )" > /dev/tty
    '';

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
        map yc yank-content

        map yy copy
        cmd yank-path &printf '\e]52;c;%s\a' "$(printf '%s' "$fx" | base64 | tr -d '\n')" > /dev/tty
        cmd yank-name &printf '\e]52;c;%s\a' "$(printf '%s\n' "$fx" | while read -r line; do basename "$line"; done | base64 | tr -d '\n')" > /dev/tty
        cmd yank-content &${yankMarkdown}/bin/yank-markdown $fx

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
