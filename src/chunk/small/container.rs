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
use core::mem;

/// Implement container which is used to store all the runs obtained by splitting
/// a macro bloc insto runs (segs of 4K thesemve splitted for given small sizes.)
pub struct SmallChunkContainer
{
    list_node: ListNode,
    empty: List<SmallChunkRun>,
    size: Size,
    reserved_runs: Size,
}

impl SmallChunkContainer {
    /// Initizalize a new small run container onto the given allocated segment.
    /// It setup the headers and make the splitting to generate free runs and keep
    /// track of them.
    pub fn setup(ptr: Addr, size: Size) -> SmallChunkContainerPtr {
        let mut cur = SmallChunkContainerPtr::new_addr(ptr);
        cur.list_node = ListNode::new();
        cur.empty = List::new();
        cur.size = size;
        cur.reserved_runs = 0;
        cur.setup_splitting();
        return cur;
    }

    /// Check if the container is empty an do not contain anymore empty runs.
    /// This is used by the allocator to know if we need to go to another
    /// container to get free runs.
    pub fn is_empty(&self,) -> bool {
        self.reserved_runs == 0
    }

    /// Register an empty run into the container to latter reuse it or wait
    /// all the runs become free again to free the container itself.
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

    /// Request and empty run, can get None if not available.
    pub fn get_empty_run(&mut self) -> Option<SmallChunkRunPtr> {
        let res = self.empty.pop_front();

        if res.is_some() {
            self.reserved_runs += 1;
            debug_assert!(self.reserved_runs <= self.size / SMALL_RUN_SIZE + 1);
        }

        res
    }

    /// Apply the splitting by creating the runs and adding them to the free list.
    pub fn setup_splitting(&mut self) {
        //vars
        let addr = (self as * const SmallChunkContainer as Addr) + mem::size_of::<SmallChunkContainer>();
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

#[cfg(test)]
mod tests
{
	use chunk::small::container::*;
	use portability::osmem;

    #[test]
    fn setup() {
        let ptr = osmem::mmap(0, 2*1024*1024);

        let container = SmallChunkContainer::setup(ptr, 2*1024*1024);
        assert_eq!(container.is_empty(), true);

        osmem::munmap(ptr, 2*1024*1024);
    }

    #[test]
    fn is_empty() {
        let ptr = osmem::mmap(0, 2*1024*1024);

        let mut container = SmallChunkContainer::setup(ptr, 2*1024*1024);
        assert_eq!(container.is_empty(), true);

        let run = container.get_empty_run();
        assert_eq!(run.is_some(), true);
        assert_eq!(container.is_empty(), false);

        container.reg_empty(run.unwrap());
        assert_eq!(container.is_empty(), true);

        osmem::munmap(ptr, 2*1024*1024);
    }

    #[test]
    fn get_empty_run_1() {
        let ptr = osmem::mmap(0, 2*1024*1024);

        let mut container = SmallChunkContainer::setup(ptr, 2*1024*1024);
        assert_eq!(container.is_empty(), true);

        let run1 = container.get_empty_run();
        assert_eq!(run1.is_some(), true);

        let run2 = container.get_empty_run();
        assert_eq!(run2.is_some(), true);

        assert!(run1.unwrap().get_addr() != run2.unwrap().get_addr());

        osmem::munmap(ptr, 2*1024*1024);
    }

    #[test]
    fn get_empty_run_2() {
        let ptr = osmem::mmap(0, 2*1024*1024);

        let mut container = SmallChunkContainer::setup(ptr, 2*1024*1024);
        assert_eq!(container.is_empty(), true);

        let mut cnt = 0;
        loop {
            let run = container.get_empty_run();
            if run.is_none() {
                break;
            } else {
                cnt += 1;
            }
        }

        assert_eq!(2*1024*1024 / SMALL_RUN_SIZE, cnt);
 
        osmem::munmap(ptr, 2*1024*1024);
    }

    #[test]
    fn get_empty_run_3() {
        let ptr = osmem::mmap(0, 2*1024*1024);

        let mut container = SmallChunkContainer::setup(ptr, 2*1024*1024);
        assert_eq!(container.is_empty(), true);

        let mut run = container.get_empty_run().unwrap();
        run.set_splitting(16);

        loop {
            let (c,_) = run.malloc(16,16,false);
            if c != NULL {
                assert!(c >= ptr + mem::size_of::<SmallChunkContainer>());
            } else {
                break;
            }
        }
        
        osmem::munmap(ptr, 2*1024*1024);
    }
}