//! CASO 1: Dynamic Cognitive Task Sizing & Pipeline Routing.
//!
//! One decision answers five questions at once — how big, which pipeline,
//! which tier, which agent, does a human need to look — and the answer routes
//! the task before any expensive model is spent on it.

use serde_json::{json, Value};

use super::client::{self, Question};


pub fn run(input: &super::SizingInput) -> Result<Value, Box<dyn std::error::Error>> {
    let jev = client::Jev::from_env();
    let state = json!({
        "taskTitle": input.title,
        "taskDescription": input.description,
        "contextSnippet": truncate(&input.context.as_deref().unwrap_or(""), 1000),
    });

    let questions: Vec<(&str, Question)> = vec![
        ("size", client::q_choice(
            "Determine o tamanho e complexidade da tarefa de engenharia de software",
            json!({
                "S": "Correção pontual, ajuste cosmético ou alteração isolada de 1-2 arquivos com zero risco",
                "M": "Funcionalidade moderada em componente único ou serviço delimitado com testes",
                "L": "Refatoração ampla, contratos de API entre serviços ou mudança com impacto estrutural",
                "EPIC": "Iniciativa de plataforma abrangente, múltiplos épicos, subsistemas ou reescrita"
            }),
        )),
        ("pipeline", client::q_choice(
            "Selecione o pipeline de orquestração adequado para governança",
            json!({
                "direct-fix": "Execução direta pelo @dev com testes e validação de diff rápida",
                "rfc-first": "Requer aprovação prévia de RFC técnico com diagramas antes de codificar",
                "full-spec": "Requer PRD de requisitos + RFC de arquitetura + aprovação humana formal"
            }),
        )),
        ("tier", client::q_choice(
            "Qual tier de modelo cognitivo é estritamente necessário?",
            json!({
                "economic": "Modelos de alta eficiência ($0/token ou flash) resolvem o problema com precisão",
                "frontier": "Exige raciocínio ultra-profundo (Claude 4.6 Thinking / Sonnet / DeepSeek R1)"
            }),
        )),
        ("target_agent", client::q_choice(
            "Qual persona do spec-harness-kit deve liderar a execução?",
            json!({
                "dev": "Especialista em implementação de código limpo e testes unitários/integrados",
                "qa": "Especialista em auditoria forense, testes automatizados e code review",
                "architect": "Arquiteto de sistemas focado em contratos, protocolos e design de soluções",
                "spec-master": "Líder de requisitos e orquestração de alto nível"
            }),
        )),
        ("requires_human_review", client::q_noul(
            "A alteração envolve risco ou complexidade que exige revisão humana obrigatória?",
            json!({
                "true": "Alto risco, impacto arquitetural, segurança ou grande raio de alcance",
                "false": "Baixo risco, alteração cosmética ou correção trivial estritamente testada"
            }),
        )),
    ];

    if let Some((answers, latency)) = jev.decide(state, questions) {
        let size = client::choice(&answers, "size").unwrap_or_else(|| "M".into());
        let pipeline = client::choice(&answers, "pipeline").unwrap_or_else(|| "direct-fix".into());
        let tier = client::choice(&answers, "tier").unwrap_or_else(|| "economic".into());
        let target_agent = client::choice(&answers, "target_agent").unwrap_or_else(|| "dev".into());
        let requires_human_review =
            client::noul(&answers, "requires_human_review").map(|p| p > 0.4).unwrap_or(true);
        let confidence = client::confidence(&answers, "size").unwrap_or(0.95);

        return Ok(json!({
            "size": size,
            "pipeline": pipeline,
            "tier": tier,
            "target_agent": target_agent,
            "requires_human_review": requires_human_review,
            "max_budget_tokens": if tier == "frontier" { 64000 } else { 32000 },
            "rationale": format!("[Jev System-1] Classificado como {} ({}) atribuído a @{}. Tier: {}. Confiança: {}%.", size, pipeline, target_agent, tier, (confidence * 100.0).round()),
            "reasoning": if tier == "frontier" { "Complexidade profunda identificada pelo Jev" } else { "Resolução ótima em tier econômico" },
            "confidence": confidence,
            "latency_ms": latency,
            "provider": "jev",
        }));
    }

    Ok(fallback(input))
}

/// The deterministic classifier, ported from `sizing.ts`: keyword routing for
/// the shape of the work, frontier tier only for the deep-reasoning vocabulary.
fn fallback(input: &super::SizingInput) -> Value {
    let text = format!("{} {}", input.title, input.description).to_lowercase();

    let (size, pipeline, agent) = if ["prd", "platform", "epic", "spec-master"].iter().any(|k| text.contains(k)) {
        ("EPIC", "full-spec", "spec-master")
    } else if ["rfc", "architect", "topology", "protocol"].iter().any(|k| text.contains(k)) {
        ("L", "rfc-first", "architect")
    } else if ["review", "qa", "audit", "test"].iter().any(|k| text.contains(k)) {
        ("M", "direct-fix", "qa")
    } else if ["fix", "bug", "typo", "tweak", "simple"].iter().any(|k| text.contains(k)) {
        ("S", "direct-fix", "dev")
    } else {
        ("M", "direct-fix", "dev")
    };
    let frontier = ["complex refactor", "distributed topology", "deep forensic", "thinking model"]
        .iter().any(|k| text.contains(k));

    json!({
        "size": size,
        "pipeline": pipeline,
        "tier": if frontier { "frontier" } else { "economic" },
        "target_agent": agent,
        "requires_human_review": true,
        "max_budget_tokens": if frontier { 64000 } else { 32000 },
        "rationale": format!("[Fallback Heurístico] Classificado como {} com pipeline {} atribuído a @{}.", size, pipeline, agent),
        "reasoning": if frontier { "Demanda raciocínio Frontier" } else { "Executável em tier econômico" },
        "confidence": 0.8,
        "latency_ms": 0,
        "provider": "heuristic-fallback",
    })
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(title: &str, description: &str) -> Value {
        fallback(&crate::jev::SizingInput {
            title: title.into(),
            description: description.into(),
            context: None,
        })
    }

    #[test]
    fn routes_by_work_shape() {
        assert_eq!(size("fix login typo", "")["size"], "S");
        assert_eq!(size("fix login typo", "")["target_agent"], "dev");

        assert_eq!(size("draft the platform PRD epic", "")["size"], "EPIC");
        assert_eq!(size("draft the platform PRD epic", "")["pipeline"], "full-spec");

        assert_eq!(size("review the auth tests", "")["target_agent"], "qa");

        assert_eq!(size("unrelated thing", "")["size"], "M");
        assert_eq!(size("unrelated thing", "")["target_agent"], "dev");
    }

    #[test]
    fn frontier_tier_only_for_deep_reasoning_vocabulary() {
        assert_eq!(size("complex refactor", "")["tier"], "frontier");
        assert_eq!(size("simple bug", "")["tier"], "economic");
    }

    #[test]
    fn fallback_always_claims_its_provider() {
        assert_eq!(size("x", "")["provider"], "heuristic-fallback");
        assert_eq!(size("x", "")["requires_human_review"], true);
    }
}
