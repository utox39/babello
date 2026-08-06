local M = {}

M.defaults = {
	-- Must be on $PATH. DEEPL_API_KEY is read by the babello binary itself.
	bin = "babello",
	-- nil = auto-detect
	source_lang = nil,
	target_lang = "EN-US",
	-- Quick-pick list offered by translate_as()/improve_as()
	favorite_languages = { "EN-US", "IT", "DE", "FR" },
	-- Set an entry to false/nil to skip that keymap
	keymaps = {
		translate = "<leader>bt",
		translate_as = "<leader>bT",
		improve = "<leader>bI",
	},
	preview = {
		width = 0.6,
		height = 0.4,
		border = "rounded",
	},
}

M.options = vim.deepcopy(M.defaults)

function M.setup(opts)
	M.options = vim.tbl_deep_extend("force", vim.deepcopy(M.defaults), opts or {})
end

return M
