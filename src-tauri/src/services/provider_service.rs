use crate::config::provider::{AuthHeader, Protocol, Provider};
use crate::config::store;
use crate::db::logs::DbConn;
use uuid::Uuid;

pub async fn create(
    db: &DbConn,
    name: String,
    base_url: String,
    api_key: String,
    protocol: String,
    model_mapping: Option<String>,
    auth_header: String,
    keyword: String,
    enabled: bool,
) -> Result<Provider, String> {
    let id = Uuid::new_v4().to_string();
    let api_key_enc = store::store_api_key(&id, &api_key);

    let provider = Provider {
        id,
        name,
        base_url,
        api_key_enc,
        protocol: Protocol::from_str(&protocol).unwrap_or(Protocol::Anthropic),
        model_mapping,
        auth_header: AuthHeader::from_str(&auth_header).unwrap_or(AuthHeader::ApiKey),
        keyword,
        enabled,
        sort_order: 0,
    };

    store::create_provider(db, &provider).await?;
    Ok(provider)
}

pub async fn update(
    db: &DbConn,
    id: String,
    name: String,
    base_url: String,
    api_key: Option<String>,
    protocol: String,
    model_mapping: Option<String>,
    auth_header: String,
    keyword: String,
    enabled: bool,
) -> Result<(), String> {
    let mut providers = store::list_providers(db).await?;
    let provider = providers
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or("Provider not found")?;

    provider.name = name;
    provider.base_url = base_url;
    if let Some(key) = api_key {
        provider.api_key_enc = store::store_api_key(&provider.id, &key);
    }
    provider.protocol = Protocol::from_str(&protocol).unwrap_or(Protocol::Anthropic);
    provider.model_mapping = model_mapping;
    provider.auth_header = AuthHeader::from_str(&auth_header).unwrap_or(AuthHeader::ApiKey);
    provider.keyword = keyword;
    provider.enabled = enabled;

    store::update_provider(db, provider).await
}

pub async fn delete(db: &DbConn, id: String) -> Result<(), String> {
    store::delete_provider(db, &id).await
}
