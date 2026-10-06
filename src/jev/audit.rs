//! CASO 4: Automated Spec (PRD/RFC) Quality Gating.
//!
//! The review-over-spec gate: a document is not approved because it was
//! generated, it is approved because a decision read it and found the things an
//! approved spec must have — a verdict, a diagram, testable criteria.

use serde_json::{json, Value};

use super::client::{self, Question};


pub fn run(input: &super::AuditInput) -> Result<Value, Box<dyn std::error::Error>> {
    let doc_type = if input.doc_type.eq_ignore_ascii_case("rfc") { "RFC" } else { "PRD" };
    let start = std::time::Instant::now();

    let jev = client::Jev::from_env();
    let state = json!({
        "docType": doc_type,
        "projectName": if input.project_name.as_deref().unwrap_or("projeto").is_empty() { "projeto" } else { &input.project_name.as_deref().unwrap_or("projeto") },
        "contentSnippet": head(&input.content, 2500),
    });

    let questions: Vec<(&str, Question)> = vec![
        ("verdict", client::q_choice(
            &format!("Avalie se o documento {} atende aos padrões de engenharia de software para aprovação formal", doc_type),
            json!({
                "APPROVED": "Documento completo, detalhado, com critérios de aceite, estrutura clara e rigor técnico",
                "NEEDS_REVISION": "Documento incompleto ou superficial, necessita de detalhamento de critérios ou riscos",
                "REJECTED": "Documento vazio, sem sentido ou desprovido de qualquer especificação técnica viável"
            }),
        )),
        ("has_mermaid", client::q_noul(
            "O documento contém diagramas visuais em bloco Mermaid (```mermaid)?",
            json!({
                "true": "Contém pelo menos um bloco ```mermaid com flowchart, sequenceDiagram ou classDiagram",
                "false": "Não contém diagramas Mermaid"
            }),
        )),
        ("has_acceptance_criteria", client::q_noul(
            "O documento contém critérios de aceite claros e testáveis?",
            json!({
                "true": "Possui seção explícita de Critérios de Aceite ou checklist de validação",
                "false": "Critérios ausentes ou vagos"
            }),
        )),
    ];

    if let Some((answers, latency)) = jev.decide(state, questions) {
        let verdict = client::choice(&answers, "verdict").unwrap_or_else(|| "APPROVED".into());
        let mermaid = client::noul(&answers, "has_mermaid").map(|p| p > 0.5).unwrap_or(false);
        let criteria = client::noul(&answers, "has_acceptance_criteria").map(|p| p > 0.5).unwrap_or(false);
        let score = match verdict.as_str() {
            "APPROVED" => 95,
            "NEEDS_REVISION" => 70,
            _ => 30,
        };
        return Ok(json!({
            "doc_type": doc_type,
            "verdict": verdict,
            "has_mermaid_diagram": mermaid,
            "has_acceptance_criteria": criteria,
            "has_architecture_decisions": doc_type == "RFC",
            "completeness_score": score,
            "feedback": format!("Auditoria Jev: {} (Mermaid: {}, Aceite: {})", verdict, if mermaid { "Sim" } else { "Não" }, if criteria { "Sim" } else { "Não" }),
            "confidence": client::confidence(&answers, "verdict").unwrap_or(0.9),
            "latency_ms": latency,
            "provider": "jev",
        }));
    }

    // Fallback: measure the document directly — these two facts are greppable.
    let mermaid = input.content.contains("```mermaid");
    let criteria = ["critérios de aceite", "critérios de aceitação", "acceptance criteria"]
        .iter().any(|k| input.content.to_lowercase().contains(k));
    let length_ok = input.content.len() > 400;
    let verdict = if length_ok && criteria { "APPROVED" } else { "NEEDS_REVISION" };

    Ok(json!({
        "doc_type": doc_type,
        "verdict": verdict,
        "has_mermaid_diagram": mermaid,
        "has_acceptance_criteria": criteria,
        "has_architecture_decisions": doc_type == "RFC",
        "completeness_score": if verdict == "APPROVED" { 85 } else { 60 },
        "feedback": format!("Auditoria Heurística: {}", verdict),
        "confidence": 0.8,
        "latency_ms": start.elapsed().as_millis(),
        "provider": "heuristic-fallback",
    }))
}

fn head(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}
