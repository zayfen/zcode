//! Agent Message Bus
//!
//! A tokio channel-based message bus for inter-agent communication.

use crate::agent::types::{AgentId, AgentMessage};
use std::collections::HashMap;
use tokio::sync::mpsc;

// ─── MessageBus ────────────────────────────────────────────────────────────────

/// The capacity of each agent's inbox channel
const INBOX_CAPACITY: usize = 64;

/// Message bus: routes messages between agents by AgentId
pub struct MessageBus {
    senders: HashMap<AgentId, mpsc::Sender<AgentMessage>>,
}

impl MessageBus {
    /// Create a new empty message bus
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
        }
    }

    /// Register an agent on the bus, returns its receiver (inbox)
    pub fn register(&mut self, id: AgentId) -> mpsc::Receiver<AgentMessage> {
        let (tx, rx) = mpsc::channel(INBOX_CAPACITY);
        self.senders.insert(id, tx);
        rx
    }

    /// Send a message to the target agent.
    /// Returns `Err` if the agent is not registered or inbox is full.
    pub async fn send(
        &self,
        target: &AgentId,
        message: AgentMessage,
    ) -> Result<(), String> {
        let sender = self
            .senders
            .get(target)
            .ok_or_else(|| format!("Agent '{}' not registered on bus", target))?;

        sender
            .send(message)
            .await
            .map_err(|_| format!("Agent '{}' inbox is closed", target))
    }

    /// Broadcast a message to all registered agents
    pub async fn broadcast(&self, message_fn: impl Fn(&AgentId) -> AgentMessage) {
        for (id, sender) in &self.senders {
            let msg = message_fn(id);
            let _ = sender.send(msg).await; // ignore individual send errors
        }
    }

    /// Check if an agent is registered
    pub fn is_registered(&self, id: &AgentId) -> bool {
        self.senders.contains_key(id)
    }

    /// Number of registered agents
    pub fn len(&self) -> usize {
        self.senders.len()
    }

    /// Whether the bus has no registered agents
    pub fn is_empty(&self) -> bool {
        self.senders.is_empty()
    }

    /// Unregister an agent (closes its inbox)
    pub fn unregister(&mut self, id: &AgentId) -> bool {
        self.senders.remove(id).is_some()
    }

    /// List all registered agent IDs
    pub fn registered_agents(&self) -> Vec<&AgentId> {
        self.senders.keys().collect()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

// ─── BusHandle ─────────────────────────────────────────────────────────────────

/// A cloneable handle for sending to the bus from within agents
#[derive(Clone)]
pub struct BusHandle {
    sender: mpsc::Sender<(AgentId, AgentMessage)>,
}

impl BusHandle {
    /// Send a message destined for a specific agent via the central dispatcher
    pub async fn send(
        &self,
        target: AgentId,
        message: AgentMessage,
    ) -> Result<(), String> {
        self.sender
            .send((target, message))
            .await
            .map_err(|_| "Bus handle sender closed".to_string())
    }
}

/// Central dispatcher that owns the bus and routes messages
pub struct BusDispatcher {
    bus: MessageBus,
    receiver: mpsc::Receiver<(AgentId, AgentMessage)>,
    handle_sender: mpsc::Sender<(AgentId, AgentMessage)>,
}

impl BusDispatcher {
    /// Create a new dispatcher with an internal routing channel
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(256);
        Self {
            bus: MessageBus::new(),
            receiver: rx,
            handle_sender: tx,
        }
    }

    /// Get a cloneable handle for agents to use to send messages
    pub fn handle(&self) -> BusHandle {
        BusHandle {
            sender: self.handle_sender.clone(),
        }
    }

    /// Register an agent on the internal bus
    pub fn register(&mut self, id: AgentId) -> mpsc::Receiver<AgentMessage> {
        self.bus.register(id)
    }

    /// Run the dispatcher loop — forwards routed messages to the correct agent inboxes.
    /// Call this in a background task.
    pub async fn run(mut self) {
        while let Some((target, message)) = self.receiver.recv().await {
            let _ = self.bus.send(&target, message).await;
        }
    }
}

impl Default for BusDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::*;

    #[tokio::test]
    async fn test_bus_register_and_send() {
        let mut bus = MessageBus::new();
        let id = AgentId::named("agent-a");
        let mut rx = bus.register(id.clone());

        let msg = AgentMessage::ProgressUpdate {
            agent: id.clone(),
            progress: 0.5,
            message: "half done".to_string(),
        };

        bus.send(&id, msg).await.unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            AgentMessage::ProgressUpdate { progress, .. } => {
                assert!((progress - 0.5).abs() < f32::EPSILON);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_bus_send_to_unknown_agent() {
        let bus = MessageBus::new();
        let id = AgentId::named("ghost");
        let msg = AgentMessage::StreamChunk {
            agent: id.clone(),
            chunk: "hello".to_string(),
        };
        let result = bus.send(&id, msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not registered"));
    }

    #[test]
    fn test_bus_is_registered() {
        let mut bus = MessageBus::new();
        let id = AgentId::named("agent-x");
        assert!(!bus.is_registered(&id));
        bus.register(id.clone());
        assert!(bus.is_registered(&id));
    }

    #[test]
    fn test_bus_len() {
        let mut bus = MessageBus::new();
        assert_eq!(bus.len(), 0);
        bus.register(AgentId::named("a1"));
        bus.register(AgentId::named("a2"));
        assert_eq!(bus.len(), 2);
    }

    #[test]
    fn test_bus_is_empty() {
        let bus = MessageBus::new();
        assert!(bus.is_empty());
    }

    #[test]
    fn test_bus_unregister() {
        let mut bus = MessageBus::new();
        let id = AgentId::named("to-remove");
        bus.register(id.clone());
        assert!(bus.is_registered(&id));
        let removed = bus.unregister(&id);
        assert!(removed);
        assert!(!bus.is_registered(&id));
    }

    #[test]
    fn test_bus_unregister_nonexistent() {
        let mut bus = MessageBus::new();
        let id = AgentId::named("nonexistent");
        assert!(!bus.unregister(&id));
    }

    #[test]
    fn test_bus_registered_agents() {
        let mut bus = MessageBus::new();
        bus.register(AgentId::named("a"));
        bus.register(AgentId::named("b"));
        let agents = bus.registered_agents();
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_bus_broadcast() {
        let mut bus = MessageBus::new();
        let id_a = AgentId::named("a");
        let id_b = AgentId::named("b");
        let mut rx_a = bus.register(id_a.clone());
        let mut rx_b = bus.register(id_b.clone());

        bus.broadcast(|id| AgentMessage::StreamChunk {
            agent: id.clone(),
            chunk: format!("hello {}", id),
        }).await;

        let msg_a = rx_a.recv().await.unwrap();
        let msg_b = rx_b.recv().await.unwrap();
        match msg_a {
            AgentMessage::StreamChunk { chunk, .. } => assert!(chunk.contains("hello")),
            _ => panic!("Wrong type"),
        }
        match msg_b {
            AgentMessage::StreamChunk { chunk, .. } => assert!(chunk.contains("hello")),
            _ => panic!("Wrong type"),
        }
    }

    #[tokio::test]
    async fn test_bus_dispatcher_handle() {
        let mut dispatcher = BusDispatcher::new();
        let id = AgentId::named("worker");
        let mut rx = dispatcher.register(id.clone());
        let handle = dispatcher.handle();

        // Spawn the dispatcher
        tokio::spawn(dispatcher.run());

        handle.send(id.clone(), AgentMessage::StreamChunk {
            agent: id.clone(),
            chunk: "dispatched!".to_string(),
        }).await.unwrap();

        let msg = rx.recv().await.unwrap();
        match msg {
            AgentMessage::StreamChunk { chunk, .. } => assert_eq!(chunk, "dispatched!"),
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_bus_default() {
        let bus = MessageBus::default();
        assert!(bus.is_empty());
    }
}
