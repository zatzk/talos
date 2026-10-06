//! The Jev decisions API client: `POST /api/alpha/decisions` on OpenRouter,
//! model `typesafe/jev-1.13`. A decision is a `state` (what the decision is
//! about) plus `questions` (what to decide, each with instructions and the
//! criteria for every possible answer). Answers come back typed: a `choice`
//! carries its winner, per-criterion probabilities and a confidence; a `noul`
//! carries the probability of `true`; a `score` carries a number.
//!
//! Every call is bounded by a timeout; a Jev that is slow, unreachable or
//! erroring returns `None` and the caller's deterministic fallback answers
//! instead. A decision engine that blocks orchestration is worse than no
//! decision engine.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Question {
    /// Pick one of `criteria` (keyed by answer).
    Choice {
        instructions: String,
        criteria: Value,
    },
    /// How true is the `true` criterion, 0..1.
    Noul {
        instructions: String,
        criteria: Value,
    },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Answer {
    Choice {
        choice: String,
        #[serde(default)]
        confidence: Option<f64>,
    },
    Noul {
        noul: f64,
    },
    Score {
        score: f64,
    },
}

#[derive(Deserialize)]
struct DecisionResponse {
    #[serde(default)]
    answers: std::collections::BTreeMap<String, Answer>,
}

pub struct Jev {
    endpoint: String,
    api_key: String,
    model: String,
    timeout: std::time::Duration,
}

impl Jev {
    pub fn from_env() -> Self {
        let endpoint = std::env::var("DECISIONS_ENDPOINT")
            .unwrap_or_else(|_| "https://openrouter.ai/api/alpha/decisions".into());
        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
        let model =
            std::env::var("JEV_MODEL").unwrap_or_else(|_| "typesafe/jev-1.13".into());
        let timeout = std::env::var("JEV_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6000);
        Jev {
            endpoint,
            api_key,
            model,
            timeout: std::time::Duration::from_millis(timeout),
        }
    }

    /// Ask. Returns the answers and how long the round trip took, or `None`
    /// when the engine could not be reached in time — which is the caller's
    /// cue to fall back, never to fail.
    pub fn decide(
        &self,
        state: Value,
        questions: Vec<(&str, Question)>,
    ) -> Option<(std::collections::BTreeMap<String, Answer>, u128)> {
        if self.api_key.is_empty() {
            return None; // no key configured; nothing to ask
        }
        let start = Instant::now();
        let questions: serde_json::Map<String, Value> = questions
            .into_iter()
            .map(|(name, q)| (name.to_string(), serde_json::to_value(&q).unwrap()))
            .collect();

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .ok()?;
        let res = client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({ "model": self.model, "state": state, "questions": questions }))
            .send()
            .ok()?;
        if !res.status().is_success() {
            eprintln!("[talos] decisions endpoint returned {}", res.status());
            return None;
        }
        let parsed: DecisionResponse = res.json().ok()?;
        Some((parsed.answers, start.elapsed().as_millis()))
    }
}

/// Convenience readers over the answer map: a choice's winner, a choice's
/// confidence, a noul's probability — each with a default for an answer that
/// never came back, so a missing question degrades one field instead of the
/// whole decision.
pub fn choice(answers: &std::collections::BTreeMap<String, Answer>, key: &str) -> Option<String> {
    match answers.get(key) {
        Some(Answer::Choice { choice, .. }) => Some(choice.clone()),
        _ => None,
    }
}

pub fn confidence(answers: &std::collections::BTreeMap<String, Answer>, key: &str) -> Option<f64> {
    match answers.get(key) {
        Some(Answer::Choice { confidence, .. }) => *confidence,
        _ => None,
    }
}

pub fn noul(answers: &std::collections::BTreeMap<String, Answer>, key: &str) -> Option<f64> {
    match answers.get(key) {
        Some(Answer::Noul { noul }) => Some(*noul),
        _ => None,
    }
}

pub fn q_choice(instructions: &str, criteria: Value) -> Question {
    Question::Choice {
        instructions: instructions.into(),
        criteria,
    }
}

pub fn q_noul(instructions: &str, criteria: Value) -> Question {
    Question::Noul {
        instructions: instructions.into(),
        criteria,
    }
}
