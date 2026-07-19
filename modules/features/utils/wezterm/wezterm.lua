-- Pull in the wezterm API
local wezterm = require 'wezterm'

-- This will hold the configuration.
local config = wezterm.config_builder()

-- CONFIG
config.hide_tab_bar_if_only_one_tab = true
config.window_decorations = "NONE"
config.front_end = "WebGpu"
config.use_fancy_tab_bar = true
--

-- Finally, return the configuration to wezterm:
return config
