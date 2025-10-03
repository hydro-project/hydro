use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use bolero::ValueGenerator;
use bolero::generator::bolero_generator::driver::object::Borrowed;
use tokio::sync::mpsc::UnboundedSender;

pub trait SimHook {
    fn current_decision(&self) -> Option<bool>;
    fn can_make_nontrivial_decision(&self) -> bool;
    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool;
    fn release_decision(&mut self);
}

pub struct StreamHook<T> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: UnboundedSender<T>,
}

impl<T> SimHook for StreamHook<T> {
    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        let count = ((if force_nontrivial { 1 } else { 0 })..=current_input.len())
            .generate(driver)
            .unwrap();
        self.to_release = Some(current_input.drain(0..count).collect());
        count > 0
    }

    fn release_decision(&mut self) {
        if let Some(to_release) = self.to_release.take() {
            for item in to_release {
                self.output.send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }
}
