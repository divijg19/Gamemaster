//! Tests for capped navigation stack behavior.
use gamemaster_bot::ui::NavStack;
use std::marker::PhantomData;
use std::sync::Arc;

struct DummyState(&'static str, PhantomData<Arc<()>>);
#[async_trait::async_trait]
impl gamemaster_bot::ui::NavState for DummyState {
    fn id(&self) -> &'static str {
        self.0
    }
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }
    fn as_any_mut(&mut self) -> &mut (dyn std::any::Any + Send + Sync) {
        self
    }
    async fn render(
        &self,
        _ctx: &gamemaster_bot::ui::ContextBag,
    ) -> (
        serenity::builder::CreateEmbed,
        Vec<serenity::builder::CreateActionRow>,
    ) {
        (serenity::builder::CreateEmbed::new().title(self.0), vec![])
    }
}

#[test]
fn capped_push_discards_oldest() {
    let mut stack = NavStack::default();
    for i in 0..20 {
        stack.push_capped(
            Box::new(DummyState(
                Box::leak(format!("s{}", i).into_boxed_str()),
                PhantomData,
            )),
            15,
        );
    }
    assert_eq!(stack.stack.len(), 15);
    // Oldest kept should be state s5 after inserting s0..s19 (20 items) with cap 15
    assert_eq!(stack.stack.first().unwrap().id(), "s5");
    assert_eq!(stack.stack.last().unwrap().id(), "s19");
}
