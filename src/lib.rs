#[allow(warnings)]
mod bindings;

use bindings::exports::rayslash::module::provider::Guest;
use bindings::rayslash::module::types::{
    Action, Icon, ModuleError, QueryContext, QueryResponse, ResultItem,
};
use serde::Deserialize;

struct Component;

#[derive(Default, Deserialize)]
struct Settings {
    #[serde(default)]
    aliases: Vec<Alias>,
}

#[derive(Deserialize)]
struct Alias {
    name: String,
    query: String,
    target: String,
    #[serde(default)]
    kind: Option<AliasKind>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AliasKind {
    Url,
    File,
    Folder,
    Command,
}

impl Guest for Component {
    fn query(context: QueryContext) -> Result<QueryResponse, ModuleError> {
        let settings: Settings = serde_json::from_str(&context.settings_json).unwrap_or_default();
        let query = context.query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Ok(QueryResponse {
                results: Vec::new(),
                exclusive: false,
            });
        }
        let results = settings
            .aliases
            .into_iter()
            .filter(|alias| matches_alias(alias, &query))
            .take(context.max_results as usize)
            .map(result)
            .collect();
        Ok(QueryResponse {
            results,
            exclusive: false,
        })
    }
}

fn matches_alias(alias: &Alias, query: &str) -> bool {
    let name = alias.name.to_ascii_lowercase();
    let trigger = alias.query.to_ascii_lowercase();
    name.contains(query) || trigger.contains(query) || query.contains(&trigger)
}

fn result(alias: Alias) -> ResultItem {
    let kind = alias.kind.unwrap_or_else(|| infer_kind(&alias.target));
    let (label, icon, action) = match kind {
        AliasKind::Url => ("URL", "↗", Action::OpenUrl(alias.target.clone())),
        AliasKind::File => ("File", "F", Action::OpenPath(alias.target.clone())),
        AliasKind::Folder => ("Folder", "D", Action::OpenPath(alias.target.clone())),
        AliasKind::Command => match shell_words::split(&alias.target) {
            Ok(arguments) if !arguments.is_empty() => {
                ("Command", ">", Action::RunApprovedCommand(arguments))
            }
            _ => (
                "Invalid command",
                "!",
                Action::ShowMessage("Alias command is empty or has invalid quoting.".into()),
            ),
        },
    };
    ResultItem {
        id: format!(
            "alias:{}:{}",
            alias.query.to_ascii_lowercase(),
            alias.name.to_ascii_lowercase()
        ),
        title: alias.name,
        subtitle: format!("{label}: {}", alias.target),
        icon: Icon::Text(icon.into()),
        score: None,
        action,
    }
}

fn infer_kind(target: &str) -> AliasKind {
    if target.starts_with("https://") || target.starts_with("http://") {
        AliasKind::Url
    } else if target.starts_with('/') || target.starts_with("~/") {
        AliasKind::File
    } else {
        AliasKind::Command
    }
}

bindings::export!(Component with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn infers_urls_and_paths() {
        assert!(matches!(infer_kind("https://example.com"), AliasKind::Url));
        assert!(matches!(infer_kind("~/notes"), AliasKind::File));
    }
    #[test]
    fn matches_names_and_triggers() {
        let alias = Alias {
            name: "Documentation".into(),
            query: "docs".into(),
            target: "https://example.com".into(),
            kind: None,
        };
        assert!(matches_alias(&alias, "doc"));
    }
}
