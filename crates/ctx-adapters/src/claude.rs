use crate::{AdapterInvocation, Agent, compose_prompt};

pub fn prepare(query: &str, compact_context: &str) -> AdapterInvocation {
    AdapterInvocation {
        agent: Agent::Claude,
        program: "claude".to_string(),
        args: Vec::new(),
        prompt: compose_prompt(query, compact_context),
    }
}
