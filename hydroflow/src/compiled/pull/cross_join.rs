use std::ops::Range;

pub struct CrossJoinState<V1, V2> {
    ltab: Vec<V1>,
    rtab: Vec<V2>,
    draw_from_left: bool,
    opposite_ix: Range<usize>,
}

impl<V1, V2> Default for CrossJoinState<V1, V2> {
    fn default() -> Self {
        Self {
            ltab: Vec::new(),
            rtab: Vec::new(),
            draw_from_left: true,
            opposite_ix: 0..0,
        }
    }
}

pub struct CrossJoin<'a, I1, V1, I2, V2>
where
    V1: Eq + Clone,
    V2: Eq + Clone,
    I1: Iterator<Item = V1>,
    I2: Iterator<Item = V2>,
{
    lhs: I1,
    rhs: I2,
    state: &'a mut CrossJoinState<V1, V2>,
}

impl<'a, I1, V1: 'static, I2, V2: 'static> Iterator for CrossJoin<'a, I1, V1, I2, V2>
where
    V1: Eq + Clone,
    V2: Eq + Clone,
    I1: Iterator<Item = V1>,
    I2: Iterator<Item = V2>,
{
    type Item = (V1, V2);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // see if there's a match from the opposite's iterator
            if let Some(i) = self.state.opposite_ix.next() {
                if self.state.draw_from_left {
                    let l = self.state.ltab.last().unwrap().clone();
                    let r = self.state.rtab.get(i).unwrap().clone();
                    return Some((l, r));
                } else {
                    let l = self.state.ltab.get(i).unwrap().clone();
                    let r = self.state.rtab.last().unwrap().clone();
                    return Some((l, r));
                }
            }
            // else fetch a new tuple, alternating the sides we fetch from,
            // so we draw from each input at the same rate.
            let mut found_new = false;
            for _i in ["opposite", "same"] {
                // toggle sides
                self.state.draw_from_left = !self.state.draw_from_left;

                // try to fetch from the specified side
                #[expect(clippy::collapsible_else_if, reason = "code symmetry")]
                if self.state.draw_from_left {
                    if let Some(l) = self.lhs.next() {
                        self.state.draw_from_left = true;
                        self.state.ltab.push(l);
                        self.state.opposite_ix = 0..self.state.rtab.len();
                        found_new = true;
                        break;
                    }
                } else {
                    if let Some(r) = self.rhs.next() {
                        self.state.draw_from_left = false;
                        self.state.rtab.push(r);
                        self.state.opposite_ix = 0..self.state.ltab.len();
                        found_new = true;
                        break;
                    }
                }
            }
            if !found_new {
                return None;
            }
        }
    }
}
impl<'a, I1, V1, I2, V2> CrossJoin<'a, I1, V1, I2, V2>
where
    V1: Eq + Clone,
    V2: Eq + Clone,
    I1: Iterator<Item = V1>,
    I2: Iterator<Item = V2>,
{
    pub fn new(lhs: I1, rhs: I2, state: &'a mut CrossJoinState<V1, V2>) -> Self {
        Self { lhs, rhs, state }
    }
}

#[cfg(test)]
mod tests {
    use super::{CrossJoin, CrossJoinState};

    #[test]
    fn cross_join() {
        let lhs = (0..3).map(|x| (format!("left {}", x)));
        let rhs = (10..13).map(|x| (format!("right {}", x)));

        let mut state = CrossJoinState::default();
        let join = CrossJoin::new(lhs, rhs, &mut state);

        assert_eq!(
            join.collect::<Vec<_>>(),
            vec![
                ("left 0".into(), "right 10".into()),
                ("left 0".into(), "right 11".into()),
                ("left 1".into(), "right 10".into()),
                ("left 1".into(), "right 11".into()),
                ("left 0".into(), "right 12".into()),
                ("left 1".into(), "right 12".into()),
                ("left 2".into(), "right 10".into()),
                ("left 2".into(), "right 11".into()),
                ("left 2".into(), "right 12".into())
            ]
        );
    }
}
