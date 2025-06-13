# Execute commands when files are updated

![CI](https://github.com/nobriot/re-execute/actions/workflows/build.yml/badge.svg)

Execute commands automatically when files are updated.

## Installation 

You will need [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
installed. Then install using:

```console
git clone https://github.com/nobriot/re-execute.git
cd re-execute/
cargo install --path .
```

The command is shortened to `rex`.

## Usage

### Possible Use Cases

Not exactly sure what you would use it for, but here are a few ideas
`re-execute` can be used for a variety of automation tasks, including but not limited to:

- **Automatic Testing**: Run unit or integration tests whenever source files are changed, speeding up the development cycle.
- **Continuous Compilation**: Recompile your project when code files are updated
- **Asset Processing**: Convert, optimize, or move assets (images, videos, etc.) when they are modified in a directory.
- **Code Linting/Formatting**: Lint or format code every time a file is saved to ensure consistency.
- **Scripted Deployments**: Automatically deploy or sync files to servers or cloud storage on file changes.
- **Static Site Generation**: Regenerate static websites when source content changes.
- **Diagram/Visualization Generation**: Run tools like PlantUML on diagrams whenever `.puml` files are edited.
- **Custom Notification**: Send notifications (email, desktop, etc.) when files of interest are updated.
- **Trigger Any Custom Command**: Use any CLI tool or script in response to file changes.

### Examples

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

## Exiting

Once the program start, it's like [VIM](https://www.vim.org/), you never exit it 😉.

Pressing Q/q and then Enter will work, but it's quirky at the moment.
Else just press Ctrl+C.

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
[entr](https://github.com/eradman/entr)
[watchexec](https://github.com/watchexec/watchexec)
[fswatch](https://github.com/emcrisostomo/fswatch)
[chokidar](https://github.com/open-cli-tools/chokidar-cli)
[checkexec](https://github.com/kurtbuilds/checkexec)
