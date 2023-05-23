//! A low level IR used for codegen.
//!
//! The following expression is lowered to the following IR:
//!
//! ```c
//! int i = 0;
//! long l = 1;
//! if (true) {
//!     i = 1;
//! } else {
//!     i = 2;
//! }
//! yeet(i);
//! ```
//!
//! ```c
//! bb0:
//!   %0 = alloca 4, 4
//!   store _0, 0
//!   %1 = alloca 8, 8
//!   store %1, 1
//!   branch true, bb1, bb2
//! bb1:
//!   store %0, 1
//!   branch bb3
//! bb2:
//!   store %0, 2
//!   branch bb3
//! bb3:
//!   %val = load %0
//!   call yeet(%val)
//! ```

mod pretty;
mod validate;

use std::fmt::{Debug, Display};

use parser::{Span, Symbol};
pub use pretty::{func_to_string, ir_to_string};
use rustc_hash::FxHashMap;
pub use validate::validate;

use crate::ty::Ty;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct TyLayout<'cx> {
    pub ty: Ty<'cx>,
    pub layout: &'cx Layout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Layout {
    pub size: u64,
    pub align: u64,
}

pub struct Ir<'cx> {
    pub funcs: FxHashMap<DefId, Func<'cx>>,
}

#[derive(Debug, Clone)]
pub struct Func<'cx> {
    pub regs: Vec<RegisterData<'cx>>,
    pub bbs: Vec<BasicBlock>,
    pub name: Symbol,
    pub def_span: Span,
    pub ret_ty: Ty<'cx>,
    /// The amount of function parameters. regs[..arity] are the parameters.
    pub arity: u32,
}

#[derive(Clone, Copy)]
pub struct BbIdx(pub u32);

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub term: Branch,
}

#[derive(Debug, Clone)]
pub struct RegisterData<'cx> {
    pub tyl: TyLayout<'cx>,
    pub name: Option<Symbol>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Register(pub u32);

#[derive(Debug, Clone)]
pub struct Statement {
    pub span: Span,
    pub kind: StatementKind,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Alloca {
        reg: Register,
        size: Operand,
        align: Operand,
    },
    Store {
        ptr_reg: Register,
        value: Operand,
        size: Operand,
        align: Operand,
    },
    Load {
        result: Register,
        ptr_reg: Register,
        size: Operand,
        align: Operand,
    },
    BinOp {
        kind: BinKind,
        lhs: Operand,
        rhs: Operand,
        result: Register,
    },
    PtrOffset {
        result: Register,
        reg: Register,
        amount: Operand,
    },
    Call {
        result: Register,
        func: Operand,
        args: Vec<Operand>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Operand {
    Reg(Register),
    Const(ConstValue),
}

#[derive(Debug, Clone)]
pub enum Branch {
    Goto(BbIdx),
    Switch {
        cond: Operand,
        yes: BbIdx,
        no: BbIdx,
    },
    Ret(Operand),
}

#[derive(Debug, Clone, Copy)]
pub enum BinKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Gt,
    Geq,
    Lt,
    Leq,
    Shl,
    Shr,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, Copy)]
pub enum ConstValue {
    Void,
    Int(u128),
}

impl Func<'_> {
    pub fn bb_mut(&mut self, i: BbIdx) -> &mut BasicBlock {
        &mut self.bbs[i.as_usize()]
    }
}

impl BbIdx {
    pub fn from_usize(n: usize) -> Self {
        Self(n.try_into().unwrap())
    }
    pub fn as_usize(self) -> usize {
        self.0 as _
    }
}

impl Debug for BbIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl Display for BbIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl Layout {
    pub fn size_align(size: u64, align: u64) -> Self {
        Self { size, align }
    }
}

impl ConstValue {
    pub fn u64(int: u64) -> Self {
        Self::Int(int.into())
    }
}

impl Operand {
    pub fn const_u64(int: u64) -> Self {
        Self::Const(ConstValue::u64(int))
    }
}

impl Branch {
    pub fn dummy() -> Self {
        Branch::Goto(BbIdx(u32::MAX))
    }
}
