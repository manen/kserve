# `kserve`

very simple http file server with builtin markdown translation

## usage

download rust, clone, build and run

by default, it'll serve the current working directory, can be overridden with:

```
kserve /path/to/directory/to/serve
```

it'll create a config file in the serve dir

allows indexing by default, can be turned off

`_frame.html` allows you to define the html skeleton that'll be used for directories and markdown files \
insert `{%body%}` where you want the content to be inserted

## that's it

serves `.md` as html, serves everything else as-is on the filesystem
