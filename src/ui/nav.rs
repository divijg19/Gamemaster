//! Navigation state system for embed-based UI flows.
use async_trait::async_trait;
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::id::UserId;

#[async_trait]
pub trait NavState: Send + Sync {
    fn id(&self) -> &'static str;
    async fn render(&self, ctx: &ContextBag) -> (CreateEmbed, Vec<CreateActionRow>);
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
    pub fn push(&mut self, s: Box<dyn NavState>) {
        self.stack.push(s)
    }
    pub fn pop(&mut self) -> Option<Box<dyn NavState>> {
        self.stack.pop()
    }
    // (Pruned unused helper methods to avoid dead_code warnings.)
}
