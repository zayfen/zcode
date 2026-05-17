use std::sync::Arc;

use zcode_orchestration::{AgentModelLabels, GraphEvent, LoopEvent};
use zcode_ui::TaskUiEvent;

pub(crate) fn print_loop_event(agent: &str, event: LoopEvent) {
    match event {
        LoopEvent::ToolStart { tool_name, command } => {
            println!("🔧 {} {}: {}", agent, tool_name, command);
        }
        LoopEvent::ToolComplete { tool_name, success } => {
            println!(
                "{} {} {}",
                if success { "✅" } else { "❌" },
                agent,
                tool_name
            );
        }
    }
}

pub(crate) fn print_task_models(models: &AgentModelLabels) {
    println!("🧭 Supervisor model: {}", models.supervisor);
    println!("🔎 Investigator model: {}", models.investigator);
    println!("🧠 Planner model: {}", models.planner);
    println!("🛠 Coder model: {}", models.coder);
    println!("🔍 Reviewer model: {}", models.reviewer);
    println!("⚡ Fast model: {}", models.fast);
}

pub(crate) fn print_cli_graph_event(prefix: &str, event: GraphEvent) {
    match event {
        GraphEvent::ToolStart {
            agent,
            tool_name,
            command,
        } => {
            println!("🔧 {} {}: {}", agent, tool_name, command);
        }
        GraphEvent::StepStart { id, title, agent } => {
            println!("▶ {} {}: {}", agent, id, title);
        }
        GraphEvent::StepComplete {
            id,
            title,
            agent,
            success,
        } => {
            println!(
                "{} {} {}: {}",
                if success { "✅" } else { "❌" },
                agent,
                id,
                title
            );
        }
        GraphEvent::ToolComplete {
            agent,
            tool_name,
            success,
        } => {
            println!(
                "{} {} {}",
                if success { "✅" } else { "❌" },
                agent,
                tool_name
            );
        }
        other => println!("{} {}", prefix, other),
    }
}

pub(crate) fn make_tui_tool_event_sink(
    tx: std::sync::mpsc::Sender<TaskUiEvent>,
) -> Arc<dyn Fn(GraphEvent) + Send + Sync> {
    Arc::new(move |event| match event {
        GraphEvent::ToolStart {
            agent,
            tool_name,
            command,
        } => {
            let _ = tx.send(TaskUiEvent::ToolStart {
                agent,
                tool_name,
                command,
            });
        }
        GraphEvent::ToolComplete {
            agent,
            tool_name,
            success,
        } => {
            let _ = tx.send(TaskUiEvent::ToolComplete {
                agent,
                tool_name,
                success,
            });
        }
        GraphEvent::StepStart { id, title, agent } => {
            let _ = tx.send(TaskUiEvent::StepStart { id, title, agent });
        }
        GraphEvent::StepComplete {
            id,
            title,
            agent,
            success,
        } => {
            let _ = tx.send(TaskUiEvent::StepComplete {
                id,
                title,
                agent,
                success,
            });
        }
        _ => {}
    })
}
