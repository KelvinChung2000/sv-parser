//! Structured records of every compiler directive seen during
//! preprocessing.
//!
//! `sv-parser`'s preprocessor strips conditional directives
//! (`` `ifdef ``/`` `ifndef `` chains), expands macro call sites
//! (`` `my_macro(...) ``), and inlines `` `include `` files. Tools that
//! consume the post-preprocess text (e.g. formatters) lose direct
//! visibility into the directives that were rewritten or dropped.
//!
//! [`DirectiveSpan`] preserves enough metadata for those consumers to
//! reconstruct the directive layer: where every directive lived in the
//! original source, where (if anywhere) its expansion landed in the
//! post-preprocess text, and per-directive structure such as which
//! branches of an `` `ifdef `` chain were taken.
//!
//! The preprocessor records spans in [`super::preprocess::PreprocessedText`]
//! as it runs; access them via
//! [`super::preprocess::PreprocessedText::directives`].

use crate::range::Range;
use std::path::PathBuf;

/// A compiler directive recovered from preprocessing.
#[derive(Clone, Debug)]
pub struct DirectiveSpan {
    pub kind: DirectiveKind,
    /// File the directive was lexed from. Differs from the top-level
    /// source when the directive lives inside an `` `include ``d file.
    pub original_path: PathBuf,
    /// Byte range of the directive in `original_path`'s source.
    pub original_range: Range,
    /// Byte range in the post-preprocess text that this directive's
    /// resulting bytes occupy. Semantics by kind:
    ///
    /// * `Define`: covers the verbatim `` `define `` line in the
    ///   post-preprocess text.
    /// * `MacroUsage`: covers the expanded body bytes that replaced the
    ///   call site. `None` if the macro had no body (`` `define EMPTY ``).
    /// * `Include`: covers the inlined contents of the included file.
    /// * `IfdefChain`: always `None` — the conditional keywords are
    ///   stripped, and per-branch body coverage in pp text can be
    ///   recovered via the existing origin map keyed on
    ///   `body_original_range`.
    /// * Plain directives (`` `pragma ``, `` `timescale ``, …): covers the
    ///   directive line as kept verbatim in the post-pp text.
    pub pp_range: Option<Range>,
    pub detail: DirectiveDetail,
}

/// Coarse classification of a [`DirectiveSpan`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DirectiveKind {
    Define,
    Undef,
    UndefineAll,
    /// One full `` `ifdef `` … `` `endif `` chain (with all its
    /// `` `elsif `` / `` `else `` branches).
    IfdefChain,
    Include,
    /// A use site of a user macro (e.g. `` `my_assert(cond) ``).
    MacroUsage,
    Pragma,
    Line,
    /// `` `__FILE__ `` / `` `__LINE__ `` substitution.
    PositionMacro,
    Timescale,
    DefaultNettype,
    Resetall,
    /// `` `unconnected_drive `` or `` `nounconnected_drive ``.
    UnconnectedDrive,
    /// `` `celldefine `` or `` `endcelldefine ``.
    CellDefine,
    /// `` `begin_keywords `` or `` `end_keywords ``.
    Keywords,
}

/// Per-kind extra metadata. Kinds that need no extra metadata carry
/// [`DirectiveDetail::Plain`].
#[derive(Clone, Debug)]
pub enum DirectiveDetail {
    Define(MacroDef),
    MacroUsage(MacroUsage),
    Include(IncludeDirective),
    IfdefChain(IfdefChain),
    Plain,
}

/// `` `define IDENT(arg, …) body `` declaration.
#[derive(Clone, Debug)]
pub struct MacroDef {
    pub name: String,
    pub arguments: Vec<MacroDefArg>,
    /// Byte range of the macro body in `original_path`. `None` for a
    /// bodyless `` `define `` (e.g. `` `define DEBUG ``).
    pub body_original_range: Option<Range>,
}

#[derive(Clone, Debug)]
pub struct MacroDefArg {
    pub name: String,
    pub default: Option<String>,
}

/// `` `my_macro(actual0, actual1) `` call site.
#[derive(Clone, Debug)]
pub struct MacroUsage {
    pub name: String,
    /// Literal call-site text in the original source — same span as
    /// `DirectiveSpan.original_range`, reproduced for ergonomics.
    pub call_text: String,
}

#[derive(Clone, Debug)]
pub struct IncludeDirective {
    /// Resolved path of the included file. Empty if the include was a
    /// macro-resolved path that failed to expand or if `ignore_include`
    /// was set during preprocessing.
    pub included_path: PathBuf,
}

/// One full `` `ifdef `` … `` `endif `` chain.
#[derive(Clone, Debug)]
pub struct IfdefChain {
    pub branches: Vec<IfdefBranch>,
}

#[derive(Clone, Debug)]
pub struct IfdefBranch {
    pub kind: IfdefBranchKind,
    /// Byte range of the keyword (`` `ifdef ``/`` `ifndef ``/`` `elsif ``/
    /// `` `else ``) in `original_path`.
    pub keyword_original_range: Range,
    /// Condition identifier. `None` for `` `else ``.
    pub condition: Option<String>,
    /// Byte range of the branch body in `original_path`. `None` for an
    /// empty branch.
    pub body_original_range: Option<Range>,
    /// Whether the preprocessor selected this branch. The body's tokens
    /// are present in the post-preprocess text iff `taken == true`.
    pub taken: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IfdefBranchKind {
    Ifdef,
    Ifndef,
    Elsif,
    Else,
}
