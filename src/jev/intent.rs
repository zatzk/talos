//! CASO 2: Natural Language Intent & Agent Routing in Chat.
//!
//! Where a message goes and what it is for: the target persona, the intent,
//! whether it added spec-worthy context, and what the cockpit should do next.

use serde_json::{json, Value};

use super::client::{self, Question};


pub fn run(input: &super::IntentInput) -> Result<Value, Box<dyn std::error::Error>> {
    let jev = client::Jev::from_env();
    let state = json!({
        "userMessage": input.user_message,
        "conversationHistorySnippet": tail(&input.history.as_deref().unwrap_or(""), 800),
        "currentBranch": input.current_branch.as_deref().unwrap_or("main"),
    });

    let questions: Vec<(&str, Question)> = vec![
        ("target_agent", client::q_choice(
            "Selecione o agente especialista mais adequado para responder",
            json!({
                "spec-master": "Definição de requisitos de negócio, PRD, escopo funcional, user stories e personas",
                "architect": "Desenho de arquitetura de software, contratos de API, RFC e diagramas Mermaid",
                "dev": "Implementação técnica de código, refatoração de métodos, classes e comandos",
                "qa": "Auditoria de testes, qualidade de código, pipelines de validação e code review",
                "fast-router": "Perguntas gerais, saudações ou dúvidas simples do cockpit"
            }),
        )),
        ("intent", client::q_choice(
            "Qual a intenção principal da mensagem?",
            json!({
                "discovery": "Descoberta de requisitos, planejamento inicial de ideias ou escopo",
                "architecture": "Definição de arquitetura técnica, contratos, topologia ou dados",
                "implementation": "Solicitação direta de código, refatoração ou criação de arquivo",
                "qa_review": "Análise de testes, cobertura, revisão de código ou auditoria",
                "clarification": "Dúvida sobre o funcionamento do projeto ou cockpit",
                "general": "Saudação trivial ou encerramento de assunto"
            }),
        )),
        ("has_new_spec_context", client::q_noul(
            "A mensagem adiciona requisitos materiais, regras de negócio ou decisões de arquitetura que justificam re-gerar ou atualizar o PRD/RFC?",
            json!({
                "true": "Contém novos requisitos, critérios, entidades ou mudanças substantivas de escopo",
                "false": "Mensagem trivial, pergunta informativa ou sem novos requisitos"
            }),
        )),
        ("suggested_action", client::q_choice(
            "Qual ação imediata deve ser tomada?",
            json!({
                "respond_chat": "Responder diretamente no chat tirando dúvidas ou elaborando a ideia",
                "recommend_spec": "Sugerir que o usuário clique no botão para Gerar Especificação (PRD & RFC)",
                "execute_task": "Criar ou disparar uma tarefa de execução técnica"
            }),
        )),
        ("execution_backend", client::q_choice(
            "Qual o backend de execução ideal para atender a requisição?",
            json!({
                "api": "Resposta rápida streaming via API direta (OpenRouter/9Router) para chat, refinamento e perguntas conceituais",
                "cli": "Execução via CLI local (claude/codex/agy) com acesso ao filesystem e ferramentas para execução técnica e testes"
            }),
        )),
        ("model_tier", client::q_choice(
            "Qual o tier de modelo requerido para esta mensagem?",
            json!({
                "flagship": "Raciocínio complexo, síntese de specs, arquitetura profunda e escrita de código crítico",
                "fast": "Roteamento rápido, respostas diretas a dúvidas simples, triagem e edições pequenas"
            }),
        )),
    ];

    if let Some((answers, latency)) = jev.decide(state, questions) {
        return Ok(json!({
            "intent": client::choice(&answers, "intent").unwrap_or_else(|| "discovery".into()),
            "target_agent": client::choice(&answers, "target_agent").unwrap_or_else(|| "fast-router".into()),
            "execution_backend": client::choice(&answers, "execution_backend").unwrap_or_else(|| "api".into()),
            "model_tier": client::choice(&answers, "model_tier").unwrap_or_else(|| "flagship".into()),
            "has_new_spec_context": client::noul(&answers, "has_new_spec_context").map(|p| p > 0.45).unwrap_or(false),
            "suggested_action": client::choice(&answers, "suggested_action").unwrap_or_else(|| "respond_chat".into()),
            "confidence": client::confidence(&answers, "target_agent").unwrap_or(0.9),
            "latency_ms": latency,
            "provider": "jev",
        }));
    }

    Ok(fallback(input))
}

fn fallback(input: &super::IntentInput) -> Value {
    let lower = input.user_message.to_lowercase();
    let (agent, intent) = if ["@spec-master", "prd", "requisitos"].iter().any(|k| lower.contains(k)) {
        ("spec-master", "discovery")
    } else if ["@architect", "rfc", "arquitetura", "mermaid"].iter().any(|k| lower.contains(k)) {
        ("architect", "architecture")
    } else if ["@dev", "código", "implementar"].iter().any(|k| lower.contains(k)) {
        ("dev", "implementation")
    } else if ["@qa", "teste", "review"].iter().any(|k| lower.contains(k)) {
        ("qa", "qa_review")
    } else {
        ("fast-router", "discovery")
    };

    let (execution_backend, model_tier) = match intent {
        "implementation" => ("cli", "flagship"),
        "architecture" | "discovery" => ("api", "flagship"),
        _ => ("api", "fast"),
    };

    json!({
        "intent": intent,
        "target_agent": agent,
        "execution_backend": execution_backend,
        "model_tier": model_tier,
        "has_new_spec_context": input.user_message.len() > 30,
        "suggested_action": "respond_chat",
        "confidence": 0.75,
        "latency_ms": 0,
        "provider": "heuristic-fallback",
    })
}

fn tail(s: &str, max: usize) -> &str {
    match s.char_indices().rev().nth(max) {
        Some((i, _)) => &s[i..],
        None => s,
    }
}
