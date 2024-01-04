use std::sync::Arc;

use deno_core::error::AnyError;
use deno_core::parking_lot::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
pub struct InMemoryBroadcastChannel(pub Arc<Mutex<broadcast::Sender<Message>>>);

pub struct InMemoryBroadcastChannelResource {
  rx: tokio::sync::Mutex<(broadcast::Receiver<Message>, mpsc::UnboundedReceiver<()>)>,
  cancel_tx: mpsc::UnboundedSender<()>,
  uuid: Uuid,
}

#[derive(Clone, Debug)]
struct Message {
  name: Arc<String>,
  data: Arc<Vec<u8>>,
  uuid: Uuid,
}

impl Default for InMemoryBroadcastChannel {
  fn default() -> Self {
    let (tx, _) = broadcast::channel(256);
    Self(Arc::new(Mutex::new(tx)))
  }
}

impl InMemoryBroadcastChannel {
  pub(crate) fn subscribe(&self) -> Result<InMemoryBroadcastChannelResource, AnyError> {
    let (cancel_tx, cancel_rx) = mpsc::unbounded_channel();
    let broadcast_rx = self.0.lock().subscribe();
    let rx = tokio::sync::Mutex::new((broadcast_rx, cancel_rx));
    let uuid = Uuid::new_v4();
    Ok(InMemoryBroadcastChannelResource { rx, cancel_tx, uuid })
  }

  pub fn unsubscribe(&self, resource: &InMemoryBroadcastChannelResource) -> Result<(), AnyError> {
    Ok(resource.cancel_tx.send(())?)
  }

  pub fn send(&self, resource: &InMemoryBroadcastChannelResource, name: String, data: Vec<u8>) -> Result<(), AnyError> {
    let name = Arc::new(name);
    let data = Arc::new(data);
    let uuid = resource.uuid;
    self.0.lock().send(Message { name, data, uuid })?;
    Ok(())
  }

  pub async fn recv(&self, resource: &InMemoryBroadcastChannelResource) -> Result<Option<crate::Message>, AnyError> {
    let mut g = resource.rx.lock().await;
    let (broadcast_rx, cancel_rx) = &mut *g;
    loop {
      let result = tokio::select! {
        r = broadcast_rx.recv() => r,
        _ = cancel_rx.recv() => return Ok(None),
      };
      use tokio::sync::broadcast::error::RecvError::*;
      match result {
        Err(Closed) => return Ok(None),
        Err(Lagged(_)) => (),                               // Backlogged, messages dropped.
        Ok(message) if message.uuid == resource.uuid => (), // Self-send.
        Ok(message) => {
          let name = String::clone(&message.name);
          let data = Vec::clone(&message.data);
          return Ok(Some((name, data)));
        }
      }
    }
  }
}

impl deno_core::Resource for InMemoryBroadcastChannelResource {}
