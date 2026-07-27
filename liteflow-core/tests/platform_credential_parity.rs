//! Java `PlatformCredential` getter 与 Provider 配置语义回归测试。

use std::collections::HashMap;

use liteflow_core::property::agent::PlatformCredential;

#[test]
fn java_named_getters_read_the_provider_configuration_written_by_setters() {
    let mut credential = PlatformCredential::default();
    credential.set_api_key(Some("secret".to_string()));
    credential.set_base_url(Some("https://gateway.example/v1".to_string()));
    credential.set_extra(HashMap::from([
        ("organization".to_string(), "acme".to_string()),
        ("project".to_string(), "liteflow".to_string()),
    ]));

    assert_eq!(credential.get_api_key(), Some("secret"));
    assert_eq!(
        credential.get_base_url(),
        Some("https://gateway.example/v1")
    );
    assert_eq!(credential.get_extra()["organization"], "acme");

    let value = serde_json::to_value(credential).expect("PlatformCredential 应可序列化");
    assert_eq!(value["apiKey"], "secret");
    assert_eq!(value["baseUrl"], "https://gateway.example/v1");
    assert_eq!(value["extra"]["project"], "liteflow");
}
