("(" @open ")" @close)
("[" @open "]" @close)
("{" @open "}" @close)
((string "\"" @open "\"" @close) (#set! rainbow.exclude))
