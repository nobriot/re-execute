# Execute commands when files are updated

![CI](https://github.com/nobriot/re-execute/actions/workflows/build.yml/badge.svg)

Execute commands automatically when files are updated.

## Installation 

You will need [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
installed. Make sure you have nightly installed.

```console
rustup update nightly  
```

Then install using:

```console
cargo install --git https://github.com/nobriot/re-execute.git
```

The command is shortened to `rex`

## Quick overview

The UI looks like this:

![re-execute UI](./assets/rex.gif).

## Usage

You can get started reading the help page: `rex -h`

By default, any file hidden or gitignored under the directories being watched
will not trigger any command execution.

```console
rex [OPTIONS] [COMMAND]...
```

If the command contains the **`{file}`** string, it will be replaced by updated
files and one command per updated file will be executed.

If the command contains the **`{files}`** string, it will be replaced by 
a space separated list of files that were updated, and one command in total will be executed.

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
Using `-e md,rst` is also valid. Use `-e ""` to allow files without extensions.

```console
rex -f docs/ -e md -e rst make docs
```

Launch some tests when you update test files:

```console
rex -f src/ -e py -r 'test' -- pytest -k {file}
```

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
* `-R` / `--ignored_regex`: Add a regex that if matches, filenames will be ignored.
* `-t` / `--time`:    Print the time when each command was executed
* `-H` / `--hidden`: Include hidden files in the triggers
* `-d` / `--deleted`: Call the commands also with files that have been deleted
* `-a` / `--abort-previous`: Abort previous ongoing command execution when files are updated while the program is running
* `--force-poll`: Use polling to get files update events. May be necessary on some machines.

## Related tools

This is just a small program I made for my own fun. 
If you'd like the more professional tools, look here:

* [reflex](https://github.com/cespare/reflex)
* [watchexec](https://github.com/watchexec/watchexec)
* [fswatch](https://github.com/emcrisostomo/fswatch)
* [entr](https://github.com/eradman/entr)
* [chokidar](https://github.com/open-cli-tools/chokidar-cli)
* [checkexec](https://github.com/kurtbuilds/checkexec)
* [When-changed](https://github.com/joh/when-changed)
