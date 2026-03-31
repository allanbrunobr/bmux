/// Adversarial harness — GAN-inspired generator ↔ evaluator loop.
///
/// Stories 2.1–2.3 (adversarial v2):
/// - 2.1  Multi-sprint loop: iterate through all sprints from adversarial:sprint_plan
/// - 2.2  Evaluator with real tools: prompt instructs cargo test / curl / grep for security
/// - 2.3  Git commits per feature: Generator commits after each feature and fix
///
/// Context keys written:
///   adversarial:status   — current phase string
///   adversarial:config   — JSON AdversarialConfig
///   adversarial:sprint   — current sprint number (u32)
///   adversarial:contract — JSON SprintContract
///   adversarial:attempt  — current attempt number (u32)
///   adversarial:scores   — JSON Vec<CriterionScore>
///   adversarial:feedback — JSON Vec<String>

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::broadcast;

use crate::adversarial::parser::{is_approved, parse_contract, parse_evaluation, parse_sprint_plan};
use crate::adversarial::types::{
    AdversarialConfig, AdversarialStatus, EvaluationResult, SprintContract, SprintSpec,
};
use crate::agents::adapter::AgentConfig;
use crate::tui::server::DaemonState;
use crate::web::events::BmuxEvent;

/// Temp-file prefix for adversarial prompt/output I/O.
fn prompt_file(session: &str, role: &str) -> String {
    let safe = session.replace(['/', '\\', ':'], "_");
    format!("/tmp/bmux-adv-{safe}-{role}-prompt.txt")
}

fn output_file(session: &str, role: &str) -> String {
    let safe = session.replace(['/', '\\', ':'], "_");
    format!("/tmp/bmux-adv-{safe}-{role}-output.txt")
}

fn done_file(session: &str, role: &str) -> String {
    let safe = session.replace(['/', '\\', ':'], "_");
    format!("/tmp/bmux-adv-{safe}-{role}-done.txt")
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run the full adversarial harness as a background tokio task.
///
/// Spawns generator + evaluator, then iterates over each sprint from the
/// Planner's sprint plan (or falls back to single sprint if no plan exists).
/// Each sprint: negotiate contract → build → evaluate → retry.
pub async fn run(
    state: Arc<DaemonState>,
    events_tx: broadcast::Sender<Arc<BmuxEvent>>,
    config: AdversarialConfig,
    stop: Arc<AtomicBool>,
) {
    if let Err(e) = run_inner(state, events_tx, config, stop).await {
        tracing::error!("Adversarial harness error: {e}");
    }
}

async fn run_inner(
    state: Arc<DaemonState>,
    events_tx: broadcast::Sender<Arc<BmuxEvent>>,
    config: AdversarialConfig,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let start_time = Instant::now();

    // Persist config
    persist_status(&state, AdversarialStatus::Spawning);
    persist_str(&state, "adversarial:config", &serde_json::to_string(&config)?);

    // ── Story 2.1: Load sprint plan from context ──────────────────────────────
    // wt1's Planner sets adversarial:sprint_plan; fall back to single sprint
    let sprints = load_sprint_plan(&state, &config);
    let total_sprints = sprints.len() as u32;

    let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialStarted {
        config: serde_json::to_value(&config).unwrap_or_default(),
        total_sprints: Some(total_sprints),
    }));

    // ── Spawn agents (once for the whole run) ─────────────────────────────────
    let (gen_pane, eval_pane) = spawn_agents(&state, &config, &events_tx).await?;

    check_stop(&stop, &state)?;

    // ── Story 2.1: Multi-sprint loop ──────────────────────────────────────────
    let mut cumulative_context = String::new();
    let mut sprints_passed: u32 = 0;

    for sprint_spec in &sprints {
        let sprint_num = sprint_spec.number;

        check_stop(&stop, &state)?;

        // Track current sprint in context
        persist_str(&state, "adversarial:sprint", &sprint_num.to_string());

        tracing::info!(sprint = sprint_num, total = total_sprints, "Starting sprint");

        // ── Contract negotiation per sprint ───────────────────────────────────
        persist_status(&state, AdversarialStatus::ContractNegotiation);
        let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialNegotiating {
            contract_proposal: serde_json::Value::Null,
        }));

        let contract =
            negotiate_contract(&state, &config, sprint_spec, gen_pane, eval_pane, &stop).await?;

        persist_str(&state, "adversarial:contract", &serde_json::to_string(&contract)?);

        check_stop(&stop, &state)?;

        // ── Build → evaluate loop for this sprint ─────────────────────────────
        let result = build_evaluate_loop(
            &state,
            &config,
            &contract,
            sprint_spec,
            &cumulative_context,
            gen_pane,
            eval_pane,
            &stop,
            &events_tx,
        )
        .await;

        match result {
            Ok(true) => {
                // Sprint passed — emit event, accumulate context, continue
                let elapsed = start_time.elapsed().as_millis() as u64;
                let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialSprintPassed {
                    sprint: sprint_num,
                    total_attempts: config.max_retries,
                }));

                // Build cumulative context for next sprint's Generator
                let features_summary = sprint_spec.features.join(", ");
                if !cumulative_context.is_empty() {
                    cumulative_context.push(' ');
                }
                cumulative_context.push_str(&format!(
                    "Sprint {sprint_num} ({title}) implemented: {features_summary}.",
                    title = sprint_spec.title,
                ));

                sprints_passed += 1;
                tracing::info!(
                    sprint = sprint_num,
                    elapsed_ms = elapsed,
                    "Sprint passed"
                );
            }
            Ok(false) => {
                // Sprint failed after max retries — stop entirely
                persist_status(&state, AdversarialStatus::Failed);
                let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialFailed {
                    sprint: sprint_num,
                    reason: format!(
                        "Sprint {sprint_num} failed after {} attempts",
                        config.max_retries + 1
                    ),
                }));
                tracing::warn!(sprint = sprint_num, "Sprint FAILED — stopping harness");
                return Ok(());
            }
            Err(e) if e.to_string().contains("Stopped") => {
                persist_status(&state, AdversarialStatus::Stopped);
                return Ok(());
            }
            Err(e) => {
                persist_status(&state, AdversarialStatus::Error(e.to_string()));
                return Err(e);
            }
        }
    }

    // ── All sprints passed ────────────────────────────────────────────────────
    persist_status(&state, AdversarialStatus::Passed);
    let total_duration_ms = start_time.elapsed().as_millis() as u64;
    let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialComplete {
        sprints_passed,
        total_duration_ms,
    }));

    tracing::info!(
        sprints_passed = sprints_passed,
        duration_ms = total_duration_ms,
        "All sprints passed — adversarial run complete"
    );

    Ok(())
}

// ── Story 2.1: Load sprint plan ───────────────────────────────────────────────

/// Load sprint plan from context (set by wt1's Planner).
/// Falls back to a single sprint using config.prompt if no plan is found.
fn load_sprint_plan(
    state: &DaemonState,
    config: &AdversarialConfig,
) -> Vec<SprintSpec> {
    if let Ok(Some(plan_json)) = state.context.get("adversarial:sprint_plan") {
        if let Some(plan) = parse_sprint_plan(&plan_json) {
            if !plan.sprints.is_empty() {
                tracing::info!(count = plan.sprints.len(), "Loaded sprint plan from context");
                return plan.sprints;
            }
        }
    }

    // Fallback: single sprint from config.prompt
    tracing::info!("No sprint plan in context — using single-sprint fallback");
    vec![SprintSpec {
        number: 1,
        title: "Sprint 1".to_string(),
        features: vec![config.prompt.clone()],
        criteria: vec![],
    }]
}

// ── Agent spawning ────────────────────────────────────────────────────────────

async fn spawn_agents(
    state: &DaemonState,
    config: &AdversarialConfig,
    events_tx: &broadcast::Sender<Arc<BmuxEvent>>,
) -> Result<(usize, usize)> {
    let gen_pane = spawn_agent(state, "generator", &config.generator_model, events_tx).await?;
    let eval_pane = spawn_agent(state, "evaluator", &config.evaluator_model, events_tx).await?;
    Ok((gen_pane, eval_pane))
}

async fn spawn_agent(
    state: &DaemonState,
    role: &str,
    model: &str,
    events_tx: &broadcast::Sender<Arc<BmuxEvent>>,
) -> Result<usize> {
    let name = format!("adversarial-{role}");
    let agent_type = format!("adversarial-{role}");

    let cmd = format!(
        "export BMUX_SESSION='{}' BMUX_SOCKET='{}' BMUX_AGENT_NAME='{}' BMUX_ADV_ROLE='{}'; \
         exec bash --norc",
        state.session_name,
        state.ipc_socket_path.display(),
        name,
        role,
    );

    let pane_id = {
        let mut sess = state.session.lock().await;
        sess.split_and_run_command(&cmd)?
    };

    tokio::time::sleep(Duration::from_millis(800)).await;

    {
        let mut reg = state.agents.lock().await;
        reg.register(
            &name,
            &agent_type,
            AgentConfig {
                binary: "claude".to_string(),
                model: model.to_string(),
                cost_per_1k_tokens: 0.0,
                args: vec!["--dangerously-skip-permissions".to_string()],
            },
        );
        if let Some(info) = reg.get_mut(&name) {
            info.pane_id = Some(pane_id);
        }
    }

    let _ = events_tx.send(Arc::new(BmuxEvent::AgentSpawned {
        agent: crate::web::routes::AgentInfo {
            id: name.clone(),
            name,
            agent_type,
            model: model.to_string(),
            status: "idle".to_string(),
            tokens_used: 0,
            cost_usd: 0.0,
            uptime_seconds: 0,
            pane_id: Some(pane_id),
            last_task: None,
            spawned_at: chrono::Utc::now().to_rfc3339(),
        },
    }));

    tracing::info!(role = %role, pane = pane_id, model = %model, "Adversarial agent spawned");
    Ok(pane_id)
}

// ── Story 2.1: Contract negotiation (sprint-specific) ────────────────────────

async fn negotiate_contract(
    state: &DaemonState,
    config: &AdversarialConfig,
    sprint_spec: &SprintSpec,
    gen_pane: usize,
    eval_pane: usize,
    stop: &AtomicBool,
) -> Result<SprintContract> {
    let features_list = if sprint_spec.features.is_empty() {
        config.prompt.clone()
    } else {
        sprint_spec.features.join("\n- ")
    };

    let criteria_hint = if sprint_spec.criteria.is_empty() {
        String::new()
    } else {
        let names: Vec<&str> = sprint_spec.criteria.iter().map(|c| c.name.as_str()).collect();
        format!("\n\nEnsure criteria include: {}", names.join(", "))
    };

    let gen_prompt = format!(
        "Propose a sprint contract for Sprint {} — {}.\n\nFeatures to implement:\n- {}{}\n\n\
         Output ONLY valid JSON with this exact structure:\n\
         {{\"features\": [\"...\"], \"criteria\": [{{\"name\": \"...\", \
         \"description\": \"...\", \"threshold\": 7.0}}]}}",
        sprint_spec.number, sprint_spec.title, features_list, criteria_hint
    );

    let gen_response = query_agent(
        state,
        gen_pane,
        &config.generator_model,
        &gen_prompt,
        &state.session_name,
        "generator",
        stop,
    )
    .await?;

    tracing::debug!(response = %gen_response, "Generator contract proposal");
    let mut contract = parse_contract(&gen_response);

    // Evaluator reviews (up to 2 rounds)
    for round in 0..2 {
        check_stop(stop, state)?;

        let contract_json = serde_json::to_string_pretty(&contract)?;
        let eval_prompt = if round == 0 {
            format!(
                "Review this sprint contract for Sprint {} — {}:\n\n{contract_json}\n\n\
                 If it is rigorous and complete, output exactly: APPROVED\n\
                 Otherwise output a tougher revised JSON contract \
                 (same structure, stricter criteria/higher thresholds).",
                sprint_spec.number, sprint_spec.title
            )
        } else {
            format!(
                "Final review — accept or reject:\n\n{contract_json}\n\n\
                 Output APPROVED or a final revised JSON."
            )
        };

        let eval_response = query_agent(
            state,
            eval_pane,
            &config.evaluator_model,
            &eval_prompt,
            &state.session_name,
            "evaluator",
            stop,
        )
        .await?;

        tracing::debug!(round = round, response = %eval_response, "Evaluator contract review");

        if is_approved(&eval_response) {
            tracing::info!("Contract approved by evaluator (round {round})");
            break;
        }

        let revised = parse_contract(&eval_response);
        if !revised.criteria.is_empty() {
            contract = revised;
        }
    }

    Ok(contract)
}

// ── Stories 2.1 + 2.2 + 2.3: Build → evaluate loop ──────────────────────────

async fn build_evaluate_loop(
    state: &DaemonState,
    config: &AdversarialConfig,
    contract: &SprintContract,
    sprint_spec: &SprintSpec,
    cumulative_context: &str,
    gen_pane: usize,
    eval_pane: usize,
    stop: &AtomicBool,
    events_tx: &broadcast::Sender<Arc<BmuxEvent>>,
) -> Result<bool> {
    let contract_json = serde_json::to_string_pretty(contract)?;
    let max_retries = config.max_retries;
    let sprint_num = sprint_spec.number;

    // Build context preamble for Generator (Story 2.1)
    let context_preamble = if cumulative_context.is_empty() {
        String::new()
    } else {
        format!("Previous sprints context:\n{cumulative_context}\n\n")
    };

    let features_list = sprint_spec.features.join("\n- ");

    for attempt in 0..=max_retries {
        check_stop(stop, state)?;

        persist_str(state, "adversarial:attempt", &attempt.to_string());

        // ── Generator builds (Stories 2.1 + 2.3) ─────────────────────────────
        persist_status(state, AdversarialStatus::Building);
        let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialBuilding {
            sprint: sprint_num,
            attempt: attempt + 1,
        }));

        // Story 2.3: include git commit instructions; Story 2.1: include cumulative context
        let build_prompt = if attempt == 0 {
            format!(
                "{context_preamble}\
                 Sprint {sprint_num} — {title}\n\
                 Sprint contract:\n{contract_json}\n\n\
                 Features to implement:\n- {features_list}\n\n\
                 Implement all features. After completing each feature, run:\n\
                 git add -A && git commit -m 'feat(sprint-{sprint_num}): <description>'\n\n\
                 Focus on meeting every criterion threshold.",
                title = sprint_spec.title,
            )
        } else {
            let feedback_json = state
                .context
                .get("adversarial:feedback")
                .ok()
                .flatten()
                .unwrap_or_else(|| "[]".to_string());
            let scores_json = state
                .context
                .get("adversarial:scores")
                .ok()
                .flatten()
                .unwrap_or_else(|| "[]".to_string());

            // Story 2.3: use fix(sprint-N) commit message on retry
            format!(
                "{context_preamble}\
                 Sprint {sprint_num} — {title}\n\
                 Sprint contract:\n{contract_json}\n\n\
                 Previous evaluation (attempt {attempt}):\n\
                 Scores: {scores_json}\n\
                 Feedback: {feedback_json}\n\n\
                 Address ALL feedback items. After each fix, run:\n\
                 git add -A && git commit -m 'fix(sprint-{sprint_num}): <what was fixed>'",
                title = sprint_spec.title,
            )
        };

        query_agent(
            state,
            gen_pane,
            &config.generator_model,
            &build_prompt,
            &state.session_name,
            "generator",
            stop,
        )
        .await?;

        check_stop(stop, state)?;

        // ── Evaluator scores (Story 2.2: real tools) ──────────────────────────
        persist_status(state, AdversarialStatus::Evaluating);
        let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialEvaluating {
            sprint: sprint_num,
        }));

        // Story 2.2: instruct Evaluator to RUN code, not just read it
        let eval_prompt = format!(
            "Sprint {sprint_num} — {title}\n\
             Sprint contract:\n{contract_json}\n\n\
             EVALUATION INSTRUCTIONS — you MUST execute, not just read:\n\
             1. Run `cargo test` (or `npm test` for TypeScript). Report EXACT test names and \
                error messages for any failures.\n\
             2. Curl API endpoints relevant to this sprint. Report URL, expected response, \
                actual response for any failures.\n\
             3. Grep for security issues: `unwrap()` on user input, hardcoded secrets, missing \
                auth checks, SQL injection patterns.\n\
             4. For every issue found, include the FILE PATH and LINE NUMBER.\n\
             5. CRITICAL: Kill any background processes (e.g. `kill %1`) BEFORE outputting \
                your evaluation.\n\
             6. The workspace is in the current directory. Use `cargo test` for Rust, \
                `npm test` for TypeScript.\n\n\
             Score each criterion 1–10. Output ONLY valid JSON:\n\
             {{\"passed\": true/false, \
              \"scores\": [{{\"name\": \"...\", \"score\": 8.0, \"threshold\": 7.0}}], \
              \"feedback\": [\"<file>:<line> — <issue>\"], \
              \"overallSummary\": \"...\"}}",
            title = sprint_spec.title,
        );

        let eval_response = query_agent(
            state,
            eval_pane,
            &config.evaluator_model,
            &eval_prompt,
            &state.session_name,
            "evaluator",
            stop,
        )
        .await?;

        tracing::debug!(sprint = sprint_num, attempt = attempt, response = %eval_response, "Evaluator result");

        let eval_result: EvaluationResult = parse_evaluation(&eval_response);

        persist_str(
            state,
            "adversarial:scores",
            &serde_json::to_string(&eval_result.scores)?,
        );
        persist_str(
            state,
            "adversarial:feedback",
            &serde_json::to_string(&eval_result.feedback)?,
        );

        let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialScores {
            sprint: sprint_num,
            attempt: attempt + 1,
            scores: eval_result
                .scores
                .iter()
                .map(|s| {
                    serde_json::json!({"name": s.name, "score": s.score, "threshold": s.threshold})
                })
                .collect(),
            passed: eval_result.passed,
        }));

        let all_passed =
            eval_result.passed || eval_result.scores.iter().all(|s| s.score >= s.threshold);

        if all_passed {
            tracing::info!(sprint = sprint_num, attempt = attempt, "Sprint PASSED");
            return Ok(true);
        }

        tracing::info!(
            sprint = sprint_num,
            attempt = attempt,
            summary = %eval_result.overall_summary,
            "Criteria not met — will retry"
        );

        if attempt < max_retries {
            persist_status(state, AdversarialStatus::Retrying);
            let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialRetry {
                sprint: sprint_num,
                attempt: attempt + 2,
                feedback: eval_result.feedback.clone(),
            }));
        }
    }

    tracing::warn!(
        sprint = sprint_num,
        "Sprint FAILED after {} attempts",
        max_retries + 1
    );
    Ok(false)
}

// ── Agent query via PTY pane ──────────────────────────────────────────────────

async fn query_agent(
    state: &DaemonState,
    pane_id: usize,
    model: &str,
    prompt: &str,
    session: &str,
    role: &str,
    stop: &AtomicBool,
) -> Result<String> {
    let pf = prompt_file(session, role);
    let of = output_file(session, role);
    let df = done_file(session, role);

    tokio::fs::write(&pf, prompt).await?;
    let _ = tokio::fs::remove_file(&of).await;
    let _ = tokio::fs::remove_file(&df).await;

    let cmd = format!(
        "claude --dangerously-skip-permissions --model {model} --print \"$(cat {pf})\" \
         > {of} 2>&1 && echo BMUX_ADV_DONE >> {of}\r"
    );

    {
        let mut sess = state.session.lock().await;
        sess.send_input_to_pane(pane_id, cmd.as_bytes())?;
    }

    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;

        if stop.load(Ordering::Relaxed) {
            anyhow::bail!("Stopped");
        }
        if Instant::now() > deadline {
            break;
        }

        if let Ok(content) = tokio::fs::read_to_string(&of).await {
            if content.contains("BMUX_ADV_DONE") {
                let clean = content.replace("BMUX_ADV_DONE\n", "").trim().to_string();
                tracing::debug!(role = %role, chars = clean.len(), "Agent response received");
                return Ok(clean);
            }
        }
    }

    let partial = tokio::fs::read_to_string(&of)
        .await
        .unwrap_or_else(|_| "(timeout — no response)".to_string());
    Ok(partial)
}

// ── Context store helpers ─────────────────────────────────────────────────────

fn persist_status(state: &DaemonState, status: AdversarialStatus) {
    let s = status.to_string();
    tracing::info!(status = %s, "Adversarial status update");
    let _ = state.context.set("adversarial:status", &s, None);
}

fn persist_str(state: &DaemonState, key: &str, value: &str) {
    let _ = state.context.set(key, value, None);
}

fn check_stop(stop: &AtomicBool, state: &DaemonState) -> Result<()> {
    if stop.load(Ordering::Relaxed) {
        persist_status(state, AdversarialStatus::Stopped);
        anyhow::bail!("Stopped");
    }
    Ok(())
}
