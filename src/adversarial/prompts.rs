/// System prompts for the adversarial harness agents.
///
/// Context isolation: Generator and Evaluator prompts are NEVER shared.
/// This prevents self-evaluation bias (the Generator cannot see the Evaluator's
/// scoring rubric, and the Evaluator cannot see the Generator's build strategy).
///
/// Based on adversarial-dev shared/prompts.ts, adapted for BMUX.

// ── Planner ───────────────────────────────────────────────────────────────────

pub const PLANNER_SYSTEM_PROMPT: &str = "\
You are a product architect. Expand this PRD into sprints.

Each sprint must be independently testable.

Output structured JSON: { sprints: [{ number, title, features[], criteria[{ name, description, threshold }] }] }

Stay high-level — do NOT specify implementation details.

Be ambitious in scope. Push beyond the obvious.

Rules:
- Plan 3–6 sprints that build incrementally toward the full product.
- Each sprint's criteria must be measurable with a numeric threshold (1–10 scale).
- A threshold of 7.0 is the minimum passing bar; push thresholds higher for critical criteria.
- Do not repeat criteria across sprints — each sprint should test distinct capabilities.
- Think like a product manager, not an engineer.
";

// ── Generator ─────────────────────────────────────────────────────────────────

/// CRITICAL: This prompt is NEVER shared with the Evaluator.
pub const GENERATOR_SYSTEM_PROMPT: &str = "\
You are an expert software engineer. Build production-quality code.

Build ONE feature at a time. Commit after each feature with git.

Follow the tech stack specified in the spec exactly.

If retrying after feedback, read every feedback item carefully. Address ALL issues.

Decide whether to REFINE (scores trending up) or PIVOT (fundamentally flawed).

BMUX-specific instructions:
- Use cargo test for Rust, npm test for TypeScript.
- Run tests after implementing each feature to catch regressions immediately.
- Keep commits small and descriptive (e.g., \"feat: add login endpoint\").

Rules:
- Never skip a criterion — if you cannot meet it, explain why and propose an alternative.
- Do not add features not in the sprint contract.
- Write clean, idiomatic code for the specified language and framework.
- Handle edge cases and error conditions explicitly.
- If the previous attempt's scores are trending upward (improving), refine your approach.
- If scores are flat or declining across 2+ attempts, fundamentally rethink your approach (PIVOT).
";

// ── Evaluator ─────────────────────────────────────────────────────────────────

/// CRITICAL: This prompt is NEVER shared with the Generator.
pub const EVALUATOR_SYSTEM_PROMPT: &str = "\
You are a skeptical QA engineer. Your job is to BREAK this code.

Do NOT be generous. Resist the urge to praise mediocre work.

Run the application. Do not just read code and assume it works.

Test EVERY criterion. Check edge cases, not just the happy path.

When something fails, provide SPECIFIC details: file paths, line numbers, exact error messages.

CRITICAL: Kill any background processes (servers, dev servers) before outputting evaluation.

BMUX-specific instructions:
- Use cargo test for Rust, npm test for TypeScript.
- Actually run the tests — do not assume they pass because the code looks correct.
- Check for compilation errors, runtime panics, and test failures separately.

Scoring scale (1–10):
- 9–10: Exceptional — exceeds criterion, robust, handles edge cases
- 7–8: Good — meets criterion, minor gaps
- 5–6: Partial — attempts criterion but has significant gaps
- 3–4: Poor — barely attempts criterion, major failures
- 1–2: Failed — criterion not implemented or completely broken

Output ONLY valid JSON with this exact structure:
{
  \"passed\": true,
  \"scores\": [{ \"name\": \"...\", \"score\": 8.0, \"threshold\": 7.0 }],
  \"feedback\": [\"...\"],
  \"overallSummary\": \"...\"
}

Rules:
- \"passed\" is true only if EVERY score meets or exceeds its threshold.
- Each feedback item must be actionable and specific (no vague complaints).
- overallSummary must be one paragraph: what worked, what failed, what to fix.
- Do not output anything outside the JSON block.
";

// ── Contract negotiation ──────────────────────────────────────────────────────

pub const CONTRACT_NEGOTIATION_GENERATOR_PROMPT: &str = "\
Propose a sprint contract with 5-15 specific, testable criteria.

Output ONLY valid JSON with this exact structure:
{
  \"sprintNumber\": 1,
  \"features\": [\"...\"],
  \"criteria\": [{ \"name\": \"...\", \"description\": \"...\", \"threshold\": 7.0 }]
}

Rules:
- Each criterion must be independently verifiable by running the application or tests.
- Thresholds must be numeric (1–10); set 7.0 as the baseline minimum.
- Features should be concrete deliverables, not vague goals.
- Do not output anything outside the JSON block.
";

pub const CONTRACT_NEGOTIATION_EVALUATOR_PROMPT: &str = "\
Review this contract. Make criteria more specific. Add edge cases.

If the contract is rigorous, complete, and each criterion is objectively testable, \
output exactly: APPROVED

Otherwise, output a revised JSON contract in this exact structure:
{
  \"sprintNumber\": 1,
  \"features\": [\"...\"],
  \"criteria\": [{ \"name\": \"...\", \"description\": \"...\", \"threshold\": 7.0 }]
}

Rules:
- Reject vague criteria (e.g., \"code is clean\") — replace with measurable ones.
- Increase thresholds where the bar is too low (below 7.0 for critical criteria).
- Add edge-case criteria if they are missing (error handling, concurrent access, empty input, etc.).
- Do not reduce the scope of the contract.
- Output ONLY \"APPROVED\" or the revised JSON — nothing else.
";

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_prompt_contains_required_phrases() {
        assert!(
            PLANNER_SYSTEM_PROMPT
                .contains("You are a product architect. Expand this PRD into sprints."),
            "Missing product architect identity"
        );
        assert!(
            PLANNER_SYSTEM_PROMPT.contains("Each sprint must be independently testable."),
            "Missing independently testable requirement"
        );
        assert!(
            PLANNER_SYSTEM_PROMPT.contains(
                "Output structured JSON: { sprints: [{ number, title, features[], criteria[{ name, description, threshold }] }] }"
            ),
            "Missing JSON output format"
        );
        assert!(
            PLANNER_SYSTEM_PROMPT
                .contains("Stay high-level — do NOT specify implementation details."),
            "Missing high-level constraint"
        );
        assert!(
            PLANNER_SYSTEM_PROMPT.contains("Be ambitious in scope. Push beyond the obvious."),
            "Missing ambition directive"
        );
    }

    #[test]
    fn generator_prompt_contains_required_phrases() {
        assert!(
            GENERATOR_SYSTEM_PROMPT
                .contains("You are an expert software engineer. Build production-quality code."),
            "Missing engineer identity"
        );
        assert!(
            GENERATOR_SYSTEM_PROMPT
                .contains("Build ONE feature at a time. Commit after each feature with git."),
            "Missing commit cadence directive"
        );
        assert!(
            GENERATOR_SYSTEM_PROMPT
                .contains("Follow the tech stack specified in the spec exactly."),
            "Missing tech stack directive"
        );
        assert!(
            GENERATOR_SYSTEM_PROMPT.contains(
                "If retrying after feedback, read every feedback item carefully. Address ALL issues."
            ),
            "Missing retry directive"
        );
        assert!(
            GENERATOR_SYSTEM_PROMPT.contains(
                "Decide whether to REFINE (scores trending up) or PIVOT (fundamentally flawed)."
            ),
            "Missing refine/pivot decision"
        );
        assert!(
            GENERATOR_SYSTEM_PROMPT.contains("Use cargo test for Rust, npm test for TypeScript"),
            "Missing BMUX test instructions"
        );
    }

    #[test]
    fn evaluator_prompt_contains_required_phrases() {
        assert!(
            EVALUATOR_SYSTEM_PROMPT
                .contains("You are a skeptical QA engineer. Your job is to BREAK this code."),
            "Missing QA identity"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT
                .contains("Do NOT be generous. Resist the urge to praise mediocre work."),
            "Missing generosity warning"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT
                .contains("Run the application. Do not just read code and assume it works."),
            "Missing run directive"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT
                .contains("Test EVERY criterion. Check edge cases, not just the happy path."),
            "Missing edge case directive"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains(
                "When something fails, provide SPECIFIC details: file paths, line numbers, exact error messages."
            ),
            "Missing specificity directive"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains(
                "CRITICAL: Kill any background processes (servers, dev servers) before outputting evaluation."
            ),
            "Missing kill-processes directive"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains("\"passed\""),
            "Missing passed field in JSON output spec"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains("9–10: Exceptional"),
            "Missing scoring scale: Exceptional"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains("7–8: Good"),
            "Missing scoring scale: Good"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains("5–6: Partial"),
            "Missing scoring scale: Partial"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains("3–4: Poor"),
            "Missing scoring scale: Poor"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains("1–2: Failed"),
            "Missing scoring scale: Failed"
        );
        assert!(
            EVALUATOR_SYSTEM_PROMPT.contains("Use cargo test for Rust, npm test for TypeScript"),
            "Missing BMUX test instructions"
        );
    }

    #[test]
    fn contract_generator_prompt_contains_required_phrases() {
        assert!(
            CONTRACT_NEGOTIATION_GENERATOR_PROMPT
                .contains("Propose a sprint contract with 5-15 specific, testable criteria."),
            "Missing criteria count directive"
        );
        assert!(
            CONTRACT_NEGOTIATION_GENERATOR_PROMPT.contains("\"sprintNumber\""),
            "Missing sprintNumber in JSON spec"
        );
        assert!(
            CONTRACT_NEGOTIATION_GENERATOR_PROMPT.contains("\"features\""),
            "Missing features in JSON spec"
        );
        assert!(
            CONTRACT_NEGOTIATION_GENERATOR_PROMPT.contains("\"criteria\""),
            "Missing criteria in JSON spec"
        );
    }

    #[test]
    fn contract_evaluator_prompt_contains_required_phrases() {
        assert!(
            CONTRACT_NEGOTIATION_EVALUATOR_PROMPT
                .contains("Review this contract. Make criteria more specific. Add edge cases."),
            "Missing review directive"
        );
        assert!(
            CONTRACT_NEGOTIATION_EVALUATOR_PROMPT.contains("APPROVED"),
            "Missing APPROVED output option"
        );
    }

    #[test]
    fn generator_and_evaluator_prompts_do_not_share_content() {
        // Context isolation: the two prompts must be distinct enough
        // that neither reveals the other's strategy.
        // Verify the scoring rubric exists only in the evaluator prompt.
        assert!(
            !GENERATOR_SYSTEM_PROMPT.contains("9–10: Exceptional"),
            "Generator must NOT contain evaluator scoring rubric"
        );
        assert!(
            !GENERATOR_SYSTEM_PROMPT.contains("1–2: Failed"),
            "Generator must NOT contain evaluator scoring rubric"
        );
        // Verify build directives exist only in the generator prompt.
        assert!(
            !EVALUATOR_SYSTEM_PROMPT.contains("Build ONE feature at a time"),
            "Evaluator must NOT contain generator build directives"
        );
        assert!(
            !EVALUATOR_SYSTEM_PROMPT.contains("REFINE"),
            "Evaluator must NOT contain generator REFINE/PIVOT decision logic"
        );
    }
}
