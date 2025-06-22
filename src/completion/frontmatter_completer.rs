use std::collections::HashSet;
use tower_lsp::lsp_types::CompletionItem;
use serde_yaml::Value;
use crate::completion::{Context, Completer, Completable};

/// Completer for YAML front-matter keys and list values across the vault
pub struct FrontmatterCompleter<'a> {
    suggestions: Vec<&'a str>,
    prefix: String,
}

impl<'a> Completer<'a> for FrontmatterCompleter<'a> {
    fn construct(ctx: Context<'a>, line: usize, character: usize) -> Option<Self> {
        // Must have parsed front-matter range
        let mdfile = ctx.vault.md_files.get(ctx.path)?;
        let range = mdfile.frontmatter_range.as_ref()?;
        // Only inside the YAML front-matter block
        if line as u32 <= range.start.line || line as u32 > range.end.line {
            return None;
        }
        // Read the line text
        let line_chars = ctx.vault.select_line(ctx.path, line as isize)?;
        let line_str: String = line_chars.iter().collect();
        let trimmed = line_str.trim_start();
        let indent = line_str.len() - trimmed.len();
        let mut suggestions = Vec::new();
        let mut seen = HashSet::new();
        let prefix;
        if trimmed.starts_with("- ") {
            // Value completion under a list key
            let start = indent + 2;
            prefix = if character >= start {
                line_str[start..character].to_string()
            } else {
                String::new()
            };
            // Find parent key by scanning upward
            let mut key = None;
            for l in (0..line).rev() {
                let prev_chars = ctx.vault.select_line(ctx.path, l as isize)?;
                let prev_str: String = prev_chars.iter().collect();
                let prev_trim = prev_str.trim_start();
                if prev_trim.ends_with(':') {
                    key = Some(prev_trim.trim_end_matches(':').to_string());
                    break;
                }
                if prev_trim == "---" {
                    break;
                }
            }
            let key = key?;
            // Collect all values for that key across all files
            for f in ctx.vault.md_files.values() {
                if let Some(Value::Sequence(seq)) = f.frontmatter.as_ref().and_then(|fm| fm.get(&Value::String(key.clone()))) {
                    for v in seq.iter() {
                        if let Value::String(s) = v {
                            if seen.insert(s.as_str()) {
                                suggestions.push(s.as_str());
                            }
                        }
                    }
                }
            }
        } else {
            // Key completion
            prefix = trimmed.chars().take(character.saturating_sub(indent)).collect();
            for f in ctx.vault.md_files.values() {
                if let Some(fm) = &f.frontmatter {
                    for k in fm.keys() {
                        if let Value::String(s) = k {
                            if seen.insert(s.as_str()) {
                                suggestions.push(s.as_str());
                            }
                        }
                    }
                }
            }
        }
        suggestions.sort_unstable();
        Some(FrontmatterCompleter { suggestions, prefix })
    }

    fn completions(&self) -> Vec<impl Completable<'a, Self>> {
        // Return each suggestion as a &str
        self.suggestions.iter().copied().collect()
    }

    type FilterParams = String;
    fn completion_filter_text(&self, _p: Self::FilterParams) -> String {
        self.prefix.clone()
    }
}

impl<'a> Completable<'a, FrontmatterCompleter<'a>> for &'a str {
    fn completions(&self, _c: &FrontmatterCompleter<'a>) -> Option<CompletionItem> {
        Some(CompletionItem::new_simple(self.to_string(), String::new()))
    }
} 