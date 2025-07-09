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

The command is shortened to `rex`

## Usage

You can get started reading the help page: `rex -h`

By default, any file hidden or gitignored under the directories being watched
will not trigger any command execution.

```console
rex [OPTIONS] [COMMAND]...
```

### Examples

Just annouce updated files in your config folder:

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

Using `-e md,rst` is also valid. Use `-e ""` to allow files without extensions.

## Exiting

Once the program start, it's like [VIM](https://www.vim.org/), you never exit it 😉.

Pressing Q/q and then Enter will work, but it's quirky at the moment.
Else just press Ctrl+C.

### Env variables

Pass additional environment variables using the --env with KEY=VALUE format

```console
rex --env FOO=BAR ./assets/file_and_env.sh {file}
```

### Parameters

A non-exhaustive list of parameters for the program:

* `-q` / `--quiet`:   Do not print children's stdout/stderr messages
* `-e` / `--extension`:  Specify extensions to allow. Will ignore other extensions. e.g. `-e md -e ""` for .md and extension-less files
* `-E` / `--env`:   Set an env variable for the command, e.g. `--env FOO=bar`
* `-r` / `--regex`:   Add a regex to match filenames with. e.g. `-r '^[a-z0-9A-Z]*$'` will only match filenames with alphanumerical characters. Note that if a file watch in in `a/path` and the updated file ia `a/path/a/file`, then the second part will be evaluated against the regex, i.e. `a/file`
* `-R` / `--ignored_regex`: Add a regex that filenames with. e.g. `-r '^[a-z0-9A-Z]*$'` will only match filenames with alphanumerical characters. Note that if a file watch in in `a/path` and the updated file ia `a/path/a/file`, then the second part will be evaluated against the regex, i.e. `a/file`
* `-t` / `--time`:    Print the time of execution of each command
* `-H` / `--hidden`: Include hidden files in the triggers
* `-d` / `--deleted`: Call the commands also with files that have been deleted
* `-a` / `--abort-previous`: Abort previous ongoing command execution when files are updated while the program is running

## Related tools

This is just a small program I made for my own fun. 
If you'd like the more professional tools, look here:

* [watchexec](https://github.com/watchexec/watchexec)
* [fswatch](https://github.com/emcrisostomo/fswatch)
* [entr](https://github.com/eradman/entr)
* [chokidar](https://github.com/open-cli-tools/chokidar-cli)
* [checkexec](https://github.com/kurtbuilds/checkexec)
* [When-changed](https://github.com/joh/when-changed)
