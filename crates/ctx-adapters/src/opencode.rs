use crate::{AdapterInvocation, Agent, compose_prompt};

pub fn prepare(query: &str, compact_context: &str) -> AdapterInvocation {
    AdapterInvocation {
        agent: Agent::OpenCode,
        program: "opencode".to_string(),
        args: vec!["run".to_string()],
        prompt: compose_prompt(query, compact_context),
    }
}
