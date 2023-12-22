# CLI documentation templates

These templates provide extra information for the automatically generated CLI
documentation.

## Adding new templates

By default, no template is necessary. If you want to modify the automatically
generated CLI documentation, you can lookup the filename for the specific
command in `../docs/commands/` and add a file with identical name to this
directory to use as a template.

Use the `default.md` file as a starting point, which provides the default
template used when no override is present.

## Placeholders

Some placeholders are available to templates, these will be replaced
automatically with their values during documentation generation.

These placeholers are currently supported:

| Name             | Description                                                                            |
| ---------------- | -------------------------------------------------------------------------------------- |
| `{PH-HEADER}`    | Docusaurus metadata header; omit this when manually overriding the header              |
| `{PH-TITLE}`     | Command title (i.e. `phylum init`); use this for the Docusaurus metadata header title  |
| `{PH-MARKDOWN}`  | Automatically generated command documentation; this should be present in all templates |

## Docusaurus metadata

Some additional metadata can be specified by overriding the Docusaurus header.
Available fields can be found in the [Docusaurus docs].

[Docusaurus docs]: https://docusaurus.io/docs/api/plugins/@docusaurus/plugin-content-docs#markdown-front-matter

## Adding links

Markdown links to other docs should be provided as relative file paths, with
extensions. This is the [recommended approach from Docusaurus][docu_links].

The relative structure is as found in the [documentation repository][doc_repo].
All docs are stored or aggregated (via git submodules and symlinks) to the
`docs` subdirectory there, which forms the root/anchor of relative paths.

[docu_links]: https://docusaurus.io/docs/markdown-features/links
[doc_repo]: https://github.com/phylum-dev/documentation
