# `kserve`

very simple http file server with builtin markdown translation

## usage

download rust, clone, build and run

by default, it'll serve the current working directory, can be overridden with:

```
kserve /path/to/kserve/config/_kserve.toml
```

it'll create a config file at the path given.
go ahead and modify the config after it's been created because it contains which directory to serve. don't leave that at the default!

allows indexing by default, can be turned off

`_frame.html` allows you to define the html skeleton that'll be used for directories and markdown files \
insert `{%body%}` where you want the content to be inserted

## that's it

serves `.md` as html, serves everything else as-is on the filesystem
