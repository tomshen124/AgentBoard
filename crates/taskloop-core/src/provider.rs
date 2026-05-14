use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use crate::contracts::{
    ApiFamily, AuthMode, ContractLoadError, HeaderPolicy, ModelAdapter, ProviderCapability,
    ProviderProtocol, UserAgentMode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub profile: String,
    pub adapter: ModelAdapter,
}

impl ProviderConfig {
    pub fn compose_headers(&self, auth_token: Option<&str>) -> Vec<(String, String)> {
        let mut headers = Vec::new();

        match (&self.adapter.auth, auth_token) {
            (AuthMode::Bearer, Some(token)) => {
                headers.push(("Authorization".into(), format!("Bearer {token}")));
            }
            (AuthMode::ApiKeyHeader, Some(token)) => {
                let header_name = self
                    .adapter
                    .auth_header_name
                    .clone()
                    .unwrap_or_else(|| match self.adapter.protocol {
                        ProviderProtocol::AnthropicCompatible => "x-api-key".to_string(),
                        _ => "X-API-Key".to_string(),
                    });
                headers.push((header_name, token.to_string()));
            }
            _ => {}
        }

        match &self.adapter.user_agent_mode {
            UserAgentMode::RuntimeDefault => {
                headers.push(("User-Agent".into(), "AgentBoard/0.1 TaskLoop".into()));
            }
            UserAgentMode::Custom(value) => {
                headers.push(("User-Agent".into(), value.clone()));
            }
            UserAgentMode::None => {}
        }

        headers.extend(self.adapter.static_headers.clone());

        if matches!(self.adapter.protocol, ProviderProtocol::AnthropicCompatible)
            && !headers
                .iter()
                .any(|(key, _)| key.eq_ignore_ascii_case("anthropic-version"))
        {
            headers.push(("anthropic-version".into(), "2023-06-01".into()));
        }

        match self.adapter.header_policy {
            HeaderPolicy::Strict | HeaderPolicy::Compatible | HeaderPolicy::Extended => headers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderCatalog {
    providers: BTreeMap<String, ProviderConfig>,
}

impl ProviderCatalog {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, ContractLoadError> {
        let path = path.into();
        if !path.exists() {
            return Ok(Self::default());
        }

        let source = fs::read_to_string(path)?;
        parse_provider_catalog(&source)
    }

    pub fn get(&self, profile: &str) -> Option<&ProviderConfig> {
        self.providers.get(profile)
    }

    pub fn all(&self) -> Vec<&ProviderConfig> {
        self.providers.values().collect()
    }

    pub fn insert(&mut self, provider: ProviderConfig) {
        self.providers.insert(provider.profile.clone(), provider);
    }
}

pub fn default_provider_config_path(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join("config").join("providers.toml")
}

fn parse_provider_catalog(source: &str) -> Result<ProviderCatalog, ContractLoadError> {
    let normalized = source.replace("\r\n", "\n");
    let mut current_section: Option<String> = None;
    let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    for raw_line in normalized.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = Some(line[1..line.len() - 1].trim().to_string());
            continue;
        }

        let Some(section) = current_section.as_ref() else {
            return Err(ContractLoadError::InvalidFormat(
                "provider config key/value must appear inside a section".to_string(),
            ));
        };

        let Some((key, value)) = line.split_once('=') else {
            return Err(ContractLoadError::InvalidFormat(format!(
                "invalid provider config line `{line}`"
            )));
        };

        sections
            .entry(section.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
    }

    let mut catalog = ProviderCatalog::default();
    for (profile, values) in sections {
        let adapter = ModelAdapter {
            provider_id: required_value(&values, "provider_id")?,
            base_url: required_value(&values, "base_url")?,
            protocol: parse_protocol(&required_value(&values, "protocol")?)?,
            api_family: parse_api_family(&required_value(&values, "api_family")?)?,
            auth: parse_auth_mode(&required_value(&values, "auth")?)?,
            auth_header_name: optional_value(&values, "auth_header_name"),
            header_policy: parse_header_policy(&required_value(&values, "header_policy")?)?,
            user_agent_mode: parse_user_agent_mode(&required_value(&values, "user_agent_mode")?)?,
            static_headers: parse_header_array(optional_value(&values, "static_headers")),
            models: parse_array(&required_value(&values, "models")?),
            capabilities: parse_capabilities(optional_value(&values, "capabilities")),
        };

        catalog.insert(ProviderConfig { profile, adapter });
    }

    Ok(catalog)
}

fn required_value(
    values: &BTreeMap<String, String>,
    key: &str,
) -> Result<String, ContractLoadError> {
    values
        .get(key)
        .map(|value| strip_quotes(value))
        .ok_or_else(|| ContractLoadError::MissingField(key.to_string()))
}

fn optional_value(values: &BTreeMap<String, String>, key: &str) -> Option<String> {
    values.get(key).map(|value| strip_quotes(value))
}

fn parse_protocol(value: &str) -> Result<ProviderProtocol, ContractLoadError> {
    match value {
        "openai-compatible" => Ok(ProviderProtocol::OpenAiCompatible),
        "anthropic-compatible" => Ok(ProviderProtocol::AnthropicCompatible),
        "local" => Ok(ProviderProtocol::Local),
        other if !other.is_empty() => Ok(ProviderProtocol::Custom(other.to_string())),
        _ => Err(ContractLoadError::InvalidField {
            field: "protocol".into(),
            reason: "empty protocol".into(),
        }),
    }
}

fn parse_api_family(value: &str) -> Result<ApiFamily, ContractLoadError> {
    match value {
        "responses" => Ok(ApiFamily::Responses),
        "chat-completions" => Ok(ApiFamily::ChatCompletions),
        "anthropic-messages" => Ok(ApiFamily::AnthropicMessages),
        other if !other.is_empty() => Ok(ApiFamily::Custom(other.to_string())),
        _ => Err(ContractLoadError::InvalidField {
            field: "api_family".into(),
            reason: "empty api family".into(),
        }),
    }
}

fn parse_auth_mode(value: &str) -> Result<AuthMode, ContractLoadError> {
    match value {
        "bearer" => Ok(AuthMode::Bearer),
        "api-key-header" => Ok(AuthMode::ApiKeyHeader),
        "none" => Ok(AuthMode::None),
        other if !other.is_empty() => Ok(AuthMode::Custom(other.to_string())),
        _ => Err(ContractLoadError::InvalidField {
            field: "auth".into(),
            reason: "empty auth mode".into(),
        }),
    }
}

fn parse_header_policy(value: &str) -> Result<HeaderPolicy, ContractLoadError> {
    match value {
        "strict" => Ok(HeaderPolicy::Strict),
        "compatible" => Ok(HeaderPolicy::Compatible),
        "extended" => Ok(HeaderPolicy::Extended),
        other => Err(ContractLoadError::InvalidField {
            field: "header_policy".into(),
            reason: format!("unsupported header policy `{other}`"),
        }),
    }
}

fn parse_user_agent_mode(value: &str) -> Result<UserAgentMode, ContractLoadError> {
    match value {
        "runtime-default" => Ok(UserAgentMode::RuntimeDefault),
        "none" => Ok(UserAgentMode::None),
        other if other.starts_with("custom:") => {
            Ok(UserAgentMode::Custom(other.trim_start_matches("custom:").to_string()))
        }
        other => Err(ContractLoadError::InvalidField {
            field: "user_agent_mode".into(),
            reason: format!("unsupported user agent mode `{other}`"),
        }),
    }
}

fn parse_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let Some(inner) = trimmed.strip_prefix('[').and_then(|value| value.strip_suffix(']')) else {
        return Vec::new();
    };
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|item| strip_quotes(item.trim()))
        .collect()
}

fn parse_header_array(value: Option<String>) -> Vec<(String, String)> {
    let Some(value) = value else {
        return Vec::new();
    };
    parse_array(&value)
        .into_iter()
        .filter_map(|item| {
            let (key, value) = item.split_once(':')?;
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                return None;
            }
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

fn parse_capabilities(value: Option<String>) -> Vec<ProviderCapability> {
    let Some(value) = value else {
        return Vec::new();
    };
    parse_array(&value)
        .into_iter()
        .map(|item| match item.as_str() {
            "streaming" => ProviderCapability::Streaming,
            "tool-calling" => ProviderCapability::ToolCalling,
            "json-output" => ProviderCapability::JsonOutput,
            "image-input" => ProviderCapability::ImageInput,
            "reasoning" => ProviderCapability::Reasoning,
            "web-search" => ProviderCapability::WebSearch,
            other => ProviderCapability::Custom(other.to_string()),
        })
        .collect()
}

fn strip_quotes(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let matching_quotes = (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'');
        if matching_quotes {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use crate::contracts::{ApiFamily, ProviderProtocol};

    use super::{default_provider_config_path, ProviderCatalog};

    #[test]
    fn provider_catalog_loads_profiles() {
        let catalog = ProviderCatalog::load_from_str(
            r#"
[default]
provider_id = "openai_compatible_default"
base_url = "https://api.example.com/v1"
protocol = "openai-compatible"
api_family = "responses"
auth = "bearer"
header_policy = "strict"
user_agent_mode = "runtime-default"
models = ["gpt-4.1", "gpt-4.1-mini"]
capabilities = ["streaming", "tool-calling", "json-output"]
"#,
        )
        .unwrap();

        let provider = catalog.get("default").unwrap();
        assert_eq!(provider.adapter.models, vec!["gpt-4.1", "gpt-4.1-mini"]);
        assert!(matches!(provider.adapter.api_family, ApiFamily::Responses));
        assert_eq!(
            provider.compose_headers(Some("secret-token")),
            vec![
                ("Authorization".into(), "Bearer secret-token".into()),
                ("User-Agent".into(), "AgentBoard/0.1 TaskLoop".into())
            ]
        );
    }

    #[test]
    fn anthropic_profile_adds_default_version_header() {
        let catalog = ProviderCatalog::load_from_str(
            r#"
[claude]
provider_id = "anthropic_default"
base_url = "https://api.anthropic.com"
protocol = "anthropic-compatible"
api_family = "anthropic-messages"
auth = "api-key-header"
header_policy = "strict"
user_agent_mode = "none"
models = ["claude-sonnet-4-5"]
capabilities = ["streaming", "tool-calling"]
"#,
        )
        .unwrap();

        let provider = catalog.get("claude").unwrap();
        assert!(matches!(
            provider.adapter.protocol,
            ProviderProtocol::AnthropicCompatible
        ));
        assert!(matches!(
            provider.adapter.api_family,
            ApiFamily::AnthropicMessages
        ));
        assert_eq!(
            provider.compose_headers(Some("secret-token")),
            vec![
                ("x-api-key".into(), "secret-token".into()),
                ("anthropic-version".into(), "2023-06-01".into())
            ]
        );
    }

    #[test]
    fn provider_config_path_uses_workspace_config_dir() {
        let path = default_provider_config_path("/tmp/agentboard");
        assert_eq!(
            path.to_string_lossy(),
            "/tmp/agentboard/config/providers.toml"
        );
    }

    impl ProviderCatalog {
        fn load_from_str(source: &str) -> Result<Self, crate::contracts::ContractLoadError> {
            super::parse_provider_catalog(source)
        }
    }
}
