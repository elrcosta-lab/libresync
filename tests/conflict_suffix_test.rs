use libresync_core::conflict::suffix::generate_conflict_suffix;

#[test]
fn test_normal_name_with_extension() {
    assert_eq!(
        generate_conflict_suffix("relatorio.docx", "drive"),
        "relatorio (conflito drive).docx"
    );
}

#[test]
fn test_name_without_extension() {
    assert_eq!(
        generate_conflict_suffix("README", "maria"),
        "README (conflito maria)"
    );
}

#[test]
fn test_multi_dot_name() {
    assert_eq!(
        generate_conflict_suffix("arquivo.tar.gz", "drive"),
        "arquivo.tar (conflito drive).gz"
    );
}

#[test]
fn test_suffix_maria_vs_drive() {
    assert_eq!(
        generate_conflict_suffix("foto.png", "maria"),
        "foto (conflito maria).png"
    );
    assert_eq!(
        generate_conflict_suffix("foto.png", "drive"),
        "foto (conflito drive).png"
    );
}

#[test]
fn test_name_already_with_suffix_does_not_duplicate() {
    let result = generate_conflict_suffix("documento (conflito maria).txt", "maria");
    assert_eq!(result, "documento (conflito maria) (conflito maria).txt");
}

#[test]
fn test_utf8_name() {
    assert_eq!(
        generate_conflict_suffix("café.pdf", "drive"),
        "café (conflito drive).pdf"
    );
}

#[test]
fn test_name_with_multiple_labels() {
    assert_eq!(
        generate_conflict_suffix("relatorio.docx", "maria"),
        "relatorio (conflito maria).docx"
    );
}
