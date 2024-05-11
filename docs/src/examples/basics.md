# Examples: Ignores & transforms

Here are some useful examples of flags for various settings files I have come across.

## KDE

### dolphinrc
```bash
ignore section "MainWindow"
ignore section "KPropertiesDialog"
ignore "General" "ViewPropsTimestamp"
ignore "Open-with settings" "History"
```

### kdeglobals
```bash
ignore "General" "ColorSchemeHash"
ignore "KFileDialog Settings" "Show hidden files"
ignore "KFileDialog Settings" "Show Inline Previews"
ignore section ""DirSelect Dialog""
```

### kglobalshortcutsrc
There are two issues in this configuration.

First, ActivityManager switch-to-activity entries. There are multiple entries,
making it a perfect fit for a regular expression. Note that this is not state
per se. It does however seem to vary between computers, having different UUID
values.

Second, certain shortcut keys like flipping between two representations. A
specialised transform has been added to handle this case. When this is needed
you will see diffs like the following:

```diff
-playmedia=none,,Play media playback
+playmedia=none,none,Play media playback
```

In summary, the following seems to work well:

```bash
# The two regex below have overlapping matches, this is OK in this case so
# turn off the warning for this file.
no-warn-multiple-key-matches

ignore regex "ActivityManager" "switch-to-activity-.*"
transform regex ".*" ".*" kde-shortcut
```

### konversationrc
Konversation has two relevant quirks:

1. It saves the password in the settings file (instead of using kwallet)
2. It resorts it alias list every time.

```bash
ignore "ServerListDialog" "Size"
transform "Aliases" "AliasList" unsorted-list separator=","
transform "Identity 0" "Password" keyring service="konversation" user="konversation_id0"
# Make sure the password isn't added back into the config file on re-add
add:hide "Identity 0" "Password"
```

To store the password for Identity 0 in your keyring of choice you can use the
`secret-tool` program from `libsecret` (`libsecret-tools` on Debian/Ubuntu):

```console
$ secret-tool store --label="Konversation password" service konversation username konversation_id0
[Enter your password at the prompt]
```

### kwinrc
Similar to kglobalshortcutsrc there are computer specific UUIDs. In addition,
the tiling configurations seem to be overwritten by KDE Plasma between computers.

```bash
ignore regex "Desktops" "Id_.*"
ignore regex "Tiling\\]\\[.*" ".*"
```

### plasmanotifyrc

```bash
ignore section "DoNotDisturb"
```

### Trolltech.conf

This is a Qt config, rather than a KDE config (strictly speaking) but since KDE
uses Qt, it is sitll relevant.

```bash
ignore "Qt" "filedialog"
```

## PrusaSlicer / SuperSlicer

PrusaSlicer and the fork SuperSlicer also use INI style files:

### PrusaSlicer.ini / SuperSlicer.ini

```bash
ignore "<NO_SECTION>" "auto_toolbar_size"
ignore "<NO_SECTION>" "downloader_url_registered"
ignore "<NO_SECTION>" "freecad_path"
ignore "<NO_SECTION>" "last_output_path_removable"
ignore "<NO_SECTION>" "last_output_path"
ignore "<NO_SECTION>" "version_online_seen"
ignore "<NO_SECTION>" "version_online"
ignore "<NO_SECTION>" "version_system_info_sent"
ignore "<NO_SECTION>" "version"
ignore "<NO_SECTION>" "window_mainframe"
ignore "font" "active_font"
ignore "presets" "filament"
ignore "presets" "print"
ignore "presets" "printer"
ignore "presets" "sla_material"
ignore "presets" "sla_print"
ignore regex "<NO_SECTION>" "desktop_integration_.*"
ignore regex "<NO_SECTION>" "print_host_queue_dialog_.*"
ignore regex "font:.*" ".*"
ignore regex "presets" "filament_.*"
ignore section "recent_projects"
ignore section "recent"
```

### PrusaSlicerGcodeViewer.ini / SuperSlicerGcodeViewer.ini

```bash
ignore "<NO_SECTION>" "version"
ignore "<NO_SECTION>" "window_mainframe"
ignore section "recent_projects"
```

### PrusaSlicer physical printer settings

PrusaSlicer allows you to configure "physical printers" (with connection details
to e.g. OctoPrint or PrusaLink). There will be one such config per physical printer
you configured, located at `.config/PrusaSlicer/physical_printer/<my_printer_name>.ini`

As these contain login details you probably want to put that in your keyring instead of
in git. This works similarly to [konversation](#konversationrc).

For example, you might use the following if you have a Prusa Mk3.9:

```bash
transform "<NO_SECTION>" "printhost_password" keyring service="ini_processor" user="prusa_mk39_password" separator=" = "
transform "<NO_SECTION>" "printhost_apikey" keyring service="ini_processor" user="prusa_mk39_apikey" separator=" = "
add:hide "<NO_SECTION>" "printhost_password"
add:hide "<NO_SECTION>" "printhost_apikey"
```

To add your password and API key you would then use:

```console
$ secret-tool store --label="Prusa Mk3.9 password" service ini_processor username prusa_mk39_password
Password: [Enter password]
$ secret-tool store --label="Prusa Mk3.9 API key" service ini_processor username prusa_mk39_apikey
Password: [Enter the API key]
```

## KeePassXC

### keepassxc.ini

KeePassXC stores private and public keys for KeeShare in the config.
You may not want to commit this to the repository.

```bash
ignore "KeeShare" "Active"
ignore "KeeShare" "Foreign"
ignore "KeeShare" "Own"
```

## GTK-3.0/GTK-4.0

### settings.ini

The file `~/.config/gtk-<version>/settings.ini` has a DPI value in it that
changes between computers. Thus, each of those setting files need the
following:

```bash
ignore "Settings" "gtk-xft-dpi"
```
