mod builder;

use parser::{
    ast::{self, Atom, DeclAttr, Expr, ExternalDecl, Stmt, TranslationUnit, TypeSpecifier},
    Span, Symbol,
};
use rustc_hash::FxHashMap;

use self::builder::FuncBuilder;
use crate::{
    ir::{BinKind, ConstValue, Func, Ir, Layout, Operand, Register, TyLayout},
    ty::Ty,
    Error,
};

type Result<T, E = Error> = std::result::Result<T, E>;

struct LoweringCx {}

pub fn lower_translation_unit(ast: &TranslationUnit) -> Result<Ir, Error> {
    let mut lcx = LoweringCx {};

    for (decl, _) in ast {
        match decl {
            ExternalDecl::Decl(_) => todo!("decl is unsupported"),
            ExternalDecl::FunctionDef(def) => {
                let decl = def.decl.uwnrap_normal();
                let body = &def.body;
                let ret_ty = lower_ty(&decl.decl_spec.ty);
                lower_body(
                    &mut lcx,
                    body,
                    decl.init_declarators[0].1,
                    decl.init_declarators[0].0.declarator.decl.name().0,
                    ret_ty,
                )?;
            }
        }
    }

    todo!()
}

#[derive(Debug)]
struct FnLoweringCtxt {
    scopes: Vec<FxHashMap<Symbol, VariableInfo>>,
    build: FuncBuilder,
}

impl FnLoweringCtxt {
    fn resolve_ident(&self, ident: Symbol) -> Option<&VariableInfo> {
        self.scopes.iter().rev().find_map(|s| s.get(&ident))
    }

    fn lower_body(&mut self, body: &[(Stmt, Span)]) -> Result<()> {
        for (stmt, stmt_span) in body {
            self.lower_stmt(stmt, *stmt_span)?;
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt, stmt_span: Span) -> Result<()> {
        match stmt {
            Stmt::Decl(decl) => {
                let decl = decl.uwnrap_normal();
                let ty = lower_ty(&decl.decl_spec.ty);
                let decl_attr = decl.decl_spec.attrs;

                for (var, def_span) in &decl.init_declarators {
                    let tyl = layout_of(ty.clone());
                    let (name, _) = var.declarator.decl.name();
                    let ptr_to = self.build.alloca(&tyl.layout, Some(name), stmt_span);

                    let variable_info = VariableInfo {
                        def_span: *def_span,
                        ptr_to,
                        decl_attr,
                        tyl,
                    };
                    self.scopes.last_mut().unwrap().insert(name, variable_info);
                }
            }
            Stmt::Labeled { .. } => todo!("labels are not implemented"),
            Stmt::Compound(_) => todo!("blocks are not implemented"),
            Stmt::If {
                cond,
                then,
                otherwise,
            } => todo!(),
            Stmt::Switch => todo!(),
            Stmt::While { cond, body } => todo!(),
            Stmt::For {
                init_decl,
                init_expr,
                cond,
                post,
                body,
            } => todo!(),
            Stmt::Goto(_) => todo!(),
            Stmt::Continue => todo!(),
            Stmt::Break => todo!(),
            Stmt::Return(_) => todo!(),
            Stmt::Expr(ast::Expr::Binary(ast::ExprBinary {
                op: ast::BinaryOp::Assign(assign),
                lhs,
                rhs,
            })) => {
                if assign.is_some() {
                    todo!("assign operation");
                }
                let rhs = self.lower_expr(&rhs.0, rhs.1)?;
                let (Expr::Atom(ast::Atom::Ident((ident, ident_span))), _) = **lhs else {
                    todo!("complex assignments")
                };
                let Some(var) = self.resolve_ident(ident) else {
                    return Err(Error::new(format!("cannot find variable {ident}"), ident_span));
                };
                self.build.store(var.ptr_to, rhs, var.tyl.layout, stmt_span);
            }
            Stmt::Expr(expr) => {
                self.lower_expr(expr, stmt_span)?;
            }
        }

        Ok(())
    }

    fn lower_expr(&mut self, expr: &ast::Expr, span: Span) -> Result<Operand> {
        match expr {
            ast::Expr::Atom(Atom::Char(c)) => Ok(Operand::Const(ConstValue::Int((*c).into()))),
            ast::Expr::Atom(Atom::Int(i)) => Ok(Operand::Const(ConstValue::Int(*i as _))),
            ast::Expr::Atom(Atom::Float(_)) => todo!("no floats"),
            ast::Expr::Atom(Atom::Ident(_)) => todo!("no idents"),
            ast::Expr::Atom(Atom::String(_)) => todo!("no string literals"),
            ast::Expr::Unary(_) => todo!("no unaries"),
            ast::Expr::Binary(binary) => {
                let lhs = self.lower_expr(&binary.lhs.0, binary.lhs.1)?;
                let rhs = self.lower_expr(&binary.rhs.0, binary.rhs.1)?;
                let kind = match binary.op {
                    ast::BinaryOp::Arith(ast::ArithOpKind::Mul) => BinKind::Mul,
                    ast::BinaryOp::Arith(ast::ArithOpKind::Div) => BinKind::Div,
                    ast::BinaryOp::Arith(ast::ArithOpKind::Mod) => BinKind::Mod,
                    ast::BinaryOp::Arith(ast::ArithOpKind::Add) => BinKind::Add,
                    ast::BinaryOp::Arith(ast::ArithOpKind::Sub) => BinKind::Sub,
                    ast::BinaryOp::Arith(ast::ArithOpKind::Shl) => todo!("shl"),
                    ast::BinaryOp::Arith(ast::ArithOpKind::Shr) => todo!("shr"),
                    ast::BinaryOp::Arith(ast::ArithOpKind::BitAnd) => todo!("&"),
                    ast::BinaryOp::Arith(ast::ArithOpKind::BitXor) => todo!("^"),
                    ast::BinaryOp::Arith(ast::ArithOpKind::BitOr) => todo!("|"),
                    ast::BinaryOp::LogicalAnd => todo!("no logical or"),
                    ast::BinaryOp::LogicalOr => todo!("no logical and"),
                    ast::BinaryOp::Comparison(ast::ComparisonKind::Lt) => BinKind::Lt,
                    ast::BinaryOp::Comparison(ast::ComparisonKind::Gt) => BinKind::Gt,
                    ast::BinaryOp::Comparison(ast::ComparisonKind::LtEq) => BinKind::Leq,
                    ast::BinaryOp::Comparison(ast::ComparisonKind::GtEq) => BinKind::Geq,
                    ast::BinaryOp::Comparison(ast::ComparisonKind::Eq) => BinKind::Eq,
                    ast::BinaryOp::Comparison(ast::ComparisonKind::Neq) => BinKind::Neq,
                    ast::BinaryOp::Comma => todo!("no comma"),
                    ast::BinaryOp::Index => todo!("no index"),
                    ast::BinaryOp::Assign(_) => todo!("no assign"),
                };

                let reg = self.build.binary(kind, lhs, rhs, span, layout_of(Ty::Void));

                Ok(Operand::Reg(reg))
            }
            Expr::Postfix(postfix) => {
                let lhs = self.lower_expr(&postfix.lhs.0, postfix.lhs.1)?;
                match &postfix.op {
                    ast::PostfixOp::Call(args) => {
                        let args = args
                            .iter()
                            .map(|(arg, sp)| self.lower_expr(arg, *sp))
                            .collect::<Result<_, _>>()?;

                        let reg = self.build.call(layout_of(Ty::Void), lhs, args, span);
                        Ok(Operand::Reg(reg))
                    }
                    ast::PostfixOp::Member(_) => todo!("member expr"),
                    ast::PostfixOp::ArrowMember(_) => todo!("arrow member expr"),
                    ast::PostfixOp::Increment => {
                        todo!("gotta have lvalues")
                    }
                    ast::PostfixOp::Decrement => todo!(),
                }
            }
        }
    }
}

#[derive(Debug)]
struct VariableInfo {
    def_span: Span,
    tyl: TyLayout,
    ptr_to: Register,
    decl_attr: DeclAttr,
}

fn lower_body(
    // may be used later
    _lcx: &mut LoweringCx,
    body: &[(Stmt, Span)],
    def_span: Span,
    name: Symbol,
    ret_ty: Ty,
) -> Result<Func, Error> {
    let mut cx: FnLoweringCtxt = FnLoweringCtxt {
        scopes: vec![FxHashMap::default()],
        build: FuncBuilder::new(name, def_span, ret_ty),
    };

    cx.lower_body(body)?;

    Ok(cx.build.finish())
}

fn lower_ty(ty: &TypeSpecifier) -> Ty {
    match ty {
        TypeSpecifier::Void => Ty::Void,
        TypeSpecifier::Char => Ty::Char,
        TypeSpecifier::SChar => Ty::SChar,
        TypeSpecifier::UChar => Ty::UChar,
        TypeSpecifier::Integer(int) => Ty::Integer(*int),
        TypeSpecifier::Float => Ty::Float,
        TypeSpecifier::Double => Ty::Double,
        TypeSpecifier::LongDouble => Ty::LongDouble,
        TypeSpecifier::Bool => Ty::Bool,
    }
}

fn layout_of(ty: Ty) -> TyLayout {
    let layout = match ty {
        Ty::Void => Layout::size_align(0, 1),
        Ty::Char => Layout::size_align(1, 1),
        Ty::SChar => Layout::size_align(1, 1),
        Ty::UChar => Layout::size_align(1, 1),
        Ty::Integer(int) => match int.kind {
            parser::ast::IntTyKind::Short => Layout::size_align(2, 2),
            parser::ast::IntTyKind::Int => Layout::size_align(4, 4),
            parser::ast::IntTyKind::Long => Layout::size_align(8, 8),
            parser::ast::IntTyKind::LongLong => Layout::size_align(8, 8),
        },
        Ty::Float => Layout::size_align(4, 4),
        Ty::Double => Layout::size_align(8, 8),
        Ty::LongDouble => Layout::size_align(8, 8),
        Ty::Bool => Layout::size_align(1, 1),
        Ty::Struct(_) => todo!("layout_of struct"),
        Ty::Union(_) => todo!("layout_of union"),
        Ty::Enum(_) => todo!("layout_of enum"),
        Ty::Ptr(_) => Layout::size_align(8, 8),
    };
    TyLayout { ty, layout }
}
