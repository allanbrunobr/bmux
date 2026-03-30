/// JSON extraction from agent output text with a multi-stage fallback chain:
/// 1. Fenced code block (```json ... ``` or ``` ... ```)
/// 2. First brace-matched `{...}` block
/// 3. Raw text as JSON
/// 4. Caller-supplied default (None returned)

use serde_json::Value;

use crate::adversarial::types::{Criterion, CriterionScore, EvaluationResult, PlannedSprint, SprintContract, SprintPlan};

// ── Public extraction API ─────────────────────────────────────────────────────

/// Extract a JSON value from agent text using the fallback chain.
pub fn extract_json(text: &str) -> Option<Value> {
    if let Some(s) = extract_code_block(text) {
        if let Ok(v) = serde_json::from_str::<Value>(s.trim()) {
            return Some(v);
        }
    }
    if let Some(s) = extract_braces(text) {
        if let Ok(v) = serde_json::from_str::<Value>(s.trim()) {
            return Some(v);
        }
    }
    if let Ok(v) = serde_json::from_str::<Value>(text.trim()) {
        return Some(v);
    }
    None
}

/// Parse a SprintContract from generator/evaluator output.
pub fn parse_contract(text: &str) -> SprintContract {
    let Some(val) = extract_json(text) else {
        return SprintContract::default();
    };

    if let Ok(c) = serde_json::from_value::<SprintContract>(val.clone()) {
        return c;
    }

    // Manual field extraction when keys exist but structure differs slightly
    let features: Vec<String> = val
        .get("features")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(|| vec!["feature".to_string()]);

    let criteria: Vec<Criterion> = val
        .get("criteria")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let name = c.get("name")?.as_str()?.to_string();
                    let description = c
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("no description")
                        .to_string();
                    let threshold = c
                        .get("threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(7.0);
                    Some(Criterion { name, description, threshold })
                })
                .collect()
        })
        .unwrap_or_default();

    SprintContract { features, criteria }
}

/// Parse an EvaluationResult from evaluator output.
pub fn parse_evaluation(text: &str) -> EvaluationResult {
    let Some(val) = extract_json(text) else {
        return default_evaluation(text);
    };

    if let Ok(e) = serde_json::from_value::<EvaluationResult>(val.clone()) {
        return e;
    }

    // Manual extraction
    let passed = val
        .get("passed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let scores: Vec<CriterionScore> = val
        .get("scores")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    let name = s.get("name")?.as_str()?.to_string();
                    let score = s.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let threshold = s.get("threshold").and_then(|v| v.as_f64()).unwrap_or(7.0);
                    Some(CriterionScore { name, score, threshold })
                })
                .collect()
        })
        .unwrap_or_default();

    let feedback: Vec<String> = val
        .get("feedback")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let overall_summary = val
        .get("overallSummary")
        .or_else(|| val.get("overall_summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("no summary")
        .to_string();

    EvaluationResult { passed, scores, feedback, overall_summary }
}

/// Parse a SprintPlan from Planner agent output.
///
/// Expects JSON: `{ "sprints": [{ "number", "title", "features": [], "criteria": [] }] }`
/// Falls back to a single sprint containing the full PRD text if parsing fails.
pub fn parse_sprint_plan(text: &str, prd_fallback: &str) -> SprintPlan {
    if let Some(val) = extract_json(text) {
        if let Ok(plan) = serde_json::from_value::<SprintPlan>(val.clone()) {
            if !plan.sprints.is_empty() {
                return plan;
            }
        }

        // Manual extraction: handle minor structural variations
        if let Some(sprints_arr) = val.get("sprints").and_then(|v| v.as_array()) {
            let sprints: Vec<PlannedSprint> = sprints_arr
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    let number = s
                        .get("number")
                        .and_then(|v| v.as_u64())
                        .unwrap_or((i + 1) as u64) as u32;
                    let title = s
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Sprint")
                        .to_string();
                    let features: Vec<String> = s
                        .get("features")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|f| f.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let criteria: Vec<Criterion> = s
                        .get("criteria")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|c| {
                                    let name = c.get("name")?.as_str()?.to_string();
                                    let description = c
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("criterion")
                                        .to_string();
                                    let threshold =
                                        c.get("threshold").and_then(|v| v.as_f64()).unwrap_or(7.0);
                                    Some(Criterion { name, description, threshold })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    Some(PlannedSprint { number, title, features, criteria })
                })
                .collect();

            if !sprints.is_empty() {
                return SprintPlan { sprints };
            }
        }
    }

    // Fallback: single sprint with full PRD and 3 default criteria
    SprintPlan {
        sprints: vec![PlannedSprint {
            number: 1,
            title: "Sprint 1".to_string(),
            features: vec![prd_fallback.to_string()],
            criteria: vec![
                Criterion {
                    name: "functionality".to_string(),
                    description: "Core functionality works as specified".to_string(),
                    threshold: 7.0,
                },
                Criterion {
                    name: "code_quality".to_string(),
                    description: "Code is clean, well-structured, and follows best practices"
                        .to_string(),
                    threshold: 7.0,
                },
                Criterion {
                    name: "tests_passing".to_string(),
                    description: "All tests pass and edge cases are handled".to_string(),
                    threshold: 7.0,
                },
            ],
        }],
    }
}

/// Check if the evaluator approved the contract without revision.
pub fn is_approved(text: &str) -> bool {
    let upper = text.trim().to_uppercase();
    // APPROVED must appear as a standalone word at the start or on its own line
    upper.starts_with("APPROVED")
        || upper.lines().any(|l| l.trim() == "APPROVED")
}

// ── Internals ─────────────────────────────────────────────────────────────────

fn extract_code_block(text: &str) -> Option<&str> {
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(&after[..end]);
        }
    }
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        // Skip optional language tag on the same line
        let content = match after.find('\n') {
            Some(nl) => &after[nl + 1..],
            None => after,
        };
        if let Some(end) = content.find("```") {
            return Some(&content[..end]);
        }
    }
    None
}

fn extract_braces(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth: i32 = 0;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn default_evaluation(text: &str) -> EvaluationResult {
    let passed = text.contains("APPROVED") || text.contains("PASS") || text.contains("passed");
    EvaluationResult {
        passed,
        scores: vec![],
        feedback: vec![text.lines().next().unwrap_or("no feedback").to_string()],
        overall_summary: if passed {
            "Approved".to_string()
        } else {
            "Needs revision".to_string()
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_block() {
        let text = "Here is the JSON:\n```json\n{\"key\": \"value\"}\n```\nDone.";
        let val = extract_json(text).unwrap();
        assert_eq!(val["key"], "value");
    }

    #[test]
    fn test_extract_braces() {
        let text = "Response: {\"features\": [\"a\", \"b\"]}";
        let val = extract_json(text).unwrap();
        assert_eq!(val["features"][0], "a");
    }

    #[test]
    fn test_parse_contract_default_on_garbage() {
        let contract = parse_contract("sorry I cannot do that");
        assert!(!contract.features.is_empty());
        assert!(!contract.criteria.is_empty());
    }

    #[test]
    fn test_is_approved() {
        assert!(is_approved("APPROVED"));
        assert!(is_approved("approved"));
        assert!(!is_approved("NOT APPROVED"));
        assert!(!is_approved("{\"passed\": true}"));
    }

    #[test]
    fn test_parse_evaluation_fallback() {
        let eval = parse_evaluation("APPROVED — everything looks good");
        assert!(eval.passed);
    }
}
