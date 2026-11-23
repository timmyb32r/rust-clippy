use rustc_ast::ExprKind;
use rustc_data_structures::fx::{FxHashMap, FxIndexMap};
use rustc_data_structures::smallvec::{SmallVec, smallvec};
use rustc_errors::emitter::SilentEmitter;
use rustc_errors::{Diag, DiagCtxt};
use rustc_hir::def_id::{DefId, LocalDefId, LocalModId};
use rustc_hir::{HirId, ItemKind, ItemLocalId, Node, OwnerId};
use rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};
use rustc_middle::ty::TyCtxt;
use rustc_parse::lexer::StripTokens;
use rustc_session::parse::ParseSess;
use rustc_span::hygiene::{ExpnId, ExpnKind, HygieneRemapper as _, LocalExpnId, LocalExpnIdRemapper, MacroKind};
use rustc_span::{BytePos, DesugaringKind, Pos as _, SourceFile, Span, SyntaxContext};
use std::cell::LazyCell;
use std::sync::Arc;

use super::{Origin, expansion_origin, sym};

// An included expression keeps its original spans, and local bindings have no DefId.
// Find the slot in the surrounding syntax where this expression was substituted.
pub(super) fn enclosing_span(tcx: TyCtxt<'_>, binding: HirId, span: Span) -> Option<(HirId, Span)> {
    tcx.hir_parent_id_iter(binding)
        .take_while(|parent| parent.owner == binding.owner)
        .find_map(|parent| {
            let parent_span = tcx.hir_span_with_body(parent);
            let enclosing = match parent_span.desugaring_kind() {
                Some(DesugaringKind::ForLoop | DesugaringKind::QuestionMark | DesugaringKind::RangeExpr) => {
                    parent_span.ctxt().outer_expn_data().call_site
                },
                Some(DesugaringKind::FormatLiteral { .. }) => {
                    parent_span.ctxt().outer_expn_data().call_site.source_callsite()
                },
                Some(_) => return None,
                None => parent_span,
            };
            (!enclosing.is_dummy() && (enclosing.ctxt() != span.ctxt() || !enclosing.contains(span)))
                .then_some((parent, enclosing))
        })
}

#[derive(Default)]
pub(super) struct IncludeCalls {
    sites: Vec<CallSite>,
    module_sites: FxHashMap<(Span, LocalModId), Arc<[ExpnId]>>,
    nested: FxHashMap<ExpnId, Vec<ExpnId>>,
    groups: FxHashMap<HirId, Option<Origin>>,
    children: FxHashMap<OwnerId, ChildExpressions>,
}

// Re-including a file produces several expansions at exactly the same call site.
// Their enclosing macro invocations can still identify the original occurrence.
struct CallSite {
    span: Span,
    includes: Vec<ExpnId>,
    ancestors: Vec<(Span, ExpnId)>,
}

impl CallSite {
    fn new(span: Span, includes: Vec<ExpnId>) -> Self {
        let mut ancestors = Vec::new();
        if includes.len() > 1 {
            for &id in &includes {
                for call in invocation_spans(id) {
                    if call != span {
                        ancestors.push((call, id));
                    }
                }
            }
            ancestors.sort_unstable_by_key(|&(call, _)| (call.ctxt().as_u32(), call.lo()));
        }
        Self {
            span,
            includes,
            ancestors,
        }
    }

    // An include may have multiple source boundaries, e.g. a statement in one
    // included file containing a macro invocation in another. Start with the
    // smallest matching range so N owners using one shared file do not scan N calls each.
    fn occurrences(&self, enclosing: &[Span]) -> Vec<ExpnId> {
        let mut ranges = Vec::new();
        for &span in enclosing {
            let ctxt = span.ctxt().as_u32();
            let first = self
                .ancestors
                .partition_point(|(call, _)| (call.ctxt().as_u32(), call.lo()) < (ctxt, span.lo()));
            let last = self
                .ancestors
                .partition_point(|(call, _)| (call.ctxt().as_u32(), call.lo()) <= (ctxt, span.hi()));
            if first != last {
                ranges.push((span, &self.ancestors[first..last]));
            }
        }
        ranges.sort_unstable_by_key(|(_, range)| range.len());
        let mut candidates = Vec::new();
        for (span, range) in ranges {
            if candidates.is_empty() {
                candidates.extend(range.iter().filter(|(call, _)| span.contains(*call)).map(|&(_, id)| id));
            } else {
                let kept = candidates
                    .iter()
                    .copied()
                    .filter(|&id| invocation_spans(id).any(|call| span.ctxt() == call.ctxt() && span.contains(call)))
                    .collect::<Vec<_>>();
                if !kept.is_empty() {
                    candidates = kept;
                }
            }
            if candidates.len() == 1 {
                break;
            }
        }
        if candidates.is_empty() {
            self.includes.clone()
        } else {
            candidates
        }
    }
}

// An out-of-line module inherits the textual macro_rules! scope at its declaration.
// When two modules load the same file, the resolved definitions can tell identical
// invocation spans apart. Leave other forms of lookup to the ordinary scope checks.
fn index_module_sites(
    tcx: TyCtxt<'_>,
    site: Span,
    includes: &[ExpnId],
    sites: &mut FxHashMap<(Span, LocalModId), Arc<[ExpnId]>>,
) {
    if includes.len() < 2 || site.from_expansion() {
        return;
    }
    let mut scope = None;
    let mut definitions = FxHashMap::<LocalDefId, Vec<ExpnId>>::default();
    for &include in includes {
        let mut data = include.expn_data();
        while data.parent != ExpnId::root() {
            data = data.parent.expn_data();
        }
        let Some(def) = data.macro_def_id.and_then(DefId::as_local) else {
            return;
        };
        let Node::Item(item) = tcx.hir_node_by_def_id(def) else {
            return;
        };
        let ItemKind::Macro(ident, mac, _) = item.kind else {
            return;
        };
        let parent = tcx.parent_module_from_def_id(def);
        let binding = (parent, ident.name);
        if data.call_site != site
            || data.kind != ExpnKind::Macro(MacroKind::Bang, ident.name)
            || !mac.macro_rules
            || tcx.expn_that_defined(def) != ExpnId::root()
            || tcx.local_parent(def) != parent.to_local_def_id()
            || scope.replace(binding).is_some_and(|previous| previous != binding)
        {
            return;
        }
        definitions.entry(def).or_default().push(include);
    }
    let Some((parent, name)) = scope else { return };
    let mut visible = None;
    for &id in tcx.hir_get_module(parent).0.item_ids {
        let item = tcx.hir_item(id);
        match item.kind {
            ItemKind::Macro(ident, mac, _) if mac.macro_rules && ident.name == name => {
                visible = definitions.remove(&id.owner_id.def_id).map(Arc::<[_]>::from);
            },
            ItemKind::Mod(_, contents)
                if let Some(ref includes) = visible
                    && !item.span.contains(site)
                    && contents.spans.inner_span.contains(site) =>
            {
                sites.insert(
                    (site, LocalModId::new_unchecked(id.owner_id.def_id)),
                    Arc::clone(includes),
                );
            },
            _ => {},
        }
    }
}

fn invocation_spans(id: ExpnId) -> impl Iterator<Item = Span> {
    std::iter::successors(Some(id), |id| {
        let parent = id.expn_data().parent;
        (parent != ExpnId::root()).then_some(parent)
    })
    .flat_map(|id| std::iter::successors(Some(id.expn_data().call_site), |span| span.parent_callsite()))
}

// An identity macro can forward a block that contains another include. Its surviving
// expression separates that inner include from siblings in the enclosing array or tuple.
#[derive(Default)]
struct ChildExpressions {
    ranges: FxIndexMap<(ItemLocalId, SyntaxContext), Vec<(BytePos, BytePos)>>,
}

impl ChildExpressions {
    fn collect(tcx: TyCtxt<'_>, owner: OwnerId) -> Self {
        let mut children = Self::default();
        for entry in &tcx.hir_owner_nodes(owner).nodes {
            if let Node::Expr(expr) = entry.node {
                // A macro may wrap its argument in an expression from its template.
                // Both the template and the invocation then belong to this child.
                for span in std::iter::successors(Some(expr.span), |span| span.parent_callsite()) {
                    children
                        .ranges
                        .entry((entry.parent, span.ctxt()))
                        .or_default()
                        .push((span.lo(), span.hi()));
                }
            }
        }
        for ranges in children.ranges.values_mut() {
            ranges.sort_unstable();
            // A sibling with an earlier start may extend past the closest one.
            let mut end = BytePos(0);
            for (_, hi) in ranges {
                end = end.max(*hi);
                *hi = end;
            }
        }
        children
    }

    fn contains(&self, parent: ItemLocalId, site: Span) -> bool {
        self.ranges.get(&(parent, site.ctxt())).is_some_and(|ranges| {
            let end = ranges.partition_point(|&(lo, _)| lo <= site.lo());
            ranges[..end].last().is_some_and(|&(_, hi)| site.hi() <= hi)
        })
    }
}

impl IncludeCalls {
    pub(super) fn collect(tcx: TyCtxt<'_>) -> Self {
        let mut calls = Self::default();
        let Some(include) = tcx.get_diagnostic_item(sym::include_macro) else {
            return calls;
        };
        let psess = LazyCell::new(|| {
            ParseSess::with_dcx(
                DiagCtxt::new(Box::new(SilentEmitter)),
                tcx.sess.psess.clone_source_map(),
            )
        });
        let mut roots = FxHashMap::default();
        let mut sites = FxIndexMap::<Span, Vec<ExpnId>>::default();
        let count = LocalExpnIdRemapper::with(|remapper| remapper.len());
        for index in 1..count {
            let id = LocalExpnId::from_usize(index).to_expn_id();
            // Reserved expansion IDs can remain uninitialized after an error.
            if id.expn_hash().is_root() {
                continue;
            }
            let data = id.expn_data();
            if data.macro_def_id != Some(include) {
                continue;
            }
            // Keep the call site in each macro's syntax context. Two different
            // statements in a macro body can share the same outermost call site.
            for site in std::iter::successors(Some(data.call_site), |site| site.parent_callsite()) {
                sites.entry(site).or_default().push(id);
            }
            // An included file may consist entirely of another macro call. No HIR
            // node from that file then survives between the outer and inner includes.
            // The first invocation parsed in the outer include still identifies its file.
            let mut invocation = id;
            let mut parent = data.parent;
            while parent != ExpnId::root() {
                let enclosing = parent.expn_data();
                if enclosing.macro_def_id == Some(include) {
                    let site = invocation.expn_data().call_site;
                    if !site.is_dummy() && !site.from_expansion() {
                        let file = tcx.sess.source_map().lookup_source_file(site.lo());
                        let root = roots
                            .entry(file.start_pos)
                            .or_insert_with(|| root_macro_calls(&psess, &file, site).unwrap_or_default());
                        if root.contains(&site) {
                            calls.nested.entry(parent).or_default().push(id);
                        }
                    }
                    break;
                }
                // A passthrough attribute retains the original call in its input. A
                // replacing attribute is itself the invocation written in the included file.
                let forwarded = matches!(enclosing.kind, ExpnKind::Macro(MacroKind::Attr, _)) && {
                    let inner = invocation.expn_data();
                    if matches!(inner.kind, ExpnKind::Macro(MacroKind::Bang, _)) {
                        super::macro_input_span(tcx, invocation, parent, false).is_some()
                    } else {
                        !super::has_mark(inner.call_site, parent)
                    }
                };
                if !forwarded {
                    invocation = parent;
                }
                parent = enclosing.parent;
            }
        }
        calls.sites = sites
            .into_iter()
            .map(|(span, ids)| {
                index_module_sites(tcx, span, &ids, &mut calls.module_sites);
                CallSite::new(span, ids)
            })
            .collect();
        calls
            .sites
            .sort_unstable_by_key(|site| (site.span.ctxt().as_u32(), site.span.lo()));
        calls
    }

    pub(super) fn origin(
        &mut self,
        tcx: TyCtxt<'_>,
        (scope, enclosing): (HirId, Span),
        binding: Span,
        owner: ExpnId,
        matches_source: bool,
    ) -> Option<Origin> {
        // A surviving macro argument belongs to the caller even if the template
        // places a generated include next to it.
        let mut expansion = enclosing.ctxt().outer_expn();
        while expansion != ExpnId::root() && expansion != owner {
            let data = expansion.expn_data();
            if data.call_site.ctxt() == binding.ctxt()
                && data.call_site.contains(binding)
                && data.parent.is_descendant_of(owner)
            {
                return Some(expansion_origin(tcx, vec![data.parent], || {
                    matches_source.then_some(binding)
                }));
            }
            expansion = data.parent;
        }
        if let Some(&origin) = self.groups.get(&scope) {
            return origin;
        }
        let origin = self.group_origin(tcx, scope, enclosing, owner);
        self.groups.insert(scope, origin);
        origin
    }

    fn group_origin(&mut self, tcx: TyCtxt<'_>, scope: HirId, enclosing: Span, owner: ExpnId) -> Option<Origin> {
        let body = tcx.hir_span_with_body(scope.owner.into());
        let lowered = tcx.hir_span_with_body(scope);
        // Each expansion of a format string has its own arguments, even when
        // the same source file is included multiple times in this owner.
        let format_args = matches!(lowered.desugaring_kind(), Some(DesugaringKind::FormatLiteral { .. }))
            .then(|| lowered.ctxt().outer_expn_data().call_site.ctxt().outer_expn());
        let anchors = LazyCell::new(|| {
            std::iter::once(enclosing)
                .chain(
                    tcx.hir_parent_id_iter(scope)
                        .take_while(|id| id.owner == scope.owner)
                        .map(|id| tcx.hir_span_with_body(id)),
                )
                .flat_map(|span| std::iter::successors(Some(span), |span| span.parent_callsite()))
                .filter(|span| !span.is_dummy())
                .collect::<Vec<_>>()
        });
        let mut pending = Vec::new();
        // The immediate HIR parent may be an expression in a macro template.
        // Look in its arguments as well if the template contains no matching call.
        for span in std::iter::successors(Some(enclosing), |span| span.parent_callsite()) {
            let key = (span.ctxt().as_u32(), span.lo());
            let start = self
                .sites
                .partition_point(|site| (site.span.ctxt().as_u32(), site.span.lo()) < key);
            for site in self.sites[start..]
                .iter()
                .take_while(|site| site.span.ctxt() == span.ctxt() && site.span.lo() <= span.hi())
            {
                // A site projected to the entire invocation may come from a
                // sibling in the template. A passed argument lies inside its call.
                if span.contains(site.span) && (span == enclosing || site.span != span) {
                    if let [include] = &site.includes[..] {
                        pending.push(*include);
                    } else if let Some(includes) = self.module_sites.get(&(site.span, tcx.parent_module(scope))) {
                        pending.extend_from_slice(includes);
                    } else {
                        pending.extend(site.occurrences(&anchors));
                    }
                }
            }
            pending.retain(|&id| {
                format_args.is_none_or(|format| id.is_descendant_of(format)) && belongs_to_owner(id, owner, body)
            });
            if !pending.is_empty() {
                break;
            }
        }
        let mut result = None;
        while let Some(id) = pending.pop() {
            if let Some(children) = self.nested.get(&id) {
                let retained = self
                    .children
                    .entry(scope.owner)
                    .or_insert_with(|| ChildExpressions::collect(tcx, scope.owner));
                let before = pending.len();
                pending.extend(children.iter().copied().filter(|child| {
                    !std::iter::successors(Some(child.expn_data().call_site), |site| site.parent_callsite())
                        .any(|site| retained.contains(scope.local_id, site))
                }));
                if pending.len() != before {
                    continue;
                }
            }
            let origin = expansion_origin(tcx, vec![id], || None);
            if result.is_some_and(|previous| previous != origin) {
                return Some(Origin::Mixed);
            }
            result = Some(origin);
        }
        result
    }
}

// Two functions can include the same file and resolve a nested invocation differently.
// The outer call identifies which occurrence belongs to this HIR owner.
fn belongs_to_owner(mut id: ExpnId, owner: ExpnId, body: Span) -> bool {
    if owner != ExpnId::root() || body.from_expansion() {
        return id.is_descendant_of(owner);
    }
    loop {
        let data = id.expn_data();
        if data.parent == ExpnId::root() {
            return body.contains(data.call_site.source_callsite());
        }
        id = data.parent;
    }
}

// include! parses just the first expression, even if rustc reports an incomplete include.
// Parentheses and outer attributes belong to the expression, but not to its macro call.
fn root_macro_calls(psess: &ParseSess, file: &SourceFile, site: Span) -> Option<SmallVec<[Span; 1]>> {
    let source = file.src.as_deref()?;
    let start = file.relative_position(site.lo()).to_usize();
    let end = file.relative_position(site.hi()).to_usize();
    match first_token(source.get(..start)?) {
        Some(TokenKind::OpenBrace) => return None,
        None if matches!(first_token(source.get(end..)?), None | Some(TokenKind::Semi)) => {
            return Some(smallvec![site]);
        },
        _ => {},
    }

    // Parse against the original source map, with diagnostics and lint buffers isolated from the compilation.
    let mut parser =
        match rustc_parse::new_parser_from_source_str(psess, file.name.clone(), source.clone(), StripTokens::Nothing) {
            Ok(parser) => parser,
            Err(errors) => {
                for error in errors {
                    error.cancel();
                }
                return None;
            },
        };
    let expr = parser.parse_expr().map_err(Diag::cancel).ok()?;
    let mut calls = SmallVec::new();
    if let ExprKind::MacCall(call) = &expr.peel_parens().kind {
        calls.push(call.span());
    }
    // An attribute on any enclosing parentheses can replace the whole expression.
    let mut outer = &*expr;
    loop {
        calls.extend(outer.attrs.iter().map(|attr| attr.span));
        let ExprKind::Paren(inner) = &outer.kind else { break };
        outer = inner;
    }
    Some(calls)
}

fn first_token(source: &str) -> Option<TokenKind> {
    tokenize(source, FrontmatterAllowed::No)
        .map(|token| token.kind)
        .find(|token| {
            !matches!(
                token,
                TokenKind::Whitespace | TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }
            )
        })
}
