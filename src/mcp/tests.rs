use super::*;
use crate::tests::fixture;

fn server(fx: &crate::tests::Fixture) -> Server {
    let root = fx.staging.parent().unwrap().to_path_buf();
    let paths = DataPaths::under(&root, &root);
    paths.create_dirs().unwrap();
    Server::new(paths, fx.config.clone())
}

fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
    let message = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": arguments}});
    server.handle(&message).unwrap()["result"].clone()
}

#[test]
fn speaks_mcp_and_manages_mods() {
    let fx = fixture();
    let mut server = server(&fx);
    let init = server
        .handle(&json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": PROTOCOL}}))
        .unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], "pailune");
    assert!(
        server
            .handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .is_none()
    );
    let listed = server
        .handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}))
        .unwrap();
    assert_eq!(
        listed["result"]["tools"].as_array().unwrap().len(),
        tools::TOOLS.len()
    );
    let unknown = server
        .handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "nope"}))
        .unwrap();
    assert_eq!(unknown["error"]["code"], -32601);

    let paths = json!([fx.sources.join("Cool Mod"), fx.sources.join("Overlay")]);
    let installed = call(&mut server, "install_mods", json!({ "paths": paths }));
    assert_eq!(installed["isError"], false, "{installed}");
    call(&mut server, "enable_mods", json!({"names": ["Overlay"]}));
    call(
        &mut server,
        "move_mod",
        json!({"name": "Overlay", "position": 1}),
    );
    let mods = call(&mut server, "list_mods", json!({}));
    let first = &mods["structuredContent"]["data"][0];
    assert_eq!(
        (first["name"].as_str(), first["enabled"].as_bool()),
        (Some("Overlay"), Some(true))
    );
    let applied = call(&mut server, "apply", json!({}));
    assert_eq!(applied["isError"], false, "{applied}");
    let status = call(&mut server, "status", json!({}));
    assert_eq!(
        status["structuredContent"]["data"]["pending"], false,
        "{status}"
    );
    let refused = call(&mut server, "clean_slate", json!({}));
    assert_eq!(refused["isError"], true);
    let missing = call(&mut server, "enable_mods", json!({"names": ["Nope"]}));
    assert_eq!(missing["isError"], true);
}

#[test]
fn failed_jobs_are_errors_and_the_lock_is_released() {
    let fx = fixture();
    let mut server = server(&fx);
    let missing = json!([fx.sources.join("No Such Mod")]);
    let failed = call(&mut server, "install_mods", json!({ "paths": missing }));
    assert_eq!(failed["isError"], true, "{failed}");
    let wait = std::time::Duration::from_millis(500);
    let lock = crate::instance::try_lock_for(&server.paths, wait).unwrap();
    assert!(lock.is_some(), "the window can start between calls");
}

#[test]
fn mcp_spawns_and_exports_mod_project() {
    let fx = fixture();
    let mut server = server(&fx);
    let temp = tempfile::tempdir().unwrap();
    let projects_dir = temp.path().join("projects");
    let exports_dir = temp.path().join("exports");

    let spawn = call(
        &mut server,
        "spawn_mod_project",
        json!({
            "name": "McpStamina",
            "location": projects_dir.to_str().unwrap(),
            "mod_type": "json",
            "author": "McpAuthor",
            "version": "1.0",
            "description": "Infinite stamina via MCP"
        }),
    );
    assert_eq!(spawn["isError"], false, "{spawn}");
    assert!(
        projects_dir
            .join("McpStamina")
            .join("McpStamina.field.json")
            .is_file()
    );

    let export = call(
        &mut server,
        "export_mod_project",
        json!({
            "project_path": projects_dir.join("McpStamina").to_str().unwrap(),
            "output_path": exports_dir.to_str().unwrap(),
            "stage": true
        }),
    );
    assert_eq!(export["isError"], false, "{export}");
    assert!(exports_dir.join("McpStamina-v1.0.zip").is_file());
    assert!(
        server
            .paths
            .staging
            .join("McpStamina/McpStamina.field.json")
            .is_file()
    );
}

#[test]
fn mcp_builds_and_reads_plugin_log() {
    let fx = fixture();
    let mut server = server(&fx);
    let temp = tempfile::tempdir().unwrap();
    let project_dir = temp.path().join("McpAsi");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(
        project_dir.join("modinfo.json"),
        r#"{"name": "McpAsi", "type": "asi"}"#,
    )
    .unwrap();
    std::fs::write(project_dir.join("config.ini"), "[Settings]\nSpeed=2.0\n").unwrap();
    let build_dir = project_dir.join("build");
    std::fs::create_dir_all(&build_dir).unwrap();
    std::fs::write(build_dir.join("McpAsi.asi"), b"MZ_MOCK_ASI").unwrap();
    std::fs::write(project_dir.join("Makefile"), "all:\n").unwrap();

    let staged = crate::sdk::stage_built_project(
        &project_dir,
        &build_dir.join("McpAsi.asi"),
        &server.paths.staging,
    )
    .unwrap();
    assert!(staged.join("bin64/McpAsi.asi").is_file());

    let bin64 = fx.config.game.dir.join("bin64");
    std::fs::create_dir_all(&bin64).unwrap();
    std::fs::write(bin64.join("McpAsi.log"), "Init\nHooked\nRunning\n").unwrap();

    let log_call = call(
        &mut server,
        "read_plugin_log",
        json!({
            "name": "McpAsi",
            "limit": 2
        }),
    );
    assert_eq!(log_call["isError"], false, "{log_call}");
    let lines = log_call["structuredContent"]["data"]["lines"]
        .as_array()
        .unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Hooked");
    assert_eq!(lines[1], "Running");
}
