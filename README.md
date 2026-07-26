# `kserve`

very simple http file server with builtin markdown translation

## usage

download rust, clone, build and run

by default, it'll create a config in the current working directory, can be overridden with:

```
kserve /path/to/kserve/config/_kserve.toml
```

it'll create a config file at the path given.
go ahead and modify the config after it's been created because it contains which directory to serve. don't leave that at the default!

## config

by default: (as of right now) \
configure to the requirements and security sensitivity of your deployment

```toml
addr = "0.0.0.0"
port = 9090
serve_directory = "."
allow_indexing = true
```

## frame

`_frame.html` allows you to define the html skeleton that'll be used for directories and markdown files \
insert `{%body%}` where you want the content to be inserted

## markdown translation

serves `.md` as html, serves everything else as-is on the filesystem
