mod claude;
mod codex;
mod generic;
mod opencode;

use std::io;
use std::process::{Command, ExitStatus};

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

pub fn build_invocation(agent: Agent, query: &str, compact_context: &str) -> String {
    prepare_invocation(agent, query, compact_context).command_preview()
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
