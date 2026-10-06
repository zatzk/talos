//! CASO 5: Task Atomicity & Decomposition Verification.
//!
//! The gate before dispatch: can this be one commit, one PR, one review — or
//! does it need to be split before it becomes somebody's problem?

use serde_json::{json, Value};

use super::client::{self, Question};


pub fn run(input: &super::AtomicityInput) -> Result<Value, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let jev = client::Jev::from_env();
    let state = json!({
        "taskTitle": input.title,
        "taskDescription": input.description,
        "moduleScope": input.module_scope.as_deref().unwrap_or("core"),
    });

    let questions: Vec<(&str, Question)> = vec![
        ("is_atomic", client::q_noul(
            "Esta tarefa é atômica (passível de ser codificada, testada e revisada em um único commit/PR)?",
            json!({
                "true": "Escopo delimitado e coeso com objetivo único e claro",
                "false": "Escopo amplo demais, abrangendo múltiplas responsabilidades desvinculadas ou épico completo"
            }),
        )),
        ("action", client::q_choice(
            "Qual ação o orquestrador deve tomar com esta tarefa?",
            json!({
                "READY_FOR_DEV": "Tarefa atômica e pronta para ser implementada pelo agente @dev",
                "NEEDS_DECOMPOSITION": "Tarefa muito ampla que deve ser dividida em sub-tarefas menores",
                "AMBIGUOUS": "Descrição ambígua ou incompleta que requer esclarecimento do usuário"
            }),
        )),
        ("has_test_criteria", client::q_noul(
            "Existe critério de teste claro para verificar o sucesso da tarefa?",
            json!({
                "true": "Comportamento esperado verificável via teste automatizado ou asserção objetiva",
                "false": "Não há critérios claros de sucesso"
            }),
        )),
    ];

    if let Some((answers, latency)) = jev.decide(state, questions) {
        let is_atomic = client::noul(&answers, "is_atomic").map(|p| p > 0.45).unwrap_or(true);
        let action = client::choice(&answers, "action")
            .unwrap_or_else(|| if is_atomic { "READY_FOR_DEV".into() } else { "NEEDS_DECOMPOSITION".into() });
        return Ok(json!({
            "is_atomic": is_atomic,
            "action": action,
            "has_clear_test_criteria": client::noul(&answers, "has_test_criteria").map(|p| p > 0.5).unwrap_or(true),
            "estimated_effort_hours": if is_atomic { 2 } else { 8 },
            "confidence": client::confidence(&answers, "action").unwrap_or(0.9),
            "latency_ms": latency,
            "provider": "jev",
        }));
    }

    let is_epic = ["plataforma", "reescrever", "arquitetura completa", "sistema de"]
        .iter().any(|k| input.title.to_lowercase().contains(k));
    Ok(json!({
        "is_atomic": !is_epic,
        "action": if is_epic { "NEEDS_DECOMPOSITION" } else { "READY_FOR_DEV" },
        "has_clear_test_criteria": true,
        "estimated_effort_hours": if is_epic { 12 } else { 3 },
        "confidence": 0.75,
        "latency_ms": start.elapsed().as_millis(),
        "provider": "heuristic-fallback",
    }))
}
