//! Crate for macro-free variadic tuple metaprogramming.
//!
//! # Rationale
//!
//! As of writing this crate, Rust does not support variadic generics
//! and does not allow to reason about tuples in general.
//!
//! Most importantly, Rust does not allow one to generically
//! implement a trait for all tuples whose elements implement it.
//!
//! This crate attempts to fill the gap by providing a way
//! to recursively define traits for tuples.
//!
//! # Tuple lists
//!
//! Tuple `(A, B, C, D)` can be unambiguously mapped into recursive tuple `(A, (B, (C, (D, ()))))`.
//!
//! On each level it consists of a pair `(Head, Tail)`, where `Head` is tuple element and
//! `Tail` is a remainder of the list. For last element `Tail` is an empty list.
//!
//! Unlike regular flat tuples, such recursive tuples can be effectively reasoned about in Rust.
//!
//! This crate calls such structures "variadics" and provides a set of traits and macros
//! allowing one to conveniently work with them.
//!
//! # Acknowledgements
//!
//! This crate is based on [`tuple_list` by VFLashM](https://github.com/VFLashM/tuple_list)
//! ```text
//! MIT License
//!
//! Copyright (c) 2020 Valerii Lashmanov
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
//! ```

/// Macro creating a variadic tuple value from a list of expressions.
///
/// # Examples
///
/// Main use of this macro is to create variadic tuple values:
/// ```rust
/// use variadics::var_expr;
///
/// let list = var_expr!(10, false, "foo");
///
/// assert_eq!(list, (10, (false, ("foo", ()))),)
/// ```
///
/// It can also be used to unpack tuples:
/// ```
/// # use variadics::var_expr;
/// let var_expr!(a, b, c) = var_expr!(10, false, "foo");
///
/// assert_eq!(a, 10);
/// assert_eq!(b, false);
/// assert_eq!(c, "foo");
/// ```
#[macro_export]
macro_rules! var_expr {
    () => ( () );
    ($a:ident $(, $b:ident)* $(,)?)  => ( ($a, $crate::var_expr!($($b),*)) );
    ($a:expr  $(, $b:expr )* $(,)?)  => ( ($a, $crate::var_expr!($($b),*)) );
}

/// Macro for pattern-matching with variadic tuples. This is mainly used for function arguments,
/// but it can also be used in `match`, `if let ...`, and `let ... else` expressions.
///
/// Although it may somtimes be possible to use `var_expr!` in place of this macro, doing so may
/// cause confusing errors.
///
/// # Examples
///
/// ```rust
/// use variadics::{var_args, var_expr, var_type};
///
/// fn my_fn(var_args!(a, b, c): var_type!(usize, &str, bool)) {
///     println!("{} {} {}", a, b, c);
/// }
/// my_fn(var_expr!(12, "hello", false));
/// ```
///
/// ```rust
/// use variadics::{var_args, var_expr};
///
/// let val = var_expr!(true, Some("foo"), 2);
/// if let var_args!(true, Some(item), 0..=3) = val {
///     println!("{}", item);
/// } else {
///     unreachable!();
/// }
/// ```
///
/// ```rust
/// # use variadics::{var_expr, var_args};
/// match var_expr!(true, Some(100), 5) {
///     var_args!(false, _, _) => unreachable!(),
///     var_args!(true, None, _) => unreachable!(),
///     var_args!(true, Some(0..=10), _) => unreachable!(),
///     var_args!(true, Some(a), b) => println!("{} {}", a, b),
/// }
#[macro_export]
macro_rules! var_args {
    () => ( () );
    ($a:pat $(, $b:pat)* $(,)?)  => ( ($a, $crate::var_args!($($b),*)) );
}

/// Macro creating a variadic tuple type from a list of types.
///
/// `var_expr!` can be used to define simple types but will result in confusing errors for more
/// complex types. Use this macro, `var_type!` instead.
///
/// # Examples
///
/// ```rust
/// # use std::collections::HashMap;
/// use variadics::{var_expr, var_type};
///
/// // A simple variadic type. Although `var_expr!` would work in this case, it cannot handle
/// // more complex types i.e. ones with generics.
/// let list: var_type!(i32, bool, String) = Default::default();
///
/// // A more complex type:
/// let list: var_type!(
///     &'static str,
///     HashMap<i32, i32>,
///     <std::vec::Vec<bool> as IntoIterator>::Item,
/// ) = var_expr!("foo", HashMap::new(), false);
/// ```
///
/// Unfortunately, expressions and types cannot be handled using the same macro due to the
/// undefeated [bastion of the turbofish](https://github.com/rust-lang/rust/blob/7fd15f09008dd72f40d76a5bebb60e3991095a5f/src/test/ui/parser/bastion-of-the-turbofish.rs).
#[macro_export]
macro_rules! var_type {
    () => ( () );
    ($a:ty $(, $b:ty)* $(,)?)  => ( ($a, $crate::var_type!($($b),*)) );
}

/// This macro generates a basic variadic trait where each element must fulfill the trait clause.
/// Currently of limited usefulness.
///
/// ```rust
/// use variadics::{var_expr, variadic_trait};
///
/// variadic_trait! {
///     /// A variadic list of `Debug` items.
///     pub variadic<T> DebugList where T: std::fmt::Debug {}
/// }
///
/// let x = &var_expr!(1, "hello", 5.6);
/// let _: &dyn DebugList = x;
/// println!("{:?}", x);
/// ```
#[macro_export]
macro_rules! variadic_trait {
    (
        $( #[$( $attrs:tt )*] )*
        $vis:vis variadic<$item:ident> $name:ident $( $clause:tt )*
    ) => {
        $( #[$( $attrs )*] )*
        $vis trait $name: $crate::Variadic {}
        impl $name for () {}
        impl<$item, __Rest: $name> $name for ($item, __Rest) $( $clause )*
    };
}

pub trait Variadic {}
impl Variadic for () {}
impl<X, T> Variadic for (X, T) where T: Variadic {}

pub trait Extend<U>: Variadic
where
    U: Variadic,
{
    type Extended: Variadic;
    fn extend(self, input: U) -> Self::Extended;
}

impl<X, T, U> Extend<U> for (X, T)
where
    T: Variadic + Extend<U>,
    U: Variadic,
{
    type Extended = (X, <T as Extend<U>>::Extended);
    fn extend(self, input: U) -> Self::Extended {
        let (x, t) = self;
        (x, t.extend(input))
    }
}
impl<U> Extend<U> for ()
where
    U: Variadic,
{
    type Extended = U;
    fn extend(self, input: U) -> Self::Extended {
        input
    }
}

pub trait SplitPrefix<U>: Variadic
where
    U: Variadic,
{
    type Suffix: Variadic;
    fn split(self) -> (U, Self::Suffix);
}
impl<X, T, U> SplitPrefix<(X, U)> for (X, T)
where
    U: Variadic,
    T: SplitPrefix<U>,
{
    type Suffix = <T as SplitPrefix<U>>::Suffix;
    fn split(self) -> ((X, U), Self::Suffix) {
        let (x, t) = self;
        let (t, u) = t.split();
        ((x, t), u)
    }
}
impl<T> SplitPrefix<()> for T
where
    T: Variadic,
{
    type Suffix = T;
    fn split(self) -> ((), Self::Suffix) {
        ((), self)
    }
}

pub trait Split<U, V>: Variadic
where
    U: Variadic,
    V: Variadic,
{
    fn split(self) -> (U, V);
}
impl<X, T, U, V> Split<(X, U), V> for (X, T)
where
    T: Split<U, V>,
    U: Variadic,
    V: Variadic,
{
    fn split(self) -> ((X, U), V) {
        let (x, t) = self;
        let (u, v) = t.split();
        ((x, u), v)
    }
}
impl<T> Split<(), T> for T
where
    T: Variadic,
{
    fn split(self) -> ((), T) {
        ((), self)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    type MyList = var_type!(u8, u16, u32, u64);
    type MyPrefix = var_type!(u8, u16);

    type MySuffix = <MyList as SplitPrefix<MyPrefix>>::Suffix;

    #[allow(dead_code)]
    const _: MySuffix = var_expr!(0_u32, 0_u64);

    #[test]
    #[allow(clippy::let_unit_value)]
    fn test_basic_expr() {
        let _ = var_expr!();
        let _ = var_expr!(1);
        let _ = var_expr!(1, "b",);
        let _ = var_expr!("a",);
        let _ = var_expr!(false, true, 1 + 2);
    }
}
