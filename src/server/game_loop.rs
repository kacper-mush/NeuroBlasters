use super::state::SharedState;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tracing::debug;

pub(super) enum GameLoopEvent {
    Snapshot(crate::protocol::GameSnapshot),
    Delta(crate::protocol::GameDelta),
}

pub(super) struct MockGameLoop;

impl MockGameLoop {
    pub fn spawn(state: Arc<SharedState>, tx: UnboundedSender<GameLoopEvent>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(250));
            let mut tick_id: u64 = 0;
            let mut last_snapshot: HashMap<String, u64> = HashMap::new();

            loop {
                interval.tick().await;
                tick_id = tick_id.wrapping_add(1);
                if tx.is_closed() {
                    break;
                }
                for room_code in state.active_rooms() {
                    if should_send_snapshot(tick_id, last_snapshot.get(&room_code)) {
                        if let Some(snapshot) = state.compose_snapshot(&room_code, tick_id) {
                            last_snapshot.insert(room_code.clone(), snapshot.tick_id);
                            let _ = tx.send(GameLoopEvent::Snapshot(snapshot));
                        }
                    } else if let Some(base_tick) = last_snapshot.get(&room_code).copied() {
                        if let Some(delta) = state.compose_delta(&room_code, tick_id, base_tick) {
                            let _ = tx.send(GameLoopEvent::Delta(delta));
                        }
                    }
                }
                debug!("mock loop tick {}", tick_id);
            }
            debug!("mock game loop stopped");
        })
    }
}

fn should_send_snapshot(current_tick: u64, last_snapshot: Option<&u64>) -> bool {
    match last_snapshot {
        None => true,
        Some(last) => current_tick.saturating_sub(*last) >= 16,
    }
}
