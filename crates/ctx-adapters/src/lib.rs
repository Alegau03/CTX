mod claude;
mod codex;
mod generic;
mod opencode;

use std::io;
use std::process::{Command, ExitStatus};
use std::time::Instant;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Agent {
    Codex,
    Claude,
    OpenCode,
    Generic,
}

impl Agent {
    pub fn label(self) -> &'static str {
        match self {
            Agent::Codex => "codex",
            Agent::Claude => "claude",
            Agent::OpenCode => "opencode",
            Agent::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterInvocation {
    pub agent: Agent,
    pub program: String,
    pub args: Vec<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterRunStatus {
    Succeeded,
    Failed,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterExecutionResult {
    pub agent: Agent,
    pub command: String,
    pub status: AdapterRunStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub fallback_used: bool,
    pub fallback_reason: Option<String>,
}

impl AdapterInvocation {
    pub fn command_preview(&self) -> String {
        let mut parts = vec![self.program.clone()];
        parts.extend(self.args.iter().cloned());
        parts.push(format!("\"{}\"", escape(&self.prompt)));
        parts.join(" ")
    }
}

pub fn compose_prompt(query: &str, compact_context: &str) -> String {
    format!("{query}\n\n[CTX COMPACT CONTEXT]\n{compact_context}\n[END CTX COMPACT CONTEXT]")
}

pub fn prepare_invocation(agent: Agent, query: &str, compact_context: &str) -> AdapterInvocation {
    match agent {
        Agent::Codex => codex::prepare(query, compact_context),
        Agent::Claude => claude::prepare(query, compact_context),
        Agent::OpenCode => opencode::prepare(query, compact_context),
        Agent::Generic => generic::prepare(query, compact_context),
    }
}

pub fn prepare_generic_invocation(
    command_template: &str,
    query: &str,
    compact_context: &str,
) -> AdapterInvocation {
    generic::prepare_from_template(command_template, query, compact_context)
}

pub fn build_invocation(agent: Agent, query: &str, compact_context: &str) -> String {
    prepare_invocation(agent, query, compact_context).command_preview()
}

pub fn execute_invocation_with_result(
    invocation: &AdapterInvocation,
) -> io::Result<AdapterExecutionResult> {
    let started = Instant::now();
    let command = invocation.command_preview();

    match Command::new(&invocation.program)
        .args(&invocation.args)
        .arg(&invocation.prompt)
        .status()
    {
        Ok(status) => Ok(AdapterExecutionResult {
            agent: invocation.agent,
            command,
            status: if status.success() {
                AdapterRunStatus::Succeeded
            } else {
                AdapterRunStatus::Failed
            },
            exit_code: status.code(),
            duration_ms: started.elapsed().as_millis() as u64,
            fallback_used: false,
            fallback_reason: None,
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(AdapterExecutionResult {
            agent: invocation.agent,
            command,
            status: AdapterRunStatus::Fallback,
            exit_code: None,
            duration_ms: started.elapsed().as_millis() as u64,
            fallback_used: true,
            fallback_reason: Some(format!(
                "program '{}' not found in PATH",
                invocation.program
            )),
        }),
        Err(err) => Err(err),
    }
}

pub fn execute_invocation(invocation: &AdapterInvocation) -> io::Result<ExitStatus> {
    Command::new(&invocation.program)
        .args(&invocation.args)
        .arg(&invocation.prompt)
        .status()
}

fn escape(input: &str) -> String {
    input.replace('"', "\\\"")
}
