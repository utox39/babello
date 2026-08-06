local config = require("babello.config")
local job = require("babello.job")
local selection = require("babello.selection")
local preview = require("babello.preview")

local M = {}

function M.setup(opts)
  config.setup(opts)

  local keymaps = config.options.keymaps or {}
  if keymaps.translate then
    vim.keymap.set("v", keymaps.translate, function()
      M.translate()
    end, { desc = "babello: translate selection" })
  end
  if keymaps.translate_as then
    vim.keymap.set("v", keymaps.translate_as, function()
      M.translate_as()
    end, { desc = "babello: translate selection as..." })
  end
  if keymaps.improve then
    vim.keymap.set("v", keymaps.improve, function()
      M.improve()
    end, { desc = "babello: improve selection" })
  end
end

-- build_args(to, from) -> extra CLI args (before the text argument itself)
local function run_action(build_args, opts, title)
  opts = opts or {}

  -- Read the selection up front: it must not depend on anything async.
  local range = selection.get()
  if range.text == "" then
    vim.notify("babello: no text selected", vim.log.levels.WARN)
    return
  end

  local to = opts.to or config.options.target_lang
  local from = opts.from or config.options.source_lang

  local args = build_args(to, from)
  table.insert(args, range.text)

  vim.notify("babello: talking to DeepL...", vim.log.levels.INFO)

  job.run(args, function(result, err)
    if err then
      vim.notify("babello: " .. err, vim.log.levels.ERROR)
      return
    end

    preview.show(range, result.text, {
      title = title,
      filetype = vim.bo[range.bufnr].filetype,
    })
  end)
end

-- opts: { to = "EN-US", from = "IT" } (both optional, override the config defaults for this call only)
function M.translate(opts)
  run_action(function(to, from)
    local args = { "--to", to }
    if from then
      vim.list_extend(args, { "--from", from })
    end
    return args
  end, opts, " babello: translate ")
end

-- opts: { to = "EN-US" } (optional, overrides the config default for this call only)
function M.improve(opts)
  run_action(function(to)
    return { "--improve", "--to", to }
  end, opts, " babello: improve ")
end

local function pick_language(on_pick)
  local choices = vim.deepcopy(config.options.favorite_languages or {})
  table.insert(choices, "Other...")

  vim.ui.select(choices, { prompt = "babello: target language" }, function(choice)
    if not choice then
      return
    end
    if choice == "Other..." then
      vim.ui.input({ prompt = "Language code (e.g. EN-US): " }, function(input)
        if input and input ~= "" then
          on_pick(input:upper())
        end
      end)
    else
      on_pick(choice)
    end
  end)
end

-- Prompts for a target language, then translates with it (one-shot override).
function M.translate_as()
  pick_language(function(lang)
    M.translate({ to = lang })
  end)
end

-- Prompts for a target language, then improves with it (one-shot override).
function M.improve_as()
  pick_language(function(lang)
    M.improve({ to = lang })
  end)
end

return M
