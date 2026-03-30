/// Adversarial harness — GAN-inspired generator ↔ evaluator loop.
///
/// Implements Stories 2.1–2.4:
/// - 2.1  Spawn generator and evaluator agents in TUI panes
/// - 2.2  Contract negotiation phase
/// - 2.3  Build → Evaluate → Feedback → Retry loop
/// - 2.4  Persist all state in the context store
///
/// Each phase writes its state to context keys:
///   adversarial:status   — current phase string
///   adversarial:config   — JSON AdversarialConfig
///   adversarial:contract — JSON SprintContract
///   adversarial:attempt  — current attempt number (u32)
///   adversarial:scores   — JSON Vec<CriterionScore>
///   adversarial:feedback — JSON Vec<String>

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::broadcast;

use crate::adversarial::parser::{is_approved, parse_contract, parse_evaluation};
use crate::adversarial::types::{
    AdversarialConfig, AdversarialStatus, EvaluationResult, SprintContract,
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
/// Spawns generator + evaluator, negotiates a contract, then enters the
/// build → evaluate → retry loop until PASSED, FAILED, or STOPPED.
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
    // Persist config
    persist_status(&state, AdversarialStatus::Spawning);
    persist_str(&state, "adversarial:config", &serde_json::to_string(&config)?);

    // ── Story 2.1: Spawn agents ───────────────────────────────────────────────
    let (gen_pane, eval_pane) =
        spawn_agents(&state, &config, &events_tx).await?;

    check_stop(&stop, &state)?;

    // ── Story 2.2: Contract negotiation ──────────────────────────────────────
    persist_status(&state, AdversarialStatus::ContractNegotiation);
    let contract = negotiate_contract(&state, &config, gen_pane, eval_pane, &stop).await?;

    // Persist agreed contract
    persist_str(
        &state,
        "adversarial:contract",
        &serde_json::to_string(&contract)?,
    );

    check_stop(&stop, &state)?;

    // ── Story 2.3: Build → evaluate loop ─────────────────────────────────────
    let result =
        build_evaluate_loop(&state, &config, &contract, gen_pane, eval_pane, &stop).await;

    match result {
        Ok(true) => {
            persist_status(&state, AdversarialStatus::Passed);
            let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialSprintPassed {
                sprint: 1,
                total_attempts: config.max_retries,
            }));
            let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialComplete {
                sprints_passed: 1,
                total_duration_ms: 0,
            }));
        }
        Ok(false) => {
            persist_status(&state, AdversarialStatus::Failed);
            let _ = events_tx.send(Arc::new(BmuxEvent::AdversarialFailed {
                sprint: 1,
                reason: "Evaluation failed after max retries".to_string(),
            }));
        }
        Err(e) if e.to_string().contains("Stopped") => {
            persist_status(&state, AdversarialStatus::Stopped);
        }
        Err(e) => {
            persist_status(&state, AdversarialStatus::Error(e.to_string()));
        }
    }

    Ok(())
}

// ── Story 2.1: Agent spawning ─────────────────────────────────────────────────

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

    // Build env + shell command.  The shell stays interactive so the TUI pane
    // remains alive across multiple prompt/response cycles.
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

    // Allow the shell to start
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Register in AgentRegistry (role stored as agent_type)
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

    // Emit AgentSpawned event
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

// ── Story 2.2: Contract negotiation ──────────────────────────────────────────

async fn negotiate_contract(
    state: &DaemonState,
    config: &AdversarialConfig,
    gen_pane: usize,
    eval_pane: usize,
    stop: &AtomicBool,
) -> Result<SprintContract> {
    // Generator proposes
    let gen_prompt = format!(
        "Propose a sprint contract for the following task:\n\n{}\n\n\
         Output ONLY valid JSON with this exact structure:\n\
         {{\"features\": [\"...\"], \"criteria\": [{{\"name\": \"...\", \
         \"description\": \"...\", \"threshold\": 7.0}}]}}",
        config.prompt
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
                "Review this sprint contract:\n\n{contract_json}\n\n\
                 If it is rigorous and complete, output exactly: APPROVED\n\
                 Otherwise output a tougher revised JSON contract \
                 (same structure, stricter criteria/higher thresholds)."
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

        // Evaluator proposed revisions — use the revised contract
        let revised = parse_contract(&eval_response);
        if !revised.criteria.is_empty() {
            contract = revised;
        }
    }

    Ok(contract)
}

// ── Story 2.3: Build → evaluate loop ─────────────────────────────────────────

async fn build_evaluate_loop(
    state: &DaemonState,
    config: &AdversarialConfig,
    contract: &SprintContract,
    gen_pane: usize,
    eval_pane: usize,
    stop: &AtomicBool,
) -> Result<bool> {
    let contract_json = serde_json::to_string_pretty(contract)?;
    let max_retries = config.max_retries;

    for attempt in 0..=max_retries {
        check_stop(stop, state)?;

        // Persist attempt counter
        persist_str(state, "adversarial:attempt", &attempt.to_string());

        // ── Generator builds ──────────────────────────────────────────────────
        persist_status(state, AdversarialStatus::Building);

        let build_prompt = if attempt == 0 {
            format!(
                "Sprint contract:\n{contract_json}\n\n\
                 Task: {}\n\n\
                 Implement all features. Commit your work after each feature. \
                 Focus on meeting every criterion threshold.",
                config.prompt
            )
        } else {
            // Retry: include evaluator feedback
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

            format!(
                "Sprint contract:\n{contract_json}\n\n\
                 Task: {}\n\n\
                 Previous evaluation (attempt {attempt}):\n\
                 Scores: {scores_json}\n\
                 Feedback: {feedback_json}\n\n\
                 Address ALL feedback and retry. Commit after each fix.",
                config.prompt
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

        // ── Evaluator scores ──────────────────────────────────────────────────
        persist_status(state, AdversarialStatus::Evaluating);

        let eval_prompt = format!(
            "Sprint contract:\n{contract_json}\n\n\
             Evaluate the implementation against every criterion. \
             Score each criterion 1–10. Output ONLY valid JSON:\n\
             {{\"passed\": true/false, \
              \"scores\": [{{\"name\": \"...\", \"score\": 8.0, \"threshold\": 7.0}}], \
              \"feedback\": [\"...\"], \
              \"overallSummary\": \"...\"}}",
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

        tracing::debug!(attempt = attempt, response = %eval_response, "Evaluator result");

        let eval_result: EvaluationResult = parse_evaluation(&eval_response);

        // Persist scores + feedback (Story 2.4)
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

        // Check pass condition: all criteria >= threshold
        let all_passed = eval_result.passed
            || eval_result.scores.iter().all(|s| s.score >= s.threshold);

        if all_passed {
            tracing::info!(attempt = attempt, "Adversarial sprint PASSED");
            return Ok(true);
        }

        tracing::info!(
            attempt = attempt,
            summary = %eval_result.overall_summary,
            "Criteria not met — will retry"
        );

        if attempt < max_retries {
            persist_status(state, AdversarialStatus::Retrying);
        }
    }

    tracing::warn!("Adversarial sprint FAILED after {} attempts", max_retries + 1);
    Ok(false)
}

// ── Agent query via PTY pane ──────────────────────────────────────────────────

/// Send a prompt to an agent pane using `claude --print`, wait for completion,
/// return the captured response text.
///
/// Protocol:
///   1. Write prompt to a temp file (avoids shell escaping issues)
///   2. Send `claude --print "$(cat PROMPT_FILE)" > OUTPUT_FILE 2>&1`
///      followed by `echo BMUX_ADV_DONE >> OUTPUT_FILE`
///   3. Poll OUTPUT_FILE until the sentinel "BMUX_ADV_DONE" appears
///   4. Return output content (sentinel stripped)
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

    // Write prompt; clean up previous output
    tokio::fs::write(&pf, prompt).await?;
    let _ = tokio::fs::remove_file(&of).await;
    let _ = tokio::fs::remove_file(&df).await;

    // Build shell command — runs in the pane's interactive shell
    let cmd = format!(
        "claude --dangerously-skip-permissions --model {model} --print \"$(cat {pf})\" \
         > {of} 2>&1 && echo BMUX_ADV_DONE >> {of}\r"
    );

    {
        let mut sess = state.session.lock().await;
        sess.send_input_to_pane(pane_id, cmd.as_bytes())?;
    }

    // Poll for completion (timeout: 5 minutes)
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

    // Timeout: return whatever we have
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
