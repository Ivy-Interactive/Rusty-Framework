//! Hook invariant checks for `#[rusty::view]`.
//!
//! Two rules, both applied to the body of `fn build` inside an `impl View for X`
//! block:
//!
//! * **`conditional_hooks`** — hook slots are keyed by call *index*
//!   (`BuildContext::next_hook_index`), so a hook reached only on some builds
//!   shifts every later hook's slot and `get_or_init_state` starts handing back
//!   another hook's value.
//! * **`set_during_build`** — `State::set` / `State::update` sends a rebuild for
//!   the view currently building, so calling it synchronously in `build` is an
//!   unconditional rebuild loop. `use_ref` returns the same `State<T>` type with
//!   `silent: true`, so the check tracks where each binding came from rather
//!   than trusting the method name.
//!
//! Both rules are syntactic and therefore heuristic. Each can be switched off
//! per impl block with `#[rusty::view(allow(conditional_hooks))]` /
//! `#[rusty::view(allow(set_during_build))]`.

use std::collections::HashSet;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ImplItem, ItemImpl};

/// Every hook that consumes a slot from `BuildContext`, directly or through a
/// hook it composes. Matched on the *last* path segment, so `use_state` and
/// `hooks::use_state` both hit.
///
/// `use_mutation`, `use_service`, `try_use_service` and `signal_registry` are
/// deliberately absent: they take `&BuildContext` and consume no slot.
pub(crate) const SLOT_CONSUMING_HOOKS: &[&str] = &[
    "use_alert",
    "use_callback",
    "use_context",
    "use_download",
    "use_download_bytes",
    "use_download_stream",
    "use_effect",
    "use_effect_with_deps",
    "use_form",
    "use_interval",
    "use_memo",
    "use_query",
    "use_receiver_id",
    "use_reducer",
    "use_ref",
    "use_signal",
    "use_state",
    "use_stream",
    "use_stream_text",
    "use_trigger",
    "use_trigger_unit",
    "use_upload",
    "use_upload_to",
];

/// Hooks whose return value triggers a rebuild when mutated.
const STATEFUL_HOOKS: &[&str] = &["use_state", "use_reducer", "use_form", "use_download"];

/// Hooks whose return value is a silent `State` (`use_ref`), safe to mutate
/// during build.
const SILENT_HOOKS: &[&str] = &["use_ref"];

/// The two rule names accepted by `#[rusty::view(allow(..))]`.
pub(crate) const RULE_CONDITIONAL_HOOKS: &str = "conditional_hooks";
pub(crate) const RULE_SET_DURING_BUILD: &str = "set_during_build";

/// A conditional context the visitor is currently inside, in the wording used
/// by the diagnostic.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    IfBranch,
    MatchArm,
    ForLoop,
    WhileLoop,
    Loop,
    Closure,
    AsyncBlock,
}

impl Context {
    /// How the diagnostic names this context ("is called inside {}").
    fn describe(self) -> &'static str {
        match self {
            Context::IfBranch => "an `if` branch",
            Context::MatchArm => "a `match` arm",
            Context::ForLoop => "a `for` loop body",
            Context::WhileLoop => "a `while` loop body",
            Context::Loop => "a `loop` body",
            Context::Closure => "a closure",
            Context::AsyncBlock => "an `async` block",
        }
    }

    /// A closure or async block runs *later*, so its hook calls are not part of
    /// this build's slot sequence at all — a different failure from a branch,
    /// and worth saying so.
    fn is_deferred(self) -> bool {
        matches!(self, Context::Closure | Context::AsyncBlock)
    }
}

/// What a local binding holds, for the `set_during_build` rule.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// From `use_state` / `use_reducer` / … — mutating it requests a rebuild.
    Stateful,
    /// From `use_ref` — mutating it is silent and safe.
    Silent,
}

/// Walks `fn build`'s body and collects rule violations.
struct HookVisitor {
    /// Stack of enclosing conditional contexts; empty means "unconditional".
    contexts: Vec<Context>,
    /// Bindings whose origin we know, by identifier.
    stateful: HashSet<String>,
    silent: HashSet<String>,
    errors: Vec<syn::Error>,
    check_conditional: bool,
    check_set: bool,
}

impl HookVisitor {
    fn new(check_conditional: bool, check_set: bool) -> Self {
        HookVisitor {
            contexts: Vec::new(),
            stateful: HashSet::new(),
            silent: HashSet::new(),
            errors: Vec::new(),
            check_conditional,
            check_set,
        }
    }

    /// The innermost conditional context, if any.
    fn current_context(&self) -> Option<Context> {
        self.contexts.last().copied()
    }

    /// True when no closure or async block encloses the cursor — i.e. code that
    /// runs *during* this build rather than after it.
    fn at_immediate_depth(&self) -> bool {
        !self.contexts.iter().any(|c| c.is_deferred())
    }

    fn in_context<F: FnOnce(&mut Self)>(&mut self, context: Context, f: F) {
        self.contexts.push(context);
        f(self);
        self.contexts.pop();
    }

    fn report_conditional_hook(&mut self, hook: &str, context: Context, span: proc_macro2::Span) {
        let message = if context.is_deferred() {
            format!(
                "`{hook}` is called inside {}, which runs after `build` returns; its hook slot is \
                 not part of this build's call sequence. Call it at the top level of `build` and \
                 move the returned handle into the closure. Silence this rule for the whole impl \
                 block with `#[rusty::view(allow({RULE_CONDITIONAL_HOOKS}))]`.",
                context.describe()
            )
        } else {
            format!(
                "`{hook}` is called inside {}; hooks are keyed by call order, so a conditional \
                 call shifts every later hook's slot. Hoist it to the top level of `build`. \
                 Silence this rule for the whole impl block with \
                 `#[rusty::view(allow({RULE_CONDITIONAL_HOOKS}))]`.",
                context.describe()
            )
        };
        self.errors.push(syn::Error::new(span, message));
    }

    fn report_set_during_build(&mut self, binding: &str, method: &str, span: proc_macro2::Span) {
        self.errors.push(syn::Error::new(
            span,
            format!(
                "`{binding}.{method}()` runs synchronously during `build`, and `{binding}` came \
                 from a rebuild-triggering hook — this requests a rebuild of the view that is \
                 currently building, i.e. an unconditional rebuild loop. Move the call into an \
                 event handler or an effect, or use `use_ref` if the value should not trigger \
                 rebuilds. Silence this rule for the whole impl block with \
                 `#[rusty::view(allow({RULE_SET_DURING_BUILD}))]`."
            ),
        ));
    }

    /// The last path segment of a call target, when the target is a plain path
    /// (`use_state`, `hooks::use_state`, `crate::hooks::use_state`).
    fn callee_name(func: &Expr) -> Option<String> {
        match func {
            Expr::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        }
    }

    /// Record what a `let` binding holds, so a later `.set()` on it can be
    /// classified. Handles `let x = use_state(..)`, `let x = y.clone()`, and the
    /// tuple-destructuring hooks (`use_reducer`, `use_form`).
    fn record_binding_origin(&mut self, local: &syn::Local) {
        let Some(init) = &local.init else { return };
        let Some(origin) = self.origin_of(&init.expr) else {
            return;
        };
        for ident in pattern_idents(&local.pat) {
            match origin {
                Origin::Stateful => {
                    self.silent.remove(&ident);
                    self.stateful.insert(ident);
                }
                Origin::Silent => {
                    self.stateful.remove(&ident);
                    self.silent.insert(ident);
                }
            }
        }
    }

    /// Classify an initializer expression as stateful, silent, or unknown.
    fn origin_of(&self, expr: &Expr) -> Option<Origin> {
        match expr {
            Expr::Call(call) => {
                let name = Self::callee_name(&call.func)?;
                if STATEFUL_HOOKS.contains(&name.as_str()) {
                    Some(Origin::Stateful)
                } else if SILENT_HOOKS.contains(&name.as_str()) {
                    Some(Origin::Silent)
                } else {
                    None
                }
            }
            // `let alias = count.clone();` propagates the origin of `count`.
            Expr::MethodCall(call) if call.method == "clone" && call.args.is_empty() => {
                match &*call.receiver {
                    Expr::Path(path) => {
                        let ident = path.path.get_ident()?.to_string();
                        if self.stateful.contains(&ident) {
                            Some(Origin::Stateful)
                        } else if self.silent.contains(&ident) {
                            Some(Origin::Silent)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            // `let x = (use_state(ctx, 0), ..)` — take the first known origin.
            Expr::Tuple(tuple) => tuple.elems.iter().find_map(|e| self.origin_of(e)),
            Expr::Reference(r) => self.origin_of(&r.expr),
            Expr::Paren(p) => self.origin_of(&p.expr),
            Expr::Group(g) => self.origin_of(&g.expr),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for HookVisitor {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        // Visit the initializer first: a hook call in it is reported at the
        // current context, and only then does the binding come into scope.
        visit::visit_local(self, node);
        self.record_binding_origin(node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if self.check_conditional {
            if let Some(name) = Self::callee_name(&node.func) {
                if SLOT_CONSUMING_HOOKS.contains(&name.as_str()) {
                    if let Some(context) = self.current_context() {
                        self.report_conditional_hook(&name, context, node.func.span());
                    }
                }
            }
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.check_set && self.at_immediate_depth() {
            let method = node.method.to_string();
            if method == "set" || method == "update" {
                if let Expr::Path(path) = &*node.receiver {
                    if let Some(ident) = path.path.get_ident() {
                        let name = ident.to_string();
                        if self.stateful.contains(&name) {
                            self.report_set_during_build(&name, &method, node.method.span());
                        }
                    }
                }
            }
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        // The condition itself runs on every build, so it is unconditional.
        self.visit_expr(&node.cond);
        self.in_context(Context::IfBranch, |v| {
            v.visit_block(&node.then_branch);
            if let Some((_, else_branch)) = &node.else_branch {
                v.visit_expr(else_branch);
            }
        });
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.visit_expr(&node.expr);
        self.in_context(Context::MatchArm, |v| {
            for arm in &node.arms {
                v.visit_arm(arm);
            }
        });
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.visit_expr(&node.expr);
        self.in_context(Context::ForLoop, |v| v.visit_block(&node.body));
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.visit_expr(&node.cond);
        self.in_context(Context::WhileLoop, |v| v.visit_block(&node.body));
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.in_context(Context::Loop, |v| v.visit_block(&node.body));
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.in_context(Context::Closure, |v| v.visit_expr(&node.body));
    }

    fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
        self.in_context(Context::AsyncBlock, |v| v.visit_block(&node.block));
    }

    /// A nested item (a helper `fn`, a nested `impl`) has its own hook sequence;
    /// do not attribute its calls to this `build`.
    fn visit_item(&mut self, _node: &'ast syn::Item) {}
}

/// Collect the identifiers a pattern binds. Enough for the shapes hooks return:
/// `x`, `mut x`, `(a, b)`, `(a, _)`, `Some(x)`.
fn pattern_idents(pat: &syn::Pat) -> Vec<String> {
    match pat {
        syn::Pat::Ident(ident) => vec![ident.ident.to_string()],
        syn::Pat::Tuple(tuple) => tuple.elems.iter().flat_map(pattern_idents).collect(),
        syn::Pat::TupleStruct(ts) => ts.elems.iter().flat_map(pattern_idents).collect(),
        syn::Pat::Reference(r) => pattern_idents(&r.pat),
        syn::Pat::Paren(p) => pattern_idents(&p.pat),
        syn::Pat::Type(t) => pattern_idents(&t.pat),
        _ => Vec::new(),
    }
}

/// Which rules to run, parsed from `#[rusty::view(allow(..))]`.
#[derive(Debug)]
pub(crate) struct RuleConfig {
    pub check_conditional: bool,
    pub check_set: bool,
}

impl Default for RuleConfig {
    fn default() -> Self {
        RuleConfig {
            check_conditional: true,
            check_set: true,
        }
    }
}

/// Parse the attribute arguments of `#[rusty::view(..)]`.
///
/// Accepts nothing at all, or one or more `allow(rule, ..)` groups. An unknown
/// rule name is an error rather than a silent no-op, so a typo in an `allow`
/// cannot look like a passing lint.
pub(crate) fn parse_rule_config(attr: proc_macro2::TokenStream) -> syn::Result<RuleConfig> {
    let mut config = RuleConfig::default();
    if attr.is_empty() {
        return Ok(config);
    }

    let parser = syn::meta::parser(|meta| {
        if !meta.path.is_ident("allow") {
            let name = path_string(&meta.path);
            return Err(meta.error(format!(
                "unknown `rusty::view` argument `{name}`; the only argument is \
                 `allow({RULE_CONDITIONAL_HOOKS})` / `allow({RULE_SET_DURING_BUILD})`"
            )));
        }
        meta.parse_nested_meta(|rule| {
            let name = path_string(&rule.path);
            match name.as_str() {
                RULE_CONDITIONAL_HOOKS => config.check_conditional = false,
                RULE_SET_DURING_BUILD => config.check_set = false,
                _ => {
                    return Err(rule.error(format!(
                        "unknown `rusty::view` rule `{name}`; expected \
                         `{RULE_CONDITIONAL_HOOKS}` or `{RULE_SET_DURING_BUILD}`"
                    )))
                }
            }
            Ok(())
        })
    });

    syn::parse::Parser::parse2(parser, attr)?;
    Ok(config)
}

fn path_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Check every `fn build` in `item` against the enabled rules.
///
/// Returns the violations, folded into one `syn::Error` so all of them report at
/// once. `Ok(())` means the impl block is clean (or every rule was allowed).
pub(crate) fn check_impl(item: &ItemImpl, config: &RuleConfig) -> syn::Result<()> {
    if !config.check_conditional && !config.check_set {
        return Ok(());
    }

    let mut visitor = HookVisitor::new(config.check_conditional, config.check_set);
    for impl_item in &item.items {
        if let ImplItem::Fn(method) = impl_item {
            if method.sig.ident == "build" {
                visitor.visit_block(&method.block);
            }
        }
    }

    let mut errors = visitor.errors.into_iter();
    match errors.next() {
        None => Ok(()),
        Some(mut combined) => {
            for error in errors {
                combined.combine(error);
            }
            Err(combined)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run the rules over an `impl` block written as a string, returning each
    /// violation's message.
    fn check(source: &str) -> Vec<String> {
        check_with(source, RuleConfig::default())
    }

    fn check_with(source: &str, config: RuleConfig) -> Vec<String> {
        let item: ItemImpl = syn::parse_str(source).expect("fixture must parse");
        match check_impl(&item, &config) {
            Ok(()) => Vec::new(),
            Err(error) => error.into_iter().map(|e| e.to_string()).collect(),
        }
    }

    fn wrap(body: &str) -> String {
        format!(
            "impl View for T {{ fn build(&self, ctx: &mut BuildContext) -> Element {{ {body} }} }}"
        )
    }

    #[test]
    fn top_level_hook_is_clean() {
        assert!(check(&wrap("let c = use_state(ctx, 0i32); c.get().into()")).is_empty());
    }

    #[test]
    fn hook_in_if_branch_is_flagged() {
        let found = check(&wrap(
            "if self.flag { let _ = use_state(ctx, 0i32); } ().into()",
        ));
        assert_eq!(found.len(), 1, "expected exactly one finding: {found:?}");
        assert!(
            found[0].contains("`use_state` is called inside an `if` branch"),
            "{found:?}"
        );
        assert!(
            found[0].contains(RULE_CONDITIONAL_HOOKS),
            "message must name the allow that silences it: {found:?}"
        );
    }

    #[test]
    fn hook_in_else_branch_is_flagged() {
        let found = check(&wrap(
            "if self.flag { } else { let _ = use_memo(ctx, &[], || 1); } ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`use_memo`"), "{found:?}");
    }

    #[test]
    fn hook_in_if_condition_is_clean() {
        // The condition runs on every build, so the slot order is stable.
        assert!(check(&wrap("if use_state(ctx, false).get() { } ().into()")).is_empty());
    }

    #[test]
    fn hook_in_match_arm_is_flagged() {
        let found = check(&wrap(
            "match self.mode { 1 => { let _ = use_ref(ctx, 0i32); } _ => {} } ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("a `match` arm"), "{found:?}");
    }

    #[test]
    fn hook_in_match_scrutinee_is_clean() {
        assert!(check(&wrap(
            "match use_state(ctx, 0i32).get() { _ => {} } ().into()"
        ))
        .is_empty());
    }

    #[test]
    fn hook_in_for_body_is_flagged() {
        let found = check(&wrap(
            "for i in 0..3 { let _ = use_state(ctx, i); } ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("a `for` loop body"), "{found:?}");
    }

    #[test]
    fn hook_in_for_iterator_is_clean() {
        assert!(check(&wrap("for _ in 0..use_state(ctx, 3).get() { } ().into()")).is_empty());
    }

    #[test]
    fn hook_in_while_and_loop_bodies_are_flagged() {
        let found = check(&wrap(
            "while self.flag { let _ = use_state(ctx, 0i32); } ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("a `while` loop body"), "{found:?}");

        let found = check(&wrap(
            "loop { let _ = use_state(ctx, 0i32); break; } ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("a `loop` body"), "{found:?}");
    }

    #[test]
    fn hook_in_closure_is_flagged_as_deferred() {
        let found = check(&wrap(
            "let f = move || { let _ = use_state(ctx, 0i32); }; ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("a closure"), "{found:?}");
        assert!(found[0].contains("runs after `build` returns"), "{found:?}");
    }

    #[test]
    fn hook_in_async_block_is_flagged_as_deferred() {
        let found = check(&wrap(
            "let f = async move { let _ = use_state(ctx, 0i32); }; ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("an `async` block"), "{found:?}");
    }

    #[test]
    fn qualified_hook_path_is_matched() {
        let found = check(&wrap(
            "if self.flag { let _ = rusty::hooks::use_state(ctx, 0i32); } ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`use_state`"), "{found:?}");
    }

    #[test]
    fn non_hook_call_in_branch_is_clean() {
        assert!(check(&wrap("if self.flag { helper(ctx); } ().into()")).is_empty());
    }

    #[test]
    fn every_slot_consuming_hook_is_detected() {
        for hook in SLOT_CONSUMING_HOOKS {
            let found = check(&wrap(&format!(
                "if self.flag {{ let _ = {hook}(ctx); }} ().into()"
            )));
            assert_eq!(found.len(), 1, "{hook} was not flagged: {found:?}");
        }
    }

    #[test]
    fn hooks_outside_build_are_ignored() {
        let found = check(
            "impl View for T {
                 fn build(&self, ctx: &mut BuildContext) -> Element { ().into() }
                 fn helper(&self, ctx: &mut BuildContext) {
                     if self.flag { let _ = use_state(ctx, 0i32); }
                 }
             }",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn nested_fn_item_inside_build_is_ignored() {
        let found = check(&wrap(
            "fn inner(ctx: &mut BuildContext) { if true { let _ = use_state(ctx, 0i32); } } \
             ().into()",
        ));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn set_on_stateful_binding_is_flagged() {
        let found = check(&wrap("let c = use_state(ctx, 0i32); c.set(5); ().into()"));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("`c.set()` runs synchronously during `build`"),
            "{found:?}"
        );
        assert!(
            found[0].contains(RULE_SET_DURING_BUILD),
            "message must name the allow that silences it: {found:?}"
        );
    }

    #[test]
    fn update_on_stateful_binding_is_flagged() {
        let found = check(&wrap(
            "let c = use_state(ctx, 0i32); c.update(|v| v + 1); ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`c.update()`"), "{found:?}");
    }

    #[test]
    fn set_on_clone_alias_is_flagged() {
        let found = check(&wrap(
            "let c = use_state(ctx, 0i32); let alias = c.clone(); alias.set(5); ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`alias.set()`"), "{found:?}");
    }

    #[test]
    fn update_on_use_ref_binding_is_clean() {
        // The exact shape in rusty/examples/hooks_showcase.rs.
        assert!(check(&wrap(
            "let renders = use_ref(ctx, 0i32); renders.update(|n| n + 1); ().into()"
        ))
        .is_empty());
    }

    #[test]
    fn clone_of_use_ref_binding_is_clean() {
        assert!(check(&wrap(
            "let r = use_ref(ctx, 0i32); let alias = r.clone(); alias.set(1); ().into()"
        ))
        .is_empty());
    }

    #[test]
    fn set_inside_closure_is_clean() {
        // The shape every example uses: move the handle into an event handler.
        assert!(check(&wrap(
            "let c = use_state(ctx, 0i32); let h = c.clone(); \
             Button::new(\"x\").on_click(move || { h.set(1); }).into()"
        ))
        .is_empty());
    }

    #[test]
    fn set_inside_async_block_is_clean() {
        // The shape in rusty/src/core/runtime.rs: tokio::spawn(async move { .. }).
        assert!(check(&wrap(
            "let c = use_state(ctx, 0i32); let h = c.clone(); \
             tokio::spawn(async move { h.set(1); }); ().into()"
        ))
        .is_empty());
    }

    #[test]
    fn set_on_unknown_binding_is_clean() {
        // Conservative: a receiver we cannot trace is not flagged.
        assert!(check(&wrap("self.thing.set(5); other.set(5); ().into()")).is_empty());
    }

    #[test]
    fn set_on_reducer_state_is_flagged() {
        let found = check(&wrap(
            "let (state, dispatch) = use_reducer(ctx, r, 0i32); state.set(1); ().into()",
        ));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`state.set()`"), "{found:?}");
    }

    #[test]
    fn shadowing_a_stateful_binding_with_a_ref_clears_it() {
        assert!(check(&wrap(
            "let c = use_state(ctx, 0i32); let c = use_ref(ctx, 0i32); c.set(1); ().into()"
        ))
        .is_empty());
    }

    #[test]
    fn both_rules_report_together() {
        let found = check(&wrap(
            "let c = use_state(ctx, 0i32); c.set(1); \
             if self.flag { let _ = use_memo(ctx, &[], || 1); } ().into()",
        ));
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn allow_conditional_hooks_suppresses_only_that_rule() {
        let config = RuleConfig {
            check_conditional: false,
            check_set: true,
        };
        let found = check_with(
            &wrap(
                "let c = use_state(ctx, 0i32); c.set(1); \
                 if self.flag { let _ = use_memo(ctx, &[], || 1); } ().into()",
            ),
            config,
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`c.set()`"), "{found:?}");
    }

    #[test]
    fn allow_set_during_build_suppresses_only_that_rule() {
        let config = RuleConfig {
            check_conditional: true,
            check_set: false,
        };
        let found = check_with(
            &wrap(
                "let c = use_state(ctx, 0i32); c.set(1); \
                 if self.flag { let _ = use_memo(ctx, &[], || 1); } ().into()",
            ),
            config,
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`use_memo`"), "{found:?}");
    }

    #[test]
    fn parse_rule_config_defaults_to_both_enabled() {
        let config = parse_rule_config(proc_macro2::TokenStream::new()).unwrap();
        assert!(config.check_conditional && config.check_set);
    }

    #[test]
    fn parse_rule_config_reads_allow_groups() {
        let config = parse_rule_config("allow(conditional_hooks)".parse().unwrap()).unwrap();
        assert!(!config.check_conditional && config.check_set);

        let config = parse_rule_config("allow(set_during_build)".parse().unwrap()).unwrap();
        assert!(config.check_conditional && !config.check_set);

        let config = parse_rule_config(
            "allow(conditional_hooks, set_during_build)"
                .parse()
                .unwrap(),
        )
        .unwrap();
        assert!(!config.check_conditional && !config.check_set);
    }

    #[test]
    fn parse_rule_config_rejects_unknown_rule() {
        let error = parse_rule_config("allow(no_such_rule)".parse().unwrap()).unwrap_err();
        assert!(
            error.to_string().contains("unknown `rusty::view` rule"),
            "{error}"
        );
    }

    #[test]
    fn parse_rule_config_rejects_unknown_argument() {
        let error = parse_rule_config("deny(conditional_hooks)".parse().unwrap()).unwrap_err();
        assert!(
            error.to_string().contains("unknown `rusty::view` argument"),
            "{error}"
        );
    }

    #[test]
    fn hook_lists_are_disjoint_and_known() {
        for hook in STATEFUL_HOOKS.iter().chain(SILENT_HOOKS) {
            assert!(
                SLOT_CONSUMING_HOOKS.contains(hook),
                "{hook} tracks an origin but is not in SLOT_CONSUMING_HOOKS"
            );
        }
        for hook in STATEFUL_HOOKS {
            assert!(
                !SILENT_HOOKS.contains(hook),
                "{hook} is listed as both stateful and silent"
            );
        }
    }
}
