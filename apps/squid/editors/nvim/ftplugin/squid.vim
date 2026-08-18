if exists('b:did_ftplugin')
  finish
endif

" Match the editing behavior users already have for Markdown.
runtime! ftplugin/markdown.vim

setlocal commentstring=<!--%s-->
let b:did_ftplugin = 1

" Tree-sitter-first configurations commonly leave the legacy syntax loader
" disabled. SQUID still needs it to compose the installed Markdown and SQL
" syntax definitions, so load this buffer's syntax explicitly when necessary.
if !exists('b:current_syntax')
  runtime! syntax/squid.vim
endif
