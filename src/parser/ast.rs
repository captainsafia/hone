#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    pub fn new(
        start: usize,
        end: usize,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> Self {
        Self {
            start,
            end,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    pub fn single_line(line: usize, start_col: usize, end_col: usize) -> Self {
        Self {
            start: 0,
            end: 0,
            start_line: line,
            start_col,
            end_line: line,
            end_col,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuoteType {
    Single,
    Double,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringLiteral {
    pub value: String,
    pub raw: String,
    pub quote_type: QuoteType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegexLiteral {
    pub pattern: String,
    pub flags: String,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DurationUnit {
    Milliseconds,
    Seconds,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Duration {
    pub value: f64,
    pub unit: DurationUnit,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputSelector {
    Stdout,
    StdoutRaw,
    Stderr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringComparisonOperator {
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputPredicate {
    Contains {
        value: StringLiteral,
    },
    Matches {
        value: RegexLiteral,
    },
    Equals {
        operator: StringComparisonOperator,
        value: StringLiteral,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExitCodePredicate {
    pub operator: StringComparisonOperator,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DurationPredicate {
    pub operator: ComparisonOperator,
    pub value: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilePredicate {
    Exists,
    Contains {
        value: StringLiteral,
    },
    Matches {
        value: RegexLiteral,
    },
    Equals {
        operator: StringComparisonOperator,
        value: StringLiteral,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssertionExpression {
    Output {
        target: Option<String>,
        selector: OutputSelector,
        predicate: OutputPredicate,
    },
    ExitCode {
        target: Option<String>,
        predicate: ExitCodePredicate,
    },
    Duration {
        target: Option<String>,
        predicate: DurationPredicate,
    },
    File {
        path: StringLiteral,
        predicate: FilePredicate,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PragmaType {
    Shell,
    Env,
    Timeout,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PragmaNode {
    pub pragma_type: PragmaType,
    pub key: Option<String>, // For env pragma
    pub value: String,
    pub line: usize,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommentNode {
    pub text: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestNode {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunNode {
    pub name: Option<String>,
    pub command: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssertNode {
    pub expression: AssertionExpression,
    pub line: usize,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvNode {
    pub key: String,
    pub value: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorNode {
    pub message: String,
    pub span: Span,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode {
    Pragma(PragmaNode),
    Comment(CommentNode),
    Test(TestNode),
    Run(RunNode),
    Assert(AssertNode),
    Env(EnvNode),
    Error(ErrorNode),
}

impl ASTNode {
    pub fn line(&self) -> usize {
        match self {
            ASTNode::Pragma(node) => node.line,
            ASTNode::Comment(node) => node.line,
            ASTNode::Test(node) => node.line,
            ASTNode::Run(node) => node.line,
            ASTNode::Assert(node) => node.line,
            ASTNode::Env(node) => node.line,
            ASTNode::Error(node) => node.span.start_line,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedFile {
    pub filename: String,
    pub pragmas: Vec<PragmaNode>,
    pub nodes: Vec<ASTNode>,
    pub warnings: Vec<ParseWarning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseWarning {
    pub message: String,
    pub line: usize,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseErrorDetail {
    pub message: String,
    pub line: usize,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseResult {
    Success {
        file: ParsedFile,
    },
    Failure {
        errors: Vec<ParseErrorDetail>,
        warnings: Vec<ParseWarning>,
    },
}

pub type HoneFile = ParsedFile;
