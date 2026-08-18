if vim.g.loaded_squid_editor_plugin then
  return
end
vim.g.loaded_squid_editor_plugin = true

local source = debug.getinfo(1, "S").source:sub(2)
local repository_root = vim.fs.dirname(vim.fs.dirname(source))
local runtime = repository_root .. "/editors/nvim"

if not vim.tbl_contains(vim.opt.runtimepath:get(), runtime) then
  vim.opt.runtimepath:append(runtime)
end

vim.filetype.add({
  extension = {
    squid = "squid",
  },
})
