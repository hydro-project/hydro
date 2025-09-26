use super::{Pusherator, PusheratorBuild};

pub struct Pivot<I, P>
where
    I: Iterator,
    P: Pusherator<Item = I::Item>,
{
    pull: I,
    push: P,
}
impl<I, P> Pivot<I, P>
where
    I: Iterator,
    P: Pusherator<Item = I::Item>,
{
    pub fn new(pull: I, push: P) -> Self {
        Self { pull, push }
    }

    pub fn step(&mut self) -> bool {
        if let Some(v) = self.pull.next() {
            self.push.give(v);
            true
        } else {
            false
        }
    }

    pub fn run(mut self) {
        for v in self.pull.by_ref() {
            self.push.give(v);
        }
    }
}

pub struct PivotBuild<I>
where
    I: Iterator,
{
    pull: I,
}
impl<I> PivotBuild<I>
where
    I: Iterator,
{
    pub fn new(pull: I) -> Self {
        Self { pull }
    }
}
impl<I> PusheratorBuild for PivotBuild<I>
where
    I: Iterator,
{
    type ItemOut = I::Item;

    type Output<O: Pusherator<Item = Self::ItemOut>> = Pivot<I, O>;
    fn push_to<O>(self, input: O) -> Self::Output<O>
    where
        O: Pusherator<Item = Self::ItemOut>,
    {
        Pivot {
            pull: self.pull,
            push: input,
        }
    }
}
