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
    Started,
    Finished,
}

#[derive(Debug)]
pub enum BarTree {
    Root(Vec<BarTree>),
    Group(
        String,
        Arc<indicatif::ProgressBar>,
        Vec<BarTree>,
        Option<usize>,
    ),
    Leaf(String, Arc<indicatif::ProgressBar>, LeafStatus),
    Finished,
}

impl BarTree {
    fn get_pb(&self) -> Option<&Arc<indicatif::ProgressBar>> {
        match self {
            BarTree::Root(_) => None,
            BarTree::Group(_, pb, _, _) | BarTree::Leaf(_, pb, _) => Some(pb),
            BarTree::Finished => None,
        }
    }

    fn status(&self) -> LeafStatus {
        match self {
            BarTree::Root(children) | BarTree::Group(_, _, children, _) => {
                if !children.is_empty()
                    && children
                        .iter()
                        .all(|child| child.status() == LeafStatus::Finished)
                {
                    LeafStatus::Finished
                } else {
                    LeafStatus::Started
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
            BarTree::Group(name, pb, children, anticipated_total) => {
                let finished_count = children
                    .iter()
                    .filter(|child| child.status() == LeafStatus::Finished)
                    .count();
                let started_count = children
                    .iter()
                    .filter(|child| child.status() == LeafStatus::Started)
                    .count();

                let progress_str = if Some(1) == *anticipated_total {
                    String::new()
                } else {
                    let started_or_finished_count = finished_count + started_count;
                    format!(
                        " ({}/{}/{})",
                        finished_count,
                        started_or_finished_count,
                        anticipated_total
                            .filter(|&total| total <= started_or_finished_count)
                            .map_or("?", |total| &*total.to_string()),
                    )
                };

                if cur_path.is_empty() {
                    pb.set_prefix(format!("{}{}", name, progress_str));
                } else {
                    pb.set_prefix(format!(
                        "{} / {}{}",
                        cur_path.join(" / "),
                        name,
                        progress_str,
                    ));
                }

                let mut path_with_group = cur_path.to_vec();
                let non_finished_count = children
                    .iter()
                    .filter(|child| child.status() != LeafStatus::Finished)
                    .count();

                if non_finished_count == 1 {
                    path_with_group.push(format!("{}{}", name, progress_str));
                } else {
                    path_with_group.push(name.clone());
                }

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

    fn find_node(&self, path: &[usize]) -> &BarTree {
        if path.is_empty() {
            return self;
        }

        match self {
            BarTree::Root(children) | BarTree::Group(_, _, children, _) => {
                children[path[0]].find_node(&path[1..])
            }
            _ => panic!(),
        }
    }

    fn find_node_mut(&mut self, path: &[usize]) -> &mut BarTree {
        if path.is_empty() {
            return self;
        }

        match self {
            BarTree::Root(children) | BarTree::Group(_, _, children, _) => {
                children[path[0]].find_node_mut(&path[1..])
            }
            _ => panic!(),
        }
    }
}

pub struct ProgressTracker {
    pub(crate) multi_progress: MultiProgress,
    tree: BarTree,
    pub(crate) current_count: usize,
    progress_list: Vec<(Arc<indicatif::ProgressBar>, bool)>,
}

impl ProgressTracker {
    pub(crate) fn new() -> ProgressTracker {
        ProgressTracker {
            multi_progress: MultiProgress::new(),
            tree: BarTree::Root(vec![]),
            current_count: 0,
            progress_list: vec![],
        }
    }

    pub fn start_task(
        &mut self,
        under_path: Vec<usize>,
        name: String,
        group: bool,
        anticipated_total: Option<usize>,
        progress: bool,
    ) -> (usize, Arc<indicatif::ProgressBar>) {
        let surrounding = self.tree.find_node(&under_path);
        let (surrounding_children, surrounding_pb) = match surrounding {
            BarTree::Root(children) => (children, None),
            BarTree::Group(_, pb, children, _) => (children, Some(pb)),
            _ => panic!(),
        };

        if let Some(surrounding_pb) = &surrounding_pb {
            let non_finished_count = surrounding_children
                .iter()
                .filter(|child| child.status() != LeafStatus::Finished)
                .count();
            if non_finished_count == 0 {
                self.multi_progress.remove(surrounding_pb.as_ref());
                let surrounding_idx = self
                    .progress_list
                    .iter()
                    .position(|(pb, _)| Arc::ptr_eq(pb, surrounding_pb))
                    .unwrap();
                self.progress_list[surrounding_idx].1 = false;
            } else if non_finished_count == 1 {
                let self_idx = self
                    .progress_list
                    .iter()
                    .position(|(pb, _)| Arc::ptr_eq(pb, surrounding_pb))
                    .unwrap();
                let last_visible_before = self.progress_list[..self_idx]
                    .iter()
                    .rposition(|(_, visible)| *visible);
                if let Some(last_visible_before) = last_visible_before {
                    self.multi_progress.insert_after(
                        &self.progress_list[last_visible_before].0,
                        surrounding_pb.as_ref().clone(),
                    );
                } else {
                    self.multi_progress
                        .insert(0, surrounding_pb.as_ref().clone());
                }

                self.progress_list[self_idx].1 = true;
            }
        }

        let surrounding = self.tree.find_node_mut(&under_path);
        let (surrounding_children, surrounding_pb) = match surrounding {
            BarTree::Root(children) => (children, None),
            BarTree::Group(_, pb, children, _) => (children, Some(pb)),
            _ => panic!(),
        };

        self.current_count += 1;

        let core_bar = indicatif::ProgressBar::new(100);
        let previous_bar = surrounding_children
            .iter()
            .rev()
            .flat_map(|c| c.get_pb())
            .next();

        let index_to_insert = if let Some(previous_bar) = previous_bar {
            let index_of_prev = self
                .progress_list
                .iter()
                .position(|pb| Arc::ptr_eq(&pb.0, previous_bar))
                .unwrap();
            index_of_prev + 1
        } else if let Some(group_pb) = surrounding_pb {
            let index_of_group = self
                .progress_list
                .iter()
                .position(|pb| Arc::ptr_eq(&pb.0, group_pb))
                .unwrap();
            index_of_group + 1
        } else if !self.progress_list.is_empty() {
            self.progress_list.len()
        } else {
            0
        };

        let last_visible = if !self.progress_list.is_empty() {
            self.progress_list[..index_to_insert]
                .iter()
                .rposition(|(_, visible)| *visible)
        } else {
            None
        };

        let created_bar = if let Some(last_visible) = last_visible {
            self.multi_progress
                .insert_after(&self.progress_list[last_visible].0, core_bar)
        } else {
            self.multi_progress.insert(0, core_bar)
        };

        let pb = Arc::new(created_bar);
        self.progress_list
            .insert(index_to_insert, (pb.clone(), true));
        if group {
            surrounding_children.push(BarTree::Group(name, pb.clone(), vec![], anticipated_total));
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
        let parent = self.tree.find_node_mut(&path[0..path.len() - 1]);
        match parent {
            BarTree::Root(children) | BarTree::Group(_, _, children, _) => {
                let removed = children[*path.last().unwrap()].get_pb().unwrap().clone();
                children[*path.last().unwrap()] = BarTree::Finished;
                self.multi_progress.remove(&removed);
                self.progress_list
                    .retain(|(pb, _)| !Arc::ptr_eq(pb, &removed));

                let non_finished_count = children
                    .iter()
                    .filter(|child| child.status() != LeafStatus::Finished)
                    .count();
                if let BarTree::Group(_, pb, _, _) = parent {
                    if non_finished_count == 1 {
                        self.multi_progress.remove(pb.as_ref());
                        self.progress_list
                            .iter_mut()
                            .find(|(pb2, _)| Arc::ptr_eq(pb2, pb))
                            .unwrap()
                            .1 = false;
                    } else if non_finished_count == 0 {
                        let self_idx = self
                            .progress_list
                            .iter()
                            .position(|(pb2, _)| Arc::ptr_eq(pb2, pb))
                            .unwrap();

                        let last_visible_before = self.progress_list[..self_idx]
                            .iter()
                            .rposition(|(_, visible)| *visible);

                        if let Some(last_visible_before) = last_visible_before {
                            self.multi_progress.insert_after(
                                &self.progress_list[last_visible_before].0,
                                pb.as_ref().clone(),
                            );
                        } else {
                            self.multi_progress.insert(0, pb.as_ref().clone());
                        }

                        self.progress_list[self_idx].1 = true;
                    }
                }
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
    pub fn println(msg: impl AsRef<str>) {
        let progress_bar = PROGRESS_TRACKER
            .get_or_init(|| Mutex::new(ProgressTracker::new()))
            .lock()
            .unwrap();

        progress_bar.multi_progress.suspend(|| {
            println!("{}", msg.as_ref());
        });
    }

    pub fn eprintln(msg: impl AsRef<str>) {
        let progress_bar = PROGRESS_TRACKER
            .get_or_init(|| Mutex::new(ProgressTracker::new()))
            .lock()
            .unwrap();

        progress_bar.multi_progress.suspend(|| {
            eprintln!("{}", msg.as_ref());
        });
    }

    pub fn with_group<'a, T, F: Future<Output = T>>(
        name: impl Into<String>,
        anticipated_total: Option<usize>,
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
            progress_bar.start_task(group.clone(), name.into(), true, anticipated_total, false)
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

    pub fn leaf<T, F: Future<Output = T>>(
        name: impl Into<String>,
        f: F,
    ) -> impl Future<Output = T> {
        let mut group = CURRENT_GROUP
            .try_with(|cur| cur.clone())
            .unwrap_or_default();

        let (leaf_i, _) = {
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.start_task(group.clone(), name.into(), false, None, false)
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
        name: impl Into<String>,
        f: impl FnOnce(Box<dyn Fn(String) + Send + Sync>) -> F + 'a,
    ) -> impl Future<Output = T> + 'a {
        let mut group = CURRENT_GROUP
            .try_with(|cur| cur.clone())
            .unwrap_or_default();

        let (leaf_i, bar) = {
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.start_task(group.clone(), name.into(), false, None, false)
        };

        group.push(leaf_i);

        async move {
            let my_bar = bar.clone();
            let out = f(Box::new(move |msg| {
                my_bar.set_message(msg);
            }))
            .await;
            let mut progress_bar = PROGRESS_TRACKER
                .get_or_init(|| Mutex::new(ProgressTracker::new()))
                .lock()
                .unwrap();
            progress_bar.end_task(group);
            out
        }
    }

    pub fn progress_leaf<'a, T, F: Future<Output = T>>(
        name: impl Into<String>,
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
            progress_bar.start_task(group.clone(), name.into(), false, None, true)
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
