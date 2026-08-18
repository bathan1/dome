local source = debug.getinfo(1, "S").source:sub(2)
local repository_root = vim.fs.dirname(vim.fs.dirname(source))

dofile(repository_root .. "/apps/squid/plugin/squid.lua")
