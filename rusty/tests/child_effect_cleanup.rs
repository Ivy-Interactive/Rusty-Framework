//! Regression test for effect cleanups colliding between a parent view and a
//! view it embeds via `child_view()`.
//!
//! `child_view` merges the child's effects into the parent's effect list, so
//! before cleanups were keyed by `(view_id, hook_index)` the child's effect at
//! hook index 0 and the parent's effect at hook index 0 shared one slot in the
//! root `HookStore`. That made the child run the parent's *fresh* cleanup during
//! the very first build, and paired cleanups with the wrong effects on rebuild.

use std::sync::{Arc, Mutex};

use rusty::core::Runtime;
use rusty::hooks::deps::DynEq;
use rusty::hooks::use_effect::use_effect_with_deps;
use rusty::views::view::{BuildContext, Element, View};
use rusty::widgets::{Layout, TextBlock};

/// Ordered record of every effect run and cleanup, in the order they happened.
type Log = Arc<Mutex<Vec<String>>>;

/// The dependency both effects watch. Bumping it between builds makes each
/// effect re-run, which is what makes the cleanup pairing observable.
type Dep = Arc<Mutex<i32>>;

fn record(log: &Log, entry: &str) {
    log.lock().unwrap().push(entry.to_string());
}

fn take_log(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.lock().unwrap())
}

struct Child {
    log: Log,
    dep: Dep,
}

impl View for Child {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let dep = *self.dep.lock().unwrap();
        let log = self.log.clone();
        // The child's only effect — hook index 0 within the child.
        use_effect_with_deps(ctx, &[&dep as &dyn DynEq], move |_| {
            record(&log, "child-run");
            let cleanup_log = log.clone();
            Some(Box::new(move || record(&cleanup_log, "child-cleanup"))
                as Box<dyn FnOnce() + Send + Sync>)
        });
        Element::Widget(Box::new(TextBlock::new("child")))
    }
}

struct Parent {
    log: Log,
    dep: Dep,
}

impl View for Parent {
    fn build(&self, ctx: &mut BuildContext) -> Element {
        let dep = *self.dep.lock().unwrap();
        let log = self.log.clone();
        // The parent's only effect — also hook index 0, the index the child's
        // effect used to clobber.
        use_effect_with_deps(ctx, &[&dep as &dyn DynEq], move |_| {
            record(&log, "parent-run");
            let cleanup_log = log.clone();
            Some(Box::new(move || record(&cleanup_log, "parent-cleanup"))
                as Box<dyn FnOnce() + Send + Sync>)
        });

        let (child_element, _child_id) = ctx.child_view(Child {
            log: self.log.clone(),
            dep: self.dep.clone(),
        });

        Layout::vertical()
            .child(TextBlock::new("parent"))
            .child(child_element)
            .into()
    }
}

#[tokio::test]
async fn child_effect_does_not_clobber_parent_cleanup() {
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let dep: Dep = Arc::new(Mutex::new(1));

    let mut runtime = Runtime::new(Parent {
        log: log.clone(),
        dep: dep.clone(),
    });

    let _ = runtime.build().await;

    // Build 1: both effects run once and no cleanup fires. A cleanup here would
    // mean one view ran the other view's freshly registered cleanup.
    assert_eq!(
        take_log(&log),
        vec!["parent-run", "child-run"],
        "first build must run both effects and no cleanup"
    );

    // Change the shared dep so both effects re-run on the next build.
    *dep.lock().unwrap() = 2;
    let _ = runtime.build().await;

    // Build 2: each view's own cleanup runs immediately before its own effect.
    assert_eq!(
        take_log(&log),
        vec!["parent-cleanup", "parent-run", "child-cleanup", "child-run"],
        "each cleanup must be paired with the effect that produced it"
    );
}
