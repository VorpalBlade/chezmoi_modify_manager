//! Defines supported transforms.

use std::collections::HashMap;

use strum::EnumIter;
use strum::EnumMessage;
use strum::EnumString;
use strum::IntoStaticStr;

use ini_merge::mutations::transforms as ini_transforms;

/// Supported transforms
///
/// This serves as a central point for documentation, parsing, generating
/// lists etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumString, IntoStaticStr, EnumMessage)]
pub(crate) enum Transform {
    /// Compare the value as an unsorted list.
    /// Useful because Konversation likes to reorder lists.
    ///
    /// Arguments:
    /// * separator="," (Separating character between list elements)
    #[strum(serialize = "unsorted-list")]
    UnsortedLists,
    /// Specialised transform to handle KDE changing certain global
    /// shortcuts back and forth between formats like:
    ///
    /// playmedia=none,,Play media playback
    /// playmedia=none,none,Play media playback
    ///
    /// No arguments.
    #[strum(serialize = "kde-shortcut")]
    KdeShortcut,
    /// Get the value for a key from the system keyring. Useful for passwords
    /// etc that you do not want in your dotfiles repo.
    ///
    /// Arguments:
    /// * service="service-name"  (service name to find entry in the keyring)
    /// * user="user-name"        (username to find entry in the keyring)
    ///
    /// On Linux you can add an entry to the keyring using:
    /// secret-tool store --label="Descriptive name" service "service-name" username "user-name"
    #[strum(serialize = "keyring")]
    Keyring,
}

impl Transform {
    /// Print help for transforms
    pub(crate) fn help() {
        use strum::IntoEnumIterator;
        let docs = Self::iter().map(|elem| {
            let name: &str = elem.into();
            format!(
                "{}\n{}\n{}",
                name,
                "-".repeat(name.len()),
                elem.get_documentation().unwrap_or("Missing docs")
            )
        });
        println!("Supported transforms:");
        println!("====================\n");
        // Workaround for https://github.com/rust-itertools/itertools/issues/942
        use itertools::Itertools;
        println!(
            "{}",
            Itertools::intersperse(docs, "\n\n".to_string()).collect::<String>()
        );
    }

    /// Construct transform with arguments
    pub(crate) fn construct(
        self,
        args: &HashMap<String, String>,
    ) -> anyhow::Result<ini_transforms::TransformerDispatch> {
        use ini_transforms::Transformer;
        match self {
            Transform::UnsortedLists => {
                Ok(ini_transforms::TransformUnsortedLists::from_user_input(args)?.into())
            }
            Transform::KdeShortcut => {
                Ok(ini_transforms::TransformKdeShortcut::from_user_input(args)?.into())
            }
            #[cfg(feature = "keyring")]
            Transform::Keyring => {
                Ok(ini_transforms::TransformKeyring::from_user_input(args)?.into())
            }
            #[cfg(not(feature = "keyring"))]
            Transform::Keyring => Err(anyhow::anyhow!(
                "This build of chezmoi_modify_manager does not support the keyring transform"
            )),
        }
    }
}
