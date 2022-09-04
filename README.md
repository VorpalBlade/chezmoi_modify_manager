# Modify script helper addon for chezmoi

Addon for [chezmoi](https://www.chezmoi.io/) for deals with settings files that
contain a mix of settings and state. So far handling INI-style files are
supported.

A typical example of this is KDE settings files. These contain (apart from
settings) state like recently opened files and positions of windows and dialog
boxes. Other programs (such as PrusaSlicer) also do the same thing.

The two scripts in this repository allows you to ignore certain sections of
those ini files when managing the configuration files with chezmoi.

* `chezmoi_ini_add` is a helper to set up a config file to be managed in this
  manner.
* `chezmoi_ini_manager.py` is the main program. This is meant to be called from
  a chezmoi `modify_` script. 

## Theory of operation

For each settings file you want to manage with `chezmoi_ini_manager` there will
be two files in your chezmoi source directory:

* `modify_<config file>.tmpl`, eg. `modify_private_kdeglobals.tmpl`
* `<config file>.src.ini`, eg. `private_kdeglobals.src.ini`

The `modify_` script is responsible for generating the new state of the file
given the current state in your home directory. The `modify_` script is set
up to use `chezmoi_ini_manager.py` to do so. The script `chezmoi_ini_manager.py`
will read the `.src.ini` file and by default will apply that file exactly
(ignoring blank lines and comments).

However, by giving additional options to `chezmoi_ini_manager.py` you can tell
it to ignore certain sections (see `chezmoi_ini_manager.py --help` for details).
For example:

```bash
-ik "KFileDialog Settings" "Show Inline Previews"
-is "DirSelect Dialog"
```

will tell it to ignore the key `Show Inline Previews` in the section
`KFileDialog Settings` and the entire section `DirSelect Dialog`.

**Note!** If a key appears before the first section, use `<NO_SECTION>` as the
section.

### Supported features

For detailed usage instructions on the filtering see `chezmoi_ini_manager.py --help`

* Ignore entire section.
* Ignore specific key in specific section.
* Ignore key in section based on regular expressions.
* Apply a transformation to the value of a specified key. These are implemented
  as python functions. A list of transforms is available via `--transform-list`.

The add script also has some nice features (see `chezmoi_ini_add --help`):

* Smart re-add mode (re-add files as managed `.src.ini` if they are already
  managed, otherwise add with plain chezmoi).
* Conversion mode (convert from plain chezmoi to managed to `.src.ini`).
* User specified hook. Can be used to filter out passwords when adding or
  re-adding configuration files. See [EXAMPLES.md](EXAMPLES.md#add-hook) for details.

## Installation

1. To your root `.chezmoiignore` add: `**/*.src.ini`. These files should not be
   checked out into your target directory, but acts as the "source of truth" for
   the modify script.
2. Add this repository as a submodule at `.utils/chezmoi_modify_manager`. If
   you use another path, the `chezmoi_ini_add` script will not work as is for you.
   This can be done as follows:
 
   ```console
   $ chezmoi cd
   $ git submodule add https://github.com/VorpalBlade/chezmoi_modify_manager.git .utils/chezmoi_modify_manager
   $ git commit [...]
   $ git push [...]
   ```

   Note that as long as you use `chezmoi init` and `chezmoi update` everything
   else will be taken care of automatically. If you use `git` commands directly
   you will need to ensure that you use `--recurse-submodules=on-demand` as needed.

3. (Optional) For your convenience considering adding `chezmoi_ini_add` to your
   `$PATH` by either a symlink into something that is in your `$PATH` or by
   adding the bin directory of this repository to your path. If you use zsh with
   a plugin manager that allows loading plugins from arbitrary paths (e.g.
   zsh4humans), this repository is set up as a zsh plugin for ease of use.

## Updating

To update to a newer version of chezmoi-modify-manager, update the revision of
the submodule that is pointed to, add it and commit your repository:

```console
$ chezmoi cd
$ cd .utils/chezmoi_modify_manager
$ git pull origin main
$ cd ../..
$ git add .utils/chezmoi_modify_manager
$ git commit [...]
$ git push [...]
```

## Usage

Details of supported actions can be seen with `chezmoi_ini_manager.py --help`.
See `chezmoi_ini --help` for details on how to use that script.

Some examples on various ignore flags and transforms can be found in
[EXAMPLES.md](EXAMPLES.md).

## Requirements

* Python 3.10 or newer for `chezmoi_ini_manager.py`.
* Bash for the `modify_` scripts themselves.
* ZSH for running `chezmoi_ini_add`.

Optional:

* python-keyring is required for the `keyring` transform to pull passwords from
  kwallet/gnome-keyring/etc.

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
