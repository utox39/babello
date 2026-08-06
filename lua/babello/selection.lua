local M = {}

local function line_length(bufnr, row)
  local line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1]
  return line and #line or 0
end

-- Reads the last visual selection ('<, '> marks) from the current buffer.
-- Supports charwise ('v') and linewise ('V') selections.
function M.get()
  local bufnr = vim.api.nvim_get_current_buf()
  local start_pos = vim.fn.getpos("'<")
  local end_pos = vim.fn.getpos("'>")

  local start_row, start_col = start_pos[2] - 1, start_pos[3] - 1
  local end_row, end_col = end_pos[2] - 1, end_pos[3] - 1

  if vim.fn.visualmode() == "V" then
    start_col = 0
    end_col = line_length(bufnr, end_row)
  else
    local end_line_len = line_length(bufnr, end_row)
    end_col = math.min(end_col + 1, end_line_len)
  end

  local lines = vim.api.nvim_buf_get_text(bufnr, start_row, start_col, end_row, end_col, {})

  return {
    bufnr = bufnr,
    start_row = start_row,
    start_col = start_col,
    end_row = end_row,
    end_col = end_col,
    lines = lines,
    text = table.concat(lines, "\n"),
  }
end

-- Overwrites the given range with `text` (may contain embedded newlines).
function M.replace(range, text)
  local lines = vim.split(text, "\n", { plain = true })
  vim.api.nvim_buf_set_text(
    range.bufnr,
    range.start_row,
    range.start_col,
    range.end_row,
    range.end_col,
    lines
  )
end

-- Inserts `text` as new lines immediately after the range's end line.
function M.paste_below(range, text)
  local lines = vim.split(text, "\n", { plain = true })
  vim.api.nvim_buf_set_lines(range.bufnr, range.end_row + 1, range.end_row + 1, false, lines)
end

return M
