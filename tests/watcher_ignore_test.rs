use libresync_core::watcher::ignore::IgnoreRules;

#[test]
fn test_swp_file_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored("file.swp"));
}

#[test]
fn test_swx_file_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored("backup.swx"));
}

#[test]
fn test_tmp_file_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored("output.tmp"));
}

#[test]
fn test_temp_file_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored("output.temp"));
}

#[test]
fn test_tilde_backup_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored("document.txt~"));
}

#[test]
fn test_ds_store_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored(".DS_Store"));
}

#[test]
fn test_thumbs_db_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored("Thumbs.db"));
}

#[test]
fn test_goutputstream_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored(".goutputstream-ABCDEF"));
}

#[test]
fn test_part_file_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored("download.part"));
}

#[test]
fn test_hidden_tilde_file_ignored() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored(".~myfile.txt"));
}

#[test]
fn test_normal_file_not_ignored() {
    let rules = IgnoreRules::default();
    assert!(!rules.is_ignored("main.rs"));
    assert!(!rules.is_ignored("index.html"));
    assert!(!rules.is_ignored("document.pdf"));
}

#[test]
fn test_path_with_directory_still_matches() {
    let rules = IgnoreRules::default();
    assert!(rules.is_ignored("project/src/backup.swp"));
    assert!(rules.is_ignored("tmp/download.part"));
    assert!(rules.is_ignored("subdir/.DS_Store"));
}

#[test]
fn test_custom_pattern_supplemented() {
    let custom = vec!["*.log".into(), "*.cache".into()];
    let rules = IgnoreRules::new(custom);
    assert!(rules.is_ignored("app.log"));
    assert!(rules.is_ignored("data.cache"));
    assert!(!rules.is_ignored("main.rs"));
}

#[test]
fn test_empty_rules_ignore_nothing() {
    let rules = IgnoreRules::new(vec![]);
    assert!(!rules.is_ignored("file.swp"));
    assert!(!rules.is_ignored(".DS_Store"));
    assert!(!rules.is_ignored("anything.txt"));
}
