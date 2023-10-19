//! Defines supported transforms.

use std::collections::HashMap;

use ini_merge::mutations::transforms as ini_transforms;
use itertools::Itertools;
use strum::{EnumIter, EnumMessage, EnumString, IntoStaticStr};

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
    /// * user="user-name"        (user name to find entry in the keyring)
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
        println!(
            "{}",
            Itertools::intersperse(docs, "\n\n".to_string()).collect::<String>()
        );
    }

    /// Construct transform with arguments
    pub(crate) fn construct(
        self,
        args: &HashMap<String, String>,
    ) -> Result<Box<dyn ini_transforms::Transformer>, ini_transforms::TransformerError> {
        use ini_transforms::Transformer;
        match self {
            Transform::UnsortedLists => Ok(Box::new(
                ini_transforms::TransformUnsortedLists::from_user_input(args)?,
            )),
            Transform::KdeShortcut => Ok(Box::new(
                ini_transforms::TransformKdeShortcut::from_user_input(args)?,
            )),
            Transform::Keyring => Ok(Box::new(ini_transforms::TransformKeyring::from_user_input(
                args,
            )?)),
        }
    }
}
