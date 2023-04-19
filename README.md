# Modify script helper addon for chezmoi (experimental branch)

NOTE! This is version 2, which is a rewrite in Rust. See the
[conversion guide](doc/conversion.md) if you are upgrading from the previous
Python version. Good news: This version is ~50x faster.

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

* A modify script. One of:
  * `modify_<config file>`, eg. `modify_private_kdeglobals` (for installs
    into `PATH`, recommended)
  * `modify_<config file>.tmpl`, eg. `modify_private_kdeglobals.tmpl` (for
    installs into the chezmoi source directory)
* `<config file>.src.ini`, eg. `private_kdeglobals.src.ini`

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

For detailed usage instructions on the filtering see `chezmoi_modify_manager --help-syntax`.

* Ignore entire section.
* Ignore specific key in specific section.
* Ignore key in section based on regular expressions.
* Apply a transformation to the value of a specified key. These are implemented
  as python functions. A list of transforms is available via `--help-transforms`.

The command can also be used to add files (see `chezmoi_ini_add --help` for details):

* Smart re-add mode (re-add files as managed `.src.ini` if they are already
  managed, otherwise add with plain chezmoi).
* Conversion mode (convert from plain chezmoi to managed to `.src.ini`).
* User specified hook. Can be used to filter out passwords when adding or
  re-adding configuration files. See [examples.md](doc/examples.md#add-hook) for details.

Finally, the command has a built in updater (similar to `chezmoi upgrade`).

## Installation

1. To your root `.chezmoiignore` add: `**/*.src.ini`. These files should not be
   checked out into your target directory, but acts as the "source of truth" for
   the modify script.
2. Do *one* of these:
   * Recommended: Install `chezmoi_modify_manager` into your `$PATH`. This can be
     done by one of:
     * Using a distro package (if available for what you use)
     * Download the binary from the releases here.
   * Not recommended: Install `chezmoi_modify_manager` from the releases page
     into `.utils/chezmoi_modify_manager-<os>-<arch>` where `<os>` is typically
     `linux` and `<arch>` is typically `x86-64`. If you use another path, the
     template modify script that is added will be wrong.

4. **You** are in control of updates. Nothing will happen unless you pass
   `--upgrade`. Consider subscribing to be notified of new releases on the
   github repository. This can be done via `Watch` -> `Custom` in the top
   right corner. Or just remember to check with `--upgrade` occasionally.

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

## Requirements

The binary is self contained, only needing:
* Linux: glibc and OpenSSL
* Other OSes: TBD

Requirements to build (if there is no native binary for your platform):
* Rust 1.68 or newer

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
