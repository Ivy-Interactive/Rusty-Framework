//! End-to-end diagnostics tests for `#[rusty::view]` and `#[derive(Widget)]`.
//!
//! `compile_fail` cases assert the *exact* stderr, so `.stderr` files are
//! snapshots: never hand-edit them. To accept a changed message, delete the
//! `.stderr` and run `cargo test -p rusty-macros --test ui` — trybuild writes
//! the new output to `rusty-macros/wip/<name>.stderr` (the crate root, *not*
//! `tests/ui/wip/`), which you move back into `tests/ui/`. Delete the `wip/`
//! directory afterwards; it is gitignored so a stray one cannot be committed.
//!
//! `pass` cases carry the shapes that already exist in the repo and must never
//! be flagged: a hook in an `if` *condition*, `.set()` inside a closure or an
//! `async` block, `.update()` on a `use_ref` binding, and each `allow(..)`
//! escape hatch.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();

    // Rule A — conditional hooks.
    t.compile_fail("tests/ui/hook_in_if.rs");
    t.compile_fail("tests/ui/hook_in_match_arm.rs");
    t.compile_fail("tests/ui/hook_in_for_body.rs");
    t.compile_fail("tests/ui/hook_in_closure.rs");

    // Rule B — set/update during build.
    t.compile_fail("tests/ui/set_during_build.rs");
    t.compile_fail("tests/ui/update_during_build.rs");

    // Attribute argument errors.
    t.compile_fail("tests/ui/unknown_allow_rule.rs");

    // Derive diagnostics.
    t.compile_fail("tests/ui/derive_misnamed_container.rs");
    t.compile_fail("tests/ui/derive_event_only_struct.rs");
    t.compile_fail("tests/ui/derive_prop_and_event.rs");
    t.compile_fail("tests/ui/derive_non_option_event.rs");
    t.compile_fail("tests/ui/derive_bad_id_type.rs");

    // Negative controls — the shapes the repo already contains.
    t.pass("tests/ui/pass_hook_in_if_condition.rs");
    t.pass("tests/ui/pass_set_in_closure.rs");
    t.pass("tests/ui/pass_set_in_async_block.rs");
    t.pass("tests/ui/pass_update_on_ref.rs");

    // The `allow(..)` hatches must suppress.
    t.pass("tests/ui/pass_allow_conditional_hooks.rs");
    t.pass("tests/ui/pass_allow_set_during_build.rs");
    t.pass("tests/ui/pass_allow_both.rs");
}
