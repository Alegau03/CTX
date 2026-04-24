use crate::{AdapterInvocation, Agent, compose_prompt};

pub fn prepare(query: &str, compact_context: &str) -> AdapterInvocation {
    prepare_from_template("agent", query, compact_context)
}

pub fn prepare_from_template(
    command_template: &str,
    query: &str,
    compact_context: &str,
) -> AdapterInvocation {
    let mut parts = command_template.split_whitespace();
    let program = parts.next().unwrap_or("agent").to_string();
    let args = parts.map(ToString::to_string).collect::<Vec<_>>();

    AdapterInvocation {
        agent: Agent::Generic,
        program,
        args,
        prompt: compose_prompt(query, compact_context),
    }
}
