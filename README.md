# Execute commands when files are updated

Execute commands automatically when files are updated.

## Installation 

This is untested for all platforms.

```console
cargo install .
```

## Usage

Run plantuml every time a puml file is modified in the current directory: 

```console
rex -e puml 'plantuml {file}'
```

Build your documentation when a `.md` or `.rst` file is updated in the `docs/` directory

```console
rex -f docs/ -e md -e rst 'make docs'
```

## Related tools

[When-changed](https://github.com/joh/when-changed)

