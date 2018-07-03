/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// A container is there to contain many runs to fit in a macro bloc.

//import
use common::shared::SharedPtrBox;
use common::types::{Addr,Size};
use common::list::{List,ListNode,Listable};
use chunk::small::run::{SMALL_RUN_SIZE,SmallChunkRun,SmallChunkRunPtr};
use common::ops;
use common::consts::*;

/// Implement container
pub struct SmallChunkContainer
{
    list_node: ListNode,
    empty: List<SmallChunkRun>,
    size: Size,
    reserved_runs: Size,
}

impl SmallChunkContainer {
    pub fn setup(ptr: Addr, size: Size) -> SmallChunkContainerPtr {
        let mut cur = SmallChunkContainerPtr::new_addr(ptr);
        cur.list_node = ListNode::new();
        cur.empty = List::new();
        cur.size = size;
        cur.reserved_runs = 0;
        cur.setup_splitting();
        return cur;
    }

    pub fn is_empty(&self,) -> bool {
        self.reserved_runs == 0
    }

    pub fn reg_empty(&mut self,run: SmallChunkRunPtr) {
        let local = self as * const SmallChunkContainer as Addr;
        debug_assert!(!run.is_null());
        debug_assert!(run.is_empty());
        debug_assert!(run.get_addr() >= ops::ceil_to_power_of_2(local,SMALL_PAGE_SIZE));
        debug_assert!(run.get_addr() < local + self.size);

        if run.is_null() == false {
            debug_assert!(self.reserved_runs > 0);
            self.empty.push_back(run);
            self.reserved_runs -= 1;
        }
    }

    pub fn get_empty_run(&mut self) -> Option<SmallChunkRunPtr> {
        let res = self.empty.pop_front();

        if res.is_some() {
            self.reserved_runs += 1;
            debug_assert!(self.reserved_runs <= self.size / SMALL_RUN_SIZE + 1);
        }

        res
    }

    pub fn setup_splitting(&mut self) {
        //vars
        let addr = self as * const SmallChunkContainer as Addr;
        let ptr_start = ops::ceil_to_power_of_2(addr, SMALL_RUN_SIZE);
        let ptr_end = ops::ceil_to_power_of_2(addr+self.size, SMALL_RUN_SIZE);
        let cnt = (ptr_end - ptr_start) / SMALL_RUN_SIZE;

        //all
        for i in 0..cnt {
            //cur
            let cur = ptr_start + i * SMALL_RUN_SIZE;

            //calc skip
            let skip;
            if cur < addr {
                skip = addr - cur;
            } else {
                skip = 0;
            }
            debug_assert!(skip < SMALL_RUN_SIZE);

            //create run
            let container = SmallChunkContainerPtr::new_ref(self);
            let run = SmallChunkRun::setup(cur, skip as u16, 0, container);

            //insert
            self.empty.push_back(run);
        }
    }
}

impl Listable<SmallChunkContainer> for SmallChunkContainer {
	fn get_list_node<'a>(&'a self) -> &'a ListNode {
        return &self.list_node;
    }

	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
        return &mut self.list_node;
    }

	fn get_from_list_node<'a>(elmt: * const ListNode) -> * const SmallChunkContainer {
        return (elmt as Addr) as * const SmallChunkContainer
    }

	fn get_from_list_node_mut<'a>(elmt: * mut ListNode) -> * mut SmallChunkContainer {
        return (elmt as Addr) as * mut SmallChunkContainer
    }
}

/// Pointer
pub type SmallChunkContainerPtr = SharedPtrBox<SmallChunkContainer>;