use rustc_data_structures::fx::{FxHashMap, FxHashSet, FxIndexMap};
use rustc_hir::def::{DefKind, Namespace, Res};
use rustc_hir::def_id::{DefId, LocalDefId, LocalModId};
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Block, HirId, Item, ItemKind, Mod, Node, Path, PrimTy, StmtKind, UseKind, UseTree};
use rustc_lint::LateContext;
use rustc_metadata::creader::CStore;
use rustc_middle::hir::nested_filter;
use rustc_middle::middle::resolve::{ModChild, Reexport};
use rustc_middle::ty::TyCtxt;
use rustc_span::{BytePos, Ident, Span};

use std::sync::OnceLock;

use super::{BuiltinTypeShadow, use_tree};

pub(super) fn check_mod<'tcx>(
    cx: &LateContext<'tcx>,
    module: &'tcx Mod<'tcx>,
    id: LocalModId,
    lint: &mut BuiltinTypeShadow,
) {
    // The resolver has already handled visibility, explicit imports, and ambiguities in this module.
    let bindings = cx
        .tcx
        .module_children_local(id.to_local_def_id())
        .iter()
        .filter_map(|child| {
            if let Some(Reexport::Glob(id)) = child.reexport_chain.first()
                && let Some(glob) = id.as_local()
                && let Some(ident) = binding_ident(cx.tcx, child)
            {
                Some((glob, ident, child.res.expect_non_local()))
            } else {
                None
            }
        });
    report(cx, bindings, lint, |uses| intravisit::walk_mod(uses, module));
}

fn report<'tcx>(
    cx: &LateContext<'tcx>,
    bindings: impl Iterator<Item = (LocalDefId, Ident, Res)>,
    lint: &mut BuiltinTypeShadow,
    walk_scope: impl Fn(&mut OpaqueUses<'_, 'tcx>),
) {
    let mut bindings: Vec<_> = bindings.collect();
    // Prefer the type diagnostic for constructors that also bind a value of the same name.
    bindings.sort_unstable_by(|(a, an, ar), (b, bn, br)| {
        (a.local_def_index.as_u32(), an.name.as_str(), ar.ns()).cmp(&(
            b.local_def_index.as_u32(),
            bn.name.as_str(),
            br.ns(),
        ))
    });
    let mut missing_uses = None;
    let mut reported = None;
    for &(id, ident, res) in &bindings {
        let name = ident.name;
        if reported == Some((id, name)) {
            continue;
        }
        let mut span = cx.tcx.def_span(id);
        let ctxt = ident.span.ctxt();
        let hir_id = cx.tcx.local_def_id_to_hir_id(id);
        if !ctxt.is_root() && ctxt != cx.tcx.hir_span(hir_id).ctxt().normalize_to_macros_2_0() {
            // A macro at the destination can still refer to this opaque name. The glob map
            // identifies the import but stores only spellings. Check the resolved reference
            // too: a different binding, or even a macro, can use the same spelling.
            if !cx
                .tcx
                .resolutions(())
                .glob_map
                .get(&id)
                .is_some_and(|names| names.contains(&name))
            {
                continue;
            }
            let missing = missing_uses.get_or_insert_with(|| {
                let mut uses = OpaqueUses {
                    tcx: cx.tcx,
                    macro_uses: &lint.glob_macro_uses,
                    pending: bindings
                        .iter()
                        .filter(|(_, ident, _)| !ident.span.ctxt().is_root())
                        .map(|&(_, ident, res)| (ident, res))
                        .collect(),
                };
                walk_scope(&mut uses);
                uses.pending
            });
            if missing.contains(&(ident, res)) {
                continue;
            }
        }
        if span.is_empty() {
            // In `use m::{*}` the empty prefix ends immediately before the star.
            span = span.with_hi(span.hi() + BytePos(1));
        }
        if lint.check_res(cx, name, res, span, hir_id) {
            reported = Some((id, name));
        }
    }
}

fn glob_source(tree: &UseTree<'_>) -> Option<DefId> {
    match tree.prefix.res.type_ns {
        Some(Res::Def(DefKind::Mod | DefKind::Enum, source)) => Some(source),
        _ => None,
    }
}

fn binding_ident(tcx: TyCtxt<'_>, child: &ModChild) -> Option<Ident> {
    if PrimTy::from_name(child.ident.name).is_none() || child.res.ns() == Some(Namespace::MacroNS) {
        return None;
    }

    // ModChild stores the original identifier span. Replay the local globs from the
    // inside out to recover the context of the binding in its destination. Single
    // imports rename the binding, and rustc loads external metadata with its stored context.
    let mut ident = child.ident.normalize_to_macros_2_0();
    let chain = &child.reexport_chain;
    let count = chain
        .iter()
        .take_while(|link| matches!(link, Reexport::Glob(id) if id.is_local()))
        .count();
    for link in chain[..count].iter().rev() {
        let id = link.id()?.expect_local();
        let span = tcx.def_span(id);
        let mut ctxt = ident.span.ctxt();
        if ctxt.is_root() && span.ctxt().normalize_to_macros_2_0().is_root() {
            continue;
        }
        let tree = use_tree(tcx.hir_node_by_def_id(id))?;
        ctxt.reverse_glob_adjust(tcx.expn_that_defined(glob_source(tree)?), span)?;
        ident.span = ident.span.with_ctxt(ctxt);
    }
    Some(ident)
}

// Anonymous block scopes have no DefId. Rustc retains only the *used* names in its
// glob map, so combine the block's imports and declarations here, once per block.
// None means the name is ambiguous or shadowed by an explicit declaration.
type BlockBindings = FxIndexMap<(Ident, Namespace), Option<(LocalDefId, Res)>>;

// Each entry records a path that continues lookup at an opaque macro definition.
pub(super) type MacroUses = FxHashSet<(LocalDefId, Ident, Res)>;

struct OpaqueUses<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    macro_uses: &'a OnceLock<MacroUses>,
    pending: FxHashSet<(Ident, Res)>,
}

// Only membership in the pending set matters. Diagnostics follow the sorted bindings in report.
#[expect(rustc::potential_query_instability)]
impl<'tcx> Visitor<'tcx> for OpaqueUses<'_, 'tcx> {
    type NestedFilter = nested_filter::All;

    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_item(&mut self, item: &'tcx Item<'tcx>) {
        if self.pending.is_empty() {
            return;
        }
        if let ItemKind::Macro(_, def, _) = item.kind
            && !def.macro_rules
        {
            // An opaque macro can resolve names here even when invoked in another module.
            let uses = self.macro_uses.get_or_init(|| {
                let mut collector = DefSiteUses {
                    tcx: self.tcx,
                    uses: MacroUses::default(),
                    scopes: FxHashMap::default(),
                };
                self.tcx.hir_walk_toplevel_module(&mut collector);
                collector.uses
            });
            self.pending
                .retain(|&(ident, res)| !uses.contains(&(item.owner_id.def_id, ident, res)));
        } else {
            intravisit::walk_item(self, item);
        }
    }

    // Unqualified paths in child modules have their own module scope.
    fn visit_mod(&mut self, _: &'tcx Mod<'tcx>, _: Span, _: LocalModId) {}

    fn visit_block(&mut self, block: &'tcx Block<'tcx>) {
        if self.pending.is_empty() {
            return;
        }
        let names = block_bindings(self.tcx, block);
        let hidden: Vec<_> = self
            .pending
            .extract_if(|(ident, res)| res.ns().is_some_and(|ns| names.contains_key(&(*ident, ns))))
            .collect();
        if !self.pending.is_empty() {
            intravisit::walk_block(self, block);
        }
        self.pending.extend(hidden);
    }

    fn visit_use(&mut self, tree: &'tcx UseTree<'tcx>, id: HirId, _: LocalDefId) {
        walk_use_prefix(self, tree, id);
    }

    fn visit_path(&mut self, path: &Path<'tcx>, _: HirId) {
        if let Some(binding) = opaque_path(path) {
            self.pending.remove(&binding);
        }
        intravisit::walk_path(self, path);
    }
}

// A braced import may start with empty groups. Only the first nonempty
// prefix in each branch resolves lexically; its nested names are qualified.
fn walk_use_prefix<'tcx>(visitor: &mut impl Visitor<'tcx>, tree: &'tcx UseTree<'tcx>, id: HirId) {
    if tree.prefix.segments.is_empty() {
        intravisit::walk_use(visitor, tree, id);
    } else {
        let prefix = tree.prefix;
        for res in prefix.res.present_items() {
            visitor.visit_path(
                &Path {
                    segments: prefix.segments,
                    res,
                    span: prefix.span,
                },
                id,
            );
        }
    }
}

fn opaque_path(path: &Path<'_>) -> Option<(Ident, Res)> {
    let (first, rest) = path.segments.split_first()?;
    PrimTy::from_name(first.ident.name)?;
    let ident = first.ident.normalize_to_macros_2_0();
    let res = if rest.is_empty() { path.res } else { first.res };
    (!ident.span.ctxt().is_root()).then_some((ident, res))
}

struct DefSiteUses<'tcx> {
    tcx: TyCtxt<'tcx>,
    uses: MacroUses,
    // A deeply nested call can share most of its parent scopes with other calls.
    scopes: FxHashMap<HirId, FxHashSet<(Ident, Namespace)>>,
}

impl<'tcx> Visitor<'tcx> for DefSiteUses<'tcx> {
    type NestedFilter = nested_filter::All;

    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_use(&mut self, tree: &'tcx UseTree<'tcx>, id: HirId, _: LocalDefId) {
        walk_use_prefix(self, tree, id);
    }

    fn visit_path(&mut self, path: &Path<'tcx>, mut site: HirId) {
        if let Some((mut ident, res)) = opaque_path(path)
            && let Some(ns) = res.ns()
        {
            // Each extra opaque mark can redirect lookup to that macro's definition.
            // Stop when a binding with the current context already exists at the use.
            loop {
                let mut parent = ident;
                let mark = parent.span.remove_mark();
                if parent.span.ctxt().is_root() || !self.lookup_reaches_def_site(site, ident, ns) {
                    break;
                }
                let Some(def) = mark.expn_data().macro_def_id.and_then(DefId::as_local) else {
                    break;
                };
                self.uses.insert((def, parent, res));
                site = self.tcx.local_def_id_to_hir_id(def);
                ident = parent;
            }
        }
        intravisit::walk_path(self, path);
    }
}

impl DefSiteUses<'_> {
    fn lookup_reaches_def_site(&mut self, site: HirId, ident: Ident, ns: Namespace) -> bool {
        let tcx = self.tcx;
        for (id, node) in tcx.hir_parent_iter(site) {
            let names = match node {
                Node::Block(block) => self
                    .scopes
                    .entry(id)
                    .or_insert_with(|| block_bindings(tcx, block).into_keys().collect()),
                Node::Crate(_)
                | Node::Item(Item {
                    kind: ItemKind::Mod(..),
                    ..
                }) => {
                    // As in rustc's hygienic_lexical_parent, lookup only removes the
                    // macro's mark when it leaves a module outside that expansion.
                    if tcx
                        .expn_that_defined(id.owner.def_id)
                        .outer_expn_is_descendant_of(ident.span.ctxt())
                    {
                        return false;
                    }
                    let names = self.scopes.entry(id).or_insert_with(|| {
                        tcx.module_children_local(id.owner.def_id)
                            .iter()
                            .filter_map(|child| Some((binding_ident(tcx, child)?, child.res.ns()?)))
                            .collect()
                    });
                    return !names.contains(&(ident, ns));
                },
                _ => continue,
            };
            if names.contains(&(ident, ns)) {
                return false;
            }
        }
        false
    }
}

pub(super) fn check_block<'tcx>(cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>, lint: &mut BuiltinTypeShadow) {
    let bindings = block_bindings(cx.tcx, block);
    report(
        cx,
        bindings
            .into_iter()
            .filter_map(|((ident, _), binding)| binding.map(|(id, res)| (id, ident, res))),
        lint,
        |uses| intravisit::walk_block(uses, block),
    );
}

fn block_bindings(tcx: TyCtxt<'_>, block: &Block<'_>) -> BlockBindings {
    let mut bindings = BlockBindings::default();
    for stmt in block.stmts {
        let StmtKind::Item(id) = stmt.kind else { continue };
        let item = tcx.hir_item(id);
        match &item.kind {
            ItemKind::Use(tree) => block_import(tcx, block, tree, id.owner_id.def_id, &mut bindings),
            ItemKind::ForeignMod { items, .. } => {
                for foreign in *items {
                    let decl = tcx.hir_foreign_item(*foreign);
                    if PrimTy::from_name(decl.ident.name).is_some()
                        && let Some(ns) = tcx.def_kind(decl.owner_id).ns()
                    {
                        bindings.insert((decl.ident.normalize_to_macros_2_0(), ns), None);
                    }
                }
            },
            kind => {
                if let Some(ident) = kind.ident()
                    && PrimTy::from_name(ident.name).is_some()
                {
                    let ns = match kind {
                        ItemKind::ExternCrate(..) => Some(Namespace::TypeNS),
                        _ => tcx.def_kind(item.owner_id).ns(),
                    };
                    if let Some(ns) = ns {
                        bindings.insert((ident.normalize_to_macros_2_0(), ns), None);
                    }
                    if matches!(kind, ItemKind::Struct(_, _, data) if data.ctor().is_some()) {
                        bindings.insert((ident.normalize_to_macros_2_0(), Namespace::ValueNS), None);
                    }
                }
            },
        }
    }
    bindings
}

fn block_import(tcx: TyCtxt<'_>, block: &Block<'_>, tree: &UseTree<'_>, id: LocalDefId, bindings: &mut BlockBindings) {
    match tree.kind {
        UseKind::Nested { items } => {
            for (tree, _, id) in items {
                block_import(tcx, block, tree, *id, bindings);
            }
        },
        UseKind::Single(ident) => {
            if PrimTy::from_name(ident.name).is_some() {
                for ns in tree.prefix.res.present_items().filter_map(|res| res.ns()) {
                    bindings.insert((ident.normalize_to_macros_2_0(), ns), None);
                }
            }
        },
        UseKind::Glob => {
            let Some(source) = glob_source(tree) else { return };
            let module = tcx.parent_module(block.hir_id).to_mod_id();
            let mut import = |child: &ModChild, ambiguous| {
                let Some(mut ident) = binding_ident(tcx, child) else {
                    return;
                };
                let mut ctxt = ident.span.ctxt();
                let Some(scope) = ctxt.reverse_glob_adjust(tcx.expn_that_defined(source), tcx.def_span(id)) else {
                    return;
                };
                let scope = scope.and_then(|expn| expn.expn_data().parent_module).unwrap_or(module);
                if !child.vis.is_accessible_from(scope, tcx) {
                    return;
                }
                ident.span = ident.span.with_ctxt(ctxt);
                let res = child.res.expect_non_local();
                if let Some(ns) = res.ns() {
                    let binding = bindings.entry((ident, ns)).or_insert(Some((id, res)));
                    // Multiple globs of the same definition introduce one binding.
                    if ambiguous || binding.is_some_and(|(_, other)| res != other) {
                        *binding = None;
                    }
                }
            };
            if let Some(local) = source.as_local() {
                for child in tcx.module_children_local(local) {
                    import(child, false);
                }
                if let Some(children) = tcx.resolutions(()).ambig_module_children.get(&local) {
                    for child in children {
                        import(&child.main, true);
                    }
                }
            } else {
                for child in tcx.module_children(source) {
                    import(child, false);
                }
                for child in CStore::from_tcx(tcx).ambig_module_children_untracked(tcx, source) {
                    import(&child.main, true);
                }
            }
        },
    }
}
