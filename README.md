# Babello

- [Description](#description)
- [Requirements](#requirements)
- [Installation](#installation)
  - [Neovim plugin Installation](#neovim-plugin-installation)
- [Usage](#usage)
  - [Neovim plugin usage](#neovim-plugin-usage)
- [Contributing](#contributing)
- [Roadmap](#roadmap)
- [License](#license)

## Description

Babel: the Babel's tower
l: the L of [DeepL](https://www.deepl.com/en/translator)
o: Oh wow, this is beautiful OR Oh wow, it surprisingly works!

Babello is a 'bello' CLI translator that uses the DeepL API. There is also a [Neovim](https://neovim.io/) plugin!

## Requirements

- Rust
- A DeepL API Key

## Installation

```sh
# Clone the repo
git clone https://github.com/utox39/babello.git

# cd to the path
cd path/to/babello

# Build babello
cargo build --release

# Then move it somewhere in your $PATH. Here is an example:
mv ./target/release/babello ~/bin/
```

### Neovim plugin Installation

> [!NOTE]
> This only installs the Neovim plugin. The plugin shells out to the `babello` binary on your `$PATH`, so you still need to follow the [Installation](#installation) section above first.

```lua
return {
  {
    "utox39/babello",
    lazy = false,
    config = function()
      require("babello").setup({
        target_lang = "EN-US",
        -- your other overrides
      })
    end,
  },
}
```

## Usage

`DEEPL_API_KEY` must be set.

```text
A 'bello' CLI translator via DeepL

Usage: babello [OPTIONS] [TEXT]...

Arguments:
  [TEXT]...  The text to translate

Options:
      --from <FROM>          The language to translate from
      --to <TO>              The language to translate to
      --usage                Get the API usage
      --languages            Get the supported languages list
      --improve              Improve text by correcting spelling and grammar errors
      --generate-hook-warn   Print a git commit-msg hook that warns about spelling/grammar issues
      --generate-hook-block  Print a git commit-msg hook that blocks the commit on spelling/grammar issues
      --json                 Print the translation/improvement as JSON instead of human-readable text
  -h, --help                 Print help
  -V, --version              Print version
```

### Neovim plugin usage

Select the text that you want to translate/improve in visual mode, then you have
2 options: use a keymap or a command.

#### Keymaps

| Keymap        | Action                                             |
| ------------- | ---------------------------------------------------   |
| `<leader>bt`  | Translate the selection                               |
| `<leader>bT`  | Translate the selection, prompting for a language first |
| `<leader>bI`  | Improve (fix spelling/grammar of) the selection       |

#### Commands

| Command | Action |
| ---------------------- | --------------------------------------------------- |
| `:BabelloTranslate` | Translate the selection |
| `:BabelloTranslateAs` | Translate the selection, prompting for a language first |
| `:BabelloImprove` | Improve (fix spelling/grammar of) the selection |
| `:BabelloImproveAs` | Improve the selection, prompting for a language first |

Either one opens a preview window with the result. From there:

| Key           | Action                                  |
| ------------- | ---------------------------------------- |
| `<CR>` or `r` | Replace the selection with the result   |
| `p`           | Paste the result below the selection    |
| `q` or `<Esc>`| Cancel, discarding the result           |

#### Configuration

Pass any of these to `require("babello").setup({ ... })` to override the defaults:

```lua
require("babello").setup({
  bin = "babello",                                    -- must be on $PATH
  source_lang = nil,                                  -- nil = auto-detect
  target_lang = "EN-US",
  favorite_languages = { "EN-US", "IT", "DE", "FR" },  -- quick-pick list
  keymaps = {
    translate = "<leader>bt",
    translate_as = "<leader>bT",
    improve = "<leader>bI",
    -- set an entry to false to disable that keymap
  },
  preview = {
    width = 0.6,   -- relative to editor width
    height = 0.4,  -- relative to editor height
    border = "rounded",
  },
})
```

`DEEPL_API_KEY` must be set in the environment Neovim runs in, same as for the CLI.

## Roadmap

- Add a TUI (Ratatui of course /s)

## Contributing

Please see [CONTIBUTING](https://github.com/utox39/babello/blob/main/CONTRIBUTING.md). Thanks!

## License

MIT License. See: [LICENSE](https://github.com/utox39/babello/blob/main/LICENSE)
