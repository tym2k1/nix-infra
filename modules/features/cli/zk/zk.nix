{ self, inputs, ... }: {
  flake.nixosModules.zk = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myZk
    ];
  };
perSystem = { pkgs, self', ... }: let
  zkConfig = pkgs.writeText "config.toml" ''
    # NOTEBOOK SETTINGS
    [notebook]
    dir = "~/Notes"

    # NOTE SETTINGS
    [note]

    # Language used when writing notes.
    # This is used to generate slugs or with date formats.
    language = "en"

    # The default title used for new note, if no `--title` flag is provided.
    default-title = "Untitled"

    # Template used to generate a note's filename, without extension.
    filename = "{{id}}-{{slug title}}"

    # The file extension used for the notes.
    extension = "md"

    # Template used to generate a note's content.
    # If not an absolute path, it is relative to .zk/templates/
    template = "default.md"

    # Configure random ID generation.

    # The charset used for random IDs.
    id-charset = "alphanum"

    # Length of the generated IDs.
    id-length = 4

    # Letter case for the random IDs.
    id-case = "lower"


    # GROUP OVERRIDES
    [group.journal]
    paths = ["journal/weekly", "journal/daily"]

    [group.journal.note]
    filename = "{{format-date now}}"


    # MARKDOWN SETTINGS
    [format.markdown]
    # Enable support for #hashtags
    hashtags = true

    # EXTERNAL TOOLS
    [tool]

    # Default editor used to open notes.
    editor = "hx"

    # Default shell used by aliases and commands.
    shell = "${pkgs.bash}/bin/bash"

    # Pager used to scroll through long output.
    pager = "${pkgs.glow}/bin/glow --pager"

    # Command used to preview a note during interactive fzf mode.
    fzf-preview = "${pkgs.glow}/bin/glow {-1}"

    # NAMED FILTERS
    [filter]
    recents = "--sort created- --created-after 'last two weeks'"

    # COMMAND ALIASES
    [alias]

    # Remove the autoprompt
    init = "zk init --no-input"

    # Edit the last modified note.
    edlast = "zk edit --limit 1 --sort modified- $@"

    # Edit the notes selected interactively among the notes created the last two weeks.
    recent = "zk edit --sort created- --created-after 'last two weeks' --interactive"

    # LSP (EDITOR INTEGRATION)
    # [lsp]

    # [lsp.diagnostics]
    # # Report titles of wiki-links as hints.
    # wiki-title = "hint"
    # # Warn for dead links between notes.
    # dead-link = "error"
    # # Warn when notes link here without backlinks.
    # missing-backlink = { level = "warning", position = "bottom" }
  '';

in {
  packages.myZk = pkgs.symlinkJoin {
    name = "zk";

    paths = [ pkgs.zk ];

    buildInputs = [ pkgs.makeWrapper ];

    postBuild = ''
      mkdir -p $out/share/zk
      ln -s ${zkConfig} $out/share/zk/config.toml
      wrapProgram $out/bin/zk \
      --set ZK_CONFIG_DIR "$out/share/zk"
    '';
    };
};
}
