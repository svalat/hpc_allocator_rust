/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This implement the medium chunk allocator by using internally the MediumFreePool
/// and MidiumChunk.

//import
use portability::spinlock::SpinLock;
use common::traits::{ChunkManager,ChunkManagerPtr,MemorySourcePtr};
use registry::registry::RegionRegistry;
use common::types::{Addr,Size,SSize};
use common::consts::*;
use common::ops;
use chunk::padding::PaddedChunk;
use common::shared::SharedPtrBox;
use core::mem;
use registry::segment::RegionSegment;
use portability::libc;
use common::list::List;
use chunk::small::run::{SmallChunkRun,SmallChunkRunPtr};
use chunk::small::container::{SmallChunkContainer,SmallChunkContainerPtr};

//consts
const NB_SIZE_CLASS: usize = 10;
const SMALL_CHUNK_MAX_SIZE: usize = 128;
const SMALL_SIZE_CLASSES: [Size;NB_SIZE_CLASS] = [8, 16, 24, 32, 48, 64, 80, 96, 112, 128];
//8 16 24 32 48 64 80 96 128

/// Group content to protect by spinlock
struct SmallChunkManagerLocked {
	mmsource: Option<MemorySourcePtr>,
    active_runs: [Option<SmallChunkRunPtr>; NB_SIZE_CLASS],
    in_use: [List<SmallChunkRun>; NB_SIZE_CLASS],
    containers: List<SmallChunkContainer>,
}

/// Implement the small chunk allocator based on MediumFreePool
pub struct SmallChunkManager {
	locked: SpinLock<SmallChunkManagerLocked>,
	use_lock: bool,
	parent: Option<ChunkManagerPtr>,
}

//implement
impl SmallChunkManager {
	/// Construct a new chunk manager.
	/// 
	/// @param use_lock Define if we use spinlocks to protect the shared state or not.
	/// This make the code more efficient if used inside thread local alloctor.
	/// @param mmsource Define the memory source to use to fetch macro blocs.
	pub fn new(use_lock: bool, mmsource: Option<MemorySourcePtr>) -> Self {
		Self {
			locked: SpinLock::new(SmallChunkManagerLocked {
				mmsource: mmsource,
                active_runs: [None,None,None,None,None,None,None,None,None,None],
                in_use: [List::new(); NB_SIZE_CLASS],
                containers: List::new(), 
			}),
			use_lock: use_lock,
			parent: None,
		}
	}

	/// Allocate a new segment.
	pub fn malloc(&mut self, mut size: Size, align:Size, zero_filled: bool) -> (Addr,bool) {
        //check align
        if align != BASIC_ALIGN {
            panic!("TODO support align");
        }

        //round if smallest size to avoid checking warning of filling ratio in SmallChunkRun
        if size < SMALL_SIZE_CLASSES[0] {
            size = SMALL_SIZE_CLASSES[0];
        }

        //get related size class
        let size_class = Self::get_size_class(size);
        debug_assert!(SMALL_SIZE_CLASSES[size_class] % align == 0);

        //lock
        let mut res = NULL;
        {
            let mut handler = self.locked.optional_lock(self.use_lock);

            //get active run for class
            {
                let mut run = &mut (handler.active_runs[size_class]);

                //try to alloc
                match run {
                    Some(ref mut run) => res = run.malloc(size,align,zero_filled).0,
                    None => {},
                }
            }

            if res == NULL {
                let mut run = Self::upate_active_run_for_size(&mut handler,size_class);
                match run {
                    Some(mut run) => res = run.malloc(size,align,zero_filled).0,
                    None => {},
                }
            }
        }

        //check
        debug_assert!(res == NULL || res % align == 0);

        //ret
        return (res,false);
	}

	pub fn rebind_mm_source(&mut self,mmsource: Option<MemorySourcePtr>) {
		self.locked.lock().mmsource = mmsource;
	}

	fn fill(&mut self,ptr: Addr, size: Size, registry: Option<SharedPtrBox<RegionRegistry>>, lock:bool) {
		//errors
        debug_assert!(ptr != NULL);

        //if need register, create macro bloc
        let mut addr = ptr;
        let mut size = size;
        match registry {
            Some(mut registry) => {
               let segment = registry.get_mut().set_entry(addr,size,ChunkManagerPtr::new_ref(self));
                addr = segment.get_content_addr();
                size = segment.get_inner_size();
            },
            None => {}
        }
        
        //setup run container
        let container = SmallChunkContainer::setup(addr,size);

        //reg in list
        {
            let mut handler = self.locked.optional_lock(self.use_lock);
            handler.containers.push_back(container);
        }
	}

    //8, 16, 24, 32, 48, 64, 80, 96, 128
    fn get_size_class(mut size: Size) -> usize {
        //errors
        debug_assert!(SMALL_SIZE_CLASSES.len() / mem::size_of::<Size>() == NB_SIZE_CLASS);
        debug_assert!(size <= SMALL_CHUNK_MAX_SIZE);
        debug_assert!(size > 0);

        //trivial
        if size > SMALL_CHUNK_MAX_SIZE {
            panic!("Invalid too big size !");
        }
        
        //if too small
        if size < 8 {
            size = 8;
        }
        
        //calc from 8 to 32
        let res;
        if size <= 32 {
            res = (size - 1) / 8;
        } else {
            res = (size - 1) / 16 + 2;
        }

        debug_assert!(SMALL_SIZE_CLASSES[res] >= size);
        if res > 0 {
            debug_assert!(SMALL_SIZE_CLASSES[res-1] < size);	
        }

        return res;
    }

    fn mark_run_as_free(&mut self, mut run: SmallChunkRunPtr) {
        //errors
        debug_assert!(run.is_some());
        debug_assert!(run.is_empty());
        
        //reg empty
        let mut container = run.get_container();
        debug_assert!(container.is_some());

        //take lock
        {
            //take
            let mut handler = self.locked.optional_lock(self.use_lock);

            //check current usage
            let size_class = Self::get_size_class(run.get_splitting() as usize);

            //if
            match handler.active_runs[size_class] {
                Some(ref mut r) => {
                    if r.get_addr() == run.get_addr() {
                        *r = run.clone();
                    } else {
                        List::remove(&mut run);
                    }
                },
                None => {
                    List::remove(&mut run);
                }
            }
            
            //register as free
            run.set_splitting(0);
            container.reg_empty(run);
            
            //if container is empty, remove it
            if container.is_empty() && handler.mmsource.is_some() {
                List::remove(&mut container);
                handler.mmsource.as_mut().unwrap().unmap(RegionSegment::get_from_content_ptr(container.get_addr()));
            }
        }
    }

    fn find_empty_run(&mut self) -> Option<SmallChunkRunPtr> {
        //search in containers
        for mut it in self.locked.optional_lock(self.use_lock).containers.iter()
        {
            match it.get_empty_run() {
                Some(res) => {return Some(res)},
                None => {},
            }
        }
        
        return None;
    }

    fn upate_active_run_for_size(handler: &mut SmallChunkManagerLocked, size_class: usize) -> Option<SmallChunkRunPtr> {
        unimplemented!();
    }

    fn get_activ_run_for_size(&self,size_class: usize) -> Option<SmallChunkRun> {
        unimplemented!();
    }

    fn get_run(&self, addr: Addr) -> Option<SmallChunkRunPtr> {
        unimplemented!();
    }
}

impl ChunkManager for SmallChunkManager {
	fn free(&mut self,addr: Addr) {
        unimplemented!();
	}

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
        unimplemented!();
	}

	fn get_inner_size(&mut self,ptr: Addr) -> Size {
		unimplemented!();
	}

    fn get_total_size(&mut self,ptr: Addr) -> Size {
		unimplemented!();
	}

    fn get_requested_size(&mut self,_ptr: Addr) -> Size {
		UNSUPPORTED
	}
	
    fn is_thread_safe(&self) -> bool {
		self.use_lock
	}

    fn remote_free(&mut self,ptr: Addr) {
		unimplemented!();
	}

    fn set_parent_chunk_manager(&mut self,parent: Option<ChunkManagerPtr>) {
		self.parent = parent;
	}

    fn get_parent_chunk_manager(&mut self) -> Option<ChunkManagerPtr> {
		self.parent.clone()
	}

    fn hard_checking(&mut self) {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests
{
	use chunk::medium::manager::*;
	use mmsource::dummy::DummyMMSource;
	use registry::registry::RegionRegistry;
	use portability::osmem;
	use chunk::padding;	
}