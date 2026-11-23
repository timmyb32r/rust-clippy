use crate::misc_early::BUILTIN_TYPE_SHADOW;
use clippy_utils::diagnostics::span_lint_hir;
use clippy_utils::{is_from_proc_macro, sym};
use rustc_hir::def::{DefKind, Namespace, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{ForeignItem, HirId, Item, ItemKind, PrimTy, UseKind, UseTree};
use rustc_lint::{LateContext, LateLintPass, LintContext as _, declare_lint_pass};
use rustc_middle::ty::{TypeVisitableExt as _, TypingEnv};
use rustc_span::{Ident, Symbol};

declare_lint_pass!(BuiltinTypeShadow => [BUILTIN_TYPE_SHADOW]);

impl<'tcx> LateLintPass<'tcx> for BuiltinTypeShadow {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Use(tree) = &item.kind {
            check_use(cx, tree, item.hir_id());
        } else if let Some(ident) = item.kind.ident()
            && PrimTy::from_name(ident.name).is_some()
        {
            let id = item.owner_id.to_def_id();
            let res = Res::Def(cx.tcx.def_kind(id), id);
            if res.ns() == Some(Namespace::TypeNS) || matches!(res, Res::Def(DefKind::ExternCrate, _)) {
                check_binding(cx, ident, res, item.hir_id());
            }
        }
    }

    fn check_foreign_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ForeignItem<'tcx>) {
        if matches!(item.kind, rustc_hir::ForeignItemKind::Type) {
            check_binding(
                cx,
                item.ident,
                Res::Def(DefKind::ForeignTy, item.owner_id.to_def_id()),
                item.hir_id(),
            );
        }
    }
}

fn check_use(cx: &LateContext<'_>, tree: &UseTree<'_>, hir_id: HirId) {
    match tree.kind {
        UseKind::Single(ident) => {
            if let Some(res) = tree.prefix.res.type_ns {
                check_binding(cx, ident, res, hir_id);
            }
        },
        UseKind::Nested { items } => {
            for (tree, id, _) in items {
                check_use(cx, tree, *id);
            }
        },
        UseKind::Glob => {},
    }
}

fn check_binding(cx: &LateContext<'_>, ident: Ident, res: Res, hir_id: HirId) {
    let name = ident.name;
    if !is_available_primitive(ident)
        || cx.tcx.hir_span(hir_id).in_external_macro(cx.sess().source_map())
        || ident.span.in_external_macro(cx.sess().source_map())
        || is_from_proc_macro(cx, &ident)
        || preserves_name(cx, name, res)
    {
        return;
    }
    span_lint_hir(
        cx,
        BUILTIN_TYPE_SHADOW,
        hir_id,
        ident.span,
        format!("this binding shadows the built-in type `{name}`"),
    );
}

pub(crate) fn is_available_primitive(ident: Ident) -> bool {
    match ident.name {
        name @ (sym::f16 | sym::f128) => {
            ident.span.allows_unstable(name) || rustc_middle::ty::tls::with(|tcx| tcx.features().enabled(name))
        },
        name => PrimTy::from_name(name).is_some(),
    }
}

fn preserves_name(cx: &LateContext<'_>, name: Symbol, res: Res) -> bool {
    match res {
        Res::PrimTy(ty) => ty.name() == name,
        Res::Def(DefKind::TyAlias, id) => bare_alias_primitive(cx, id) == Some(name),
        Res::Def(DefKind::Mod, id) if cx.tcx.opt_item_name(id) == Some(name) => {
            is_standard_primitive_module(cx, id, name)
        },
        _ => false,
    }
}

fn bare_alias_primitive(cx: &LateContext<'_>, id: DefId) -> Option<Symbol> {
    let tcx = cx.tcx;
    let generics = tcx.generics_of(id);
    let defaults = generics.own_defaults();
    if defaults.types + defaults.consts != generics.own_params.len() {
        // In particular, a lifetime cannot be elided in a struct field's type.
        return None;
    }

    if !generics.own_params.is_empty() && tcx.type_alias_is_checked(id) {
        // Even an unused const parameter must be inferred in a checked alias's expression path.
        // For `type u8<const N: usize = 0> = core::primitive::u8`, `u8::MAX` can fail to infer N.
        return None;
    }

    let rhs = tcx.type_of(id).instantiate_identity();
    if rhs.has_param() {
        // Expression paths infer arguments, even when type annotations use defaults:
        // `type u8<T = core::primitive::u8> = T` lets `u8::clone(&1)` return i32.
        return None;
    }
    tcx.try_normalize_erasing_regions(TypingEnv::non_body_analysis(tcx, id), rhs)
        .ok()?
        .primitive_symbol()
}

fn is_standard_primitive_module(cx: &LateContext<'_>, id: DefId, name: Symbol) -> bool {
    let tcx = cx.tcx;
    let lang = tcx.lang_items();
    let library_item = match tcx.crate_name(id.krate) {
        sym::core => lang.sized_trait(),
        sym::alloc => lang.string(),
        sym::std => lang.start_fn(),
        _ => return false,
    };
    if library_item.is_none_or(|item| item.krate != id.krate) {
        return false;
    }

    // Check exports of the crate root: integer modules are reexported from a private module,
    // and std::str is an alias for alloc::str.
    let root = id.krate.as_def_id();
    let exports = match root.as_local() {
        Some(root) => tcx.module_children_local(root),
        None => tcx.module_children(root),
    };
    exports
        .iter()
        .any(|child| child.ident.name == name && child.vis.is_public() && child.res == Res::Def(DefKind::Mod, id))
}
