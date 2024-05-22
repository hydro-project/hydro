use stageleft::{Quoted, RuntimeData};

use super::{Cluster, LocalDeploy, Location, ProcessSpec};

pub struct SingleProcessGraph {}

impl<'a> LocalDeploy<'a> for SingleProcessGraph {
    type ClusterId = ();
    type Process = SingleNode;
    type Cluster = SingleNode;
    type Meta = ();
    type GraphId = ();
}

impl<'a> ProcessSpec<'a, SingleProcessGraph> for () {
    fn build(&self, _id: usize, _meta: &mut ()) -> SingleNode {
        SingleNode {}
    }
}

#[derive(Clone)]
pub struct SingleNode {}

impl Location for SingleNode {
    type Port = ();
    type Meta = ();

    fn id(&self) -> usize {
        0
    }

    fn next_port(&self) {
        panic!();
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {}
}

impl<'a> Cluster<'a> for SingleNode {
    type Id = ();

    fn ids(&self) -> impl Quoted<'a, &'a Vec<()>> + Copy + 'a {
        panic!();
        #[allow(unreachable_code)]
        RuntimeData::new("")
    }

    fn self_id(&self) -> impl Quoted<'a, Self::Id> + Copy + 'a {
        panic!();
        #[allow(unreachable_code)]
        RuntimeData::new("")
    }
}

pub struct MultiGraph {}

impl<'a> LocalDeploy<'a> for MultiGraph {
    type ClusterId = u32;
    type Process = MultiNode;
    type Cluster = MultiNode;
    type Meta = ();
    type GraphId = usize;
}

impl<'a> ProcessSpec<'a, MultiGraph> for () {
    fn build(&self, id: usize, _meta: &mut ()) -> MultiNode {
        MultiNode { id }
    }
}

#[derive(Clone)]
pub struct MultiNode {
    id: usize,
}

impl Location for MultiNode {
    type Port = ();
    type Meta = ();

    fn id(&self) -> usize {
        self.id
    }

    fn next_port(&self) {
        panic!();
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {}
}

impl<'a> Cluster<'a> for MultiNode {
    type Id = u32;

    fn ids(&self) -> impl Quoted<'a, &'a Vec<u32>> + Copy + 'a {
        panic!();
        #[allow(unreachable_code)]
        RuntimeData::new("")
    }

    fn self_id(&self) -> impl Quoted<'a, Self::Id> + Copy + 'a {
        panic!();
        #[allow(unreachable_code)]
        RuntimeData::new("")
    }
}
