use libresync_core::autostart::Autostart;
use tempfile::tempdir;

#[test]
fn test_install_creates_desktop_file() {
    let dir = tempdir().unwrap();
    let bin_path = "/usr/bin/libresync-core --tray";

    Autostart::install_in(bin_path, dir.path()).unwrap();

    let desktop = dir.path().join("libresync.desktop");
    assert!(desktop.exists());

    let content = std::fs::read_to_string(&desktop).unwrap();
    assert!(content.contains("[Desktop Entry]"));
    assert!(content.contains("Name=LibreSync"));
    assert!(content.contains("Exec=/usr/bin/libresync-core --tray"));
    assert!(content.contains("X-GNOME-Autostart-enabled=true"));
}

#[test]
fn test_uninstall_removes_file() {
    let dir = tempdir().unwrap();
    let desktop = dir.path().join("libresync.desktop");

    Autostart::install_in("/usr/bin/test", dir.path()).unwrap();
    assert!(desktop.exists());

    Autostart::uninstall_in(dir.path()).unwrap();
    assert!(!desktop.exists());
}

#[test]
fn test_is_installed_returns_true() {
    let dir = tempdir().unwrap();

    Autostart::install_in("/usr/bin/test", dir.path()).unwrap();
    assert!(Autostart::is_installed_in(dir.path()));
}

#[test]
fn test_is_installed_returns_false() {
    let dir = tempdir().unwrap();
    assert!(!Autostart::is_installed_in(dir.path()));
}
