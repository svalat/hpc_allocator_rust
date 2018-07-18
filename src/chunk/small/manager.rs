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
use common::types::{Addr,Size};
use common::consts::*;
use common::ops;
use common::shared::SharedPtrBox;
use core::mem;
use registry::segment::RegionSegment;
use portability::libc;
use common::list::List;
use chunk::small::run::{SmallChunkRun,SmallChunkRunPtr,SMALL_RUN_SIZE};
use chunk::small::container::{SmallChunkContainer};

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
                let run = &mut (handler.active_runs[size_class]);

                //try to alloc
                match run {
                    Some(ref mut run) => res = run.malloc(size,align,zero_filled).0,
                    None => {},
                }
            }

            if res == NULL {
                let run = handler.upate_active_run_for_size(size_class,ChunkManagerPtr::new_ref(self));
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

	fn fill(&mut self,ptr: Addr, size: Size, registry: Option<SharedPtrBox<RegionRegistry>>) {
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

    fn mark_run_as_free(handler: &mut SmallChunkManagerLocked, mut run: SmallChunkRunPtr) {
        //errors
        debug_assert!(run.is_some());
        debug_assert!(run.is_empty());
        
        //reg empty
        let mut container = run.get_container();
        debug_assert!(container.is_some());

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

    fn get_run(&self, ptr: Addr) -> Option<SmallChunkRunPtr> {
        //trivial
		if ptr == NULL {
			return None;
		}
		
		//round add
		let run = SmallChunkRunPtr::new_addr(ops::ceil_to_power_of_2(ptr, SMALL_RUN_SIZE));
		
		//check 
		debug_assert!(run.contain(ptr));
		if run.contain(ptr) {
			return Some(run);
		} else {
			return None;
		}
    }
}

impl ChunkManager for SmallChunkManager {
	fn free(&mut self,ptr: Addr) {
        //trivial
		if ptr == NULL {
			return;
		}
		
		//find small chunk
		let run = self.get_run(ptr);
		debug_assert!(run.is_some());

		//if found
		match run {
			Some(mut run) => {
				//lock
				let mut handler = self.locked.optional_lock(self.use_lock);

				//free
				run.free(ptr);
				
				//if empty move to empty list
				if run.is_empty() {
					Self::mark_run_as_free(&mut handler,run);
				}
			},
			None => {},
		}
	}

	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr {
        //trivial cases
		if ptr == NULL {
			return self.malloc(size,BASIC_ALIGN,false).0;
		} else if size == 0 {
			self.free(ptr);
			return NULL;
		}
		
		//get size classes
		let old_run = self.get_run(ptr);
		if old_run.is_none() {
			panic!("Invalid old pointer for realloc on SmallAllocator cannot proceed to keep data !");
		}
		let old_run = old_run.unwrap();
		let old_class = Self::get_size_class(old_run.get_splitting() as usize);
		let new_class = Self::get_size_class(size);
		
		//if same class, do nothing, otherwise to realloc
		if new_class == old_class {
			return ptr;
		}
		
		//alloc, copy, free
		let (res,_) = self.malloc(size,BASIC_ALIGN,false);
		if res != NULL {
			let mut cpy_size = old_run.get_splitting() as usize;
			if size < cpy_size {
				cpy_size = size;
			}
			libc::memcpy(res,ptr,cpy_size);
		}
		self.free(ptr);
		
		//ok return the segment
		return res;
	}

	fn get_inner_size(&mut self,ptr: Addr) -> Size {
		//get the run to request the size
		let run = self.get_run(ptr);
		debug_assert!(run.is_some());
		
		//case
		match run {
			Some(run) => run.get_inner_size(ptr),
			None => 0,
		}
	}

    fn get_total_size(&mut self,ptr: Addr) -> Size {
		//get the run to request the size
		let run = self.get_run(ptr);
		debug_assert!(run.is_some());
		
		//case
		match run {
			Some(run) => run.get_total_size(ptr),
			None => 0,
		}
	}

    fn get_requested_size(&mut self,ptr: Addr) -> Size {
		//get the run to request the size
		let run = self.get_run(ptr);
		debug_assert!(run.is_some());
		
		//case
		match run {
			Some(run) => run.get_requested_size(ptr),
			None => 0,
		}
	}
	
    fn is_thread_safe(&self) -> bool {
		self.use_lock
	}

    fn remote_free(&mut self,ptr: Addr) {
		if self.use_lock {
			self.free(ptr);
		} else {
			panic!("Unsupported remote free without locks.");
		}
	}

    fn set_parent_chunk_manager(&mut self,parent: Option<ChunkManagerPtr>) {
		self.parent = parent;
	}

    fn get_parent_chunk_manager(&mut self) -> Option<ChunkManagerPtr> {
		self.parent.clone()
	}

    fn hard_checking(&mut self) {
        //TODO
    }
}

impl SmallChunkManagerLocked {
	fn refill(&mut self,manager:ChunkManagerPtr) {
		//trivial
		if self.mmsource.is_none() {
			return;
		}
		
		//request mem
		let (segment,_) = self.mmsource.as_mut().unwrap().map(REGION_SPLITTING-mem::size_of::<RegionSegment>(),false,Some(manager));
		if segment.is_null() {
			return;
		}
		debug_assert!(segment.get_total_size() == REGION_SPLITTING);
		
		//get inner segment
		let ptr = segment.get_content_addr();
		
		//build chunk
		let inner_size = segment.get_inner_size();
		
		//setup run container
        let container = SmallChunkContainer::setup(ptr,inner_size);

		//register to list
		self.containers.push_back(container);
	}

	fn find_empty_run(&mut self) -> Option<SmallChunkRunPtr> {
        //search in containers
        for mut it in self.containers.iter()
        {
            match it.get_empty_run() {
                Some(res) => {return Some(res)},
                None => {},
            }
        }
        
        return None;
    }

    fn upate_active_run_for_size(&mut self, size_class: usize, manager:ChunkManagerPtr) -> Option<SmallChunkRunPtr> {
        //errors
        debug_assert!(size_class < NB_SIZE_CLASS);
        match self.active_runs[size_class] {
            Some(ref mut r) => debug_assert!(r.is_full()),
			None => {},
        }

        //search in list
        let mut run = None;
        for ref mut it in self.in_use[size_class].iter() {
            if it.is_full() == false {
                run = Some(it.clone());
                List::remove(it);
                break;
            }
        }
        
        //if have not, try in empty list
        if run.is_none() {
            run = self.find_empty_run();
            //need to refill
            if run.is_none() {
                self.refill(manager);
                run = self.find_empty_run();
            }
            //setup splitting in run
            match run {
                Some(ref mut r) => r.set_splitting(SMALL_SIZE_CLASSES[size_class] as u16),
				None => {},
			}
        }

        //if have one
        if run.is_some() {
            //insert in FIFO
			match self.active_runs[size_class] {
                Some(ref r) => self.in_use[size_class].push_back(r.clone()),
				None => {}
			}

            self.active_runs[size_class] = run.clone();
        }

        //return it
        return run;
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