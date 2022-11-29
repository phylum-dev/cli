# CLI documentation templates

These templates provide extra information for the automatically generated CLI
documentation.

## Adding new templates

By default, no template is necessary. If you want to modify the automatically
generated CLI documentation, you can lookup the filename for the specific
command in `../docs/command_line_tool/` and add a file with identical name to
this repository to use as a template.

Use the `default.md` file as a start-off point, which provides the default
template used when no override is present.

## Placeholders

Some placeholders are available to templates, these will be replaced
automatically with their values during documentation generation.

These placeholers are currently supported:

| Name             | Description                                                                            |
| ---------------- | -------------------------------------------------------------------------------------- |
| `{PH-HEADER}`    | ReadMe.com metadata header; omit this when manually overriding the header              |
| `{PH-TITLE}`     | Command title (i.e. `phylum init`); use this for the ReadMe.com metadata header title  |
| `{PH-MARKDOWN}`  | Automatically generated command documentation; this should be present in all templates |
