" SQUID is Markdown with SQL enclosed by line-oriented braces.
if exists('b:current_syntax')
  finish
endif

" Use the user's normal Markdown syntax as the base language.
runtime! syntax/markdown.vim

" Load the user's normal SQL syntax into a contained cluster.
let s:markdown_syntax = get(b:, 'current_syntax', '')
unlet! b:current_syntax
syntax include @squidSql syntax/sql.vim
unlet! b:current_syntax

" Fill gaps in Neovim's default SQL syntax (which currently defaults to its
" Oracle rules). These remain contained inside SQUID SQL blocks and use standard
" highlight groups, so the active colorscheme still controls their appearance.
syntax keyword squidSqlKeyword
      \ as from join inner left right full cross natural on using where
      \ group by having order limit offset distinct all union intersect except
      \ into values set returning with recursive case when then else end
      \ create alter drop table view index trigger insert update delete replace
      \ primary foreign key references unique check default conflict
      \ contained
syntax keyword squidSqlWordOperator
      \ and or not is in between like glob regexp match escape exists
      \ contained
syntax match squidSqlOperator
      \ '\%(||\|<<\|>>\|<=\|>=\|<>\|!=\|==\|[-+*/%=<>&|~]\)'
      \ contained
syntax match squidSqlDelimiter '[,.;()]' contained
syntax keyword squidSqlRelationKeyword
      \ from join into update table view
      \ nextgroup=squidSqlTableName skipwhite contained
syntax keyword squidSqlAsKeyword
      \ as
      \ nextgroup=squidSqlAlias skipwhite contained
syntax match squidSqlTableName
      \ '\%("[^"]\+"\|`[^`]\+`\|\[[^]]\+\]\|\h\w*\%(\.\h\w*\)*\)'
      \ contained
syntax match squidSqlAlias
      \ '\%("[^"]\+"\|`[^`]\+`\|\[[^]]\+\]\|\h\w*\)'
      \ contained

" Inline LIMIT 1 templates may occur among Markdown text.
syntax region squidSqlInline
      \ matchgroup=squidSqlDelimiter
      \ start='{'
      \ end='}'
      \ oneline
      \ keepend
      \ containedin=ALL
      \ contains=@squidSql,squidSqlKeyword,squidSqlRelationKeyword,squidSqlAsKeyword,squidSqlWordOperator,squidSqlOperator,squidSqlDelimiter

" Multiline and standalone SQL blocks are line-oriented.
syntax region squidSqlBlock
      \ matchgroup=squidSqlDelimiter
      \ start='^\s*{'
      \ end='}\s*$'
      \ keepend
      \ containedin=ALL
      \ contains=@squidSql,squidSqlKeyword,squidSqlRelationKeyword,squidSqlAsKeyword,squidSqlWordOperator,squidSqlOperator,squidSqlDelimiter

highlight default link squidSqlDelimiter Delimiter
highlight default link squidSqlKeyword Keyword
highlight default link squidSqlRelationKeyword Keyword
highlight default link squidSqlAsKeyword Keyword
highlight default link squidSqlWordOperator Operator
highlight default link squidSqlOperator Operator
highlight default link squidSqlTableName Type
highlight default link squidSqlAlias Identifier

let b:current_syntax = 'squid'
