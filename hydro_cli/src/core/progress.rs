use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use futures::Future;
use indicatif::MultiProgress;

static PROGRESS_TRACKER: OnceLock<Mutex<ProgressTracker>> = OnceLock::new();

tokio::task_local! {
    static CURRENT_GROUP: Vec<usize>;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LeafStatus {
    Queued,
    Started,
    Finished,
}

#[derive(Debug)]
pub enum BarTree {
    Root(Vec<BarTree>),
    Group(String, Arc<indicatif::ProgressBar>, Vec<BarTree>),
    Leaf(String, Arc<indicatif::ProgressBar>, LeafStatus),
    Finished,
}

impl BarTree {
    fn get_pb(&self) -> Option<&Arc<indicatif::ProgressBar>> {
        match self {
            BarTree::Root(_) => None,
            BarTree::Group(_, pb, _) | BarTree::Leaf(_, pb, _) => Some(pb),
            BarTree::Finished => None,
        }
    }

    fn status(&self) -> LeafStatus {
        match self {
            BarTree::Root(children) | BarTree::Group(_, _, children) => {
                if children
                    .iter()
                    .all(|child| child.status() == LeafStatus::Finished)
                {
                    LeafStatus::Finished
                } else if children
                    .iter()
                    .any(|child| child.status() == LeafStatus::Started)
                {
                    LeafStatus::Started
                } else {
                    LeafStatus::Queued
                }
            }
            BarTree::Leaf(_, _, status) => status.clone(),
            BarTree::Finished => LeafStatus::Finished,
        }
    }

    fn refresh_prefix(&mut self, cur_path: &[String]) {
        match self {
            BarTree::Root(children) => {
                for child in children {
                    child.refresh_prefix(cur_path);
                }
            }
            BarTree::Group(name, pb, children) => {
                let mut path_with_group = cur_path.to_vec();
                path_with_group.push(name.clone());

                let finished_count = children
                    .iter()
                    .filter(|child| child.status() == LeafStatus::Finished)
                    .count();
                let started_count = children
                    .iter()
                    .filter(|child| child.status() == LeafStatus::Started)
                    .count();
                let queued_count = children
                    .iter()
                    .filter(|child| child.status() == LeafStatus::Queued)
                    .count();

                pb.set_prefix(format!(
                    "{} ({}/{}/{})",
                    path_with_group.join(" / "),
                    finished_count,
                    started_count,
                    queued_count
                ));
                for child in children {
                    child.refresh_prefix(&path_with_group);
                }
            }
            BarTree::Leaf(name, pb, _) => {
                let mut path_with_group = cur_path.to_vec();
                path_with_group.push(name.clone());
                pb.set_prefix(path_with_group.join(" / "));
            }
            BarTree::Finished => {}
        }
    }

    fn find_node(&mut self, path: &[usize]) -> &mut BarTree {
        if path.is_empty() {
            return self;
        }

        match self {
            BarTree::Root(children) | BarTree::Group(_, _, children) => {
                children[path[0]].find_node(&path[1..])
            }
            _ => panic!(),
        }
    }
}

pub struct ProgressTracker {
    pub(crate) multi_progress: MultiProgress,
    tree: BarTree,
    pub(crate) current_count: usize,
}

impl ProgressTracker {
    pub(crate) fn new() -> ProgressTracker {
        ProgressTracker {
            multi_progress: MultiProgress::new(),
            tree: BarTree::Root(vec![]),
            current_count: 0,
        }
    }

    pub fn start_task(
        &mut self,
        under_path: Vec<usize>,
        name: String,
        group: bool,
        progress: bool,
    ) -> (usize, Arc<indicatif::ProgressBar>) {
        let surrounding = self.tree.find_node(&under_path);
        let (surrounding_children, surrounding_pb) = match surrounding {
            BarTree::Root(children) => (children, None),
            BarTree::Group(_, pb, children) => (children, Some(pb)),
            _ => panic!(),
        };

        self.current_count += 1;

        let core_bar = indicatif::ProgressBar::new(100);
        let previous_bar = surrounding_children
            .iter()
            .rev()
            .flat_map(|c| c.get_pb())
            .next();
        let created_bar = if let Some(previous_bar) = previous_bar {
            self.multi_progress.insert_after(previous_bar, core_bar)
        } else if let Some(group_pb) = surrounding_pb {
            self.multi_progress.insert_after(group_pb, core_bar)
        } else {
            self.multi_progress.add(core_bar)
        };

        let pb = Arc::new(created_bar);
        if group {
            surrounding_children.push(BarTree::Group(name, pb.clone(), vec![]));
        } else {
            surrounding_children.push(BarTree::Leaf(name, pb.clone(), LeafStatus::Started));
        }

        let inserted_index = surrounding_children.len() - 1;

        if progress {
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.green} {prefix} {wide_msg} {bar} ({elapsed} elapsed)")
                    .unwrap(),
            );
        } else {
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.green} {prefix} {wide_msg} ({elapsed} elapsed)")
                    .unwrap(),
            );
        }
        pb.enable_steady_tick(Duration::from_millis(100));

        self.tree.refresh_prefix(&[]);
        (inserted_index, pb)
    }

    pub fn end_task(&mut self, path: Vec<usize>) {
        match self.tree.find_node(&path[0..path.len() - 1]) {
            BarTree::Root(children) | BarTree::Group(_, _, children) => {
                let removed = children[*path.last().unwrap()].get_pb().unwrap().clone();
                children[*path.last().unwrap()] = BarTree::Finished;
                self.multi_progress.remove(&removed);
            }

            _ => panic!(),
        };

        self.tree.refresh_prefix(&[]);

        self.current_count -= 1;
        if self.current_count == 0 {
            self.multi_progress.clear().unwrap();
        }
    }
}

impl ProgressTracker {
    pub fn println(msg: &str) {
        let progress_bar = PROGRESS_TRACKER
            .get_or_init(|| Mutex::new(ProgressTracker::new()))
            .lock()
            .unwrap();
        progress_bar.multi_progress.println(msg).unwrap();
    }

    pub fn with_group<'a, T, F: Future<Output = T>>(
        name: &str,
        f: impl FnOnce() -> F + 'a,
    ) -> impl Future<Output = T> + 'a {
        let mut group = CURRENT_GROUP
            .try_with(|cur| cur.clone())
            .unwrap_or_default();

        let (group_i, _) = {
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.start_task(group.clone(), name.to_string(), true, false)
        };

        group.push(group_i);

        CURRENT_GROUP.scope(group.clone(), async {
            let out = f().await;
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.end_task(group);
            out
        })
    }

    pub fn leaf<T, F: Future<Output = T>>(name: String, f: F) -> impl Future<Output = T> {
        let mut group = CURRENT_GROUP
            .try_with(|cur| cur.clone())
            .unwrap_or_default();

        let (leaf_i, _) = {
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.start_task(group.clone(), name, false, false)
        };

        group.push(leaf_i);

        async move {
            let out = f.await;
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.end_task(group);
            out
        }
    }

    pub fn rich_leaf<'a, T, F: Future<Output = T>>(
        name: String,
        f: impl FnOnce(Box<dyn Fn(u64) + Send + Sync>, Box<dyn Fn(String) + Send + Sync>) -> F + 'a,
    ) -> impl Future<Output = T> + 'a {
        let mut group = CURRENT_GROUP
            .try_with(|cur| cur.clone())
            .unwrap_or_default();

        let (leaf_i, bar) = {
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.start_task(group.clone(), name, false, true)
        };

        group.push(leaf_i);

        async move {
            let my_bar = bar.clone();
            let my_bar_2 = bar.clone();
            let out = f(
                Box::new(move |progress| {
                    my_bar.set_position(progress);
                }),
                Box::new(move |msg| {
                    my_bar_2.set_message(msg);
                }),
            )
            .await;
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.end_task(group);
            out
        }
    }
}
