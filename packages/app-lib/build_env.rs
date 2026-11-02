/// Resolve the embedded key without expanding `$` in dotenv values.
pub fn curseforge_api_key(
    process_value: Option<String>,
    dotenv_contents: Option<&str>,
) -> Option<String> {
    process_value
        .filter(|value| !value.trim().is_empty())
        .or_else(|| dotenv_literal(dotenv_contents?, "CURSEFORGE_API_KEY"))
}

pub fn dotenv_literal(contents: &str, name: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let line = line.trim_start();
        let line = line.strip_prefix("export ").unwrap_or(line);
        let (candidate, value) = line.split_once('=')?;
        if candidate.trim() != name {
            return None;
        }
        let value = value.trim();
        let value = if value.len() >= 2
            && ((value.starts_with('\'') && value.ends_with('\''))
                || (value.starts_with('"') && value.ends_with('"')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        (!value.trim().is_empty()).then(|| value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ci_secret_falls_back_to_literal_dotenv() {
        for value in [None, Some(String::new()), Some(" \t".to_string())] {
            assert_eq!(
                curseforge_api_key(
                    value,
                    Some("CURSEFORGE_API_KEY='$2a$10$example'")
                ),
                Some("$2a$10$example".to_string())
            );
        }
    }

    #[test]
    fn explicit_key_takes_precedence() {
        assert_eq!(
            curseforge_api_key(
                Some("$2a$10$process".into()),
                Some("CURSEFORGE_API_KEY=dotenv")
            ),
            Some("$2a$10$process".into())
        );
    }

    #[test]
    fn missing_and_blank_keys_are_absent() {
        assert_eq!(curseforge_api_key(None, None), None);
        assert_eq!(
            curseforge_api_key(None, Some("CURSEFORGE_API_KEY='  '")),
            None
        );
    }

    #[test]
    fn dotenv_supports_quotes_exports_and_indentation() {
        assert_eq!(
            dotenv_literal(
                "  export CURSEFORGE_API_KEY=\"$2a$10$example\"",
                "CURSEFORGE_API_KEY"
            ),
            Some("$2a$10$example".into())
        );
    }
}
