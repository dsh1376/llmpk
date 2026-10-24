use crate::aa;
use crate::aihot;
use crate::aitracker;
use crate::coding_agents;
use crate::frontiercode;
use crate::slopalytics;

pub struct FilterCache {
    pub(super) indices: Vec<usize>,
    pub(super) query: String,
}

pub fn filter_tokens(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// Check if a lowercase token appears in any of the provided (already-lowered) field values.
fn token_in_lowered_fields(token: &str, lowered_fields: &[String]) -> bool {
    lowered_fields.iter().any(|f| f.contains(token))
}

pub fn aa_matches_filter(model: &aa::Model, tokens: &[String]) -> bool {
    let weights = match model.is_open_weights {
        Some(true) => "open open-weights",
        Some(false) => "closed proprietary",
        None => "",
    };
    let lowered: Vec<String> = [
        model.slug.as_str(),
        model.name.as_str(),
        model.provider(),
        model.release_date.as_deref().unwrap_or(""),
        weights,
    ]
    .iter()
    .map(|f| f.to_lowercase())
    .collect();
    tokens
        .iter()
        .all(|token| token_in_lowered_fields(token, &lowered))
}

pub fn agent_matches_filter(row: &coding_agents::AgentRow, tokens: &[String]) -> bool {
    let lowered: Vec<String> = [
        row.id.as_str(),
        row.agent_name.as_str(),
        row.agent(),
        row.model(),
        row.provider(),
        row.display_label.as_deref().unwrap_or(""),
        row.model_name.as_deref().unwrap_or(""),
        row.host_model_slug.as_deref().unwrap_or(""),
        row.release_date.as_deref().unwrap_or(""),
    ]
    .iter()
    .map(|f| f.to_lowercase())
    .collect();
    tokens
        .iter()
        .all(|token| token_in_lowered_fields(token, &lowered))
}

pub(super) fn count_matching_aa(models: &[aa::Model], query: &str) -> usize {
    let tokens = filter_tokens(query);
    if tokens.is_empty() {
        return models.len();
    }
    models
        .iter()
        .filter(|model| aa_matches_filter(model, &tokens))
        .count()
}

pub(super) fn count_matching_agents(rows: &[coding_agents::AgentRow], query: &str) -> usize {
    let tokens = filter_tokens(query);
    if tokens.is_empty() {
        return rows.len();
    }
    rows.iter()
        .filter(|row| agent_matches_filter(row, &tokens))
        .count()
}

pub fn slop_matches_filter(row: &slopalytics::Row, tokens: &[String]) -> bool {
    let lowered: Vec<String> = [
        row.id.as_str(),
        row.name.as_str(),
        row.variant.as_deref().unwrap_or(""),
        row.lab(),
        row.effort().unwrap_or(""),
        row.released.as_deref().unwrap_or(""),
    ]
    .iter()
    .map(|f| f.to_lowercase())
    .collect();
    tokens
        .iter()
        .all(|token| token_in_lowered_fields(token, &lowered))
}

pub(super) fn count_matching_slop(rows: &[slopalytics::Row], query: &str) -> usize {
    let tokens = filter_tokens(query);
    if tokens.is_empty() {
        return rows.len();
    }
    rows.iter()
        .filter(|row| slop_matches_filter(row, &tokens))
        .count()
}

pub fn frontier_matches_filter(row: &frontiercode::Row, tokens: &[String]) -> bool {
    let lowered: Vec<String> = [
        row.model.as_str(),
        row.lab(),
        row.effort.as_deref().unwrap_or(""),
        row.harness.as_deref().unwrap_or(""),
    ]
    .iter()
    .map(|f| f.to_lowercase())
    .collect();
    tokens
        .iter()
        .all(|token| token_in_lowered_fields(token, &lowered))
}

pub(super) fn count_matching_frontier(rows: &[frontiercode::Row], query: &str) -> usize {
    let tokens = filter_tokens(query);
    if tokens.is_empty() {
        return rows.len();
    }
    rows.iter()
        .filter(|row| frontier_matches_filter(row, &tokens))
        .count()
}

pub fn tracker_matches_filter(page: &aitracker::ModelPage, tokens: &[String]) -> bool {
    let lowered: Vec<String> = [
        page.slug.as_str(),
        page.model.as_str(),
        page.maker().unwrap_or(""),
        page.stage().unwrap_or(""),
        page.last_seen_at.as_deref().unwrap_or(""),
    ]
    .iter()
    .map(|f| f.to_lowercase())
    .collect();
    tokens
        .iter()
        .all(|token| token_in_lowered_fields(token, &lowered))
}

pub(super) fn count_matching_tracker(pages: &[aitracker::ModelPage], query: &str) -> usize {
    let tokens = filter_tokens(query);
    if tokens.is_empty() {
        return pages.len();
    }
    pages
        .iter()
        .filter(|page| tracker_matches_filter(page, &tokens))
        .count()
}

pub fn hot_matches_filter(topic: &aihot::HotTopic, tokens: &[String]) -> bool {
    let mut fields = vec![
        topic.title.as_str(),
        topic.source.name.as_deref().unwrap_or(""),
    ];
    fields.extend(topic.source_names.iter().map(String::as_str));
    lowered_fields_match(&fields, tokens)
}

pub fn daily_matches_filter(item: &aihot::DailyItem, tokens: &[String]) -> bool {
    let fields = [
        item.section.as_str(),
        item.title.as_str(),
        item.summary.as_deref().unwrap_or(""),
        item.source.as_deref().unwrap_or(""),
    ];
    lowered_fields_match(&fields, tokens)
}

pub fn reset_matches_filter(event: &aihot::ResetEvent, tokens: &[String]) -> bool {
    let fields = [
        event.kind_label(),
        event.status_label(),
        event.scope().unwrap_or(""),
        event.latest_text().unwrap_or(""),
        event.created_at.as_deref().unwrap_or(""),
    ];
    lowered_fields_match(&fields, tokens)
}

fn lowered_fields_match(fields: &[&str], tokens: &[String]) -> bool {
    let lowered: Vec<String> = fields.iter().map(|f| f.to_lowercase()).collect();
    tokens
        .iter()
        .all(|token| token_in_lowered_fields(token, &lowered))
}

pub(super) fn count_matching<T>(
    rows: &[T],
    query: &str,
    matches: impl Fn(&T, &[String]) -> bool,
) -> usize {
    let tokens = filter_tokens(query);
    if tokens.is_empty() {
        return rows.len();
    }
    rows.iter().filter(|row| matches(row, &tokens)).count()
}

pub(super) fn matching_indices<T>(
    rows: &[T],
    query: &str,
    matches: impl Fn(&T, &[String]) -> bool,
) -> Vec<usize> {
    let tokens = filter_tokens(query);
    rows.iter()
        .enumerate()
        .filter(|(_, row)| tokens.is_empty() || matches(row, &tokens))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_tokens_splits_and_lowercases() {
        assert_eq!(filter_tokens("Hello World"), vec!["hello", "world"]);
    }

    #[test]
    fn filter_tokens_empty_query() {
        assert!(filter_tokens("").is_empty());
    }

    #[test]
    fn filter_tokens_whitespace_only() {
        assert!(filter_tokens("   ").is_empty());
    }

    #[test]
    fn aa_matches_filter_by_id() {
        let model =
            crate::ui::test_helpers::aa_model("claude-sonnet-4", "Claude Sonnet 4", "Anthropic");
        assert!(aa_matches_filter(&model, &["claude".into()]));
        assert!(!aa_matches_filter(&model, &["gpt".into()]));
    }

    #[test]
    fn aa_matches_filter_by_provider() {
        let model = crate::ui::test_helpers::aa_model("gpt-4o", "GPT-4o", "OpenAI");
        assert!(aa_matches_filter(&model, &["openai".into()]));
    }

    #[test]
    fn aa_matches_filter_multi_token_and() {
        let model =
            crate::ui::test_helpers::aa_model("claude-sonnet-4", "Claude Sonnet 4", "Anthropic");
        assert!(aa_matches_filter(
            &model,
            &["claude".into(), "sonnet".into()]
        ));
        assert!(!aa_matches_filter(&model, &["claude".into(), "gpt".into()]));
    }

    #[test]
    fn aa_matches_filter_open_weights() {
        let mut model = crate::ui::test_helpers::aa_model("llama-3", "Llama 3", "Meta");
        model.is_open_weights = Some(true);
        assert!(aa_matches_filter(&model, &["open".into()]));
        assert!(aa_matches_filter(&model, &["open-weights".into()]));
        assert!(!aa_matches_filter(&model, &["proprietary".into()]));
    }

    #[test]
    fn aa_matches_filter_proprietary() {
        let mut model = crate::ui::test_helpers::aa_model("gpt-4o", "GPT-4o", "OpenAI");
        model.is_open_weights = Some(false);
        assert!(aa_matches_filter(&model, &["proprietary".into()]));
        assert!(aa_matches_filter(&model, &["closed".into()]));
        assert!(!aa_matches_filter(&model, &["open-weights".into()]));
    }

    #[test]
    fn count_matching_aa_returns_total_on_empty_query() {
        let models = vec![
            crate::ui::test_helpers::aa_model("a", "A", "P"),
            crate::ui::test_helpers::aa_model("b", "B", "P"),
        ];
        assert_eq!(count_matching_aa(&models, ""), 2);
    }

    #[test]
    fn count_matching_aa_filters() {
        let models = vec![
            crate::ui::test_helpers::aa_model("claude-sonnet", "Claude", "Anthropic"),
            crate::ui::test_helpers::aa_model("gpt-4o", "GPT", "OpenAI"),
        ];
        assert_eq!(count_matching_aa(&models, "claude"), 1);
        assert_eq!(count_matching_aa(&models, "gpt"), 1);
        assert_eq!(count_matching_aa(&models, "xyz"), 0);
    }

    #[test]
    fn agent_matches_filter_by_id() {
        let row = crate::ui::test_helpers::agent_row("claude-code", "Claude Code", "Anthropic");
        assert!(agent_matches_filter(&row, &["claude".into()]));
        assert!(!agent_matches_filter(&row, &["copilot".into()]));
    }

    #[test]
    fn agent_matches_filter_by_provider() {
        let row = crate::ui::test_helpers::agent_row("copilot", "Copilot", "GitHub");
        assert!(agent_matches_filter(&row, &["github".into()]));
    }
}
