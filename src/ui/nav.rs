//! Navigation state system for embed-based UI flows.
use async_trait::async_trait;
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::id::UserId;

#[async_trait]
pub trait NavState: Send + Sync {
    fn id(&self) -> &'static str;
    async fn render(&self, ctx: &ContextBag) -> (CreateEmbed, Vec<CreateActionRow>);
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub struct ContextBag {
    pub db: sqlx::PgPool,
    pub user_id: UserId,
}
impl ContextBag {
    pub fn new(db: sqlx::PgPool, user_id: UserId) -> Self {
        Self { db, user_id }
    }
}

#[derive(Default)]
pub struct NavStack {
    pub stack: Vec<Box<dyn NavState>>,
}
impl NavStack {
    pub fn pop(&mut self) -> Option<Box<dyn NavState>> {
        self.stack.pop()
    }
    /// Pushes a new state while enforcing a maximum depth (discarding the oldest when exceeded).
    pub fn push_capped(&mut self, s: Box<dyn NavState>, max_depth: usize) {
        if self.stack.len() >= max_depth {
            // Remove oldest (front) to keep recent navigation context.
            self.stack.remove(0);
        }
        self.stack.push(s);
    }
    // (Pruned unused helper methods to avoid dead_code warnings.)
}
