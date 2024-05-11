# Installation & upgrades

It is assumed you already have [chezmoi](https://www.chezmoi.io/) set up
and understand the basics of how it works.

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
3. Run `chezmoi_modify_manager --doctor` and make sure it reports no major issues
   with your installation.


## Tab completion

Optionally you can install tab completion. The tab completion can be generated
using the hidden command line flag `--bpaf-complete-style-SHELL_NAME`, (e.g.
`--bpaf-complete-style-zsh`, `--bpaf-complete-style-bash`, ...). As this is
handled internally by the command line parsing library we use, please see
[their documentation](https://docs.rs/bpaf/0.9.12/bpaf/_documentation/_2_howto/_1_completion/index.html)
for detailed instructions.

> For the Arch Linux AUR package, the completions are already installed for you
(except for elvish, which doesn't support a global install).

## Upgrading

Depending on the installation method:
* `chezmoi_modify_manager --upgrade`
* With your package manager
* For each OS and architecture, update the file `.utils/chezmoi_modify_manager-<os>-<arch>`.
  Note! For executables that you can run (i.e. the native one) you can still use `--upgrade`
  to do this.

> **You** are in control of updates. Nothing will happen unless you pass
`--upgrade`. Consider subscribing to be notified of new releases on the
[GitHub repository]. This can be done via `Watch` -> `Custom` in the top
right corner on that page after logging in to GitHub. Or just remember to
check with `--upgrade` occasionally.

[GitHub repository]: https://github.com/VorpalBlade/chezmoi_modify_manager
