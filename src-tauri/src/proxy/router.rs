use crate::config::provider::Provider;

/// Route every request to the currently enabled provider.
///
/// The incoming model name is ignored on purpose: `keyword` is now only a UI label,
/// not a routing rule.
pub fn match_provider<'a>(_model: &str, providers: &'a [Provider]) -> Option<&'a Provider> {
    providers.first()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::provider::{AuthHeader, Protocol};

    fn make_provider(keyword: &str) -> Provider {
        Provider {
            id: format!("test-{}", keyword),
            name: format!("Test {}", keyword),
            base_url: "https://api.test.com".to_string(),
            api_key_enc: "".to_string(),
            protocol: Protocol::Anthropic,
            model_mapping: None,
            auth_header: AuthHeader::ApiKey,
            keyword: keyword.to_string(),
            enabled: true,
            sort_order: 0,
        }
    }

    #[test]
    fn test_routes_to_first_enabled_provider() {
        let providers = vec![
            make_provider("opus"),
            make_provider("sonnet"),
            make_provider("haiku"),
        ];

        assert_eq!(
            match_provider("claude-opus-4-6", &providers)
                .unwrap()
                .keyword,
            "opus"
        );
        assert_eq!(
            match_provider("claude-sonnet-4-6", &providers)
                .unwrap()
                .keyword,
            "opus"
        );
        assert_eq!(
            match_provider("claude-haiku-4-5", &providers)
                .unwrap()
                .keyword,
            "opus"
        );
    }

    #[test]
    fn test_single_provider_is_used_for_any_model() {
        let providers = vec![make_provider("opus")];
        assert_eq!(
            match_provider("unknown-model", &providers).unwrap().keyword,
            "opus"
        );
        assert_eq!(
            match_provider("claude-opus-4-6", &providers)
                .unwrap()
                .keyword,
            "opus"
        );
        assert_eq!(
            match_provider("claude-sonnet-4-6", &providers)
                .unwrap()
                .keyword,
            "opus"
        );
    }
}
