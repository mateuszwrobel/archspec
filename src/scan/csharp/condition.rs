//! Tri-state MSBuild condition evaluator (workplan `csharp_condition_eval`,
//! scenario 01). Pure functions over condition text and one consuming
//! project's file stem: no file IO, no XML, no walk integration. The design
//! text is cited, not restated — the adversarial record of 2026-11-18 §6:
//! B1 chain rule (`:250`), M1 case law (`:252`), M2 tri-state (`:253`),
//! M3 `Regex.IsMatch` dropped (`:254`), m2 unquoted operands (`:257`);
//! the F9 reserved-property fence (decision-check `:104-107`, workplan D3)
//! and the F10 engine-boundary note (workplan D7). Scenario 02 integrated
//! the module: the csharp csproj and props walks call `evaluate` per
//! `Condition` attribute, `choose_verdict`/`effective_arm_verdicts` per
//! `Choose` arm, and `combine_and` to fold scope verdicts onto an item.

use self::ConditionVerdict::{False, True, Unevaluated};

/// The three answers the evaluator emits — the whole verdict grammar of M2.
/// There is no fourth: a condition outside the subset is never coerced to
/// `False` and no property outside the reserved set expands to the empty
/// string (M2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionVerdict {
    True,
    False,
    Unevaluated,
}

/// The one reserved substitution (D3, F9): `$(MSBuildProjectName)` expands
/// to the consuming project's file stem. Every other property needs an
/// assignment-order model the engine does not carry, so every other property
/// answers `Unevaluated` — the fence, stated as a boundary law in scenario
/// 01 and decision-check F9.
const RESERVED_PROPERTY: &str = "MSBuildProjectName";

/// Per-consuming-project input the evaluator is allowed to see (M4: truth is
/// evaluated per consuming project). Carries exactly the reserved property's
/// value; no file IO, no cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalContext {
    project_stem: String,
}

impl EvalContext {
    pub fn new(project_stem: impl Into<String>) -> Self {
        Self {
            project_stem: project_stem.into(),
        }
    }

    /// Derives the context from the consuming project's file name: the stem
    /// (the name minus its extension) is the reserved property's value.
    pub fn from_project_file_name(file_name: &str) -> Self {
        let stem = std::path::Path::new(file_name)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_name.to_string());
        Self::new(stem)
    }

    pub fn project_stem(&self) -> &str {
        &self.project_stem
    }
}

/// Which arm of a `Choose` chain an entry describes (B1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmKind {
    When,
    Otherwise,
}

/// One arm of a `Choose` chain, collected in document order. A `When` arm's
/// `condition` carries the `Condition` attribute text; `None` models an arm
/// with no `Condition`, whose grammar is unconditional truth. An `Otherwise`
/// arm's text is ignored: its effective condition is the negation of every
/// `When` condition (B1), not an attribute of its own.
#[derive(Debug, Clone)]
pub struct ChooseArm<'a> {
    pub kind: ArmKind,
    pub condition: Option<&'a str>,
}

impl<'a> ChooseArm<'a> {
    pub fn when(condition: Option<&'a str>) -> Self {
        Self {
            kind: ArmKind::When,
            condition,
        }
    }

    pub fn otherwise() -> Self {
        Self {
            kind: ArmKind::Otherwise,
            condition: None,
        }
    }
}

/// Decides one condition string for one consuming project. Any shape the
/// subset does not cover — a reserved-set-external property (F9), a static
/// function call such as `Regex.IsMatch` (M3), a partial `And`/`Or` parse
/// (M2) — answers `Unevaluated`; this function never fails and never
/// guesses.
pub fn evaluate(condition_text: &str, ctx: &EvalContext) -> ConditionVerdict {
    let Some(tokens) = tokenize(condition_text) else {
        return Unevaluated;
    };
    let mut parser = Parser {
        tokens: &tokens,
        index: 0,
        ctx,
    };
    match parser.expression() {
        Ok(verdict) if parser.index == tokens.len() => verdict,
        _ => Unevaluated,
    }
}

/// Decides a `Choose` chain by the B1 rule: effective(`When_k`) = `C_k` AND
/// NOT `C_1` … NOT `C_{k-1}`; effective(`Otherwise`) = NOT `C_1` … NOT
/// `C_n`. The chain verdict is the disjunction of the effective verdicts:
/// first match wins, and an arm that is unevaluable while no earlier arm has
/// matched answers every later arm — and the `Otherwise` — through
/// `Unevaluated` (B1's veto to the diagnostic path).
pub fn choose_verdict(arms: &[ChooseArm<'_>], ctx: &EvalContext) -> ConditionVerdict {
    effective_arm_verdicts(arms, ctx)
        .into_iter()
        .fold(False, |chain, arm| match (chain, arm) {
            (True, _) | (_, True) => True,
            (Unevaluated, _) | (_, Unevaluated) => Unevaluated,
            _ => False,
        })
}

/// The B1 effective verdict of every arm, index-aligned with `arms`. A
/// proven-matched earlier arm makes every later effective verdict `False`
/// (the negation it carries is definite); an unevaluable earlier arm makes
/// every later effective verdict `Unevaluated` while no match has proven.
/// The scenario-02 walk calls this per opened arm on the chain prefix seen
/// so far (the rule depends on no later arm), so a streaming XML walk
/// consults the chain rule without restating it.
pub fn effective_arm_verdicts(arms: &[ChooseArm<'_>], ctx: &EvalContext) -> Vec<ConditionVerdict> {
    let mut matched = false;
    let mut vetoed = false;
    arms.iter()
        .map(|arm| {
            let own = match arm.kind {
                ArmKind::When => arm
                    .condition
                    .map_or(True, |condition| evaluate(condition, ctx)),
                // The Otherwise arm's own contribution is the empty
                // conjunction of negations — truth before the fold below.
                ArmKind::Otherwise => True,
            };
            let effective = if matched {
                False
            } else if vetoed {
                Unevaluated
            } else {
                own
            };
            match own {
                True => matched = true,
                Unevaluated => vetoed = true,
                False => {}
            }
            effective
        })
        .collect()
}

/// One resolved operand value: text computable for this consuming project,
/// or a structural parse that names nothing reservable (the F9 fence, the
/// M3 arm). An `Outside` operand poisons the whole condition.
enum Operand {
    Value(String),
    Outside,
}

#[derive(Clone, Debug, PartialEq)]
enum Token<'a> {
    LeftParen,
    RightParen,
    Equal,
    NotEqual,
    And,
    Or,
    Not,
    /// Quoted or unquoted literal text, with property substitutions still
    /// embedded (m2: an unquoted operand is the quoted one's equivalent).
    Literal(&'a str),
    /// Raw `$(...)` token text, trailing method calls included.
    Property(&'a str),
}

struct Parser<'a, 'c> {
    tokens: &'a [Token<'a>],
    index: usize,
    ctx: &'c EvalContext,
}

impl<'a, 'c> Parser<'a, 'c> {
    fn peek(&self) -> Option<Token<'a>> {
        self.tokens.get(self.index).cloned()
    }

    fn eat(&mut self, token: &Token<'a>) -> bool {
        if self.peek() == Some(token.clone()) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn expression(&mut self) -> Result<ConditionVerdict, ()> {
        self.or_expression()
    }

    fn or_expression(&mut self) -> Result<ConditionVerdict, ()> {
        let mut value = self.and_expression()?;
        while self.eat(&Token::Or) {
            value = combine_or(value, self.and_expression()?);
        }
        Ok(value)
    }

    fn and_expression(&mut self) -> Result<ConditionVerdict, ()> {
        let mut value = self.not_expression()?;
        while self.eat(&Token::And) {
            value = combine_and(value, self.not_expression()?);
        }
        Ok(value)
    }

    fn not_expression(&mut self) -> Result<ConditionVerdict, ()> {
        if self.eat(&Token::Not) {
            return Ok(negate(self.not_expression()?));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<ConditionVerdict, ()> {
        match self.peek() {
            Some(Token::LeftParen) => {
                self.index += 1;
                let value = self.or_expression()?;
                if !self.eat(&Token::RightParen) {
                    return Err(());
                }
                Ok(value)
            }
            Some(Token::Literal(_)) | Some(Token::Property(_)) => {
                let left = self.operand()?;
                match self.peek() {
                    Some(Token::Equal) => {
                        self.index += 1;
                        Ok(compare(left, self.operand()?, false))
                    }
                    Some(Token::NotEqual) => {
                        self.index += 1;
                        Ok(compare(left, self.operand()?, true))
                    }
                    _ => Ok(coerce_bool(left)),
                }
            }
            _ => Err(()),
        }
    }

    fn operand(&mut self) -> Result<Operand, ()> {
        match self.peek() {
            Some(Token::Literal(text)) => {
                self.index += 1;
                Ok(expand_literal(text, self.ctx))
            }
            Some(Token::Property(raw)) => {
                self.index += 1;
                Ok(resolve_property(raw, self.ctx))
            }
            _ => Err(()),
        }
    }
}

/// AND of two verdicts under the M2 posture: no Kleene shortcuts, so an
/// `Unevaluated` member poisons the conjunction whatever the other says.
/// The scenario-02 walk folds scope verdicts with this one implementation
/// (a scope is a conjunction of its open conditions and the item's own).
pub fn combine_and(left: ConditionVerdict, right: ConditionVerdict) -> ConditionVerdict {
    if left == Unevaluated || right == Unevaluated {
        Unevaluated
    } else if left == True && right == True {
        True
    } else {
        False
    }
}

fn combine_or(left: ConditionVerdict, right: ConditionVerdict) -> ConditionVerdict {
    if left == Unevaluated || right == Unevaluated {
        Unevaluated
    } else if left == True || right == True {
        True
    } else {
        False
    }
}

fn negate(value: ConditionVerdict) -> ConditionVerdict {
    match value {
        True => False,
        False => True,
        Unevaluated => Unevaluated,
    }
}

/// A lone value is a boolean literal or answers `Unevaluated` — a bare
/// name is never coerced to truth (M2).
fn coerce_bool(operand: Operand) -> ConditionVerdict {
    match operand {
        Operand::Value(text) if text.eq_ignore_ascii_case("true") => True,
        Operand::Value(text) if text.eq_ignore_ascii_case("false") => False,
        Operand::Value(_) | Operand::Outside => Unevaluated,
    }
}

/// `==`/`!=` compare case-INSENSITIVELY (M1); an operand the fence keeps out
/// answers the whole comparison `Unevaluated` — never equality against the
/// empty string (M2).
fn compare(left: Operand, right: Operand, negate_result: bool) -> ConditionVerdict {
    match (left, right) {
        (Operand::Value(left), Operand::Value(right)) => {
            let equal = left.eq_ignore_ascii_case(&right);
            if equal == !negate_result {
                True
            } else {
                False
            }
        }
        _ => Unevaluated,
    }
}

fn bool_text(value: bool) -> String {
    if value { "True".to_string() } else { "False".to_string() }
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+') || byte >= 0x80
}

fn tokenize(text: &str) -> Option<Vec<Token<'_>>> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            byte if byte.is_ascii_whitespace() => index += 1,
            b'(' => {
                tokens.push(Token::LeftParen);
                index += 1;
            }
            b')' => {
                tokens.push(Token::RightParen);
                index += 1;
            }
            b'=' if bytes.get(index + 1) == Some(&b'=') => {
                tokens.push(Token::Equal);
                index += 2;
            }
            b'!' if bytes.get(index + 1) == Some(&b'=') => {
                tokens.push(Token::NotEqual);
                index += 2;
            }
            quote @ (b'\'' | b'"') => {
                let close = bytes[index + 1..].iter().position(|&b| b == quote)? + index + 1;
                tokens.push(Token::Literal(text.get(index + 1..close)?));
                index = close + 1;
            }
            b'$' if bytes.get(index + 1) == Some(&b'(') => {
                let end = consume_property(text, index)?;
                tokens.push(Token::Property(text.get(index..end)?));
                index = end;
            }
            byte if is_word_byte(byte) => {
                let start = index;
                while index < bytes.len() && is_word_byte(bytes[index]) {
                    index += 1;
                }
                let word = text.get(start..index)?;
                if word.eq_ignore_ascii_case("and") {
                    tokens.push(Token::And);
                } else if word.eq_ignore_ascii_case("or") {
                    tokens.push(Token::Or);
                } else if word.eq_ignore_ascii_case("not") {
                    tokens.push(Token::Not);
                } else {
                    tokens.push(Token::Literal(word));
                }
            }
            _ => return None,
        }
    }
    Some(tokens)
}

/// Index just after the balanced close of the group opening at `open`.
/// Quote-aware and nesting-aware; all structural bytes are ASCII, so a
/// byte scan stays on UTF-8 boundaries.
fn balanced_end(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut quote: Option<u8> = None;
    let mut index = open;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(open_quote) = quote {
            if byte == open_quote {
                quote = None;
            }
        } else {
            match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(index + 1);
                    }
                }
                _ => {}
            }
        }
        index += 1;
    }
    None
}

/// Index just after a maximal property token: the `$(...)` group plus every
/// method call attached without whitespace.
fn consume_property(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut end = balanced_end(text, start + 1)?;
    while bytes.get(end) == Some(&b'.') {
        let mut probe = end + 1;
        while bytes
            .get(probe)
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        {
            probe += 1;
        }
        if bytes.get(probe) != Some(&b'(') {
            break;
        }
        end = balanced_end(text, probe)?;
    }
    Some(end)
}

/// Resolves a raw property token (first group, then any attached chain) to
/// an operand. Every shape outside the subset resolves `Outside` (F9, M3);
/// the subset never fails the parse with a guess.
fn resolve_property(raw: &str, ctx: &EvalContext) -> Operand {
    let bytes = raw.as_bytes();
    let Some(group_end) = balanced_end(raw, 1) else {
        return Operand::Outside;
    };
    let Some(inner) = raw.get(2..group_end - 1) else {
        return Operand::Outside;
    };
    let mut value = resolve_group(inner, ctx);
    let mut index = group_end;
    while bytes.get(index) == Some(&b'.') {
        let mut probe = index + 1;
        let name_start = probe;
        while bytes
            .get(probe)
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        {
            probe += 1;
        }
        let Some(name) = raw.get(name_start..probe) else {
            return Operand::Outside;
        };
        if bytes.get(probe) != Some(&b'(') {
            return Operand::Outside;
        }
        let Some(args_end) = balanced_end(raw, probe) else {
            return Operand::Outside;
        };
        let Some(args) = raw.get(probe + 1..args_end - 1) else {
            return Operand::Outside;
        };
        value = apply_method(value, name, args, ctx);
        index = args_end;
    }
    value
}

fn resolve_group(inner: &str, ctx: &EvalContext) -> Operand {
    let inner = inner.trim();
    if inner.is_empty() || inner.contains('[') {
        // `[Class]::Member(...)` static functions are never computed; the
        // `Regex.IsMatch` form rides this arm (M3).
        return Operand::Outside;
    }
    match inner.find('.') {
        None if inner.eq_ignore_ascii_case(RESERVED_PROPERTY) => {
            Operand::Value(ctx.project_stem.clone())
        }
        None => Operand::Outside,
        Some(dot) => {
            let name = inner[..dot].trim();
            if !name.eq_ignore_ascii_case(RESERVED_PROPERTY) {
                return Operand::Outside;
            }
            let rest = inner[dot + 1..].trim();
            let Some(open) = rest.find('(') else {
                return Operand::Outside;
            };
            if !rest.ends_with(')') {
                return Operand::Outside;
            }
            apply_method(
                Operand::Value(ctx.project_stem.clone()),
                rest[..open].trim(),
                &rest[open + 1..rest.len() - 1],
                ctx,
            )
        }
    }
}

fn apply_method(base: Operand, name: &str, args: &str, ctx: &EvalContext) -> Operand {
    // The comparison the methods perform is case-SENSITIVE (M1); method
    // names match at their exact spelling.
    let (base, argument) = match (base, one_argument(args, ctx)) {
        (Operand::Value(base), Operand::Value(argument)) => (base, argument),
        _ => return Operand::Outside,
    };
    let outcome = match name {
        "EndsWith" => Some(base.ends_with(argument.as_str())),
        "StartsWith" => Some(base.starts_with(argument.as_str())),
        "Equals" => Some(base == argument),
        _ => None,
    };
    match outcome {
        Some(outcome) => Operand::Value(bool_text(outcome)),
        None => Operand::Outside,
    }
}

/// The single-argument spelling only: a top-level comma, an unbalanced
/// group, or an empty spelling resolves `Outside`.
fn one_argument(args: &str, ctx: &EvalContext) -> Operand {
    let bytes = args.as_bytes();
    let mut depth = 0usize;
    let mut quote: Option<u8> = None;
    for &byte in bytes {
        if let Some(open_quote) = quote {
            if byte == open_quote {
                quote = None;
            }
        } else {
            match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'(' => depth += 1,
                b')' => match depth.checked_sub(1) {
                    Some(next) => depth = next,
                    None => return Operand::Outside,
                },
                b',' if depth == 0 => return Operand::Outside,
                _ => {}
            }
        }
    }
    if depth != 0 {
        return Operand::Outside;
    }
    let args = args.trim();
    let bytes = args.as_bytes();
    if bytes.len() >= 2 && matches!(bytes[0], b'\'' | b'"') && bytes.last() == bytes.first() {
        return expand_literal(&args[1..args.len() - 1], ctx);
    }
    if bytes.first() == Some(&b'(') {
        // A parenthesized argument spelling is beyond the subset.
        return Operand::Outside;
    }
    if bytes.first() == Some(&b'$') && bytes.get(1) == Some(&b'(') {
        if consume_property(args, 0) == Some(args.len()) {
            return resolve_property(args, ctx);
        }
        return Operand::Outside;
    }
    if args.is_empty() {
        return Operand::Outside;
    }
    expand_literal(args, ctx)
}

/// Literal text with embedded `$(...)` substitutions expanded. A
/// reserved-set-external substitution yields `Outside` — the expansion
/// never answers the empty string for a property it cannot compute (M2).
fn expand_literal(text: &str, ctx: &EvalContext) -> Operand {
    if !text.contains("$(") {
        return Operand::Value(text.to_string());
    }
    let bytes = text.as_bytes();
    let mut expanded = String::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'$' && bytes.get(index + 1) == Some(&b'(') {
            let Some(end) = consume_property(text, index) else {
                return Operand::Outside;
            };
            let Some(raw) = text.get(index..end) else {
                return Operand::Outside;
            };
            match resolve_property(raw, ctx) {
                Operand::Value(value) => expanded.push_str(&value),
                Operand::Outside => return Operand::Outside,
            }
            index = end;
        } else {
            let start = index;
            index += text[start..].chars().next().map_or(1, |c| c.len_utf8());
            expanded.push_str(&text[start..index]);
        }
    }
    Operand::Value(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EvalContext {
        EvalContext::new("App.Tests")
    }

    fn corpus() -> Vec<(&'static str, &'static str, ConditionVerdict)> {
        vec![
            // Choose-arm comparison shapes (issue #45 repro shapes, names
            // synthetic).
            ("'$(MSBuildProjectName)'=='App.Tests'", "App.Tests", True),
            ("'$(MSBuildProjectName)' != 'App.Core'", "App.Tests", True),
            ("'$(MSBuildProjectName)'=='App.Core'", "App.Tests", False),
            ("'$(MSBuildProjectName)' != 'App.Tests'", "App.Tests", False),
            // m2: unquoted operands parse as their quoted equivalents.
            ("$(MSBuildProjectName)==app.tests", "App.Tests", True),
            ("$(MSBuildProjectName)!=app.tests", "App.Tests", False),
            // Single-argument string-method spellings.
            ("$(MSBuildProjectName.EndsWith('.Tests'))", "App.Tests", True),
            ("$(MSBuildProjectName.StartsWith('App'))", "App.Tests", True),
            ("$(MSBuildProjectName.Equals('App.Tests'))", "App.Tests", True),
            ("$(MSBuildProjectName).EndsWith('.Tests')", "App.Tests", True),
            // Control constants.
            ("'true'", "App.Tests", True),
            ("'false'", "App.Tests", False),
            ("true", "App.Tests", True),
            ("false", "App.Tests", False),
            // Well-formed composites.
            (
                "'$(MSBuildProjectName)'=='Widget.Lib' Or $(MSBuildProjectName.EndsWith('.Tests'))",
                "App.Tests",
                True,
            ),
            (
                "'$(MSBuildProjectName)'=='App.Tests' And Not false",
                "App.Tests",
                True,
            ),
            ("Not $(MSBuildProjectName)=='App.Core'", "App.Tests", True),
            // Out-of-subset members answer Unevaluated, never a guess.
            ("'$(IsTestProject)'=='true'", "App.Tests", Unevaluated),
            ("$(Configuration.Contains('Debug'))", "App.Tests", Unevaluated),
            (
                "$([System.Text.RegularExpressions.Regex]::IsMatch($(MSBuildProjectName), '^App'))",
                "App.Tests",
                Unevaluated,
            ),
            ("'$(NoSuchProperty)'==''", "App.Tests", Unevaluated),
            ("$(Foo.EndsWith('.Tests'))", "App.Tests", Unevaluated),
            ("@(Compile)=='App.Tests'", "App.Tests", Unevaluated),
            ("'$(MSBuildProjectName)'==", "App.Tests", Unevaluated),
            ("", "App.Tests", Unevaluated),
            ("'$(MSBuildProjectName)'=='Widget.Lib'", "Widget.Lib", True),
        ]
    }

    #[test]
    fn tri_state_verdict_shape() {
        for (condition, stem, expected) in corpus() {
            let verdict = evaluate(condition, &EvalContext::new(stem));
            assert!(
                matches!(
                    verdict,
                    ConditionVerdict::True | ConditionVerdict::False | ConditionVerdict::Unevaluated
                ),
                "condition {condition:?} answered a verdict outside the three (M2)"
            );
            assert_eq!(verdict, expected, "condition {condition:?}");
        }
    }

    #[test]
    fn equals_and_not_equals_compare_case_insensitively_on_quoted_and_unquoted_operands() {
        // M1 case law + m2 unquoted operands (adversarial record :252, :257).
        assert_eq!(
            evaluate("'$(MSBuildProjectName)'=='app.tests'", &ctx()),
            True
        );
        assert_eq!(
            evaluate("'$(MSBuildProjectName)'!='APP.TESTS'", &ctx()),
            False
        );
        assert_eq!(evaluate("$(MSBuildProjectName)==app.tests", &ctx()), True);
        assert_eq!(evaluate("$(MSBuildProjectName)!=APP.TESTS", &ctx()), False);
        assert_eq!(
            evaluate("'$(MSBuildProjectName)' == 'widget.lib'", &ctx()),
            False
        );
    }

    #[test]
    fn string_methods_compare_case_sensitive() {
        // M1: the methods are String methods — case-SENSITIVE comparison.
        assert_eq!(
            evaluate("$(MSBuildProjectName.EndsWith('.tests'))", &ctx()),
            False
        );
        assert_eq!(
            evaluate("$(MSBuildProjectName.EndsWith('.Tests'))", &ctx()),
            True
        );
        assert_eq!(
            evaluate("$(MSBuildProjectName.StartsWith('app'))", &ctx()),
            False
        );
        assert_eq!(
            evaluate("$(MSBuildProjectName.Equals('app.tests') )", &ctx()),
            False
        );
        assert_eq!(
            evaluate("$(MSBuildProjectName.Equals('App.Tests'))", &ctx()),
            True
        );
    }

    #[test]
    fn choose_chain_first_match_wins() {
        // B1 (adversarial record :250): the first arm whose condition is
        // true is effective; no later arm — nor the Otherwise — can be.
        let arms = [
            ChooseArm::when(Some("'$(MSBuildProjectName)'=='App.Core'")),
            ChooseArm::when(Some("'$(MSBuildProjectName)'=='App.Tests'")),
            ChooseArm::when(Some("'$(MSBuildProjectName)'=='Widget.Lib'")),
            ChooseArm::otherwise(),
        ];
        assert_eq!(
            effective_arm_verdicts(&arms, &ctx()),
            vec![False, True, False, False]
        );
        assert_eq!(choose_verdict(&arms, &ctx()), True);
        // A proven match at the first arm leaves no later arm effective.
        let arms = [
            ChooseArm::when(Some("'$(MSBuildProjectName)'=='App.Tests'")),
            ChooseArm::when(Some("'$(NoSuchProperty)'==''")),
            ChooseArm::otherwise(),
        ];
        assert_eq!(effective_arm_verdicts(&arms, &ctx()), vec![True, False, False]);
        assert_eq!(choose_verdict(&arms, &ctx()), True);
        // No arm matches: only the Otherwise is effective.
        let arms = [
            ChooseArm::when(Some("'$(MSBuildProjectName)'=='App.Core'")),
            ChooseArm::when(Some("'$(MSBuildProjectName)'=='Widget.Lib'")),
            ChooseArm::otherwise(),
        ];
        assert_eq!(
            effective_arm_verdicts(&arms, &ctx()),
            vec![False, False, True]
        );
        assert_eq!(choose_verdict(&arms, &ctx()), True);
        // No arm matches and no Otherwise stands: the chain is False.
        let arms = [ChooseArm::when(Some("'$(MSBuildProjectName)'=='App.Core'"))];
        assert_eq!(choose_verdict(&arms, &ctx()), False);
    }

    #[test]
    fn unevaluable_arm_vetoes_later_arms_to_the_diagnostic() {
        // B1: an earlier arm whose condition is Unevaluated answers every
        // later arm — and the Otherwise — through Unevaluated.
        // B1 (adversarial record :250): an earlier arm that is unevaluable
        // answers every later arm — and the `Otherwise` — through
        // `Unevaluated` for as long as no arm has proven its match. A
        // proven-matched arm makes every later exclusion DEFINITE (it rests
        // on the match, not on the unknown condition), so the fold still
        // answers `Unevaluated` through the vetoed arm and the chain rides
        // the diagnostic path (B1's veto).
        let arms = [
            ChooseArm::when(Some("'$(IsTestProject)'=='true'")),
            ChooseArm::when(Some("'$(MSBuildProjectName)'=='App.Tests'")),
            ChooseArm::otherwise(),
        ];
        assert_eq!(
            effective_arm_verdicts(&arms, &ctx()),
            vec![Unevaluated, Unevaluated, False]
        );
        assert_eq!(choose_verdict(&arms, &ctx()), Unevaluated);
        // False arms before the unevaluable one carry no veto of their own:
        // the veto answers the later arms and the `Otherwise` (done-when
        // wording, verbatim).
        let arms = [
            ChooseArm::when(Some("'$(MSBuildProjectName)'=='App.Core'")),
            ChooseArm::when(Some("'$(IsTestProject)'=='true'")),
            ChooseArm::otherwise(),
        ];
        assert_eq!(
            effective_arm_verdicts(&arms, &ctx()),
            vec![False, Unevaluated, Unevaluated]
        );
        assert_eq!(choose_verdict(&arms, &ctx()), Unevaluated);
    }

    #[test]
    fn unknown_property_never_resolves_to_the_empty_string() {
        // M2 (:253): no expansion path answers the empty string for a
        // property outside the reserved set.
        assert_eq!(evaluate("'$(NoSuchProperty)'==''", &ctx()), Unevaluated);
        assert_eq!(evaluate("'$(NoSuchProperty)'!=''", &ctx()), Unevaluated);
        assert_eq!(evaluate("'$(NoSuchProperty)'", &ctx()), Unevaluated);
    }

    #[test]
    fn partial_and_or_parse_leaves_the_whole_condition_unevaluated() {
        // M2: no Kleene shortcuts — a poisoned or unparsable member makes
        // the WHOLE condition Unevaluated, whatever the other side says.
        assert_eq!(
            evaluate("'$(MSBuildProjectName)'=='App.Core' And @(Compile)", &ctx()),
            Unevaluated
        );
        assert_eq!(
            evaluate("'true' Or @('App.Tests')", &ctx()),
            Unevaluated
        );
        assert_eq!(
            evaluate("'$(MSBuildProjectName)'=='App.Core' And '$(IsTestProject)'=='true'", &ctx()),
            Unevaluated
        );
        assert_eq!(
            evaluate("'$(MSBuildProjectName)'=='App.Tests' And And 'true'", &ctx()),
            Unevaluated
        );
        assert_eq!(evaluate("'$(MSBuildProjectName)'=='App.Tests' junk", &ctx()), Unevaluated);
    }

    #[test]
    fn reserved_property_fence_holds_every_other_property_out() {
        // F9 boundary law (:104-107), workplan D3: the reserved set is
        // exactly the project-stem substitute — one name, matching at its
        // own spelling case-insensitively. Everything else answers
        // Unevaluated: convention properties, SDK properties, and any
        // name a tree's PropertyGroup could declare.
        assert_eq!(evaluate("$(IsTestProject)", &ctx()), Unevaluated);
        assert_eq!(evaluate("$(Configuration)", &ctx()), Unevaluated);
        assert_eq!(evaluate("$(AssemblyName)=='App.Tests'", &ctx()), Unevaluated);
        assert_eq!(evaluate("$(CustomProperty.EndsWith('.Tests'))", &ctx()), Unevaluated);
        assert_eq!(
            evaluate("$(msbuildprojectname)=='App.Tests'", &ctx()),
            True
        );
    }

    #[test]
    fn regex_ismatch_is_unevaluated() {
        // M3 (:254): regex spellings are dropped from the subset; the
        // static-function arm answers Unevaluated, never a guess.
        assert_eq!(
            evaluate(
                "$([System.Text.RegularExpressions.Regex]::IsMatch($(MSBuildProjectName), 'Tests$'))",
                &ctx()
            ),
            Unevaluated
        );
        assert_eq!(
            evaluate("'$(MSBuildProjectName)'=='App.Tests' And $([Regex]::IsMatch('a','b'))", &ctx()),
            Unevaluated
        );
    }

    #[test]
    fn engine_boundary_carries_no_vocabulary() {
        // F10 (decision-check :108-114) / workplan D7: the evaluator code
        // carries no application, framework, vendor or name-convention
        // vocabulary. The scan is source-level: the module body before its
        // test section — the corpus the tests carry is synthetic fixture
        // text, not engine vocabulary.
        let source = include_str!("condition.rs");
        let module = source
            .split("#[cfg(test)]")
            .next()
            .expect("the test section marker");
        for forbidden in [
            "Xunit",
            "xunit",
            "nunit",
            "NUnit",
            "MSTest",
            "Microsoft",
            "NuGet",
            "dotnet",
            ".NET",
            "App.Tests",
            "App.Core",
            "Widget.Lib",
            "IsTestProject",
            "Configuration",
            "AssemblyName",
            "Test",
            "Tests",
        ] {
            assert!(
                !module.contains(forbidden),
                "engine module carries vocabulary {forbidden:?} (F10 / D7)"
            );
        }
    }

    #[test]
    fn context_reads_the_project_file_stem() {
        assert_eq!(
            EvalContext::from_project_file_name("App.Tests.csproj").project_stem(),
            "App.Tests"
        );
        assert_eq!(
            EvalContext::from_project_file_name("Widget.Lib.csproj").project_stem(),
            "Widget.Lib"
        );
        assert_eq!(
            EvalContext::from_project_file_name("Widget").project_stem(),
            "Widget"
        );
    }
}
