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
    assert_eq!(init["result"]["serverInfo"]["name"], "pylune");
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
