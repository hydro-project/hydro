#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
//! &nbsp;
//!
//! # Main Macros
//!
//! ## [`var_expr!`]
#![doc = include_str!("../var_expr.md")]
//! ## [`var_type!`]
#![doc = include_str!("../var_type.md")]
//! ## [`var_args!`]
#![doc = include_str!("../var_args.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;

use core::any::Any;

use sealed::sealed;
/// module of collection types for variadics
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[cfg(feature = "std")]
pub mod variadic_collections;

#[doc = include_str!("../var_expr.md")]
#[macro_export]
macro_rules! var_expr {
    () => ( () );

    (...$a:ident $(,)? ) => ( $a );
    (...$a:expr  $(,)? ) => ( $a );
    (...$a:ident, $( $b:tt )+) => ( $crate::VariadicExt::extend($a, $crate::var_expr!( $( $b )* )) );
    (...$a:expr,  $( $b:tt )+) => ( $crate::VariadicExt::extend($a, $crate::var_expr!( $( $b )* )) );

    ($a:ident $(,)? ) => ( ($a, ()) );
    ($a:expr  $(,)? ) => ( ($a, ()) );
    ($a:ident, $( $b:tt )+) => ( ($a, $crate::var_expr!( $( $b )* )) );
    ($a:expr,  $( $b:tt )+) => ( ($a, $crate::var_expr!( $( $b )* )) );
}

#[doc = include_str!("../var_type.md")]
#[macro_export]
macro_rules! var_type {
    () => ( () );

    (...$a:ty $(,)? ) => ( $a );
    (...$a:ty, $( $b:tt )+) => ( <$a as $crate::VariadicExt>::Extend::<$crate::var_type!( $( $b )* )> );

    ($a:ty $(,)? ) => ( ($a, ()) );
    ($a:ty, $( $b:tt )+) => ( ($a, $crate::var_type!( $( $b )* )) );
}

#[doc = include_str!("../var_args.md")]
#[macro_export]
macro_rules! var_args {
    () => ( () );

    (...$a:pat $(,)? ) => ( $a );
    (...$a:ty, $( $b:tt )+) => ( ::core::compile_error!("`var_args!` can only have the `...` spread syntax on the last field.") );

    ($a:pat $(,)? ) => ( ($a, ()) );
    ($a:pat, $( $b:tt )+) => ( ($a, $crate::var_args!( $( $b )* )) );
}

/// This macro generates a basic variadic trait where each element must fulfill the `where` clause.
///
/// ```rust
/// use variadics::{var_expr, variadic_trait};
///
/// variadic_trait! {
///     /// A variadic list of `Debug` items.
///     pub variadic<Item> DebugList where Item: core::fmt::Debug {}
/// }
///
/// let x = &var_expr!(1, "hello", 5.6);
/// let _: &dyn DebugList = x;
/// println!("{:?}", x);
/// ```
///
/// This uses a special syntax similar to traits, but with the `trait` keyword replaced with
/// `variadic<T>` where `T` is the generic parameter name for each item in the variadic list. `T`
/// can be changed to any valid generic identifier. The bounds on `T` must be put in the where
/// clause; they cannot be expressed directly-- `variadic<T: Clone>` is invalid.
///
/// For now this can only create traits which bounds the `Item`s and cannot have associated
/// methods. This means the body of the variadic trait must be empty. But in the future this
/// declarative macro may be converted into a more powerful procedural macro with associated
/// method support.
#[macro_export]
macro_rules! variadic_trait {
    (
        $( #[$( $attrs:tt )*] )*
        $vis:vis variadic<$item:ident> $name:ident $( $clause:tt )*
    ) => {
        $( #[$( $attrs )*] )*
        $vis trait $name: $crate::Variadic {}
        $( #[$( $attrs )*] )*
        impl $name for $crate::var_type!() {}
        $( #[$( $attrs )*] )*
        impl<$item, __Rest: $name> $name for $crate::var_type!($item, ...__Rest) $( $clause )*
    };
}

/// A variadic tuple list.
///
/// This is a sealed trait, implemented only for `(Item, Rest) where Rest: Variadic` and `()`.
#[sealed]
pub trait Variadic {}
#[sealed]
impl<Item, Rest> Variadic for (Item, Rest) where Rest: Variadic {}
#[sealed]
impl Variadic for () {}

/// Extension methods/types for [`Variadic`]s.
///
/// This is a sealed trait.
#[sealed]
pub trait VariadicExt: Variadic {
    /// The number of items in this variadic (its length).
    const LEN: usize;

    /// Creates a new (longer) variadic type by appending `Suffix` onto the end of this variadc.
    type Extend<Suffix>: VariadicExt
    where
        Suffix: VariadicExt;
    /// Extends this variadic value by appending `suffix` onto the end.
    fn extend<Suffix>(self, suffix: Suffix) -> Self::Extend<Suffix>
    where
        Suffix: VariadicExt;

    /// The reverse of this variadic type.
    type Reverse: VariadicExt;
    /// Reverses this variadic value.
    fn reverse(self) -> Self::Reverse;
    /// Reverses an AsRefVar variadic value
    fn reverse_ref(this: Self::AsRefVar<'_>) -> <Self::Reverse as VariadicExt>::AsRefVar<'_>;

    /// The length of this variadic type
    fn len(&self) -> usize {
        Self::LEN
    }

    /// Checks if this variadic type is empty.
    fn is_empty(&self) -> bool {
        Self::LEN == 0
    }

    /// This as a variadic of references.
    type AsRefVar<'a>: RefVariadic<UnRefVar = Self, RefVar = Self::AsRefVar<'a>, MutVar = Self::AsMutVar<'a>>
    where
        Self: 'a;
    /// Convert a reference to this variadic into a variadic of references.
    /// ```rust
    /// # use variadics::*;
    /// let as_ref: var_type!(&u32, &String, &bool) =
    ///     var_expr!(1_u32, "Hello".to_owned(), false).as_ref_var();
    /// ```
    fn as_ref_var(&self) -> Self::AsRefVar<'_>;

    /// This as a variadic of exclusive (`mut`) references.
    type AsMutVar<'a>: MutVariadic<UnRefVar = Self, RefVar = Self::AsRefVar<'a>, MutVar = Self::AsMutVar<'a>>
    where
        Self: 'a;
    /// Convert an exclusive (`mut`) reference to this variadic into a variadic of exclusive
    /// (`mut`) references.
    /// ```rust
    /// # use variadics::*;
    /// let as_mut: var_type!(&mut u32, &mut String, &mut bool) =
    ///     var_expr!(1_u32, "Hello".to_owned(), false).as_mut_var();
    /// ```
    fn as_mut_var(&mut self) -> Self::AsMutVar<'_>;

    /// Iterator type returned by [`Self::iter_any_ref`].
    type IterAnyRef<'a>: Iterator<Item = &'a dyn Any>
    where
        Self: 'static;
    /// Iterate this variadic as `&dyn Any` references.
    fn iter_any_ref(&self) -> Self::IterAnyRef<'_>
    where
        Self: 'static;

    /// Iterator type returned by [`Self::iter_any_mut`].
    type IterAnyMut<'a>: Iterator<Item = &'a mut dyn Any>
    where
        Self: 'static;
    /// Iterate this variadic as `&mut dyn Any` exclusive references.
    fn iter_any_mut(&mut self) -> Self::IterAnyMut<'_>
    where
        Self: 'static;

    /// type for all elements of the variadic being wrapped in `Option`
    type IntoOption;
    /// wrap all elements of the variadic in `Option``
    fn into_option(self) -> Self::IntoOption;

    /// type for all elements of the variadic being wrapped in `Vec`
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    #[cfg(feature = "std")]
    type IntoVec: VecVariadic<UnVec = Self> + Default;
    /// wrap all elements of the variadic in a `Vec`
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    #[cfg(feature = "std")]
    fn into_singleton_vec(self) -> Self::IntoVec;
}

#[sealed]
impl<Item, Rest> VariadicExt for (Item, Rest)
where
    Rest: VariadicExt,
{
    const LEN: usize = 1 + Rest::LEN;

    type Extend<Suffix>
        = (Item, Rest::Extend<Suffix>)
    where
        Suffix: VariadicExt;
    fn extend<Suffix>(self, suffix: Suffix) -> Self::Extend<Suffix>
    where
        Suffix: VariadicExt,
    {
        let (item, rest) = self;
        (item, rest.extend(suffix))
    }

    type Reverse = <Rest::Reverse as VariadicExt>::Extend<(Item, ())>;
    fn reverse(self) -> Self::Reverse {
        let (item, rest) = self;
        rest.reverse().extend((item, ()))
    }
    fn reverse_ref(this: Self::AsRefVar<'_>) -> <Self::Reverse as VariadicExt>::AsRefVar<'_> {
        let (item, rest) = this;
        let out = Rest::reverse_ref(rest).extend((item, ()));
        // TODO(mingwei): check if use of unsafe is necessary
        let out2 = unsafe { core::mem::transmute_copy(&out) };
        core::mem::forget(out);
        out2
    }

    type AsRefVar<'a>
        = (&'a Item, Rest::AsRefVar<'a>)
    where
        Self: 'a;
    fn as_ref_var(&self) -> Self::AsRefVar<'_> {
        let (item, rest) = self;
        (item, rest.as_ref_var())
    }

    type AsMutVar<'a>
        = (&'a mut Item, Rest::AsMutVar<'a>)
    where
        Self: 'a;
    fn as_mut_var(&mut self) -> Self::AsMutVar<'_> {
        let (item, rest) = self;
        (item, rest.as_mut_var())
    }

    type IterAnyRef<'a>
        = core::iter::Chain<core::iter::Once<&'a dyn Any>, Rest::IterAnyRef<'a>>
    where
        Self: 'static;
    fn iter_any_ref(&self) -> Self::IterAnyRef<'_>
    where
        Self: 'static,
    {
        let var_args!(item, ...rest) = self;
        let item: &dyn Any = item;
        core::iter::once(item).chain(rest.iter_any_ref())
    }

    type IterAnyMut<'a>
        = core::iter::Chain<core::iter::Once<&'a mut dyn Any>, Rest::IterAnyMut<'a>>
    where
        Self: 'static;
    fn iter_any_mut(&mut self) -> Self::IterAnyMut<'_>
    where
        Self: 'static,
    {
        let var_args!(item, ...rest) = self;
        let item: &mut dyn Any = item;
        core::iter::once(item).chain(rest.iter_any_mut())
    }

    type IntoOption = (Option<Item>, Rest::IntoOption);
    fn into_option(self) -> Self::IntoOption {
        let var_args!(item, ...rest) = self;
        var_expr!(Some(item), ...rest.into_option())
    }

    type IntoVec = (Vec<Item>, Rest::IntoVec);
    fn into_singleton_vec(self) -> Self::IntoVec {
        let var_args!(item, ...rest) = self;
        var_expr!(vec!(item), ...rest.into_singleton_vec())
    }
}

#[sealed]
impl VariadicExt for () {
    const LEN: usize = 0;

    type Extend<Suffix>
        = Suffix
    where
        Suffix: VariadicExt;
    fn extend<Suffix>(self, suffix: Suffix) -> Self::Extend<Suffix>
    where
        Suffix: VariadicExt,
    {
        suffix
    }

    type Reverse = ();
    fn reverse(self) -> Self::Reverse {}
    fn reverse_ref(_this: Self::AsRefVar<'_>) -> <Self::Reverse as VariadicExt>::AsRefVar<'_> {}

    type AsRefVar<'a> = ();
    fn as_ref_var(&self) -> Self::AsRefVar<'_> {}

    type AsMutVar<'a> = ();
    fn as_mut_var(&mut self) -> Self::AsMutVar<'_> {}

    type IterAnyRef<'a>
        = core::iter::Empty<&'a dyn Any>
    where
        Self: 'static;
    fn iter_any_ref(&self) -> Self::IterAnyRef<'_>
    where
        Self: 'static,
    {
        core::iter::empty()
    }

    type IterAnyMut<'a>
        = core::iter::Empty<&'a mut dyn Any>
    where
        Self: 'static;
    fn iter_any_mut(&mut self) -> Self::IterAnyMut<'_>
    where
        Self: 'static,
    {
        core::iter::empty()
    }

    type IntoOption = ();
    fn into_option(self) -> Self::IntoOption {}

    type IntoVec = ();
    fn into_singleton_vec(self) -> Self::IntoVec {}
}

/// A variadic of either shared references, exclusive references, or both.
///
/// Provides the [`Self::UnRefVar`] associated type which is the original variadic of owned values.
///
/// This is a sealed trait.
#[sealed]
pub trait EitherRefVariadic: VariadicExt {
    /// The un-referenced variadic. Each item will have one layer of shared references removed.
    ///
    /// The inverse of [`VariadicExt::AsRefVar`] and [`VariadicExt::AsMutVar`].
    ///
    /// ```rust
    /// # use variadics::*;
    /// let un_ref: <var_type!(&u32, &String, &bool) as EitherRefVariadic>::UnRefVar =
    ///     var_expr!(1_u32, "Hello".to_owned(), false);
    /// ```
    type UnRefVar: VariadicExt;

    /// This type with all exclusive `&mut` references replaced with shared `&` references.
    ///
    /// Returned by [`Self::mut_to_ref`].
    type RefVar: RefVariadic<UnRefVar = Self::UnRefVar, RefVar = Self::RefVar>;
    /// Convert all exclusive (`mut`) references into shared references: [`RefVariadic`].
    ///
    /// ```rust
    /// # use variadics::*;
    /// let mut original = var_expr!(1_u32, "Hello".to_owned(), false);
    /// let as_mut: var_type!(&mut u32, &mut String, &mut bool) = original.as_mut_var();
    /// let as_ref_1: var_type!(&u32, &String, &bool) = as_mut.mut_to_ref();
    /// let as_ref_2: var_type!(&u32, &String, &bool) = as_ref_1; // Can copy the reference version.
    /// drop((as_ref_1, as_ref_2));
    /// ```
    fn mut_to_ref(self) -> Self::RefVar;

    /// This type with all shared `&` references replaced with exclusive references `&mut`.
    ///
    /// Conversion from `&` to `&mut` is generally invalid, so a `ref_to_mut()` method does not exist.
    type MutVar: MutVariadic<UnRefVar = Self::UnRefVar, MutVar = Self::MutVar>;

    /// convert entries to [`<UnRefVar as VariadicExt>::AsRefVar`](VariadicExt::AsRefVar)
    fn unref_ref(&self) -> <Self::UnRefVar as VariadicExt>::AsRefVar<'_>;
}
#[sealed]
impl<'a, Item, Rest> EitherRefVariadic for (&'a Item, Rest)
where
    Rest: EitherRefVariadic,
{
    type UnRefVar = (Item, Rest::UnRefVar);

    type RefVar = (&'a Item, Rest::RefVar);
    fn mut_to_ref(self) -> Self::RefVar {
        let var_args!(item, ...rest) = self;
        var_expr!(item, ...rest.mut_to_ref())
    }

    type MutVar = (&'a mut Item, Rest::MutVar);

    fn unref_ref(&self) -> <Self::UnRefVar as VariadicExt>::AsRefVar<'_> {
        let var_args!(item, ...rest) = self;
        var_expr!(item, ...rest.unref_ref())
    }
}
#[sealed]
impl<'a, Item, Rest> EitherRefVariadic for (&'a mut Item, Rest)
where
    Rest: EitherRefVariadic,
{
    type UnRefVar = (Item, Rest::UnRefVar);

    type RefVar = (&'a Item, Rest::RefVar);
    fn mut_to_ref(self) -> Self::RefVar {
        let var_args!(item, ...rest) = self;
        var_expr!(&*item, ...rest.mut_to_ref())
    }

    type MutVar = (&'a mut Item, Rest::MutVar);

    fn unref_ref(&self) -> <Self::UnRefVar as VariadicExt>::AsRefVar<'_> {
        let var_args!(item, ...rest) = self;
        var_expr!(item, ...rest.unref_ref())
    }
}
#[sealed]
impl EitherRefVariadic for () {
    type UnRefVar = ();

    type RefVar = ();
    fn mut_to_ref(self) -> Self::RefVar {}

    type MutVar = ();

    fn unref_ref(&self) -> <Self::UnRefVar as VariadicExt>::AsRefVar<'_> {}
}

/// A variadic where each item is a shared reference `&item`.
///
/// This can be created using [`VariadicExt::as_ref_var`]:
/// ```rust
/// # use variadics::*;
/// let as_ref: var_type!(&u32, &String, &bool) =
///     var_expr!(1_u32, "Hello".to_owned(), false).as_ref_var();
/// ```
///
/// This is a sealed trait.
#[sealed]
pub trait RefVariadic: EitherRefVariadic<RefVar = Self>
where
    Self: Copy,
{
}
#[sealed]
impl<Item, Rest> RefVariadic for (&Item, Rest) where Rest: RefVariadic {}
#[sealed]
impl RefVariadic for () {}

/// A variadic where each item is an exclusive reference `&mut item`.
///
/// This can be created using [`VariadicExt::as_mut_var`]:
/// ```rust
/// # use variadics::*;
/// let as_mut: var_type!(&mut u32, &mut String, &mut bool) =
///     var_expr!(1_u32, "Hello".to_owned(), false).as_mut_var();
/// ```
///
/// This is a sealed trait.
#[sealed]
pub trait MutVariadic: EitherRefVariadic<MutVar = Self> {}
#[sealed]
impl<Item, Rest> MutVariadic for (&mut Item, Rest) where Rest: MutVariadic {}
#[sealed]
impl MutVariadic for () {}

/// Copy a variadic of references [`EitherRefVariadic`] into a variadic of owned values [`EitherRefVariadic::UnRefVar`].
///
/// ```rust
/// # use variadics::*;
/// let ref_var = var_expr!(&1, &"hello", &false);
/// let copy_var = ref_var.copy_var();
/// assert_eq!(var_expr!(1, "hello", false), copy_var);
/// ```
#[sealed]
pub trait CopyRefVariadic: EitherRefVariadic {
    /// Copy self per-value.
    fn copy_var(&self) -> Self::UnRefVar;
}
#[sealed]
impl<Item, Rest> CopyRefVariadic for (&Item, Rest)
where
    Item: Copy,
    Rest: CopyRefVariadic,
{
    fn copy_var(&self) -> Self::UnRefVar {
        let &var_args!(&item, ...ref rest) = self;
        var_expr!(item, ...rest.copy_var())
    }
}
#[sealed]
impl<Item, Rest> CopyRefVariadic for (&mut Item, Rest)
where
    Item: Copy,
    Rest: CopyRefVariadic,
{
    fn copy_var(&self) -> Self::UnRefVar {
        let &var_args!(&mut item, ...ref rest) = self;
        var_expr!(item, ...rest.copy_var())
    }
}
#[sealed]
impl CopyRefVariadic for () {
    fn copy_var(&self) -> Self::UnRefVar {}
}

/// Clone a variadic of references [`AsRefVar`](VariadicExt::AsRefVar) into a variadic of owned values.
///
/// ```rust
/// # use variadics::*;
/// let ref_var = var_expr!(&1, &format!("hello {}", "world"), &vec![1, 2, 3]);
/// let clone_var = CloneVariadic::clone_ref_var(ref_var);
/// assert_eq!(
///     var_expr!(1, "hello world".to_owned(), vec![1, 2, 3]),
///     clone_var
/// );
/// ```
#[sealed]
pub trait CloneVariadic: VariadicExt + Clone {
    /// Clone a variadic of references [`AsRefVar`](VariadicExt::AsRefVar) into a variadic of owned values.
    fn clone_ref_var(this: Self::AsRefVar<'_>) -> Self;
}
#[sealed]
impl<Item, Rest> CloneVariadic for (Item, Rest)
where
    Item: Clone,
    Rest: CloneVariadic,
{
    fn clone_ref_var(this: Self::AsRefVar<'_>) -> Self {
        let var_args!(item, ...rest) = this;
        var_expr!(item.clone(), ...Rest::clone_ref_var(rest))
    }
}
#[sealed]
impl CloneVariadic for () {
    fn clone_ref_var(_this: Self::AsRefVar<'_>) -> Self {}
}

/// A variadic where all item implement [`PartialEq`].
#[sealed]
pub trait PartialEqVariadic: VariadicExt {
    /// `PartialEq` between a referenced variadic and a variadic of references, of the same types.
    fn eq(&self, other: &Self) -> bool;

    /// `PartialEq` for the `AsRefVar` version op `Self`.
    fn eq_ref(this: Self::AsRefVar<'_>, other: Self::AsRefVar<'_>) -> bool;
}
#[sealed]
impl<Item, Rest> PartialEqVariadic for (Item, Rest)
where
    Item: PartialEq,
    Rest: PartialEqVariadic,
{
    fn eq(&self, other: &Self) -> bool {
        let var_args!(item_self, ...rest_self) = self;
        let var_args!(item_other, ...rest_other) = other;
        item_self == item_other && rest_self.eq(rest_other)
    }

    fn eq_ref(
        this: <Self as VariadicExt>::AsRefVar<'_>,
        other: <Self as VariadicExt>::AsRefVar<'_>,
    ) -> bool {
        let var_args!(item_self, ...rest_self) = this;
        let var_args!(item_other, ...rest_other) = other;
        item_self == item_other && Rest::eq_ref(rest_self, rest_other)
    }
}
#[sealed]
impl PartialEqVariadic for () {
    fn eq(&self, _other: &Self) -> bool {
        true
    }

    fn eq_ref(
        _this: <Self as VariadicExt>::AsRefVar<'_>,
        _other: <Self as VariadicExt>::AsRefVar<'_>,
    ) -> bool {
        true
    }
}

/// A variadic where all elements are the same type, `T`.
///
/// This is a sealed trait.
#[sealed]
pub trait HomogenousVariadic<T>: Variadic {
    /// Returns a reference to an element.
    fn get(&self, i: usize) -> Option<&T>;
    /// Returns an exclusive reference to an element.
    fn get_mut(&mut self, i: usize) -> Option<&mut T>;

    /// Iterator type returned by `into_iter`.
    type IntoIter: Iterator<Item = T>;
    /// Turns this `HomogenousVariadic<T>` into an iterator of items `T`.
    fn into_iter(self) -> Self::IntoIter;
}
#[sealed]
impl<T> HomogenousVariadic<T> for () {
    fn get(&self, _i: usize) -> Option<&T> {
        None
    }
    fn get_mut(&mut self, _i: usize) -> Option<&mut T> {
        None
    }

    type IntoIter = core::iter::Empty<T>;
    fn into_iter(self) -> Self::IntoIter {
        core::iter::empty()
    }
}
#[sealed]
impl<T, Rest> HomogenousVariadic<T> for (T, Rest)
where
    Rest: HomogenousVariadic<T>,
{
    fn get(&self, i: usize) -> Option<&T> {
        let (item, rest) = self;
        if i == 0 { Some(item) } else { rest.get(i - 1) }
    }
    fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        let (item, rest) = self;
        if i == 0 {
            Some(item)
        } else {
            rest.get_mut(i - 1)
        }
    }

    type IntoIter = core::iter::Chain<core::iter::Once<T>, Rest::IntoIter>;
    fn into_iter(self) -> Self::IntoIter {
        let (item, rest) = self;
        core::iter::once(item).chain(rest.into_iter())
    }
}

/// Helper trait for splitting a variadic into two parts. `Prefix` is the first part, everything
/// after is the `Suffix` or second part.
///
/// This is a sealed trait.
#[sealed]
pub trait Split<Prefix>: VariadicExt
where
    Prefix: VariadicExt,
{
    /// The second part when splitting this variadic by `Prefix`.
    type Suffix: VariadicExt;
    /// Splits this variadic into two parts, first the `Prefix`, and second the `Suffix`.
    fn split(self) -> (Prefix, Self::Suffix);
    /// Splits a refvar variadic
    fn split_ref(
        this: Self::AsRefVar<'_>,
    ) -> (
        Prefix::AsRefVar<'_>,
        <Self::Suffix as VariadicExt>::AsRefVar<'_>,
    );
}
#[sealed]
impl<Item, Rest, PrefixRest> Split<(Item, PrefixRest)> for (Item, Rest)
where
    PrefixRest: VariadicExt,
    Rest: Split<PrefixRest>,
{
    /// The second part when splitting this variadic by `Prefix`.
    type Suffix = <Rest as Split<PrefixRest>>::Suffix;
    /// Splits this variadic into two parts, first the `Prefix`, and second the `Suffix`.
    fn split(self) -> ((Item, PrefixRest), Self::Suffix) {
        let (item, rest) = self;
        let (prefix_rest, suffix) = rest.split();
        ((item, prefix_rest), suffix)
    }
    /// Splits a refvar variadic
    fn split_ref(
        this: Self::AsRefVar<'_>,
    ) -> (
        <(Item, PrefixRest) as VariadicExt>::AsRefVar<'_>,
        <Self::Suffix as VariadicExt>::AsRefVar<'_>,
    ) {
        let (item, rest) = this;
        let (prefix_rest, suffix) = Rest::split_ref(rest);
        ((item, prefix_rest), suffix)
    }
}
#[sealed]
impl<Rest> Split<var_type!()> for Rest
where
    Rest: VariadicExt,
{
    type Suffix = Rest;
    fn split(self) -> (var_type!(), Self::Suffix) {
        (var_expr!(), self)
    }
    fn split_ref(
        this: Self::AsRefVar<'_>,
    ) -> (var_type!(), <Self::Suffix as VariadicExt>::AsRefVar<'_>) {
        (var_expr!(), this)
    }
}

#[sealed]
/// Helper trait for splitting a variadic into two parts. `Prefix` is the first part, everything
/// after is the `Suffix` or second part.
///
/// This is a sealed trait.
pub trait SplitBySuffix<Suffix>: VariadicExt
where
    Suffix: VariadicExt,
{
    /// The first part when splitting this variadic by `Suffix`.
    type Prefix: VariadicExt;
    /// Splits this variadic into two parts, first the `Prefix`, and second the `Suffix`.
    fn split_by_suffix(self) -> (Self::Prefix, Suffix);
    /// Splits a refvar variadic
    fn split_by_suffix_ref(
        this: Self::AsRefVar<'_>,
    ) -> (
        <Self::Prefix as VariadicExt>::AsRefVar<'_>,
        Suffix::AsRefVar<'_>,
    );
}
#[sealed]
impl<Suffix, This> SplitBySuffix<Suffix> for This
where
    Suffix: VariadicExt,
    This: VariadicExt,
    This::Reverse: Split<Suffix::Reverse>,
    Suffix::Reverse: VariadicExt<Reverse = Suffix>,
{
    /// The second part when splitting this variadic by `Prefix`.
    type Prefix = <<This::Reverse as Split<Suffix::Reverse>>::Suffix as VariadicExt>::Reverse;
    /// Splits this variadic into two parts, first the `Prefix`, and second the `Suffix`.
    fn split_by_suffix(self) -> (Self::Prefix, Suffix) {
        let (rsuffix, rprefix) = self.reverse().split();
        (rprefix.reverse(), rsuffix.reverse())
    }

    fn split_by_suffix_ref(
        this: Self::AsRefVar<'_>,
    ) -> (
        <Self::Prefix as VariadicExt>::AsRefVar<'_>,
        Suffix::AsRefVar<'_>,
    ) {
        let rev = This::reverse_ref(this);
        let (rsuffix, rprefix) = <This::Reverse as Split<Suffix::Reverse>>::split_ref(rev);
        let out = (rprefix.reverse(), rsuffix.reverse());
        // TODO!!!!
        let out2 = unsafe { core::mem::transmute_copy(&out) };
        core::mem::forget(out);
        out2
    }
}

/// trait for Variadic of vecs, as formed by `VariadicExt::into_vec()`
#[sealed]
pub trait VecVariadic: VariadicExt {
    /// Individual variadic items without the Vec wrapper
    type UnVec: VariadicExt<IntoVec = Self>;

    /// zip across all the vecs in this VariadicVec
    fn zip_vecs(&self) -> impl Iterator<Item = <Self::UnVec as VariadicExt>::AsRefVar<'_>>;

    /// append an unvec'ed Variadic into this VariadicVec
    fn push(&mut self, item: Self::UnVec);

    /// get the unvec'ed Variadic at position `index`
    fn get(&mut self, index: usize) -> Option<<Self::UnVec as VariadicExt>::AsRefVar<'_>>;

    /// result type from into_zip
    type IntoZip: Iterator<Item = Self::UnVec>;
    /// Turns into an iterator of items `UnVec` -- i.e. iterate through rows (not columns!).
    fn into_zip(self) -> Self::IntoZip;

    /// result type from drain
    type Drain<'a>: Iterator<Item = Self::UnVec>
    where
        Self: 'a;
    /// Turns into a Drain of items `UnVec` -- i.e. iterate through rows (not columns!).
    fn drain<R>(&mut self, range: R) -> Self::Drain<'_>
    where
        R: core::ops::RangeBounds<usize> + Clone;
}

#[sealed]
impl<Item, Rest> VecVariadic for (Vec<Item>, Rest)
where
    Rest: VecVariadic,
{
    type UnVec = var_type!(Item, ...Rest::UnVec);

    fn zip_vecs(&self) -> impl Iterator<Item = <Self::UnVec as VariadicExt>::AsRefVar<'_>> {
        let (this, rest) = self;
        core::iter::zip(this.iter(), rest.zip_vecs())
    }

    fn push(&mut self, row: Self::UnVec) {
        let (this_vec, rest_vecs) = self;
        let (this_col, rest_cols) = row;
        this_vec.push(this_col);
        rest_vecs.push(rest_cols);
    }

    fn get(&mut self, index: usize) -> Option<<Self::UnVec as VariadicExt>::AsRefVar<'_>> {
        let (this_vec, rest_vecs) = self;
        if let Some(rest) = VecVariadic::get(rest_vecs, index) {
            this_vec.get(index).map(|item| var_expr!(item, ...rest))
        } else {
            None
        }
    }

    type IntoZip = core::iter::Zip<alloc::vec::IntoIter<Item>, Rest::IntoZip>;
    fn into_zip(self) -> Self::IntoZip {
        let (this, rest) = self;
        core::iter::zip(this, rest.into_zip())
    }

    type Drain<'a>
        = core::iter::Zip<alloc::vec::Drain<'a, Item>, Rest::Drain<'a>>
    where
        Self: 'a;
    fn drain<R>(&mut self, range: R) -> Self::Drain<'_>
    where
        R: core::ops::RangeBounds<usize> + Clone,
    {
        let (this, rest) = self;
        core::iter::zip(this.drain(range.clone()), rest.drain(range))
    }
}

#[sealed]
impl VecVariadic for var_type!() {
    type UnVec = var_type!();

    fn zip_vecs(&self) -> impl Iterator<Item = <Self::UnVec as VariadicExt>::AsRefVar<'_>> {
        core::iter::repeat(var_expr!())
    }

    fn push(&mut self, _item: Self::UnVec) {}

    fn get(&mut self, _index: usize) -> Option<<Self::UnVec as VariadicExt>::AsRefVar<'_>> {
        Some(())
    }

    type IntoZip = core::iter::Repeat<var_type!()>;
    fn into_zip(self) -> Self::IntoZip {
        core::iter::repeat(var_expr!())
    }

    type Drain<'a>
        = core::iter::Repeat<var_type!()>
    where
        Self: 'a;
    fn drain<R>(&mut self, _range: R) -> Self::Drain<'_>
    where
        R: core::ops::RangeBounds<usize> + Clone,
    {
        core::iter::repeat(var_expr!())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    type MyList = var_type!(u8, u16, u32, u64);
    type MyPrefix = var_type!(u8, u16);
    type MySuffix = <MyList as Split<MyPrefix>>::Suffix;

    const _: MySuffix = var_expr!(0_u32, 0_u64);

    #[test]
    // #[expect(clippy::let_unit_value, reason = "var_expr macro test")]
    fn test_basic_expr() {
        let _ = var_expr!();
        let _ = var_expr!(1);
        let _ = var_expr!(1, "b",);
        let _ = var_expr!("a",);
        let _ = var_expr!(false, true, 1 + 2);
    }

    // commented out because neither #[allow(dead_code)] nor #[expect(dead_code)] made clippy happy
    // variadic_trait! {
    //     /// Variaidic list of futures.
    //     pub variadic<F> FuturesList where F: core::future::Future {
    //     }
    // }

    type _ListA = var_type!(u32, u8, i32);
    type _ListB = var_type!(..._ListA, bool, Option<()>);
    type _ListC = var_type!(..._ListA, bool, Option::<()>);

    #[test]
    fn test_as_ref_var() {
        let my_owned = var_expr!("Hello".to_owned(), Box::new(5));
        let my_ref_a = my_owned.as_ref_var();
        let my_ref_b = my_owned.as_ref_var();
        assert_eq!(my_ref_a, my_ref_b);
    }

    #[test]
    fn test_as_mut_var() {
        let mut my_owned = var_expr!("Hello".to_owned(), Box::new(5));
        let var_args!(mut_str, mut_box) = my_owned.as_mut_var();
        *mut_str += " World";
        *mut_box.as_mut() += 1;

        assert_eq!(var_expr!("Hello World".to_owned(), Box::new(6)), my_owned);
    }

    #[test]
    fn test_iter_any() {
        let mut var = var_expr!(1_i32, false, "Hello".to_owned());

        let mut mut_iter = var.iter_any_mut();
        *mut_iter.next().unwrap().downcast_mut::<i32>().unwrap() += 1;
        *mut_iter.next().unwrap().downcast_mut::<bool>().unwrap() |= true;
        *mut_iter.next().unwrap().downcast_mut::<String>().unwrap() += " World";
        assert!(mut_iter.next().is_none());

        let mut ref_iter = var.iter_any_ref();
        assert_eq!(
            Some(&2),
            ref_iter
                .next()
                .map(<dyn Any>::downcast_ref)
                .map(Option::unwrap)
        );
        assert_eq!(
            Some(&true),
            ref_iter
                .next()
                .map(<dyn Any>::downcast_ref)
                .map(Option::unwrap)
        );
        assert_eq!(
            Some("Hello World"),
            ref_iter
                .next()
                .map(|any| &**any.downcast_ref::<String>().unwrap())
        );
        assert!(ref_iter.next().is_none());
    }

    #[test]
    fn test_homogenous_get() {
        let mut var = var_expr!(0, 1, 2, 3, 4);
        for i in 0..5 {
            assert_eq!(Some(i), var.get(i).copied());
            assert_eq!(Some(i), var.get_mut(i).copied());
        }
    }

    #[test]
    fn test_into_vec() {
        use crate::VecVariadic;

        type Item = var_type!(i32, String);
        let first: Item = var_expr!(1, "Joe".to_string());
        let second: Item = var_expr!(2, "Mingwei".to_string());
        let mut column_store = first.clone().into_singleton_vec();
        column_store.push(second.clone());
        assert_eq!(column_store.len(), 2);
        assert_eq!(column_store.get(0).unwrap(), first.as_ref_var());
        assert_eq!(column_store.get(1).unwrap(), second.as_ref_var());
    }
}

#[test]
fn test_eq_ref_vec() {
    type MyVar = var_type!(i32, bool, &'static str);
    let vec: Vec<MyVar> = vec![
        var_expr!(0, true, "hello"),
        var_expr!(1, true, "world"),
        var_expr!(2, false, "goodnight"),
        var_expr!(3, false, "moon"),
    ];
    let needle: <MyVar as VariadicExt>::AsRefVar<'_> =
        var_expr!(2, false, "goodnight").as_ref_var();
    assert_eq!(
        Some(2),
        vec.iter()
            .position(|item| <MyVar as PartialEqVariadic>::eq_ref(needle, item.as_ref_var()))
    );

    let missing: <MyVar as VariadicExt>::AsRefVar<'_> =
        var_expr!(3, false, "goodnight").as_ref_var();
    assert_eq!(
        None,
        vec.iter()
            .position(|item| <MyVar as PartialEqVariadic>::eq_ref(missing, item.as_ref_var()))
    );
}

#[test]
fn clone_var_test() {
    let ref_var = var_expr!(&1, &format!("hello {}", "world"), &vec![1, 2, 3]);
    let clone_var = CloneVariadic::clone_ref_var(ref_var);
    assert_eq!(
        var_expr!(1, "hello world".to_owned(), vec![1, 2, 3]),
        clone_var
    );
}
