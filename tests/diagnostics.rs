use nexa_runtime::compiler;

#[test]
fn unknown_variable_reports_line_and_column() {
    let source = "fn main() {\n    print(missing);\n}";
    let error = compiler::compile(source).unwrap_err();

    assert!(error.contains("unknown variable 'missing'"), "{error}");
    assert!(error.contains("line 2, column 11"), "{error}");
}

#[test]
fn syntax_error_reports_line_and_column() {
    let source = "fn main() {\n    print(1;\n}";
    let error = compiler::compile(source).unwrap_err();

    assert!(error.contains("expected RParen"), "{error}");
    assert!(error.contains("line 2, column"), "{error}");
}

#[test]
fn lexer_error_reports_line_and_column() {
    let source = "fn main() {\n    @\n}";
    let error = compiler::compile(source).unwrap_err();

    assert!(error.contains("unexpected character '@'"), "{error}");
    assert!(error.contains("line 2, column 5"), "{error}");
}
