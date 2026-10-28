use super::*;
use std::fs;

#[test]
fn test_scaffold_asi_project() {
    let temp = tempfile::tempdir().unwrap();
    let cfg = ProjectConfig {
        name: "FastTravelPlus".into(),
        mod_type: ProjectType::Asi,
        author: "Modder1".into(),
        version: "1.0.0".into(),
        description: "Instant fast travel to any discovered bonfire".into(),
        target_dir: temp.path().to_path_buf(),
    };

    let report = scaffold_project(&cfg).expect("scaffold asi project");
    let pdir = &report.project_dir;
    assert!(pdir.is_dir());
    assert!(pdir.join("Cargo.toml").is_file());
    assert!(pdir.join(".cargo/config.toml").is_file());
    assert!(pdir.join("src/lib.rs").is_file());
    assert!(pdir.join("config.ini").is_file());
    assert!(pdir.join("modinfo.json").is_file());
    assert!(pdir.join("README.md").is_file());
    assert!(pdir.join("DEVELOPMENT.md").is_file());

    let cargo_text = fs::read_to_string(pdir.join("Cargo.toml")).unwrap();
    assert!(cargo_text.contains("crate-type = [\"cdylib\"]"));
    assert!(cargo_text.contains("name = \"FastTravelPlus\""));

    let lib_text = fs::read_to_string(pdir.join("src/lib.rs")).unwrap();
    assert!(lib_text.contains("DllMain"));
    assert!(lib_text.contains("DLL_PROCESS_ATTACH"));
    assert!(lib_text.contains("find_pattern"));
    assert!(lib_text.contains("patch_memory"));
}

#[test]
fn test_scaffold_cpp_asi_project() {
    let temp = tempfile::tempdir().unwrap();
    let cfg = ProjectConfig {
        name: "FastInteract".into(),
        mod_type: ProjectType::AsiCpp,
        author: "CppDev".into(),
        version: "1.0.0".into(),
        description: "Instant interaction time via memory hook".into(),
        target_dir: temp.path().to_path_buf(),
    };

    let report = scaffold_project(&cfg).expect("scaffold cpp asi project");
    let pdir = &report.project_dir;
    assert!(pdir.is_dir());
    assert!(pdir.join("Makefile").is_file());
    assert!(pdir.join("src/main.cpp").is_file());
    assert!(pdir.join("config.ini").is_file());
    assert!(pdir.join("modinfo.json").is_file());
    assert!(pdir.join("README.md").is_file());
    assert!(pdir.join("DEVELOPMENT.md").is_file());

    let makefile = fs::read_to_string(pdir.join("Makefile")).unwrap();
    assert!(makefile.contains("x86_64-w64-mingw32-g++"));
    assert!(makefile.contains("build/FastInteract.asi"));

    let cpp = fs::read_to_string(pdir.join("src/main.cpp")).unwrap();
    assert!(cpp.contains("DllMain"));
    assert!(cpp.contains("FindPattern"));
    assert!(cpp.contains("PatchMemory"));

    let (mtype, name) = detect_project_type(pdir).unwrap();
    assert_eq!(mtype, ProjectType::AsiCpp);
    assert_eq!(name, "FastInteract");
}

#[test]
fn test_stage_built_project() {
    let temp = tempfile::tempdir().unwrap();
    let project_dir = temp.path().join("MyPlugin");
    let staging_dir = temp.path().join("staging");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join("config.ini"), "[General]\nEnabled=1\n").unwrap();
    fs::write(
        project_dir.join("modinfo.json"),
        r#"{"name":"MyPlugin","type":"asi"}"#,
    )
    .unwrap();

    let fake_bin = temp.path().join("MyPlugin.asi");
    fs::write(&fake_bin, b"MZ_FAKE_BINARY").unwrap();

    let staged = stage_built_project(&project_dir, &fake_bin, &staging_dir).unwrap();
    assert_eq!(staged, staging_dir.join("MyPlugin"));
    assert!(staged.join("bin64/MyPlugin.asi").is_file());
    assert!(staged.join("bin64/MyPlugin.ini").is_file());
    assert!(staged.join("modinfo.json").is_file());
    assert_eq!(
        fs::read(staged.join("bin64/MyPlugin.asi")).unwrap(),
        b"MZ_FAKE_BINARY"
    );
}

#[test]
fn test_scaffold_json_project() {
    let temp = tempfile::tempdir().unwrap();
    let cfg = ProjectConfig {
        name: "BetterStamina".into(),
        mod_type: ProjectType::Json,
        author: "Modder2".into(),
        version: "1.0".into(),
        description: "Reduces stamina consumption".into(),
        target_dir: temp.path().to_path_buf(),
    };

    let report = scaffold_project(&cfg).expect("scaffold json project");
    let pdir = &report.project_dir;
    assert!(pdir.is_dir());
    assert!(pdir.join("BetterStamina.field.json").is_file());
    assert!(pdir.join("modinfo.json").is_file());
    assert!(pdir.join("README.md").is_file());
    assert!(pdir.join("DEVELOPMENT.md").is_file());

    let json_text = fs::read_to_string(pdir.join("BetterStamina.field.json")).unwrap();
    assert!(json_text.contains("\"format\": 3"));
    assert!(json_text.contains("iteminfo.staticinfobody"));
}

#[test]
fn test_export_scaffolded_project() {
    let temp = tempfile::tempdir().unwrap();
    let projects_dir = temp.path().join("projects");
    let exports_dir = temp.path().join("exports");
    let staging_dir = temp.path().join("staging");

    let cfg = ProjectConfig {
        name: "AutoLoot".into(),
        mod_type: ProjectType::Json,
        author: "Dev".into(),
        version: "2.0".into(),
        description: "Auto loot items".into(),
        target_dir: projects_dir.clone(),
    };

    let report = scaffold_project(&cfg).expect("scaffold");
    let pkg =
        export_project(&report.project_dir, &exports_dir, Some(&staging_dir)).expect("export");

    assert!(pkg.archive_path.is_file());
    assert!(pkg.file_count >= 2);
    assert!(staging_dir.join("AutoLoot/modinfo.json").is_file());
}
