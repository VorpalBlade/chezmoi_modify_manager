# Modify script helper addon for chezmoi

[ [lib.rs] ] [ [crates.io] ] [ [AUR] ]


## News

* If you are upgrading from version 2.x to 3.x see this [migration guide](doc/migration_3.md).
* If you are upgrading from version 1.x *also* see this [migration guide](doc/migration_2.md).

---

Addon for [chezmoi](https://www.chezmoi.io/) for deals with settings files that
contain a mix of settings and state. So far handling INI-style files are
supported.

A typical example of this is KDE settings files. These contain (apart from
settings) state like recently opened files and positions of windows and dialog
boxes. Other programs (such as PrusaSlicer) also do the same thing.

The program in this repository allows you to ignore certain sections of those
INI files when managing the configuration files with chezmoi.

## Theory of operation

For each settings file you want to manage with `chezmoi_modify_manager` there
will be two files in your chezmoi source directory:

* `modify_<config file>.tmpl`, eg. `modify_private_kdeglobals.tmpl` \
  This is the modify script/configuration file that calls `chezmoi_modify_manager`.
  It contains the directives describing what to ignore.
* `<config file>.src.ini`, eg. `private_kdeglobals.src.ini`\
  This is the source state of the INI file.

The `modify_` script is responsible for generating the new state of the file
given the current state in your home directory. The `modify_` script is set
up to use `chezmoi_modify_manager` as an interpreter to do so.
`chezmoi_modify_manager` will read the modify script to read configuration and
the `.src.ini` file and by default will apply that file exactly (ignoring blank
lines and comments).

However, by giving additional directives to `chezmoi_modify_manager` in the
`modify_` script you can tell it to ignore certain sections (see
`chezmoi_modify_manager --help-syntax` for details). For example:

```bash
ignore "KFileDialog Settings" "Show Inline Previews"
ignore section "DirSelect Dialog"
```

will tell it to ignore the key `Show Inline Previews` in the section
`KFileDialog Settings` and the entire section `DirSelect Dialog`.

**Note!** If a key appears before the first section, use `<NO_SECTION>` as the
section.

### Supported features

#### Feature: Merging & filtering INI files

This is the main mode and reason for the existance of this tool.

`chezmoi_modify_manager` allows you to:

* Ignore an entire section.
* Ignore a specific key in specific section.
* Ignore a key in section based on regular expressions.
* Force set a value (useful together with templating).
* Force remove a section, key or entries matching a regex (useful together with templating).
* Apply a transformation to the value of a specified key. These are special
  operations that are built in and provide more complicated transformations.
  A list of transforms is available via `--help-transforms`. Some examples
  that this can do:
  * Look up a password in the platform keyring
  * Ignore the sorting order of a list style value (`key=a,b,c,d`)
  * etc.

For detailed usage instructions see `chezmoi_modify_manager --help-syntax`.

#### Feature: Assisted adding to the chezmoi source state

The command can also be used to add files (see `chezmoi_ini_add --help` for details):

* Smart re-add mode (re-add files as managed `.src.ini` if they are already
  managed, otherwise add with plain chezmoi).
* Conversion mode (convert from plain chezmoi to managed to `.src.ini`).

`chezmoi_modify_manager` also allows filtering the added files when re-adding
them after they changed:

* Any ignored keys will be removed (since we always use the system version of
  these, this reduces churn and the diff size in git).
* The value can be hidden (`add:hide` directive), useful in case of passwords
  that comes from keyrings.
* Or they can be removed entirely using the `add:remove` directive (useful in
  combination with `set` and a templated modify script).

See the section on `add:hide` & `add:remove` in `--help-syntax` for more details.

#### Feature: Self updater

Finally, the command has a built-in updater (similar to `chezmoi upgrade`).

Note! This can (and should) be configured out using cargo features if you are
building a distro package.

## Installation

1. To your root `.chezmoiignore` add: `**/*.src.ini`. These files should not be
   checked out into your target directory, but acts as the "source of truth" for
   the modify script.
2. Do *one* of these:
   * Recommended: Install `chezmoi_modify_manager` into your `$PATH`. This can be
     done by one of (in descending order of preference):
     * Using a distro package (if available for what you use)
     * Download the binary from the [releases on GitHub](https://github.com/VorpalBlade/chezmoi_modify_manager/releases) and install it somewhere into your `PATH`.
     * Install from [crates.io] using `cargo` (only do this if you know what you are doing).
   * Not recommended: Install `chezmoi_modify_manager` from the releases page
     into `<chezmoi-source-directory>/.utils/chezmoi_modify_manager-<os>-<arch>`
     where `<os>` is typically `linux` and `<arch>` is typically `x86-64`. If
     you use another path, the template modify script that is added will be wrong.

4. **You** are in control of updates. Nothing will happen unless you pass
   `--upgrade`. Consider subscribing to be notified of new releases on the
   github repository. This can be done via `Watch` -> `Custom` in the top
   right corner. Or just remember to check with `--upgrade` occasionally.

### Tab completion

Optionally you can install tab completion. The tab completion can be generated
using the hidden command line flag `--bpaf-complete-style-SHELL_NAME`, (e.g.
`--bpaf-complete-style-zsh`, `--bpaf-complete-style-bash`, ...). As this is
handled internally by the command line parsing library we use, please see
[their documentation](https://docs.rs/bpaf/0.9.8/bpaf/_documentation/_2_howto/_1_completion/index.html)
for detailed instructions.

## Updating

Depending on the installation method:
* `chezmoi_modify_manager --upgrade`
* With your package manager
* For each OS and architecture, update the file `.utils/chezmoi_modify_manager-<os>-<arch>`.
  Note! For executables that you can run (i.e. the native one) you can still use `--upgrade`
  to do this.

## Usage

Details of supported actions can be seen with `chezmoi_modify_manager --help`.

Some example usages to add new files:

```bash
# Add configs to be handled by chezmoi_modify_manager (or convert configs
# managed by chezmoi to be managed by chezmoi_modify_manager).
chezmoi_modify_manager --add ~/.config/kdeglobals ~/.config/kwinrc

# Re-add config after changes in the live system.
chezmoi_modify_manager --add ~/.config/kdeglobals

# Don't remember if chezmoi_modify_manager handles the file or if it is raw chezmoi?
# Use smart mode (-s/--smart-add) to update the file!
chezmoi_modify_manager --smart-add ~/.config/PrusaSlicer/PrusaSlicer.ini
```

Some examples on various ignore flags and transforms can be found in
[examples.md](doc/examples.md).

## Platform support and requirements

The binary is self contained with no non-optional dependencies. For certain platforms where RustTLS isn't supported, the optional self-updater needs OpenSSL, which can be from either the system or built and linked statically.

Requirements to build (if there is no native binary for your platform):
* Rust 1.70 or newer

Platforms:

| Platform         | Architecture | Continuous Integration | Tested manually           |
|------------------|--------------|------------------------|---------------------------|
| Linux with Glibc | All major    | Yes                    | Yes (x86-64, i686, ARMv7) |
| Linux with Musl  | All major    | Yes                    | Yes (x86-64)              |
| Windows          | x86-64       | Yes                    | No                        |
| MacOS            | x86-64       | Yes                    | No                        |

The above table is limited to what I myself have access to (and use) as well as what works in GitHub CI. Other Unixes are likely to work, if [Rust has support](https://doc.rust-lang.org/stable/rustc/platform-support.html).

## Troubleshooting

The first step should be to run `chezmoi_modify_manager --doctor` and correct any issues reported.
This will help identify the two common issues:

* chezmoi_modify_manager needs to be in `PATH`
* `**/*.src.ini` needs to be ignored in the root `.chezmoiignore` file

## Limitations

* When a key exists in the `.src.ini` file but not in the target state it will
  be added to the end of the relevant section. This is not an issue as the
  program will usually just resort the file next time it writes out its
  settings.
* `modify_` scripts bypass the check for "Did the file change in the target
  state" that chezmoi performs. This is essential for proper operation.
  However it also means that you will not be asked about overwriting changes.
  Always look at `chezmoi diff` first! I do have some ideas on how to mitigate
  this in the future. See also [this chezmoi bug](https://github.com/twpayne/chezmoi/issues/2244)
  for a more detailed discussion on this.

[AUR]: https://aur.archlinux.org/packages/chezmoi_modify_manager
[crates.io]: https://crates.io/crates/chezmoi_modify_manager
[lib.rs]: https://lib.rs/crates/chezmoi_modify_manager
