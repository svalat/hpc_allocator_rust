/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This file define the allocator abstraction layout
///In rust this represent mostly the Trait.

use common::types::{Size};
use common::region_registry::{RegionSegment};

pub trait ChunkManager {

}

pub trait Allocator {

}

pub trait MemoryMSource {
	fn map(innerSize: Size, zeroFilled: bool, manager: & mut ChunkManager) -> (RegionSegment, bool);
	fn remap(oldSegment: *mut RegionSegment,newInnerSize: Size, manager: & mut ChunkManager) -> RegionSegment;
	fn unmap(segment: & mut RegionSegment);
	//virtual bool haveEfficientRemap(void) const = 0;
}