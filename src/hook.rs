pub type HookFn = Box<dyn Fn() + Send + Sync>;
pub enum HookType {
    Connect,
}
pub struct Hook {}

impl Hook {
    pub fn add(&self) {
        println!("add hook");
    }
    pub fn run(&self) {
        println!("run hook");
    }
}