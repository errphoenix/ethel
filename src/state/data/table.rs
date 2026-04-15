use std::{
    fmt::Debug,
    iter::{Map, Zip},
    slice::{Iter, IterMut},
};

use crate::state::data::{Column, DirectIndex, IndirectIndex};

#[derive(Clone, Copy, Debug)]
pub struct SoloView<'row, Def, A>
where
    Def: Sized,
    A: Sized,
{
    pub alpha: &'row [A],
    pub _definition: std::marker::PhantomData<Def>,
}

#[derive(Clone, Copy, Debug)]
pub struct DualView<'row, Def, A, B>
where
    Def: Sized,
    A: Sized,
{
    pub alpha: &'row [A],
    pub beta: &'row [B],
    pub _definition: std::marker::PhantomData<Def>,
}

#[derive(Clone, Copy, Debug)]
pub struct TrioView<'row, Def, A, B, Y>
where
    Def: Sized,
    A: Sized,
{
    pub alpha: &'row [A],
    pub beta: &'row [B],
    pub gamma: &'row [Y],
    pub _definition: std::marker::PhantomData<Def>,
}

#[derive(Clone, Copy, Debug)]
pub struct QuatView<'row, Def, A, B, Y, D>
where
    Def: Sized,
    A: Sized,
{
    pub alpha: &'row [A],
    pub beta: &'row [B],
    pub gamma: &'row [Y],
    pub delta: &'row [D],
    pub _definition: std::marker::PhantomData<Def>,
}

impl<'row, Def, A> IntoIterator for SoloView<'row, Def, A>
where
    Def: Sized,
    A: Sized,
{
    type Item = &'row A;

    type IntoIter = Iter<'row, A>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.alpha.iter()
    }
}

impl<'row, Def, A, B> IntoIterator for DualView<'row, Def, A, B>
where
    Def: Sized,
    A: Sized,
    B: Sized,
{
    type Item = (&'row A, &'row B);

    type IntoIter = Zip<Iter<'row, A>, Iter<'row, B>>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.alpha.iter().zip(self.beta)
    }
}

impl<'row, Def, A, B, Y> IntoIterator for TrioView<'row, Def, A, B, Y>
where
    Def: Sized,
    A: Sized,
    B: Sized,
    Y: Sized,
{
    type Item = (&'row A, &'row B, &'row Y);

    type IntoIter = Map<
        Zip<Zip<Iter<'row, A>, Iter<'row, B>>, Iter<'row, Y>>,
        fn(((&'row A, &'row B), &'row Y)) -> (&'row A, &'row B, &'row Y),
    >;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.alpha
            .iter()
            .zip(self.beta)
            .zip(self.gamma)
            .map(|((a, b), y)| (a, b, y))
    }
}

impl<'row, Def, A, B, Y, D> IntoIterator for QuatView<'row, Def, A, B, Y, D>
where
    Def: Sized,
    A: Sized,
    B: Sized,
    Y: Sized,
    D: Sized,
{
    type Item = (&'row A, &'row B, &'row Y, &'row D);

    type IntoIter = Map<
        Zip<Zip<Iter<'row, A>, Iter<'row, B>>, Zip<Iter<'row, Y>, Iter<'row, D>>>,
        fn(((&'row A, &'row B), (&'row Y, &'row D))) -> (&'row A, &'row B, &'row Y, &'row D),
    >;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.alpha
            .iter()
            .zip(self.beta)
            .zip(self.gamma.iter().zip(self.delta))
            .map(|((a, b), (y, d))| (a, b, y, d))
    }
}

impl<'row, Def, A> SoloView<'row, Def, A>
where
    Def: Sized,
    A: Sized,
{
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &'row A> {
        self.alpha.iter()
    }

    #[inline(always)]
    pub const fn alpha(&self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn join<B: Sized>(self, other: SoloView<'row, Def, B>) -> DualView<'row, Def, A, B> {
        DualView {
            alpha: self.alpha,
            beta: other.alpha,
            _definition: std::marker::PhantomData,
        }
    }
}

impl<'row, Def, A, B> DualView<'row, Def, A, B>
where
    Def: Sized,
    A: Sized,
    B: Sized,
{
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (&'row A, &'row B)> {
        self.alpha.iter().zip(self.beta)
    }

    #[inline(always)]
    pub const fn alpha(&self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta(&self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn join<Y: Sized>(
        self,
        other: SoloView<'row, Def, Y>,
    ) -> TrioView<'row, Def, A, B, Y> {
        TrioView {
            alpha: self.alpha,
            beta: self.beta,
            gamma: other.alpha,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_left(self) -> SoloView<'row, Def, B> {
        SoloView {
            alpha: self.beta,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_right(self) -> SoloView<'row, Def, A> {
        SoloView {
            alpha: self.alpha,
            _definition: std::marker::PhantomData,
        }
    }
}

impl<'row, Def, A, B, Y> TrioView<'row, Def, A, B, Y>
where
    Def: Sized,
    A: Sized,
    B: Sized,
    Y: Sized,
{
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = ((&'row A, &'row B), &'row Y)> {
        self.alpha.iter().zip(self.beta).zip(self.gamma)
    }

    #[inline(always)]
    pub const fn alpha(&self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta(&self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn gamma(&self) -> &'row [Y] {
        self.gamma
    }

    #[inline(always)]
    pub const fn join<D: Sized>(
        self,
        other: SoloView<'row, Def, D>,
    ) -> QuatView<'row, Def, A, B, Y, D> {
        QuatView {
            alpha: self.alpha,
            beta: self.beta,
            gamma: self.gamma,
            delta: other.alpha,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_left(self) -> DualView<'row, Def, B, Y> {
        DualView {
            alpha: self.beta,
            beta: self.gamma,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_right(self) -> DualView<'row, Def, A, B> {
        DualView {
            alpha: self.alpha,
            beta: self.beta,
            _definition: std::marker::PhantomData,
        }
    }
}

impl<'row, Def, A, B, Y, D> QuatView<'row, Def, A, B, Y, D>
where
    Def: Sized,
    A: Sized,
    B: Sized,
    Y: Sized,
    D: Sized,
{
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = ((&'row A, &'row B), (&'row Y, &'row D))> {
        self.alpha
            .iter()
            .zip(self.beta)
            .zip(self.gamma.iter().zip(self.delta))
    }

    #[inline(always)]
    pub const fn alpha(&self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta(&self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn gamma(&self) -> &'row [Y] {
        self.gamma
    }

    #[inline(always)]
    pub const fn delta(&self) -> &'row [D] {
        self.delta
    }

    #[inline(always)]
    pub const fn pop_left(self) -> TrioView<'row, Def, B, Y, D> {
        TrioView {
            alpha: self.beta,
            beta: self.gamma,
            gamma: self.delta,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_right(self) -> TrioView<'row, Def, A, B, Y> {
        TrioView {
            alpha: self.alpha,
            beta: self.beta,
            gamma: self.gamma,
            _definition: std::marker::PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct SoloViewMut<'row, Def, A>
where
    Def: Sized,
    A: Sized,
{
    pub alpha: &'row mut [A],
    pub _definition: std::marker::PhantomData<Def>,
}

#[derive(Debug)]
pub struct DualViewMut<'row, Def, A, B>
where
    Def: Sized,
    A: Sized,
{
    pub alpha: &'row mut [A],
    pub beta: &'row mut [B],
    pub _definition: std::marker::PhantomData<Def>,
}

#[derive(Debug)]
pub struct TrioViewMut<'row, Def, A, B, Y>
where
    Def: Sized,
    A: Sized,
{
    pub alpha: &'row mut [A],
    pub beta: &'row mut [B],
    pub gamma: &'row mut [Y],
    pub _definition: std::marker::PhantomData<Def>,
}

#[derive(Debug)]
pub struct QuatViewMut<'row, Def, A, B, Y, D>
where
    Def: Sized,
    A: Sized,
{
    pub alpha: &'row mut [A],
    pub beta: &'row mut [B],
    pub gamma: &'row mut [Y],
    pub delta: &'row mut [D],
    pub _definition: std::marker::PhantomData<Def>,
}

impl<'row, Def, A> IntoIterator for SoloViewMut<'row, Def, A>
where
    Def: Sized,
    A: Sized,
{
    type Item = &'row mut A;

    type IntoIter = IterMut<'row, A>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.alpha.iter_mut()
    }
}

impl<'row, Def, A, B> IntoIterator for DualViewMut<'row, Def, A, B>
where
    Def: Sized,
    A: Sized,
    B: Sized,
{
    type Item = (&'row mut A, &'row mut B);

    type IntoIter = Zip<IterMut<'row, A>, IterMut<'row, B>>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.alpha.iter_mut().zip(self.beta)
    }
}

impl<'row, Def, A, B, Y> IntoIterator for TrioViewMut<'row, Def, A, B, Y>
where
    Def: Sized,
    A: Sized,
    B: Sized,
    Y: Sized,
{
    type Item = (&'row mut A, &'row mut B, &'row mut Y);

    type IntoIter = Map<
        Zip<Zip<IterMut<'row, A>, IterMut<'row, B>>, IterMut<'row, Y>>,
        fn(((&'row mut A, &'row mut B), &'row mut Y)) -> (&'row mut A, &'row mut B, &'row mut Y),
    >;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.alpha
            .iter_mut()
            .zip(self.beta)
            .zip(self.gamma)
            .map(|((a, b), y)| (a, b, y))
    }
}

impl<'row, Def, A, B, Y, D> IntoIterator for QuatViewMut<'row, Def, A, B, Y, D>
where
    Def: Sized,
    A: Sized,
    B: Sized,
    Y: Sized,
    D: Sized,
{
    type Item = (&'row mut A, &'row mut B, &'row mut Y, &'row mut D);

    type IntoIter = Map<
        Zip<Zip<IterMut<'row, A>, IterMut<'row, B>>, Zip<IterMut<'row, Y>, IterMut<'row, D>>>,
        fn(
            ((&'row mut A, &'row mut B), (&'row mut Y, &'row mut D)),
        ) -> (&'row mut A, &'row mut B, &'row mut Y, &'row mut D),
    >;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.alpha
            .iter_mut()
            .zip(self.beta)
            .zip(self.gamma.iter_mut().zip(self.delta))
            .map(|((a, b), (y, d))| (a, b, y, d))
    }
}

impl<'row, Def, A> SoloViewMut<'row, Def, A>
where
    Def: Sized,
    A: Sized,
{
    #[inline(always)]
    pub fn iter(&'row self) -> impl Iterator<Item = &'row A> {
        self.alpha.iter()
    }

    #[inline(always)]
    pub fn iter_mut(&'row mut self) -> impl Iterator<Item = &'row mut A> {
        self.alpha.iter_mut()
    }

    #[inline(always)]
    pub const fn alpha(&'row self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn alpha_mut(&'row mut self) -> &'row mut [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn join<B: Sized>(
        self,
        other: SoloViewMut<'row, Def, B>,
    ) -> DualViewMut<'row, Def, A, B> {
        DualViewMut {
            alpha: self.alpha,
            beta: other.alpha,
            _definition: std::marker::PhantomData,
        }
    }
}

impl<'row, Def, A, B> DualViewMut<'row, Def, A, B>
where
    Def: Sized,
    A: Sized,
    B: Sized,
{
    #[inline(always)]
    pub fn iter(&'row self) -> impl Iterator<Item = (&'row A, &'row B)> {
        self.alpha.iter().zip(self.beta.iter())
    }

    #[inline(always)]
    pub fn iter_mut(&'row mut self) -> impl Iterator<Item = (&'row mut A, &'row mut B)> {
        self.alpha.iter_mut().zip(self.beta.iter_mut())
    }

    #[inline(always)]
    pub const fn alpha(&'row self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta(&'row self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn alpha_mut(&'row mut self) -> &'row mut [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta_mut(&'row mut self) -> &'row mut [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn join<Y: Sized>(
        self,
        other: SoloViewMut<'row, Def, Y>,
    ) -> TrioViewMut<'row, Def, A, B, Y> {
        TrioViewMut {
            alpha: self.alpha,
            beta: self.beta,
            gamma: other.alpha,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_left(self) -> SoloViewMut<'row, Def, B> {
        SoloViewMut {
            alpha: self.beta,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_right(self) -> SoloViewMut<'row, Def, A> {
        SoloViewMut {
            alpha: self.alpha,
            _definition: std::marker::PhantomData,
        }
    }
}

impl<'row, Def, A, B, Y> TrioViewMut<'row, Def, A, B, Y>
where
    Def: Sized,
    A: Sized,
    B: Sized,
    Y: Sized,
{
    #[inline(always)]
    pub fn iter(&'row self) -> impl Iterator<Item = ((&'row A, &'row B), &'row Y)> {
        self.alpha
            .iter()
            .zip(self.beta.iter())
            .zip(self.gamma.iter())
    }

    #[inline(always)]
    pub fn iter_mut(
        &'row mut self,
    ) -> impl Iterator<Item = ((&'row mut A, &'row mut B), &'row mut Y)> {
        self.alpha
            .iter_mut()
            .zip(self.beta.iter_mut())
            .zip(self.gamma.iter_mut())
    }

    #[inline(always)]
    pub const fn alpha(&'row self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta(&'row self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn gamma(&'row self) -> &'row [Y] {
        self.gamma
    }

    #[inline(always)]
    pub const fn alpha_mut(&'row mut self) -> &'row mut [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta_mut(&'row mut self) -> &'row mut [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn gamma_mut(&'row mut self) -> &'row mut [Y] {
        self.gamma
    }

    #[inline(always)]
    pub const fn join<D: Sized>(
        self,
        other: SoloViewMut<'row, Def, D>,
    ) -> QuatViewMut<'row, Def, A, B, Y, D> {
        QuatViewMut {
            alpha: self.alpha,
            beta: self.beta,
            gamma: self.gamma,
            delta: other.alpha,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_left(self) -> DualViewMut<'row, Def, B, Y> {
        DualViewMut {
            alpha: self.beta,
            beta: self.gamma,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_right(self) -> DualViewMut<'row, Def, A, B> {
        DualViewMut {
            alpha: self.alpha,
            beta: self.beta,
            _definition: std::marker::PhantomData,
        }
    }
}

impl<'row, Def, A, B, Y, D> QuatViewMut<'row, Def, A, B, Y, D>
where
    Def: Sized,
    A: Sized,
    B: Sized,
    Y: Sized,
    D: Sized,
{
    #[inline(always)]
    pub fn iter(&'row self) -> impl Iterator<Item = ((&'row A, &'row B), (&'row Y, &'row D))> {
        self.alpha
            .iter()
            .zip(self.beta.iter())
            .zip(self.gamma.iter().zip(self.delta.iter()))
    }

    #[inline(always)]
    pub fn iter_mut(
        &'row mut self,
    ) -> impl Iterator<Item = ((&'row mut A, &'row mut B), (&'row mut Y, &'row mut D))> {
        self.alpha
            .iter_mut()
            .zip(self.beta.iter_mut())
            .zip(self.gamma.iter_mut().zip(self.delta.iter_mut()))
    }

    #[inline(always)]
    pub const fn alpha(&'row self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta(&'row self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn gamma(&'row self) -> &'row [Y] {
        self.gamma
    }

    #[inline(always)]
    pub const fn delta(&'row self) -> &'row [D] {
        self.delta
    }

    #[inline(always)]
    pub const fn alpha_mut(&'row mut self) -> &'row mut [A] {
        self.alpha
    }

    #[inline(always)]
    pub const fn beta_mut(&'row mut self) -> &'row mut [B] {
        self.beta
    }

    #[inline(always)]
    pub const fn gamma_mut(&'row mut self) -> &'row mut [Y] {
        self.gamma
    }

    #[inline(always)]
    pub const fn delta_mut(&'row mut self) -> &'row mut [D] {
        self.delta
    }

    #[inline(always)]
    pub const fn pop_left(self) -> TrioViewMut<'row, Def, B, Y, D> {
        TrioViewMut {
            alpha: self.beta,
            beta: self.gamma,
            gamma: self.delta,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn pop_right(self) -> TrioViewMut<'row, Def, A, B, Y> {
        TrioViewMut {
            alpha: self.alpha,
            beta: self.beta,
            gamma: self.gamma,
            _definition: std::marker::PhantomData,
        }
    }
}

pub trait Table<Def: Sized + Default>: Column<Def> {}

pub trait TableView<'view, Def: Sized + Default>: Debug + Clone + Copy {
    /// The indirect indices map of the whole table, regardless of this
    /// view's offset or length.
    fn indirect_indices(&self) -> &'view [DirectIndex];

    /// The "reverse" indices map of each entry of this table view.
    fn handles(&self) -> &'view [IndirectIndex];

    /// The indexing offset between sparse index values and contiguous slices.
    fn view_offset(&self) -> usize;

    /// The length of this view's contiguous slice(s).
    fn len(&self) -> usize {
        self.handles().len()
    }

    fn solve(&self, indirect: IndirectIndex) -> DirectIndex {
        let global = self.indirect_indices()[indirect.as_index()];
        DirectIndex::from_index(global.as_index() - self.view_offset(), global.generation())
    }

    unsafe fn solve_unchecked(&self, indirect: IndirectIndex) -> DirectIndex {
        let global = unsafe { self.indirect_indices().get_unchecked(indirect.as_index()) };
        DirectIndex::from_index(global.as_index() - self.view_offset(), global.generation())
    }
}

#[macro_export]
macro_rules! table_spec {
    (
        struct $name:ident {
            $row_0:ident : $rt_0:ty;
            $($row:ident : $rt:ty;)+
        }
    ) => {
        paste::paste! {
            #[derive(Clone, Debug, Default)]
            pub struct [< $name TableDef >](
                (
                    $rt_0,
                        $($rt,)+
                )
            );

            impl From<($rt_0, $($rt,)+)> for [< $name TableDef >] {
                fn from(value: ($rt_0, $($rt,)+)) -> [< $name TableDef >] {
                    [< $name TableDef >](value)
                }
            }

            #[derive(Debug, Clone, Copy)]
            pub struct [< $name RowTableView >]<'view> {
                pub indirect_indices: &'view [$crate::state::data::DirectIndex],
                view_offset: usize,

                pub handles: &'view [$crate::state::data::IndirectIndex],
                pub $row_0: &'view [$rt_0],
                $(
                    pub $row: &'view [$rt],
                )+
            }

            impl<'view> [< $name RowTableView >]<'view> {
                pub fn from(table: &'view [< $name RowTable >]) -> Self {
                    use $crate::state::data::SparseSlot;

                    Self {
                        indirect_indices: table.slots_map(),
                        view_offset: 0,

                        handles: table.handles(),
                        $row_0: table. [< $row_0 _slice >] (),
                        $(
                            $row: table. [< $row _slice >] (),
                        )+
                    }
                }

                pub fn from_range(table: &'view [< $name RowTable >], offset: usize, length: usize) -> Self {
                    use $crate::state::data::{SparseSlot, Column};

                    debug_assert!(
                        offset < table.len(),
                        "cannot construct RowTableView: offset {offset} goes beyond table length of {}",
                        table.len()
                    );
                    debug_assert!(
                        (offset + length) < table.len(),
                        "cannot construct RowTableView: attempted to create view over range {offset}..{} for table of length {}",
                        offset + length,
                        table.len()
                    );

                    Self {
                        indirect_indices: table.slots_map(),
                        view_offset: offset,

                        handles: &table.handles()[offset..(offset+length)],
                        $row_0: &table. [< $row_0 _slice >]()[offset..(offset+length)],
                        $(
                            $row: &table. [< $row _slice >]()[offset..(offset+length)],
                        )+
                    }
                }

                pub fn coalesced(&self, indirect: $crate::state::data::IndirectIndex) -> (
                    &$rt_0,
                    $(
                        &$rt,
                    )+
                ) {
                    use $crate::state::data::table::TableView;
                    let direct = self.solve(indirect).as_index();

                    (
                        &self.$row_0[direct],
                        $(
                            &self.$row[direct],
                        )+
                    )
                }

                pub unsafe fn coalesced_unchecked(&self, indirect: $crate::state::data::IndirectIndex) -> (
                    &$rt_0,
                    $(
                        &$rt,
                    )+
                ) {
                    use $crate::state::data::table::TableView;
                    let direct = unsafe { self.solve_unchecked(indirect).as_index() };

                    (
                        unsafe { self.$row_0.get_unchecked(direct) },
                        $(
                            unsafe { self.$row.get_unchecked(direct) },
                        )+
                    )
                }

                pub fn $row_0(&self, indirect: $crate::state::data::IndirectIndex) -> &$rt_0 {
                    use $crate::state::data::table::TableView;
                    let direct = self.solve(indirect);
                    &self.$row_0[direct.as_index()]
                }

                pub unsafe fn [< $row_0 _unchecked >](&self, indirect: $crate::state::data::IndirectIndex) -> &$rt_0 {
                    use $crate::state::data::table::TableView;
                    let direct = unsafe { self.solve_unchecked(indirect) };
                    unsafe { self.$row_0.get_unchecked(direct.as_index()) }
                }

                $(
                    pub fn $row(&self, indirect: $crate::state::data::IndirectIndex) -> &$rt {
                        use $crate::state::data::table::TableView;
                        let direct = self.solve(indirect);
                        &self.$row[direct.as_index()]
                    }

                    pub unsafe fn [< $row _unchecked >](&self, indirect: $crate::state::data::IndirectIndex) -> &$rt {
                        use $crate::state::data::table::TableView;
                        let direct = unsafe { self.solve_unchecked(indirect) };
                        unsafe { self.$row.get_unchecked(direct.as_index()) }
                    }
                )+
            }

            impl<'view> $crate::state::data::table::TableView<'view, [< $name TableDef >]> for [< $name RowTableView >]<'view> {
                fn indirect_indices(&self) -> &'view [$crate::state::data::DirectIndex] {
                    &self.indirect_indices
                }

                fn handles(&self) -> &'view [$crate::state::data::IndirectIndex] {
                    &self.handles
                }

                fn view_offset(&self) -> usize {
                    self.view_offset
                }
            }

            #[derive(Debug)]
            pub struct [< $name RowTable >] {
                indices: Vec<$crate::state::data::DirectIndex>,
                free: Vec<$crate::state::data::IndirectIndex>,

                pub handles: Vec<$crate::state::data::IndirectIndex>,

                pub $row_0: Vec<$rt_0>,
                $(
                    pub $row: Vec<$rt>,
                )+
            }

            impl Default for [< $name RowTable >] {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl $crate::state::data::SparseSlot for [< $name RowTable >] {
                fn slots_map(&self) -> &Vec<$crate::state::data::DirectIndex> {
                    &self.indices
                }

                fn slots_map_mut(&mut self) -> &mut Vec<$crate::state::data::DirectIndex> {
                    &mut self.indices
                }

                fn free_list(&self) -> &Vec<$crate::state::data::IndirectIndex> {
                    &self.free
                }

                fn free_list_mut(&mut self) -> &mut Vec<$crate::state::data::IndirectIndex> {
                    &mut self.free
                }
            }

            impl $crate::state::data::Column < [< $name TableDef >]> for [< $name RowTable >] {
                fn len(&self) -> usize {
                    self.$row_0.len()
                }

                fn size(&self) -> usize {
                    self.indices.len()
                }

                fn free(&mut self, slot: $crate::state::data::IndirectIndex) {
                    if slot.as_int() == 0 {
                        panic!("slot 0 is reserved for degenerate elements and must not be freed");
                    }

                    let contiguous_slot = self.indices[slot.as_index()];
                    if !contiguous_slot.related_to_indirect(&slot) || contiguous_slot.as_int() == 0 {
                        return;
                    }

                    let last_owner = *self
                        .handles
                        .last()
                        .expect("contiguous vectors are never empty");

                    self.indices[slot.as_index()] = contiguous_slot.next_generation();
                    // do not reassign slot if we are freeing last
                    if last_owner.as_index() != slot.as_index() {
                        self.indices[last_owner.as_index()] = contiguous_slot;
                    }

                    let contiguous_index = contiguous_slot.as_index();
                    self.handles.swap_remove(contiguous_index);
                    self.$row_0.swap_remove(contiguous_index);
                    $(
                        self.$row.swap_remove(contiguous_index);
                    )+
                    self.free.push(slot.next_generation());
                }

                fn insert<V: Into<[< $name TableDef >]>>(&mut self, element: V) -> $crate::state::data::IndirectIndex {
                    use $crate::state::data::SparseSlot;

                    let [< $name TableDef >] ( ( $row_0, $($row, )+) ) = element.into();

                    let index = self.next_slot_index();
                    let head = self.$row_0.len();

                    self.indices[index.as_index()] = $crate::state::data::DirectIndex::from_index(head, index.generation());
                    self.handles.push(index);

                    self.$row_0.push($row_0);
                    $(
                        self.$row.push($row);
                    )+
                    index
                }
            }

            impl [< $name RowTable >] {
                pub fn new() -> Self {
                    Self {
                        indices: vec![$crate::state::data::DirectIndex::default()],
                        handles: vec![$crate::state::data::IndirectIndex::default()],
                        free: Vec::new(),

                        $row_0: vec![Default::default()],
                        $($row: vec![Default::default()],)+
                    }
                }

                pub fn with_capacity(capacity: usize) -> Self {
                    let mut indices = Vec::with_capacity(capacity);
                    let mut handles = Vec::with_capacity(capacity);
                    let mut $row_0 = Vec::with_capacity(capacity);

                    indices.push($crate::state::data::DirectIndex::default());
                    handles.push($crate::state::data::IndirectIndex::default());
                    $row_0.push(Default::default());

                    $(
                        let mut $row = Vec::with_capacity(capacity);
                        $row.push(Default::default());
                    )+

                    Self {
                        indices,
                        handles,
                        free: Vec::new(),

                        $row_0,
                        $($row,)+
                    }
                }

                pub fn clear(&mut self) {
                    self.indices.resize(1, $crate::state::data::DirectIndex::default());
                    self.handles.resize(1, $crate::state::data::IndirectIndex::default());

                    self.$row_0.resize(1, Default::default());
                    $(
                        self.$row.resize(1, Default::default());
                    )+

                    self.free.clear();
                }

                /// Returns the "reverse map" for the handle of each element.
                ///
                /// Each handle corresponds in parallel to an element in all
                /// rows. The value of this handle is the indirect index of
                /// that element across all rows of the same index.
                pub fn handles(&self) -> &[$crate::state::data::IndirectIndex] {
                    &self.handles
                }

                /// Returns the "reverse map" for owner indices.
                ///
                /// Each handle corresponds in parallel to an element in all
                /// rows. The value of this handle is the indirect index of
                /// that element across all rows of the same index.
                pub fn handles_view(&self) -> $crate::state::data::table::SoloView<'_, [< $name TableDef >], $crate::state::data::IndirectIndex> {
                    $crate::state::data::table::SoloView {
                        alpha: &self.handles[1..],
                        _definition: std::marker::PhantomData,
                    }
                }

                pub fn split(&self) -> (
                    $crate::state::data::table::SoloView<'_, [< $name TableDef >], $rt_0>,
                    $(
                        $crate::state::data::table::SoloView<'_, [< $name TableDef >], $rt>,
                    )+
                ) {
                    (
                        $crate::state::data::table::SoloView {
                            alpha: &self.$row_0[1..],
                            _definition: std::marker::PhantomData,
                        },
                        $(
                            $crate::state::data::table::SoloView {
                            alpha: &self.$row[1..],
                                _definition: std::marker::PhantomData,
                            },
                        )+
                    )
                }

                pub fn split_mut(&mut self) -> (
                    $crate::state::data::table::SoloViewMut<'_, [< $name TableDef >], $rt_0>,
                    $(
                        $crate::state::data::table::SoloViewMut<'_, [< $name TableDef >], $rt>,
                    )+
                ) {
                    (
                        $crate::state::data::table::SoloViewMut {
                            alpha: &mut self.$row_0[1..],
                            _definition: std::marker::PhantomData,
                        },
                        $(
                            $crate::state::data::table::SoloViewMut {
                                alpha: &mut self.$row[1..],
                                _definition: std::marker::PhantomData,
                            },
                        )+
                    )
                }

                pub fn split_range<Range>(&self, range: Range) -> (
                    $crate::state::data::table::SoloView<'_, [< $name TableDef >], $rt_0>,
                    $(
                        $crate::state::data::table::SoloView<'_, [< $name TableDef >], $rt>,
                    )+
                )
                    where Range: Clone + Copy +
                        std::slice::SliceIndex<[$rt_0], Output = [$rt_0]>
                        $(
                            + std::slice::SliceIndex<[$rt], Output = [$rt]>
                        )+
                {
                    (
                        $crate::state::data::table::SoloView {
                            alpha: &self.$row_0[range],
                            _definition: std::marker::PhantomData,
                        },
                        $(
                            $crate::state::data::table::SoloView {
                                alpha: &self.$row[range],
                                _definition: std::marker::PhantomData,
                            },
                        )+
                    )
                }

                pub fn split_mut_range<Range>(&mut self, range: Range) -> (
                    $crate::state::data::table::SoloViewMut<'_, [< $name TableDef >], $rt_0>,
                    $(
                        $crate::state::data::table::SoloViewMut<'_, [< $name TableDef >], $rt>,
                    )+
                )
                    where Range: Clone + Copy +
                        std::slice::SliceIndex<[$rt_0], Output = [$rt_0]>
                        $(
                            + std::slice::SliceIndex<[$rt], Output = [$rt]>
                        )+
                {
                    (
                        $crate::state::data::table::SoloViewMut {
                            alpha: &mut self.$row_0[range],
                            _definition: std::marker::PhantomData,
                        },
                        $(
                            $crate::state::data::table::SoloViewMut {
                                alpha: &mut self.$row[range],
                                _definition: std::marker::PhantomData,
                            },
                        )+
                    )
                }

                pub fn [< $row_0 _slice >](&self) -> &[$rt_0] {
                    &self.$row_0
                }

                pub fn [< $row_0 _mut_slice >](&mut self) -> &mut [$rt_0] {
                    &mut self.$row_0
                }

                pub fn [< $row_0 _view >](&self) -> $crate::state::data::table::SoloView<'_, [< $name TableDef >], $rt_0> {
                    $crate::state::data::table::SoloView {
                        alpha: &self.$row_0[1..],
                        _definition: std::marker::PhantomData,
                    }
                }

                pub fn [< $row_0 _mut_view >](&mut self) -> $crate::state::data::table::SoloViewMut<'_, [< $name TableDef >], $rt_0> {
                    $crate::state::data::table::SoloViewMut {
                        alpha: &mut self.$row_0[1..],
                        _definition: std::marker::PhantomData,
                    }
                }

                pub fn [< $row_0 _view_range >]<Range>(&self, range: Range) -> $crate::state::data::table::SoloView<'_, [< $name TableDef >], $rt_0>
                    where Range: std::slice::SliceIndex<[$rt_0], Output = [$rt_0]>
                {
                    $crate::state::data::table::SoloView {
                        alpha: &self.$row_0[range],
                        _definition: std::marker::PhantomData,
                    }
                }

                pub fn [< $row_0 _mut_view_range >]<Range>(&mut self, range: Range) -> $crate::state::data::table::SoloViewMut<'_, [< $name TableDef >], $rt_0>
                    where Range: std::slice::SliceIndex<[$rt_0], Output = [$rt_0]>
                {
                    $crate::state::data::table::SoloViewMut {
                        alpha: &mut self.$row_0[range],
                        _definition: std::marker::PhantomData,
                    }
                }

                $(
                    pub fn [< $row _slice >](&self) -> &[$rt] {
                        &self.$row
                    }

                    pub fn [< $row _mut_slice >](&mut self) -> &mut [$rt] {
                        &mut self.$row
                    }

                    pub fn [< $row _view >](&self) -> $crate::state::data::table::SoloView<'_, [< $name TableDef >], $rt> {
                        $crate::state::data::table::SoloView {
                            alpha: &self.$row[1..],
                            _definition: std::marker::PhantomData,
                        }
                    }

                    pub fn [< $row _mut_view >](&mut self) -> $crate::state::data::table::SoloViewMut<'_, [< $name TableDef >], $rt> {
                        $crate::state::data::table::SoloViewMut {
                            alpha: &mut self.$row[1..],
                            _definition: std::marker::PhantomData,
                        }
                    }


                    pub fn [< $row _view_range >]<Range>(&self, range: Range) -> $crate::state::data::table::SoloView<'_, [< $name TableDef >], $rt>
                        where Range: std::slice::SliceIndex<[$rt], Output = [$rt]>
                    {
                        $crate::state::data::table::SoloView {
                            alpha: &self.$row[range],
                            _definition: std::marker::PhantomData,
                        }
                    }

                    pub fn [< $row _mut_view_range >]<Range>(&mut self, range: Range) -> $crate::state::data::table::SoloViewMut<'_, [< $name TableDef >], $rt>
                        where Range: std::slice::SliceIndex<[$rt], Output = [$rt]>
                    {
                        $crate::state::data::table::SoloViewMut {
                            alpha: &mut self.$row[range],
                            _definition: std::marker::PhantomData,
                        }
                    }
                )+
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[allow(unused)]
    #[test]
    fn macro_table() {
        table_spec! {
            struct Test {
                names: String;
                health: f32;
                positions: (f32, f32);
            }
        };

        let tab = TestRowTable::new();
        let view = TestRowTableView::from(&tab);
    }

    #[test]
    fn free_last_after_random_free() {
        use crate::state::data::{Column, IndirectIndex};

        table_spec! {
            struct Test {
                a: u32;
                b: u32;
            }
        };

        let mut table = TestRowTable::new();

        for i in 0..50 {
            table.insert((i as u32, i as u32 + 50));
        }
        let last = table.insert((200, 400));

        // free random
        {
            table.free(IndirectIndex::from_int(37, 0));
            table.free(IndirectIndex::from_int(14, 0));
            table.free(IndirectIndex::from_int(32, 0));
            table.free(IndirectIndex::from_int(45, 0));
            table.free(IndirectIndex::from_int(24, 0));
            table.free(IndirectIndex::from_int(3, 0));
            table.free(IndirectIndex::from_int(7, 0));
            table.free(IndirectIndex::from_int(35, 0));
        }

        // free last
        table.free(last);
    }
}
