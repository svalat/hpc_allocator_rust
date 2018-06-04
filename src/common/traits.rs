/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This file define the allocator abstraction layout
///In rust this represent mostly the Trait.

use common::types::{Size,Addr};
use registry::segment::RegionSegment;

/// A chunk manager is an object handling the sub allocation inside a macro bloc. We will
/// find many types inside the allocator : huge, medium and small with various way to handle it.
/// This chunk manager will be pointed by the RegionRegistry so we can now how to deallocate, reallocate....
/// the chunk inside a given macro bloc.
///
/// The chunk manager provide only function to handle the already allocated chunks, so
/// free/realloc/request size... The malloc function itself is not needed in this abstraction
/// and it provided by the allocator trait.
///
/// This is because the chunk manager are registered into the region registry to 
/// handle alive chunks. In practive they also export a malloc function but which
/// is not needed in the abstract view (ChunkManager).
pub trait ChunkManager {
    /// Free the given address.
	fn free(&mut self,addr: Addr);

    /// Realloc the given address. Might fail if it is not owned by current allocator.
    /// @param ptr Old address to reallocate. This must match with an address returned by malloc.
    /// @param size Define the new size.
	fn realloc(&mut self,ptr: Addr,size:Size) -> Addr;

    /// Retutn the inner size of the allocated segment (could be larger than requested).
	fn get_inner_size(&mut self,ptr: Addr) -> Size;

    /// Return the total size of the allocated segment (considering the allocator headers).
    /// This account the header manager by current level, not adding the macro blocs manager
    /// in which the allocation is embdeded except for huge allocation which are directly placed
    /// into a macro bloc.
	fn get_total_size(&mut self,ptr: Addr) -> Size;

    /// Return the requested size when available otherwise return the actual inner size.
	fn get_requested_size(&mut self,ptr: Addr) -> Size;
	
    /// Make safety checking to help debugging
    fn hard_checking(&mut self,);

	/// Check if the chunk manager if by itself thread safe, this can avoid to take some locks
    /// into the upper layer.
	fn is_thread_safe(&self) -> bool;

    /// Registry the given segment as a remote free. It can be handled now or latter
    /// on call for flush_remote().
	fn remote_free(&mut self,ptr: Addr);

    /// attach a parent to the current chunk manager if not already done at build time.
    fn set_parent_chunk_manager(&mut self,parent: Option<* mut ChunkManager>);

    /// get the current parent chunk manager if has a hierarchie.
    /// This is used to support remote free and realloc going from one chunk to another.
    fn get_parent_chunk_manager(&mut self) -> Option<* mut ChunkManager>;
}

/// Define the interace which need to be followed by a memory allocator.
pub trait Allocator: ChunkManager
{
    /// Allocate a chunk of given size and alignement and with given zero constrain.
    /// It return a tuple with the given address (0 if fail) and a boolean telling if the
    /// memoury has already been zeroed.
    fn malloc(&mut self,size: Size,align: Size,zero_filled: bool) -> (Addr,bool);

    /// Check if the given chunk manager is the current one
	fn is_local_chunk_manager(&self, manager: * const ChunkManager) -> bool;

    /// Apply flush operation on pending remote frees registred into the chunk manager
    fn flush_remote(&mut self);
}

/// Define a memory source which is used to allocate, deallocate and resize macro blocs. It also
/// required ChunkManager to link it to the segments it produce and register them inside the 
/// global region registry.
pub trait MemorySource {
    /// Map a new macro bloc with the given size and register it to the RegionRegistry.
    /// 
    /// @param inner_size Define the size which can be stored into the macro bloc (so we need to add header size to allocate)
    /// @param zero_filled Tell if the chunk need to be zeroed or not. This is usefull to make some optimization about zeroing.
    ///                     Notice this is for possible optimization with the OS (eg. our zeroing kernel patch. 
    ///                     If the segment you return is not zeroed you don't
    ///                     strictly have to zero it, you can delay it to the caller function by returning false in return value.
    ///                     This is better to make the zeroing in the last layer of allocator as is might init a smaller part of the
    ///                     macro bloc if he split it and keep the rest for latter use.
    /// @parma manager Optionally define a chunk manager to be used to register the segment into the region registry.
    /// 
    /// @return Return the RegionSegment and a boolean telling is the segment has been zeroed of not.
	fn map(&mut self,inner_size: Size, zero_filled: bool, manager: Option<* mut ChunkManager>) -> (RegionSegment, bool);

    /// Remap an existing segment. This on Linux directly redirect to mremap but can on some other system
    /// rely on allocator + copy + deallocation. It also take care of moving the registration into the
    /// RegionRegisty. We can also change the ChunkManager owning the segment.
	fn remap(&mut self,old_segment: RegionSegment,new_inner_size: Size, manager: Option<* mut ChunkManager>) -> RegionSegment;
	
    /// Unmap the segment. Then we can decide in the MemorySource if we keep it for latter use of if we return
    /// it to the OS.
    fn unmap(&mut self,segment: RegionSegment);

    //TODO see if we need.
	//virtual bool haveEfficientRemap(void) const = 0;
}