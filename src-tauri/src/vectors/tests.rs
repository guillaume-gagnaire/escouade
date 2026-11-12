//! The vectors as the app keeps them: their folders, and what the settings read of the model.

use std::path::PathBuf;

#[test]
fn the_model_and_the_index_live_in_the_app_s_folder_never_beside_a_name_that_climbs() {
    let data = crate::paths::DataDir::new(PathBuf::from("/data"));
    assert!(data
        .vectors_model("multilingual-e5-base")
        .starts_with("/data/vectors"));
    assert!(data.vectors_model("../x").ends_with("vectors/_"));
    assert!(data.vectors_index("p1").ends_with("projects/p1/vectors.db"));
    assert!(data
        .vectors_index("../p")
        .ends_with("projects/_/vectors.db"));
}

/// What « Recherche par le sens » reads of the model (ESC-254): its size, what a cut download left, and that « Effacer »
/// takes that away too.
#[tokio::test]
async fn the_settings_read_the_model_and_its_deletion_takes_what_a_download_left() {
    use super::model::MODEL;
    let h = crate::core_tests::harness("esc254-vectors-info");
    let info = h.core.vectors_model_info();
    assert_eq!(info.id, MODEL.id);
    assert_eq!(info.size, MODEL.size());
    assert!(!info.downloaded && !info.downloading);
    assert_eq!(info.have, 0);
    let dir = h.core.data.vectors_model(MODEL.id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("model.onnx.part"), vec![0u8; 1234]).unwrap();
    assert_eq!(h.core.vectors_model_info().have, 1234);
    let after = h.core.vectors_model_delete().unwrap();
    assert_eq!(after.have, 0);
    assert!(!dir.join("model.onnx.part").exists());
    // « Annuler » with nothing under way does nothing.
    h.core.vectors_model_cancel();
}
