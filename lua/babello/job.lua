local config = require("babello.config")

local M = {}

-- Runs `babello --json <args>` asynchronously and decodes the first result.
-- on_done(result, err) is always called via vim.schedule.
function M.run(args, on_done)
  local cmd = { config.options.bin, "--json" }
  vim.list_extend(cmd, args)

  local ok, err = pcall(vim.system, cmd, { text = true }, function(result)
    vim.schedule(function()
      if result.code ~= 0 then
        local stderr = vim.trim(result.stderr or "")
        on_done(nil, stderr ~= "" and stderr or "babello exited with an error")
        return
      end

      local decode_ok, decoded = pcall(vim.json.decode, result.stdout)
      if not decode_ok or type(decoded) ~= "table" or decoded[1] == nil then
        on_done(nil, "failed to parse babello output")
        return
      end

      on_done(decoded[1], nil)
    end)
  end)

  if not ok then
    vim.schedule(function()
      on_done(nil, "failed to run babello: " .. tostring(err))
    end)
  end
end

return M
