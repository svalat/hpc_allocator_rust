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
use chunk::medium::pools::{ChunkInsertMode,MediumFreePool};
use chunk::medium::chunk::*;
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
//8 16 24 32 48 64 80 96 128

/// Group content to protect by spinlock
struct SmallChunkManagerLocked {
	pools: MediumFreePool,
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
				pools: MediumFreePool::new(),
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
	pub fn malloc(&mut self, size: Size, align:Size, zero_filled: bool) -> (Addr,bool) {
        unimplemented!();
	}

	pub fn rebind_mm_source(&mut self,mmsource: Option<MemorySourcePtr>) {
		self.locked.lock().mmsource = mmsource;
	}

	fn fill(&mut self,addr: Addr, size: Size, registry: Option<SharedPtrBox<RegionRegistry>>, lock:bool) {
		unimplemented!();
	}

    fn get_size_class(size: Size) -> usize {
        unimplemented!();
    }

    fn mark_run_as_free(&mut self, run: SmallChunkRunPtr) {
        unimplemented!();
    }

    fn find_empty_run(&mut self) -> Option<SmallChunkRunPtr> {
        unimplemented!();
    }

    fn upate_active_run_for_size(&mut self, size_class: usize) {
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