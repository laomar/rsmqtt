use crate::{Error, Packet};
use std::sync::RwLock;

type HookFn = Box<dyn Fn(Packet) -> Result<Packet, Error> + Send + Sync>;
pub struct Hook(RwLock<Vec<HookFn>>);

impl Hook {
    pub fn new() -> Self {
        Hook(RwLock::new(Vec::new()))
    }

    pub fn register(&self, hook: impl Fn(Packet) -> Result<Packet, Error> + Send + Sync + 'static) {
        let mut hooks = self.0.write().unwrap();
        hooks.push(Box::new(hook));
    }

    pub fn trigger(&self, packet: Packet) -> Result<Packet, Error> {
        let hooks = self.0.read().unwrap();
        for hook in hooks.iter() {
            let result = hook(packet.clone());
            match result {
                Ok(Packet::None) => continue,
                _ => return result,
            }
        }
        Ok(Packet::None)
    }
}
