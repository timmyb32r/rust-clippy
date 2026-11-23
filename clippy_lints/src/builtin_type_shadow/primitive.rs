use clippy_utils::sym;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_lint::LateContext;
use rustc_middle::ty::{TypeVisitableExt as _, TypingEnv};
use rustc_span::Symbol;

pub(super) fn preserves_name(cx: &LateContext<'_>, name: Symbol, res: Res) -> bool {
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
