//! CASO 6: Test Failure & Diff Triage (Self-Healing Loop).
//!
//! A failed run is a routing question: what broke, what fixes it, and can the
//! loop fix it alone or does it owe the operator an escalation? Bounded retry
//! loops read this answer to decide their next move.

use serde_json::{json, Value};

use super::client::{self, Question};


pub fn run(input: &super::TriageInput) -> Result<Value, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let jev = client::Jev::from_env();
    let state = json!({
        "command": input.command,
        "exitCode": input.exit_code,
        "outputSnippet": tail(&input.test_output, 1500),
        "diffSnippet": head(&input.git_diff.as_deref().unwrap_or(""), 1000),
    });

    let questions: Vec<(&str, Question)> = vec![
        ("root_cause", client::q_choice(
            "Identifique a causa raiz da falha de teste/execução",
            json!({
                "MISSING_DEPENDENCY": "Módulo não encontrado, dependência ausente em package.json ou imports inválidos",
                "ASSERTION_FAILURE": "Divergência entre valor esperado e retornado na asserção do teste",
                "SYNTAX_OR_TYPE_ERROR": "Erro de compilação TypeScript, erro de sintaxe ou tipo inválido",
                "ENVIRONMENT_CONFIG": "Variável de ambiente ausente, arquivo .env não configurado ou porta ocupada",
                "TIMEOUT_OR_FLAKY": "Timeout de execução assíncrona ou teste instável (flaky)"
            }),
        )),
        ("healing_strategy", client::q_choice(
            "Qual a estratégia recomendada de autocorreção?",
            json!({
                "INSTALL_DEPENDENCY": "Executar instalação de pacote npm/pip ausente",
                "FIX_IMPLEMENTATION": "Modificar e corrigir o código da implementação para satisfazer o contrato",
                "UPDATE_TEST_MOCK": "Ajustar o mock ou asserção do arquivo de teste",
                "CHECK_ENV": "Ajustar variáveis de ambiente ou credenciais locais",
                "RETRY": "Reexecutar o teste para verificar instabilidade passageira"
            }),
        )),
        ("requires_human", client::q_noul(
            "Esta falha requer intervenção manual do usuário ou pode ser corrigida pelo agente autonomamente?",
            json!({
                "true": "Problema de permissão, credencial externa ou arquitetura fundamental",
                "false": "Erro de código, asserção ou dependência corrigível por agente @dev"
            }),
        )),
    ];

    if let Some((answers, latency)) = jev.decide(state, questions) {
        return Ok(json!({
            "root_cause": client::choice(&answers, "root_cause").unwrap_or_else(|| "ASSERTION_FAILURE".into()),
            "healing_strategy": client::choice(&answers, "healing_strategy").unwrap_or_else(|| "FIX_IMPLEMENTATION".into()),
            "requires_human_intervention": client::noul(&answers, "requires_human").map(|p| p > 0.5).unwrap_or(false),
            "confidence": client::confidence(&answers, "root_cause").unwrap_or(0.9),
            "latency_ms": latency,
            "provider": "jev",
        }));
    }

    let out = input.test_output.to_lowercase();
    let (cause, strategy) = if ["cannot find module", "module not found", "err_module_not_found"]
        .iter().any(|k| out.contains(k))
    {
        ("MISSING_DEPENDENCY", "INSTALL_DEPENDENCY")
    } else if ["typeerror", "syntaxerror", "ts2304"].iter().any(|k| out.contains(k)) {
        ("SYNTAX_OR_TYPE_ERROR", "FIX_IMPLEMENTATION")
    } else {
        ("ASSERTION_FAILURE", "FIX_IMPLEMENTATION")
    };

    Ok(json!({
        "root_cause": cause,
        "healing_strategy": strategy,
        "requires_human_intervention": false,
        "confidence": 0.7,
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

fn tail(s: &str, max: usize) -> &str {
    match s.char_indices().rev().nth(max) {
        Some((i, _)) => &s[i..],
        None => s,
    }
}
