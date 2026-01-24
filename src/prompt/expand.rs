//! Placeholder expansion for prompts.
//!
//! Supports `{{variable}}` syntax for simple value substitution.
//! Complex Handlebars-like features (`{{#if}}`, `{{#each}}`) are rendered
//! as static text blocks for now; full templating can be added later.

use std::collections::HashMap;

/// A value that can be inserted into a placeholder.
#[derive(Debug, Clone, PartialEq)]
pub enum PlaceholderValue {
    /// A simple string value.
    String(String),

    /// A list of values (for potential `{{#each}}` expansion).
    List(Vec<HashMap<String, String>>),

    /// A boolean value (for potential `{{#if}}` expansion).
    Bool(bool),
}

impl From<&str> for PlaceholderValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for PlaceholderValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<bool> for PlaceholderValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<Vec<HashMap<String, String>>> for PlaceholderValue {
    fn from(list: Vec<HashMap<String, String>>) -> Self {
        Self::List(list)
    }
}

/// Context for placeholder expansion.
///
/// This is a map of variable names to their values.
#[derive(Debug, Clone, Default)]
pub struct PlaceholderContext {
    values: HashMap<String, PlaceholderValue>,
}

impl PlaceholderContext {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a value into the context.
    ///
    /// # Arguments
    ///
    /// * `key` - The variable name (without braces)
    /// * `value` - The value to insert
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Into<PlaceholderValue>,
    ) -> &mut Self {
        self.values.insert(key.into(), value.into());
        self
    }

    /// Gets a value from the context.
    pub fn get(&self, key: &str) -> Option<&PlaceholderValue> {
        self.values.get(key)
    }

    /// Returns the number of values in the context.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if the context is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Creates a context from an iterator of key-value pairs.
    #[allow(dead_code)]
    pub fn from_iter<K, V, I>(iter: I) -> Self
    where
        K: Into<String>,
        V: Into<PlaceholderValue>,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut ctx = Self::new();

        for (k, v) in iter {
            ctx.insert(k, v);
        }

        ctx
    }
}

/// Expands placeholders in a template string.
///
/// Supports the following syntax:
/// - `{{variable}}` — replaced with the value from context
/// - `{{#if variable}}...{{/if}}` — conditional blocks (rendered based on bool value)
/// - `{{#each variable}}...{{/each}}` — list iteration blocks
///
/// Unknown placeholders are left unchanged.
///
/// # Arguments
///
/// * `template` - The template string with placeholders
/// * `context` - The context containing values for placeholders
///
/// # Returns
///
/// The expanded string.
pub fn expand_placeholders(template: &str, context: &PlaceholderContext) -> String {
    let mut result = template.to_string();

    // First, handle {{#each ...}} blocks.
    result = expand_each_blocks(&result, context);

    // Then, handle {{#if ...}} blocks.
    result = expand_if_blocks(&result, context);

    // Finally, expand simple {{variable}} placeholders.
    result = expand_simple_placeholders(&result, context);

    result
}

/// Expands simple `{{variable}}` placeholders.
fn expand_simple_placeholders(template: &str, context: &PlaceholderContext) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'

            // Check for block tags (skip them here, they're handled separately).
            if chars.peek() == Some(&'#') || chars.peek() == Some(&'/') {
                result.push('{');
                result.push('{');
                continue;
            }

            // Collect the variable name.
            let mut var_name = String::new();
            let mut found_close = false;

            while let Some(c) = chars.next() {
                if c == '}' && chars.peek() == Some(&'}') {
                    chars.next(); // consume second '}'
                    found_close = true;
                    break;
                }

                var_name.push(c);
            }

            if found_close {
                let var_name = var_name.trim();

                // Skip block references like `@index`.
                if var_name.starts_with('@') {
                    result.push_str("{{");
                    result.push_str(var_name);
                    result.push_str("}}");
                    continue;
                }

                if let Some(value) = context.get(var_name) {
                    match value {
                        PlaceholderValue::String(s) => result.push_str(s),
                        PlaceholderValue::Bool(b) => result.push_str(&b.to_string()),
                        PlaceholderValue::List(_) => {
                            // List values should be used with {{#each}}, not directly.
                            result.push_str("[list]");
                        }
                    }
                } else {
                    // Unknown placeholder, leave it unchanged.
                    result.push_str("{{");
                    result.push_str(var_name);
                    result.push_str("}}");
                }
            } else {
                // Unclosed placeholder.
                result.push_str("{{");
                result.push_str(&var_name);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Expands `{{#if variable}}...{{/if}}` blocks.
fn expand_if_blocks(template: &str, context: &PlaceholderContext) -> String {
    let mut result = template.to_string();

    // Find and process all if blocks.
    loop {
        let Some(if_start) = result.find("{{#if ") else {
            break;
        };

        let Some(if_tag_end) = result[if_start..].find("}}") else {
            break;
        };
        let if_tag_end = if_start + if_tag_end;

        // Extract variable name.
        let var_name = result[if_start + 6..if_tag_end].trim();

        // Find matching {{/if}}.
        let content_start = if_tag_end + 2;
        let Some(if_end) = result[content_start..].find("{{/if}}") else {
            break;
        };
        let if_end = content_start + if_end;
        let block_end = if_end + 7;

        // Extract content.
        let content = &result[content_start..if_end];

        // Determine if we should include the content.
        let include = match context.get(var_name) {
            Some(PlaceholderValue::Bool(b)) => *b,
            Some(PlaceholderValue::String(s)) => !s.is_empty(),
            Some(PlaceholderValue::List(l)) => !l.is_empty(),
            None => false,
        };

        // Replace the block.
        let replacement = if include {
            content.to_string()
        } else {
            String::new()
        };

        result = format!(
            "{}{}{}",
            &result[..if_start],
            replacement,
            &result[block_end..]
        );
    }

    result
}

/// Expands `{{#each variable}}...{{/each}}` blocks.
fn expand_each_blocks(template: &str, context: &PlaceholderContext) -> String {
    let mut result = template.to_string();

    // Find and process all each blocks.
    loop {
        let Some(each_start) = result.find("{{#each ") else {
            break;
        };

        let Some(each_tag_end) = result[each_start..].find("}}") else {
            break;
        };
        let each_tag_end = each_start + each_tag_end;

        // Extract variable name.
        let var_name = result[each_start + 8..each_tag_end].trim();

        // Find matching {{/each}}.
        let content_start = each_tag_end + 2;
        let Some(each_end) = result[content_start..].find("{{/each}}") else {
            break;
        };
        let each_end = content_start + each_end;
        let block_end = each_end + 9;

        // Extract template content.
        let template_content = &result[content_start..each_end];

        // Get the list value.
        let items = match context.get(var_name) {
            Some(PlaceholderValue::List(list)) => list,
            _ => {
                // Not a list or not found, leave the block as-is.
                break;
            }
        };

        // Expand for each item.
        let mut expanded = String::new();

        for (index, item) in items.iter().enumerate() {
            let mut item_result = template_content.to_string();

            // Replace {{@index}} with the index.
            item_result = item_result.replace("{{@index}}", &index.to_string());

            // Replace item fields.
            for (key, value) in item {
                let placeholder = format!("{{{{{}}}}}", key);
                item_result = item_result.replace(&placeholder, value);
            }

            expanded.push_str(&item_result);
        }

        result = format!(
            "{}{}{}",
            &result[..each_start],
            expanded,
            &result[block_end..]
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_value_from_str() {
        let v: PlaceholderValue = "hello".into();
        assert_eq!(v, PlaceholderValue::String("hello".to_string()));
    }

    #[test]
    fn test_placeholder_value_from_string() {
        let v: PlaceholderValue = String::from("world").into();
        assert_eq!(v, PlaceholderValue::String("world".to_string()));
    }

    #[test]
    fn test_placeholder_value_from_bool() {
        let v: PlaceholderValue = true.into();
        assert_eq!(v, PlaceholderValue::Bool(true));
    }

    #[test]
    fn test_placeholder_context_insert() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("name", "Alice");
        ctx.insert("active", true);

        assert_eq!(
            ctx.get("name"),
            Some(&PlaceholderValue::String("Alice".to_string()))
        );
        assert_eq!(ctx.get("active"), Some(&PlaceholderValue::Bool(true)));
    }

    #[test]
    fn test_placeholder_context_from_iter() {
        let ctx = PlaceholderContext::from_iter([("a", "1"), ("b", "2")]);

        assert_eq!(ctx.len(), 2);
        assert_eq!(
            ctx.get("a"),
            Some(&PlaceholderValue::String("1".to_string()))
        );
    }

    #[test]
    fn test_expand_simple_placeholder() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("name", "Alice");

        let result = expand_placeholders("Hello, {{name}}!", &ctx);
        assert_eq!(result, "Hello, Alice!");
    }

    #[test]
    fn test_expand_multiple_placeholders() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("task_id", "T-001");
        ctx.insert("prd_id", "PRD-0001");

        let template = "Task {{task_id}} from {{prd_id}}";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "Task T-001 from PRD-0001");
    }

    #[test]
    fn test_expand_unknown_placeholder_unchanged() {
        let ctx = PlaceholderContext::new();

        let result = expand_placeholders("Hello, {{unknown}}!", &ctx);
        assert_eq!(result, "Hello, {{unknown}}!");
    }

    #[test]
    fn test_expand_if_true() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("show_details", true);

        let template = "Start{{#if show_details}} with details{{/if}} end";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "Start with details end");
    }

    #[test]
    fn test_expand_if_false() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("show_details", false);

        let template = "Start{{#if show_details}} with details{{/if}} end";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "Start end");
    }

    #[test]
    fn test_expand_if_string_nonempty() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("user_description", "A feature description");

        let template = "{{#if user_description}}Has description{{/if}}";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "Has description");
    }

    #[test]
    fn test_expand_if_string_empty() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("user_description", "");

        let template = "{{#if user_description}}Has description{{/if}}";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "");
    }

    #[test]
    fn test_expand_if_missing() {
        let ctx = PlaceholderContext::new();

        let template = "{{#if missing}}Hidden{{/if}}visible";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "visible");
    }

    #[test]
    fn test_expand_each() {
        let items = vec![
            [
                ("id".to_string(), "PRD-0001".to_string()),
                ("title".to_string(), "First".to_string()),
            ]
            .into_iter()
            .collect(),
            [
                ("id".to_string(), "PRD-0002".to_string()),
                ("title".to_string(), "Second".to_string()),
            ]
            .into_iter()
            .collect(),
        ];

        let mut ctx = PlaceholderContext::new();
        ctx.insert("prds", PlaceholderValue::List(items));

        let template = "{{#each prds}}- {{id}}: {{title}}\n{{/each}}";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "- PRD-0001: First\n- PRD-0002: Second\n");
    }

    #[test]
    fn test_expand_each_with_index() {
        let items = vec![
            [("name".to_string(), "Alice".to_string())]
                .into_iter()
                .collect(),
            [("name".to_string(), "Bob".to_string())]
                .into_iter()
                .collect(),
        ];

        let mut ctx = PlaceholderContext::new();
        ctx.insert("users", PlaceholderValue::List(items));

        let template = "{{#each users}}{{@index}}: {{name}}\n{{/each}}";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "0: Alice\n1: Bob\n");
    }

    #[test]
    fn test_expand_nested_placeholders_in_if() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("show_name", true);
        ctx.insert("name", "Alice");

        let template = "{{#if show_name}}Name: {{name}}{{/if}}";
        let result = expand_placeholders(template, &ctx);

        assert_eq!(result, "Name: Alice");
    }

    #[test]
    fn test_expand_real_world_prompt() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("task_id", "T-001");
        ctx.insert("prd_id", "PRD-0001");
        ctx.insert("task_title", "Implement feature X");
        ctx.insert("task_priority", "1");
        ctx.insert("task_notes", "Follow existing patterns");

        let template = r#"## Task Details

- **ID**: {{task_id}}
- **Title**: {{task_title}}
- **Priority**: {{task_priority}}
- **Notes**: {{task_notes}}"#;

        let result = expand_placeholders(template, &ctx);

        assert!(result.contains("- **ID**: T-001"));
        assert!(result.contains("- **Title**: Implement feature X"));
        assert!(result.contains("- **Priority**: 1"));
        assert!(result.contains("- **Notes**: Follow existing patterns"));
    }

    #[test]
    fn test_expand_empty_template() {
        let ctx = PlaceholderContext::new();
        let result = expand_placeholders("", &ctx);
        assert_eq!(result, "");
    }

    #[test]
    fn test_expand_no_placeholders() {
        let ctx = PlaceholderContext::new();
        let template = "Just regular text with no placeholders.";
        let result = expand_placeholders(template, &ctx);
        assert_eq!(result, template);
    }

    #[test]
    fn test_expand_whitespace_in_placeholder() {
        let mut ctx = PlaceholderContext::new();
        ctx.insert("name", "value");

        let result = expand_placeholders("{{ name }}", &ctx);
        assert_eq!(result, "value");
    }

    #[test]
    fn test_placeholder_context_len_and_empty() {
        let mut ctx = PlaceholderContext::new();
        assert!(ctx.is_empty());
        assert_eq!(ctx.len(), 0);

        ctx.insert("key", "value");
        assert!(!ctx.is_empty());
        assert_eq!(ctx.len(), 1);
    }
}
