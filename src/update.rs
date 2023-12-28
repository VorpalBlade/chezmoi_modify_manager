//! Perform update check and auto update.
#![cfg(feature = "self_update")]

use self_update::cargo_crate_version;

/// Perform update check from github.
pub(super) fn update(no_confirm: bool) -> anyhow::Result<()> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("VorpalBlade")
        .repo_name("chezmoi_modify_manager")
        .bin_name("chezmoi_modify_manager")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .no_confirm(no_confirm)
        .build()?
        .update()?;
    println!("Update status: `{}`!", status.version());
    Ok(())
}
