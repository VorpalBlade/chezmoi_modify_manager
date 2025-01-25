# Modify script helper addon for chezmoi

[ [User Manual] ] [ [lib.rs] ] [ [crates.io] ] [ [AUR] ]


## News

* If you are upgrading across major releases see the [migration guides]

---

Addon for [chezmoi](https://www.chezmoi.io/) that deals with settings files that
contain a mix of settings and state. So far handling INI-style files are
supported.

A typical example of this is KDE settings files. These contain (apart from
settings) state like recently opened files and positions of windows and dialog
boxes. Other programs (such as PrusaSlicer) also do the same thing.

The program in this repository allows you to ignore certain sections of those
INI files when managing the configuration files with chezmoi.

## Documentation

See the [user manual] for the full documentation on how to use
`chezmoi_modify_manager`.

## Supported features

#### Feature: Merging & filtering INI files

This is the main mode and reason for the existance of this tool.

`chezmoi_modify_manager` allows you to:

* Ignore entire sections or specific keys in an INI style file.
* Ignore a key in a section based on regular expressions.
* Force set a value (useful together with templating).
* Force remove a section, key or entries matching a regex (useful together with templating).
* Apply a transformation to the value of a specified key. These are special
  operations that are built in and provide more complicated transformations.
  Some examples that this can do:
  * Look up a password in the platform keyring
  * Ignore the sorting order of a list style value (`key=a,b,c,d`)
  * etc.

For detailed usage instructions see the [user manual].

#### Feature: Assisted adding to the chezmoi source state

The command can also be used to add files (see `chezmoi_modify_manager --help` for details):

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

For detailed usage instructions see the [user manual].

## Platform support and requirements

The binary is self-contained with no non-optional system dependencies apart
from the platform provided basic libraries (typically libc & libm on Linux).

Requirements to build (if there is no native binary for your platform):

* Rust 1.85.0 or newer
* A C compiler and associated toolchain (linker, headers, libraries, etc).\
  This is needed as some dependencies may include some C code.

Platforms:

| Platform         | Architecture | Continuous Integration | Tested manually     |
|------------------|--------------|------------------------|---------------------|
| Linux with Glibc | All major    | Yes                    | Yes (x86-64, ARM64) |
| Linux with Musl  | All major    | Yes                    | Yes (x86-64)        |
| Windows          | x86-64       | Yes                    | No                  |
| MacOS            | x86-64       | Yes                    | No                  |

The above table is limited to what I myself have access to (and use) as well as
what works in GitHub CI. Other Unixes are likely to work, if
[Rust has support](https://doc.rust-lang.org/stable/rustc/platform-support.html).

## Minimum Supported Rust Version (MSRV) policy

The current Minimum Supported Rust Version (MSRV) is documented in the previous
[section](#platform-support-and-requirements). The MSRV may be bumped as needed.
It is guaranteed that `chezmoi_modify_manager` will at least build on the current
and previous stable Rust release. An MSRV change is not considered a breaking
change and as such may change even in a patch version.

[AUR]: https://aur.archlinux.org/packages/chezmoi_modify_manager
[crates.io]: https://crates.io/crates/chezmoi_modify_manager
[lib.rs]: https://lib.rs/crates/chezmoi_modify_manager
[user manual]: https://vorpalblade.github.io/chezmoi_modify_manager
[migration guides]: https://vorpalblade.github.io/chezmoi_modify_manager/migration