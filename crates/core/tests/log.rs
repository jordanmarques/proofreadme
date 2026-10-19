//! The agent log, as the platform adapters write to it: one line per problem that does not stop
//! the agent, so that none is silent.

use proofreadme_core::AgentLog;

#[test]
fn a_problem_of_an_adapter_is_one_line_of_the_agent_log() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("logs").join("agent.txt");
    let log = AgentLog::new(path.clone());

    log.note(&"the session icon could not change its look: refused");
    log.note(&"a notification could not be shown: busy");

    let text = std::fs::read_to_string(path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].ends_with("  the session icon could not change its look: refused"), "{text}");
    assert!(lines[1].ends_with("  a notification could not be shown: busy"), "{text}");
}
