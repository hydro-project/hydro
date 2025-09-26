use sealed::sealed;

use crate::{VariadicExt, var_expr, var_type};

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

    type IntoZip = core::iter::Zip<std::vec::IntoIter<Item>, Rest::IntoZip>;
    fn into_zip(self) -> Self::IntoZip {
        let (this, rest) = self;
        core::iter::zip(this, rest.into_zip())
    }

    type Drain<'a>
        = core::iter::Zip<std::vec::Drain<'a, Item>, Rest::Drain<'a>>
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
        R: core::ops::RangeBounds<usize>,
    {
        core::iter::repeat(var_expr!())
    }
}
