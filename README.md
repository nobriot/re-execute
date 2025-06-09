# Execute commands when files are updated

![Build](https://github.com/nobriot/re-execute/actions/workflows/build.yml/badge.svg)

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
rex -f ~/Desktop "sleep 6 && echo {file}"
```

Run plantuml every time a puml file is modified in the current directory: 

```console
rex -e puml plantuml {file}
```

Build your documentation when a `.md` or `.rst` file is updated in the `docs/` directory

```console
rex -f docs/ -e md -e rst make docs
```

### Env variables

Pass additional environment variables using the --env with KEY=VALUE formatj

```console
rex --env FOO=BAR ./assets/file_and_env.sh {file}
```

### Parameters

* `-q` / `--quiet`: Do not print children's stdout/stderr messages
* `-t` / `--time`: Print the time of execution of each command


## Related tools

[When-changed](https://github.com/joh/when-changed)

