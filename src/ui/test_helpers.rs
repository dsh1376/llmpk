use crate::aa;
use crate::coding_agents;

pub fn aa_model(slug: &str, name: &str, provider: &str) -> aa::Model {
    aa::Model {
        slug: slug.to_string(),
        name: name.to_string(),
        creator_name: Some(provider.to_string()),
        creator_color: None,
        intelligence_index: Some(50.0),
        median_output_speed: Some(100.0),
        price_1m_input_tokens: Some(1.0),
        price_1m_output_tokens: Some(3.0),
        cache_hit_price: Some(0.1),
        cache_write_price: Some(0.5),
        context_window_tokens: Some(200_000),
        release_date: Some("2026-01-01".to_string()),
        is_open_weights: Some(false),
    }
}

pub fn agent_row(id: &str, agent: &str, provider: &str) -> coding_agents::AgentRow {
    coding_agents::AgentRow {
        id: id.to_string(),
        agent_name: agent.to_string(),
        provider: Some(provider.to_string()),
        host_name: Some(provider.to_string()),
        host_short_name: Some(provider.to_string()),
        model_name: Some(format!("{provider} Model")),
        host_model_slug: Some(format!("{}_model", provider.to_lowercase())),
        display_label: Some(format!("{agent} - {provider} Model")),
        release_date: Some("2026-01-01".to_string()),
        index_score: Some(0.6),
        display: coding_agents::AgentDisplay {
            agent: Some(agent.to_string()),
            model: Some(format!("{provider} Model")),
            creator: Some(coding_agents::AgentCreator {
                model: Some(provider.to_string()),
            }),
        },
        mean: coding_agents::AgentMean {
            reward: Some(0.55),
            cost_usd: Some(1.25),
            agent_wall_time_sec: Some(420.0),
            steps: Some(42.0),
            total_tokens: Some(1_500_000.0),
        },
        model_creators: Vec::new(),
    }
}

pub fn make_models(n: usize) -> Vec<aa::Model> {
    (0..n)
        .map(|i| aa_model(&format!("m{i}"), &format!("Model {i}"), "Provider"))
        .collect()
}
