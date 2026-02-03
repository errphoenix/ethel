use std::{
    iter::{Map, Zip},
    slice::{Iter, IterMut},
};

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
    alpha: &'row [A],
    beta: &'row [B],
    _definition: std::marker::PhantomData<Def>,
}

#[derive(Clone, Copy, Debug)]
pub struct TrioView<'row, Def, A, B, Y>
where
    Def: Sized,
    A: Sized,
{
    alpha: &'row [A],
    beta: &'row [B],
    gamma: &'row [Y],
    _definition: std::marker::PhantomData<Def>,
}

#[derive(Clone, Copy, Debug)]
pub struct QuatView<'row, Def, A, B, Y, D>
where
    Def: Sized,
    A: Sized,
{
    alpha: &'row [A],
    beta: &'row [B],
    gamma: &'row [Y],
    delta: &'row [D],
    _definition: std::marker::PhantomData<Def>,
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
    pub fn alpha(&self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn join<B: Sized>(self, other: SoloView<'row, Def, B>) -> DualView<'row, Def, A, B> {
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
    pub fn alpha(&self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta(&self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub fn join<Y: Sized>(self, other: SoloView<'row, Def, Y>) -> TrioView<'row, Def, A, B, Y> {
        TrioView {
            alpha: self.alpha,
            beta: self.beta,
            gamma: other.alpha,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn pop_left(self) -> SoloView<'row, Def, B> {
        SoloView {
            alpha: self.beta,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn pop_right(self) -> SoloView<'row, Def, A> {
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
    pub fn alpha(&self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta(&self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub fn gamma(&self) -> &'row [Y] {
        self.gamma
    }

    #[inline(always)]
    pub fn join<D: Sized>(self, other: SoloView<'row, Def, D>) -> QuatView<'row, Def, A, B, Y, D> {
        QuatView {
            alpha: self.alpha,
            beta: self.beta,
            gamma: self.gamma,
            delta: other.alpha,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn pop_left(self) -> DualView<'row, Def, B, Y> {
        DualView {
            alpha: self.beta,
            beta: self.gamma,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn pop_right(self) -> DualView<'row, Def, A, B> {
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
    pub fn alpha(&self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta(&self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub fn gamma(&self) -> &'row [Y] {
        self.gamma
    }

    #[inline(always)]
    pub fn delta(&self) -> &'row [D] {
        self.delta
    }

    #[inline(always)]
    pub fn pop_left(self) -> TrioView<'row, Def, B, Y, D> {
        TrioView {
            alpha: self.beta,
            beta: self.gamma,
            gamma: self.delta,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn pop_right(self) -> TrioView<'row, Def, A, B, Y> {
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
    alpha: &'row mut [A],
    beta: &'row mut [B],
    _definition: std::marker::PhantomData<Def>,
}

#[derive(Debug)]
pub struct TrioViewMut<'row, Def, A, B, Y>
where
    Def: Sized,
    A: Sized,
{
    alpha: &'row mut [A],
    beta: &'row mut [B],
    gamma: &'row mut [Y],
    _definition: std::marker::PhantomData<Def>,
}

#[derive(Debug)]
pub struct QuatViewMut<'row, Def, A, B, Y, D>
where
    Def: Sized,
    A: Sized,
{
    alpha: &'row mut [A],
    beta: &'row mut [B],
    gamma: &'row mut [Y],
    delta: &'row mut [D],
    _definition: std::marker::PhantomData<Def>,
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
    pub fn alpha(&'row self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn alpha_mut(&'row mut self) -> &'row mut [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn join<B: Sized>(self, other: SoloViewMut<'row, Def, B>) -> DualViewMut<'row, Def, A, B> {
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
    pub fn alpha(&'row self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta(&'row self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub fn alpha_mut(&'row mut self) -> &'row mut [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta_mut(&'row mut self) -> &'row mut [B] {
        self.beta
    }

    #[inline(always)]
    pub fn join<Y: Sized>(
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
    pub fn pop_left(self) -> SoloViewMut<'row, Def, B> {
        SoloViewMut {
            alpha: self.beta,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn pop_right(self) -> SoloViewMut<'row, Def, A> {
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
    pub fn alpha(&'row self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta(&'row self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub fn gamma(&'row self) -> &'row [Y] {
        self.gamma
    }

    #[inline(always)]
    pub fn alpha_mut(&'row mut self) -> &'row mut [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta_mut(&'row mut self) -> &'row mut [B] {
        self.beta
    }

    #[inline(always)]
    pub fn gamma_mut(&'row mut self) -> &'row mut [Y] {
        self.gamma
    }

    #[inline(always)]
    pub fn join<D: Sized>(
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
    pub fn pop_left(self) -> DualViewMut<'row, Def, B, Y> {
        DualViewMut {
            alpha: self.beta,
            beta: self.gamma,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn pop_right(self) -> DualViewMut<'row, Def, A, B> {
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
    pub fn alpha(&'row self) -> &'row [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta(&'row self) -> &'row [B] {
        self.beta
    }

    #[inline(always)]
    pub fn gamma(&'row self) -> &'row [Y] {
        self.gamma
    }

    #[inline(always)]
    pub fn delta(&'row self) -> &'row [D] {
        self.delta
    }

    #[inline(always)]
    pub fn alpha_mut(&'row mut self) -> &'row mut [A] {
        self.alpha
    }

    #[inline(always)]
    pub fn beta_mut(&'row mut self) -> &'row mut [B] {
        self.beta
    }

    #[inline(always)]
    pub fn gamma_mut(&'row mut self) -> &'row mut [Y] {
        self.gamma
    }

    #[inline(always)]
    pub fn delta_mut(&'row mut self) -> &'row mut [D] {
        self.delta
    }

    #[inline(always)]
    pub fn pop_left(self) -> TrioViewMut<'row, Def, B, Y, D> {
        TrioViewMut {
            alpha: self.beta,
            beta: self.gamma,
            gamma: self.delta,
            _definition: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn pop_right(self) -> TrioViewMut<'row, Def, A, B, Y> {
        TrioViewMut {
            alpha: self.alpha,
            beta: self.beta,
            gamma: self.gamma,
            _definition: std::marker::PhantomData,
        }
    }
}

pub trait Table<Def: Sized + Default>: super::column::Column<Def> {}

#[macro_export]
macro_rules! table_spec {
    (
        struct $name:ident {
            $row_0:ident : $rt_0:ty;
            $($row:ident : $rt:ty;)+
        }
    ) => {
        paste::paste! {
            pub type [< $name TableDef >] = (
                $rt_0,
                    $($rt,)+
                );

            #[derive(Default, Debug)]
            pub struct [< $name RowTable >] {
                indices: Vec<u32>,
                free: Vec<u32>,
                owners: Vec<u32>,

                pub $row_0: Vec<$rt_0>,
                pub $($row: Vec<$rt>,)+
            }

            impl $crate::state::column::SparseSlot for [< $name RowTable >] {
                fn slots_map(&self) -> &Vec<u32> {
                    &self.indices
                }

                fn slots_map_mut(&mut self) -> &mut Vec<u32> {
                    &mut self.indices
                }

                fn free_list(&self) -> &Vec<u32> {
                    &self.free
                }

                fn free_list_mut(&mut self) -> &mut Vec<u32> {
                    &mut self.free
                }
            }

            impl $crate::state::column::Column < [< $name TableDef >]> for [< $name RowTable >] {
                fn len(&self) -> usize {
                    self.$row_0.len()
                }

                fn size(&self) -> usize {
                    self.indices.len()
                }

                fn free(&mut self, slot: u32) {
                    if slot == 0 {
                        panic!("slot 0 is reserved for degenerate elements and must not be freed");
                    }

                    let contiguous_slot = self.indices[slot as usize];
                    if contiguous_slot == 0 {
                        return;
                    }

                    self.indices[slot as usize] = 0;
                    self.owners.swap_remove(contiguous_slot as usize);

                    self.$row_0.swap_remove(contiguous_slot as usize);
                    $(
                        self.$row.swap_remove(contiguous_slot as usize);
                    )+
                    self.free.push(slot);
                }

                fn put(&mut self, ($row_0, $($row, )+): [< $name TableDef >]) -> u32 {
                    use $crate::state::column::SparseSlot;

                    let index = self.next_slot_index();
                    let slot = self.$row_0.len();

                    self.indices[index as usize] = slot as u32;
                    self.owners.push(index);

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
                        indices: vec![0],
                        free: Vec::new(),

                        $row_0: vec![Default::default()],
                        $($row: vec![Default::default()],)+
                    }
                }

                pub fn with_capacity(capacity: usize) -> Self {
                    let mut indices = Vec::with_capacity(capacity);
                    let mut $row_0 = Vec::with_capacity(capacity);

                    indices.push(0);
                    $row_0.push(Default::default());

                    $(
                        let mut $row = Vec::with_capacity(capacity);
                        $row.push(Default::default());
                    )+

                    Self {
                        indices,
                        free: Vec::new(),

                        $row_0,
                        $($row,)+
                    }
                }

                pub fn split(&self) -> (
                    $crate::state::table::SoloView<'_, [< $name TableDef >], $rt_0>,
                    $(
                        $crate::state::table::SoloView<'_, [< $name TableDef >], $rt>,
                    )+
                ) {
                    (
                        $crate::state::table::SoloView {
                            alpha: &self.$row_0,
                            _definition: std::marker::PhantomData,
                        },
                        $(
                            $crate::state::table::SoloView {
                            alpha: &self.$row,
                                _definition: std::marker::PhantomData,
                            },
                        )+
                    )
                }

                pub fn split_mut(&mut self) -> (
                    $crate::state::table::SoloViewMut<'_, [< $name TableDef >], $rt_0>,
                    $(
                        $crate::state::table::SoloViewMut<'_, [< $name TableDef >], $rt>,
                    )+
                ) {
                    (
                        $crate::state::table::SoloViewMut {
                            alpha: &mut self.$row_0,
                            _definition: std::marker::PhantomData,
                        },
                        $(
                            $crate::state::table::SoloViewMut {
                            alpha: &mut self.$row,
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

                pub fn [< $row_0 _view >](&self) -> $crate::state::table::SoloView<'_, [< $name TableDef >], $rt_0> {
                    $crate::state::table::SoloView {
                        alpha: &self.$row_0,
                        _definition: std::marker::PhantomData,
                    }
                }

                pub fn [< $row_0 _mut_view >](&mut self) -> $crate::state::table::SoloViewMut<'_, [< $name TableDef >], $rt_0> {
                    $crate::state::table::SoloViewMut {
                        alpha: &mut self.$row_0,
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

                    pub fn [< $row _view >](&self) -> $crate::state::table::SoloView<'_, [< $name TableDef >], $rt> {
                        $crate::state::table::SoloView {
                            alpha: &self.$row,
                            _definition: std::marker::PhantomData,
                        }
                    }

                    pub fn [< $row _mut_view >](&mut self) -> $crate::state::table::SoloViewMut<'_, [< $name TableDef >], $rt> {
                        $crate::state::table::SoloViewMut {
                            alpha: &mut self.$row,
                            _definition: std::marker::PhantomData,
                        }
                    }
                )+
            }
        }
    };
}
