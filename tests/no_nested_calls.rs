use std::fs;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprMethodCall};

#[derive(Default)]
struct NestedCallVisitor {
    violations: Vec<usize>,
}

impl<'ast> Visit<'ast> for NestedCallVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if !is_constructor(&node.func) {
            for argument in &node.args {
                if contains_call(argument) {
                    self.violations.push(argument.span().start().line);
                }
            }
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        for argument in &node.args {
            if contains_call(argument) {
                self.violations.push(argument.span().start().line);
            }
        }
        visit::visit_expr_method_call(self, node);
    }
}

fn contains_call(expression: &Expr) -> bool {
    match expression {
        Expr::Await(expression) => contains_call(&expression.base),
        Expr::Call(expression) => !is_constructor(&expression.func),
        Expr::MethodCall(_) => true,
        Expr::Group(expression) => contains_call(&expression.expr),
        Expr::Paren(expression) => contains_call(&expression.expr),
        Expr::Reference(expression) => contains_call(&expression.expr),
        Expr::Try(expression) => contains_call(&expression.expr),
        _ => false,
    }
}

fn is_constructor(function: &Expr) -> bool {
    let Expr::Path(function) = function else {
        return false;
    };
    function
        .path
        .segments
        .last()
        .and_then(|segment| segment.ident.to_string().chars().next())
        .is_some_and(char::is_uppercase)
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut directories = vec![directory.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).expect("read source directory") {
            let path = entry.expect("read source entry").path();
            if path.is_dir() {
                directories.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn function_calls_are_named_before_being_passed_to_other_calls() {
    let mut failures = Vec::new();
    for path in rust_files(Path::new("src")) {
        let source = fs::read_to_string(&path).expect("read source file");
        let syntax = syn::parse_file(&source).expect("parse source file");
        let mut visitor = NestedCallVisitor::default();
        visitor.visit_file(&syntax);
        for line in visitor.violations {
            failures.push(format!("{}:{line}", path.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "function calls must be assigned before being passed as arguments:\n{}",
        failures.join("\n")
    );
}
