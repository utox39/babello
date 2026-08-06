if vim.g.loaded_babello then
  return
end
vim.g.loaded_babello = true

vim.api.nvim_create_user_command("BabelloTranslate", function()
  require("babello").translate()
end, { range = true, desc = "Translate the visual selection with babello" })

vim.api.nvim_create_user_command("BabelloTranslateAs", function()
  require("babello").translate_as()
end, { range = true, desc = "Translate the visual selection with a chosen language" })

vim.api.nvim_create_user_command("BabelloImprove", function()
  require("babello").improve()
end, { range = true, desc = "Fix spelling/grammar of the visual selection with babello" })

vim.api.nvim_create_user_command("BabelloImproveAs", function()
  require("babello").improve_as()
end, { range = true, desc = "Fix spelling/grammar of the visual selection with a chosen language" })
