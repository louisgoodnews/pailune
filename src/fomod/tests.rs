use super::resolve::default_selection;
use super::*;
use std::fs;

/// A typical installer: a required core, a texture choice (recommended
/// default), an optional patch that sets a flag, a step only shown when that
/// flag is set, and a conditional install.
pub const XML: &str = r#"<?xml version="1.0" encoding="UTF-16"?>
<config xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <moduleName>Better Things</moduleName>
  <requiredInstallFiles>
    <folder source="core" destination="bin64" />
  </requiredInstallFiles>
  <installSteps order="Explicit">
    <installStep name="Textures">
      <optionalFileGroups order="Explicit">
        <group name="Resolution" type="SelectExactlyOne">
          <plugins order="Explicit">
            <plugin name="2K">
              <description>Smaller</description>
              <files><folder source="tex\2k" destination="files\0012" /></files>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
            <plugin name="4K">
              <description>Sharper</description>
              <files><folder source="tex\4k" destination="files\0012" /></files>
              <typeDescriptor><type name="Recommended"/></typeDescriptor>
            </plugin>
          </plugins>
        </group>
        <group name="Extras" type="SelectAny">
          <plugins order="Explicit">
            <plugin name="Patch">
              <description>Adds a patch</description>
              <files><file source="extra\patch.asi" destination="bin64\patch.asi" priority="5" /></files>
              <conditionFlags><flag name="patched">On</flag></conditionFlags>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
    <installStep name="Patch settings">
      <visible><flagDependency flag="patched" value="On" /></visible>
      <optionalFileGroups>
        <group name="Mode" type="SelectExactlyOne">
          <plugins>
            <plugin name="Strong"><description/><files><file source="extra\strong.ini" destination="bin64\patch.ini"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
            <plugin name="Mild"><description/><files><file source="extra\mild.ini" destination="bin64\patch.ini"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
  </installSteps>
  <conditionalFileInstalls><patterns><pattern>
    <dependencies operator="And"><flagDependency flag="patched" value="On"/></dependencies>
    <files><file source="extra\readme_patch.txt" destination="docs\patch.txt"/></files>
  </pattern></patterns></conditionalFileInstalls>
</config>"#;

/// Writes the installer (as UTF-16 LE with BOM, like many real ones) and
/// its files into `root`.
pub fn write_mod(root: &std::path::Path) {
    let files = [
        "core/core.asi",
        "tex/2k/a.dds",
        "tex/4k/a.dds",
        "extra/patch.asi",
        "extra/strong.ini",
        "extra/mild.ini",
        "extra/readme_patch.txt",
    ];
    files.iter().for_each(|f| {
        let path = root.join(f);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, f).unwrap();
    });
    let utf16: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain(XML.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    fs::create_dir_all(root.join("FOMOD")).unwrap();
    fs::write(root.join("FOMOD/ModuleConfig.xml"), utf16).unwrap();
}

fn dests(list: &[(std::path::PathBuf, std::path::PathBuf)]) -> Vec<String> {
    list.iter()
        .map(|(src, dest)| format!("{} <- {}", dest.display(), fs::read_to_string(src).unwrap()))
        .collect()
}

#[test]
fn defaults_choices_flags_and_visibility() {
    let dir = tempfile::tempdir().unwrap();
    write_mod(dir.path());
    let config = load(dir.path())
        .unwrap()
        .expect("installer found in any case");
    assert_eq!(
        (config.name.as_str(), config.steps.len()),
        ("Better Things", 2)
    );
    // Untouched: the recommended 4K, no patch, so the second step is hidden.
    assert_eq!(
        dests(&resolve(dir.path(), &config, None)),
        [
            "bin64/core.asi <- core/core.asi",
            "files/0012/a.dds <- tex/4k/a.dds"
        ]
    );
    // 2K + patch: the hidden step appears and defaults to its first option
    // (no `order` attribute, so ascending: "Mild" before "Strong").
    let picks = |items: &[(&str, &str, &str)]| Selection {
        picks: items
            .iter()
            .map(|(s, g, p)| (s.to_string(), g.to_string(), p.to_string()))
            .collect(),
        at: Vec::new(),
    };
    let chosen = picks(&[
        ("Textures", "Resolution", "2K"),
        ("Textures", "Extras", "Patch"),
    ]);
    assert_eq!(
        dests(&resolve(dir.path(), &config, Some(&chosen))),
        [
            "bin64/core.asi <- core/core.asi",
            "bin64/patch.asi <- extra/patch.asi",
            "bin64/patch.ini <- extra/mild.ini",
            "docs/patch.txt <- extra/readme_patch.txt",
            "files/0012/a.dds <- tex/2k/a.dds",
        ]
    );
    let strong = picks(&[
        ("Textures", "Resolution", "2K"),
        ("Textures", "Extras", "Patch"),
        ("Patch settings", "Mode", "Strong"),
    ]);
    let resolved = dests(&resolve(dir.path(), &config, Some(&strong)));
    assert!(resolved.contains(&"bin64/patch.ini <- extra/strong.ini".to_string()));
    // Choices survive the variant field round trip.
    assert_eq!(Selection::decode(Some(&strong.encode())), Some(strong));
    assert_eq!(Selection::decode(Some("Red")), None);
    assert_eq!(default_selection(&config).picks.len(), 1);
}

#[test]
fn rejects_broken_xml_and_sorts_ascending_by_default() {
    assert!(parse("<config><moduleName>x</config>").is_err());
    let config = parse(
        r#"<config><installSteps><installStep name="B"/><installStep name="a"/></installSteps></config>"#,
    )
    .unwrap();
    let names: Vec<_> = config.steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["a", "B"]);
}

#[test]
fn paths_leaving_the_mod_or_game_folder_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("mod");
    fs::create_dir_all(root.join("fomod")).unwrap();
    fs::write(root.join("x.asi"), "x").unwrap();
    fs::write(dir.path().join("secret.txt"), "outside").unwrap();
    let xml = r#"<config><requiredInstallFiles>
        <file source="x.asi" destination="bin64\..\..\..\.config\autostart\x.desktop"/>
        <file source="..\secret.txt" destination="bin64\secret.txt"/>
        <folder source="..\.." destination="bin64"/>
        <file source="x.asi" destination="C:\Windows\x.asi"/>
        <file source="x.asi" destination="bin64\ok.asi"/>
    </requiredInstallFiles></config>"#;
    fs::write(root.join("fomod/ModuleConfig.xml"), xml).unwrap();
    let config = load(&root).unwrap().unwrap();
    let files: Vec<String> = resolve(&root, &config, None)
        .iter()
        .map(|(_, d)| d.display().to_string())
        .collect();
    assert_eq!(files, ["bin64/ok.asi"]);
}

#[test]
fn plugins_sharing_a_name_are_told_apart_and_deep_conditions_are_bounded() {
    let twins = XML.replacen(r#"<plugin name="2K">"#, r#"<plugin name="4K">"#, 1);
    let config = parse(&twins).unwrap();
    let first = resolve::selection_of(&config, &[(0, 0, 0)]);
    let stored = Selection::decode(Some(&first.encode())).unwrap();
    let picks = resolve::walk(&config, Some(&stored)).picks;
    assert!(
        picks.contains(&(0, 0, 0)) && !picks.contains(&(0, 0, 1)),
        "{picks:?}"
    );

    let deep = format!(
        "{}<flagDependency flag=\"x\" value=\"y\"/>{}",
        "<dependencies>".repeat(5000),
        "</dependencies>".repeat(5000)
    );
    let hostile = XML.replacen(
        r#"<installStep name="Textures">"#,
        &format!(r#"<installStep name="Textures"><visible>{deep}</visible>"#),
        1,
    );
    // Parsing either refuses the document or bounds the nesting; neither crashes.
    if let Ok(config) = parse(&hostile) {
        assert!(!resolve::walk(&config, None).visible.is_empty());
    }
}

#[test]
fn a_broken_installer_deploys_nothing_and_says_why() {
    let dir = tempfile::tempdir().unwrap();
    write_mod(dir.path());
    fs::write(dir.path().join("FOMOD/ModuleConfig.xml"), "<config><oops").unwrap();
    let files = crate::mod_engine::plan::deployable_files(dir.path(), None).unwrap();
    assert!(files.is_empty(), "{files:?}");
    let details = crate::mod_engine::info::details(dir.path(), None).unwrap();
    assert!(details.installer_error.is_some() && details.variants.is_empty());
}
