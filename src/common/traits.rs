/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This file define the allocator abstraction layout
///In rust this represent mostly the Trait.

use common::types::Size;
use registry::segment::RegionSegment;

/// A chunk manager is an object handling the sub allocation inside a macro bloc. We will
/// find many types inside the allocator : huge, medium and small with various way to handle it.
/// This chunk manager will be pointed by the RegionRegistry so we can now how to deallocate, reallocate....
/// the chunk inside a given macro bloc.
pub trait ChunkManager {

}

/// Define the interace which need to be followed by a memory allocator.
pub trait Allocator {

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
	fn map(&mut self,inner_size: Size, zero_filled: bool, manager: Option<& mut ChunkManager>) -> (RegionSegment, bool);

    /// Remap an existing segment. This on Linux directly redirect to mremap but can on some other system
    /// rely on allocator + copy + deallocation. It also take care of moving the registration into the
    /// RegionRegisty. We can also change the ChunkManager owning the segment.
	fn remap(&mut self,old_segment: RegionSegment,new_inner_size: Size, manager: Option<& mut ChunkManager>) -> RegionSegment;
	
    /// Unmap the segment. Then we can decide in the MemorySource if we keep it for latter use of if we return
    /// it to the OS.
    fn unmap(&mut self,segment: RegionSegment);

    //TODO see if we need.
	//virtual bool haveEfficientRemap(void) const = 0;
}