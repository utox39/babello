local config = require("babello.config")
local selection = require("babello.selection")

local M = {}

-- Shows `text` in a floating scratch window. <CR>/r replaces `range` with it,
-- p pastes it below `range`, q/<Esc> discards it.
function M.show(range, text, opts)
  opts = opts or {}
  local lines = vim.split(text, "\n", { plain = true })

  local buf = vim.api.nvim_create_buf(false, true)
  vim.bo[buf].buftype = "nofile"
  vim.bo[buf].bufhidden = "wipe"
  vim.bo[buf].filetype = opts.filetype or ""
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].modifiable = false

  local ui = vim.api.nvim_list_uis()[1] or { width = 80, height = 24 }
  local pconf = config.options.preview
  local width = math.max(20, math.floor(ui.width * pconf.width))
  local height = math.max(3, math.floor(ui.height * pconf.height))

  local win = vim.api.nvim_open_win(buf, true, {
    relative = "editor",
    width = width,
    height = height,
    row = math.floor((ui.height - height) / 2),
    col = math.floor((ui.width - width) / 2),
    border = pconf.border,
    title = opts.title or " babello ",
    title_pos = "center",
    footer = " <CR>/r replace   p paste   q/<Esc> cancel ",
    footer_pos = "center",
    style = "minimal",
  })

  local function close()
    if vim.api.nvim_win_is_valid(win) then
      vim.api.nvim_win_close(win, true)
    end
  end

  local function map(lhs, fn)
    vim.keymap.set("n", lhs, fn, { buffer = buf, nowait = true, silent = true })
  end

  map("<CR>", function()
    selection.replace(range, text)
    close()
  end)
  map("r", function()
    selection.replace(range, text)
    close()
  end)
  map("p", function()
    selection.paste_below(range, text)
    close()
  end)
  map("q", close)
  map("<Esc>", close)
end

return M
