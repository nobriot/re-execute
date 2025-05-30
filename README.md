# Execute commands when files are updated

Execute commands automatically when files are updated.

## Installation 

You will need [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
installed. Then install using:

```console
cargo install --path .
```

The command is shortened to `rex`. (I pronounce it re-X).

## Usage

Just annouce updated files in the config folder:

```console
rex -f $HOME/.config echo "updated files: {files}"
```

You will have to escape commands with quotes if they contain tokens interpreted
by the shell, e.g. `&&`:

```console
rex -f ~/Desktop "sleep 20 && echo {file}"
```

Run plantuml every time a puml file is modified in the current directory: 

```console
rex -e puml plantuml {file}
```

Build your documentation when a `.md` or `.rst` file is updated in the `docs/` directory

```console
rex -f docs/ -e md -e rst make docs
```

## Related tools

[When-changed](https://github.com/joh/when-changed)

