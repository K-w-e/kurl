use kurl::http::{from_curl, to_curl};
use kurl::models::{AuthConfig, Environment, HttpMethod, KeyValuePair, Request};

#[test]
fn test_parse_posting_yaml_full_compatibility() {
    let yaml_content = r#"
name: Create user
description: Adds a new user to the system.
method: POST
url: https://jsonplaceholder.typicode.com/users
headers:
  - name: Content-Type
    value: application/json
  - name: Accept
    value: application/json
params:
  - name: sendWelcomeEmail
    value: 'true'
path_params:
  - name: postId
    value: '3'
body:
  content: |-
    {
      "firstName": "John",
      "email": "john.doe@example.com"
    }
"#;

    let req = Request::from_yaml_str(yaml_content).expect("Should parse Posting YAML format");
    assert_eq!(req.name, "Create user");
    assert_eq!(
        req.description.as_deref(),
        Some("Adds a new user to the system.")
    );
    assert_eq!(req.method, HttpMethod::Post);
    assert_eq!(req.url, "https://jsonplaceholder.typicode.com/users");
    assert_eq!(req.headers.len(), 2);
    assert_eq!(req.headers[0].name, "Content-Type");
    assert_eq!(req.headers[0].value, "application/json");
    assert!(req.headers[0].enabled);
    assert_eq!(req.params.len(), 1);
    assert_eq!(req.params[0].name, "sendWelcomeEmail");
    assert_eq!(req.params[0].value, "true");
    assert_eq!(req.path_params.len(), 1);
    assert_eq!(req.path_params[0].name, "postId");
    assert_eq!(req.path_params[0].value, "3");

    let body = req.body.expect("Body should be present");
    let content = body.content.expect("Content should be present");
    assert!(content.contains("\"firstName\": \"John\""));
}

#[test]
fn test_roundtrip_yaml_serialization() {
    let original = Request::default();
    let serialized = original.to_yaml_str().expect("Serialization must succeed");
    let deserialized = Request::from_yaml_str(&serialized).expect("Deserialization must succeed");

    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.method, deserialized.method);
    assert_eq!(original.url, deserialized.url);
    assert_eq!(original.headers, deserialized.headers);
}

#[test]
fn test_url_validation() {
    let valid_req = Request::new("Valid", HttpMethod::Get, "https://httpbin.org/get");
    assert!(valid_req.validate_url().is_ok());

    let invalid_scheme = Request::new("Invalid", HttpMethod::Get, "ftp://example.com");
    assert!(invalid_scheme.validate_url().is_err());

    let empty_url = Request::new("Empty", HttpMethod::Get, "");
    assert!(empty_url.validate_url().is_err());
}

#[test]
fn test_environment_interpolation() {
    let mut env = Environment::new("test");
    env.variables
        .insert("HOST".to_string(), "api.example.com".to_string());
    env.variables
        .insert("USER_ID".to_string(), "99".to_string());

    let mut req = Request::new("Test", HttpMethod::Get, "https://{{HOST}}/users/:userId");
    req.path_params = vec![KeyValuePair::new("userId", "{{USER_ID}}")];

    let resolved = env.resolve_request(&req);
    assert_eq!(resolved.url, "https://api.example.com/users/99");
}

#[test]
fn test_curl_export_and_import() {
    let mut req = Request::new("Sample", HttpMethod::Post, "https://httpbin.org/post");
    req.headers = vec![KeyValuePair::new("Authorization", "Bearer mysecrettoken")];

    let curl_str = to_curl(&req);
    assert!(curl_str.contains("-X POST"));
    assert!(curl_str.contains("https://httpbin.org/post"));
    assert!(curl_str.contains("-H \"Authorization: Bearer mysecrettoken\""));

    let imported = from_curl(
        "curl -X PUT https://httpbin.org/put -H 'Content-Type: application/json' -d '{\"ok\":true}'",
    )
    .expect("Should parse cURL");

    assert_eq!(imported.method, HttpMethod::Put);
    assert_eq!(imported.url, "https://httpbin.org/put");
    assert_eq!(imported.headers.len(), 1);
    assert_eq!(imported.headers[0].name, "Content-Type");
    assert!(imported
        .body
        .unwrap()
        .content
        .unwrap()
        .contains("{\"ok\":true}"));
}

#[test]
fn test_auth_config_type_cycling() {
    let none = AuthConfig::None;
    assert_eq!(none.type_name(), "None");

    let bearer = none.next_type();
    assert_eq!(bearer.type_name(), "Bearer Token");

    let basic = bearer.next_type();
    assert_eq!(basic.type_name(), "Basic Auth");

    let api_key = basic.next_type();
    assert_eq!(api_key.type_name(), "API Key");

    let back_to_none = api_key.next_type();
    assert_eq!(back_to_none.type_name(), "None");
}

#[test]
fn test_response_search_and_navigation() {
    let mut session = kurl::app::RequestTabSession::blank(1);
    session.active_response = Some(kurl::models::Response {
        status_code: 200,
        status_text: "OK".to_string(),
        duration_ms: 10,
        headers: Vec::new(),
        body: "{\n  \"apple\": 1,\n  \"banana\": 2,\n  \"pineapple\": 3\n}".to_string(),
        body_size_bytes: 50,
        content_type: Some("application/json".to_string()),
    });

    session.update_search(Some("apple".to_string()));
    assert_eq!(session.search_matches.len(), 2);
    assert_eq!(session.selected_match_idx, 0);

    session.next_search_match();
    assert_eq!(session.selected_match_idx, 1);

    session.next_search_match();
    assert_eq!(session.selected_match_idx, 0);

    session.prev_search_match();
    assert_eq!(session.selected_match_idx, 1);

    session.update_search(None);
    assert!(session.search_query.is_none());
    assert!(session.search_matches.is_empty());
}

#[test]
fn test_sidebar_entry_mapping_and_accurate_click() {
    let collections = vec![
        kurl::models::CollectionItem {
            name: "First Request".to_string(),
            path: std::path::PathBuf::from("first.posting.yaml"),
            group: None,
            request: Request::new("First Request", HttpMethod::Get, "https://httpbin.org/get"),
        },
        kurl::models::CollectionItem {
            name: "Second Request".to_string(),
            path: std::path::PathBuf::from("auth/second.posting.yaml"),
            group: Some("auth".to_string()),
            request: Request::new("Second Request", HttpMethod::Post, "https://httpbin.org/post"),
        },
        kurl::models::CollectionItem {
            name: "Third Request".to_string(),
            path: std::path::PathBuf::from("auth/third.posting.yaml"),
            group: Some("auth".to_string()),
            request: Request::new("Third Request", HttpMethod::Get, "https://httpbin.org/auth"),
        },
    ];
    let groups = vec!["auth".to_string()];

    let entries = kurl::ui::sidebar::build_sidebar_entries(&collections, &groups);

    assert_eq!(entries.len(), 5);
    assert_eq!(
        entries[0],
        kurl::ui::sidebar::SidebarEntry::GroupHeader {
            name: "Root / Default".to_string(),
            is_root: true,
        }
    );
    assert_eq!(
        entries[1],
        kurl::ui::sidebar::SidebarEntry::RequestItem { collection_idx: 0 }
    );
    assert_eq!(
        entries[2],
        kurl::ui::sidebar::SidebarEntry::GroupHeader {
            name: "auth".to_string(),
            is_root: false,
        }
    );
    assert_eq!(
        entries[3],
        kurl::ui::sidebar::SidebarEntry::RequestItem { collection_idx: 1 }
    );
    assert_eq!(
        entries[4],
        kurl::ui::sidebar::SidebarEntry::RequestItem { collection_idx: 2 }
    );
}

#[test]
fn test_tab_horizontal_scroll_layout() {
    let temp_dir = std::env::temp_dir().join(format!("kurl_test_tab_{}", uuid::Uuid::new_v4()));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut config = kurl::config::Config::default();
    config.persistence_dir = temp_dir.join(".kurl");
    config.collections_dir = temp_dir.join("collections");

    let mut app = kurl::app::App::new(config, tx).expect("App creation must succeed");
    app.open_tabs.clear();

    for i in 1..=10 {
        let mut tab = kurl::app::RequestTabSession::blank(i);
        tab.title = format!("Request Tab Number {}", i);
        app.open_tabs.push(tab);
    }
    assert_eq!(app.open_tabs.len(), 10);

    app.term_width = 80;
    app.active_tab_index = 9;
    app.ensure_active_tab_visible();
    assert!(app.workspace_tabs_scroll > 0);

    app.active_tab_index = 0;
    app.ensure_active_tab_visible();
    assert_eq!(app.workspace_tabs_scroll, 0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_footer_click_targets_and_mouse_interaction() {
    let temp_dir = std::env::temp_dir().join(format!("kurl_test_footer_{}", uuid::Uuid::new_v4()));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut config = kurl::config::Config::default();
    config.persistence_dir = temp_dir.join(".kurl");
    config.collections_dir = temp_dir.join("collections");

    let mut app = kurl::app::App::new(config, tx).expect("App creation must succeed");
    app.sidebar_view_mode = kurl::events::SidebarViewMode::Collections;

    let (_, targets) = kurl::ui::footer::build_footer(&app, 160);
    assert!(!targets.is_empty());

    let send_target = targets
        .iter()
        .find(|t| t.action == kurl::events::Action::SendRequest)
        .expect("Send button target must exist");
    assert!(send_target.end_x > send_target.start_x);

    let new_req_target = targets
        .iter()
        .find(|t| t.action == kurl::events::Action::OpenNewRequestModal)
        .expect("New Req button target must exist");
    assert!(new_req_target.end_x > new_req_target.start_x);

    let area = ratatui::layout::Rect::new(0, 0, 160, 40);
    let layout = kurl::ui::layout::AppLayout::compute(area, app.sidebar_width, app.request_split_percent);

    let mouse_send = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: layout.footer.x + send_target.start_x + 1,
        row: layout.footer.y,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let action_send = app.handle_mouse_event(mouse_send, area);
    assert_eq!(action_send, Some(kurl::events::Action::SendRequest));

    let mouse_new_req = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: layout.footer.x + new_req_target.start_x + 1,
        row: layout.footer.y,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let action_new_req = app.handle_mouse_event(mouse_new_req, area);
    assert_eq!(action_new_req, Some(kurl::events::Action::OpenNewRequestModal));

    app.sidebar_view_mode = kurl::events::SidebarViewMode::History;
    let (_, hist_targets) = kurl::ui::footer::build_footer(&app, 160);
    let has_clear = hist_targets
        .iter()
        .any(|t| t.action == kurl::events::Action::OpenClearHistoryModal);
    assert!(has_clear);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_modal_mouse_interaction() {
    let temp_dir = std::env::temp_dir().join(format!("kurl_test_modal_mouse_{}", uuid::Uuid::new_v4()));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut config = kurl::config::Config::default();
    config.persistence_dir = temp_dir.join(".kurl");
    config.collections_dir = temp_dir.join("collections");

    let mut app = kurl::app::App::new(config, tx).expect("App creation must succeed");
    let area = ratatui::layout::Rect::new(0, 0, 120, 40);

    app.active_modal = kurl::app::Modal::ConfirmClearHistory;
    let modal_rect = kurl::ui::modal::modal_rect(&app.active_modal, area).expect("Modal rect must exist");

    let mouse_outside = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let action_outside = app.handle_mouse_event(mouse_outside, area);
    assert_eq!(action_outside, Some(kurl::events::Action::CloseModal));

    let btn_row = modal_rect.y + modal_rect.height - 2;
    let mouse_confirm = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: modal_rect.x + 5,
        row: btn_row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let action_confirm = app.handle_mouse_event(mouse_confirm, area);
    assert_eq!(action_confirm, Some(kurl::events::Action::SubmitModal));

    let mouse_cancel = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: modal_rect.x + modal_rect.width - 5,
        row: btn_row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let action_cancel = app.handle_mouse_event(mouse_cancel, area);
    assert_eq!(action_cancel, Some(kurl::events::Action::CloseModal));

    app.active_modal = kurl::app::Modal::NewRequest {
        name_input: "Test".to_string(),
        selected_method: HttpMethod::Get,
        group_input: String::new(),
        url_input: "http://example.com".to_string(),
        active_field: 0,
    };
    let req_modal_rect = kurl::ui::modal::modal_rect(&app.active_modal, area).expect("Modal rect must exist");
    let mouse_field2 = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: req_modal_rect.x + 10,
        row: req_modal_rect.y + 8,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let _ = app.handle_mouse_event(mouse_field2, area);
    if let kurl::app::Modal::NewRequest { active_field, .. } = app.active_modal {
        assert_eq!(active_field, 2);
    } else {
        panic!("Modal should still be NewRequest");
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_persistence_state_and_history() {
    let temp_dir = std::env::temp_dir().join(format!("kurl_test_{}", uuid::Uuid::new_v4()));
    let manager = kurl::app::PersistenceManager::new(&temp_dir);

    let mut tab1 = kurl::app::RequestTabSession::blank(1);
    tab1.title = "Tab One".to_string();
    tab1.request.url = "https://httpbin.org/get".to_string();

    let mut tab2 = kurl::app::RequestTabSession::blank(2);
    tab2.title = "Tab Two".to_string();
    tab2.request.method = HttpMethod::Post;

    let saved_state = kurl::app::AppSavedState {
        active_tab_index: 1,
        open_tabs: vec![tab1, tab2],
        sidebar_width: 35,
        request_split_percent: 60,
        active_env_index: 2,
        sidebar_view_mode: kurl::events::SidebarViewMode::History,
    };

    manager.save_state(&saved_state).expect("Saving state should succeed");
    let loaded_state = manager.load_state().expect("Loading state should succeed").expect("State should exist");

    assert_eq!(loaded_state.active_tab_index, 1);
    assert_eq!(loaded_state.open_tabs.len(), 2);
    assert_eq!(loaded_state.open_tabs[0].title, "Tab One");
    assert_eq!(loaded_state.open_tabs[1].request.method, HttpMethod::Post);
    assert_eq!(loaded_state.sidebar_width, 35);
    assert_eq!(loaded_state.request_split_percent, 60);
    assert_eq!(loaded_state.active_env_index, 2);
    assert_eq!(loaded_state.sidebar_view_mode, kurl::events::SidebarViewMode::History);

    let hist_item = kurl::models::HistoryItem::new(
        1,
        Request::new("HistReq", HttpMethod::Get, "https://httpbin.org/status/200"),
        kurl::models::Response {
            status_code: 200,
            status_text: "OK".to_string(),
            duration_ms: 42,
            headers: Vec::new(),
            body: "{\"status\": \"healthy\"}".to_string(),
            body_size_bytes: 23,
            content_type: Some("application/json".to_string()),
        },
    );

    manager.save_history(&[hist_item]).expect("Saving history should succeed");
    let loaded_history = manager.load_history().expect("Loading history should succeed");
    assert_eq!(loaded_history.len(), 1);
    assert_eq!(loaded_history[0].status_code, 200);
    assert_eq!(loaded_history[0].duration_ms, 42);
    assert_eq!(loaded_history[0].request.name, "HistReq");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_bruno_posting_config_resolution() {
    let global_dir = kurl::config::Config::global_dir();
    assert!(global_dir.to_string_lossy().contains("kurl"));

    let custom_path = std::path::PathBuf::from("/custom/path/collections");
    let cfg = kurl::config::Config::new(Some(custom_path.clone()));
    assert_eq!(cfg.collections_dir, custom_path);
    assert_eq!(cfg.persistence_dir, global_dir);
}

#[test]
fn test_persistence_migration_from_local() {
    let temp_dir = std::env::temp_dir().join(format!("kurl_test_migration_{}", uuid::Uuid::new_v4()));
    let local_kurl = temp_dir.join(".kurl");
    let target_global = temp_dir.join("global_kurl");

    std::fs::create_dir_all(&local_kurl).expect("Create local .kurl");
    std::fs::write(local_kurl.join("state.json"), r#"{"active_tab_index":0,"open_tabs":[],"sidebar_width":32,"request_split_percent":50,"active_env_index":0,"sidebar_view_mode":"Collections"}"#).expect("Write mock state");
    std::fs::write(local_kurl.join("history.json"), r#"[]"#).expect("Write mock history");

    let current_dir = std::env::current_dir().expect("Get current dir");
    std::env::set_current_dir(&temp_dir).expect("Set temp dir");

    let pm = kurl::app::PersistenceManager::new(&target_global);
    let state = pm.load_state().expect("Load migrated state");
    assert!(state.is_some());
    assert!(!temp_dir.join(".kurl").exists());

    let _ = std::env::set_current_dir(&current_dir);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_postman_collection_import_and_export() {
    let postman_json = r#"{
      "info": {
        "name": "Sample Postman Collection",
        "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
      },
      "item": [
        {
          "name": "Auth",
          "item": [
            {
              "name": "Login User",
              "request": {
                "method": "POST",
                "header": [
                  { "key": "Content-Type", "value": "application/json" }
                ],
                "url": {
                  "raw": "https://api.example.com/auth/login?redirect=dashboard",
                  "query": [
                    { "key": "redirect", "value": "dashboard" }
                  ]
                },
                "body": {
                  "mode": "raw",
                  "raw": "{\"username\":\"admin\",\"password\":\"secret\"}"
                },
                "auth": {
                  "type": "bearer",
                  "bearer": [
                    { "key": "token", "value": "test_token_123" }
                  ]
                }
              }
            }
          ]
        },
        {
          "name": "Get Status",
          "request": {
            "method": "GET",
            "url": "https://api.example.com/health"
          }
        }
      ]
    }"#;

    let (coll_name, items) = kurl::formats::import_postman_collection(postman_json)
        .expect("Postman import must succeed");

    assert_eq!(coll_name, "Sample Postman Collection");
    assert_eq!(items.len(), 2);

    let (login_req, login_group) = &items[0];
    assert_eq!(login_req.name, "Login User");
    assert_eq!(login_req.method, HttpMethod::Post);
    assert_eq!(login_group.as_deref(), Some("Auth"));
    assert_eq!(login_req.headers.len(), 1);
    assert_eq!(login_req.headers[0].name, "Content-Type");
    assert!(login_req.body.is_some());
    assert_eq!(
        login_req.auth,
        AuthConfig::Bearer {
            token: "test_token_123".to_string()
        }
    );

    let (status_req, status_group) = &items[1];
    assert_eq!(status_req.name, "Get Status");
    assert_eq!(status_req.method, HttpMethod::Get);
    assert_eq!(status_group, &None);

    let collection_items = vec![
        kurl::models::CollectionItem {
            path: std::path::PathBuf::from("auth/login.posting.yaml"),
            name: login_req.name.clone(),
            group: login_group.clone(),
            request: login_req.clone(),
        },
        kurl::models::CollectionItem {
            path: std::path::PathBuf::from("status.posting.yaml"),
            name: status_req.name.clone(),
            group: status_group.clone(),
            request: status_req.clone(),
        },
    ];

    let exported_json = kurl::formats::export_postman_collection(&coll_name, &collection_items)
        .expect("Postman export must succeed");
    assert!(exported_json.contains("Sample Postman Collection"));
    assert!(exported_json.contains("Login User"));
    assert!(exported_json.contains("Get Status"));
}

#[test]
fn test_postman_environment_import_and_export() {
    let env_json = r#"{
      "name": "Staging Environment",
      "values": [
        { "key": "BASE_URL", "value": "https://staging.api.io", "enabled": true },
        { "key": "API_KEY", "value": "sk_stage_999", "enabled": true },
        { "key": "DISABLED_VAR", "value": "ignore_me", "enabled": false }
      ]
    }"#;

    let env = kurl::formats::import_postman_environment(env_json)
        .expect("Postman environment import must succeed");

    assert_eq!(env.name, "Staging Environment");
    assert_eq!(env.variables.get("BASE_URL").map(|s| s.as_str()), Some("https://staging.api.io"));
    assert_eq!(env.variables.get("API_KEY").map(|s| s.as_str()), Some("sk_stage_999"));
    assert!(!env.variables.contains_key("DISABLED_VAR"));

    let exported = kurl::formats::export_postman_environment(&env)
        .expect("Postman environment export must succeed");
    assert!(exported.contains("Staging Environment"));
    assert!(exported.contains("BASE_URL"));
    assert!(exported.contains("sk_stage_999"));
}

#[test]
fn test_bruno_format_import_and_export() {
    let bru_content = r#"meta {
  name: Create Customer
  type: http
  seq: 1
}

post {
  url: https://api.example.com/customers
  body: json
  auth: bearer
}

headers {
  Content-Type: application/json
  X-Request-Id: req_12345
}

params:query {
  notify: true
}

auth:bearer {
  token: bruno_secret_token
}

body:json {
  {
    "company": "Acme Inc"
  }
}
"#;

    let req = kurl::formats::parse_bru(bru_content).expect("Bruno parse must succeed");
    assert_eq!(req.name, "Create Customer");
    assert_eq!(req.method, HttpMethod::Post);
    assert_eq!(req.url, "https://api.example.com/customers");
    assert_eq!(req.headers.len(), 2);
    assert_eq!(req.headers[0].name, "Content-Type");
    assert_eq!(req.headers[1].name, "X-Request-Id");
    assert_eq!(req.params.len(), 1);
    assert_eq!(req.params[0].name, "notify");
    assert_eq!(
        req.auth,
        AuthConfig::Bearer {
            token: "bruno_secret_token".to_string()
        }
    );
    assert!(req.body.as_ref().unwrap().content.as_ref().unwrap().contains("Acme Inc"));

    let exported_bru = kurl::formats::to_bru(&req);
    assert!(exported_bru.contains("name: Create Customer"));
    assert!(exported_bru.contains("post {"));
    assert!(exported_bru.contains("url: https://api.example.com/customers"));
    assert!(exported_bru.contains("Content-Type: application/json"));
    assert!(exported_bru.contains("token: bruno_secret_token"));
}

#[test]
fn test_universal_format_detector() {
    let curl_sample = "curl -X DELETE https://api.example.com/users/1";
    let detected_curl = kurl::formats::detect_import_payload(curl_sample).expect("Detect curl");
    assert!(matches!(detected_curl, kurl::formats::ImportPayload::Curl(_)));

    let postman_coll_sample = r#"{"info":{"name":"Coll","schema":"http..."},"item":[]}"#;
    let detected_postman = kurl::formats::detect_import_payload(postman_coll_sample).expect("Detect postman");
    assert!(matches!(detected_postman, kurl::formats::ImportPayload::PostmanCollection(_)));

    let postman_env_sample = r#"{"name":"Env","values":[{"key":"K","value":"V"}]}"#;
    let detected_env = kurl::formats::detect_import_payload(postman_env_sample).expect("Detect env");
    assert!(matches!(detected_env, kurl::formats::ImportPayload::PostmanEnvironment(_)));

    let bru_sample = "meta {\n  name: Test\n}\nget {\n  url: http://a.com\n}\n";
    let detected_bru = kurl::formats::detect_import_payload(bru_sample).expect("Detect bru");
    assert!(matches!(detected_bru, kurl::formats::ImportPayload::BrunoText(_)));
}
