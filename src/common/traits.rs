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
use registry::segment::{RegionSegment};

pub trait ChunkManager {

}

pub trait Allocator {

}

pub trait MemorySource {
	fn map(&mut self,inner_size: Size, zero_filled: bool, manager: Option<& mut ChunkManager>) -> (RegionSegment, bool);
	fn remap(&mut self,old_segment: RegionSegment,new_inner_size: Size, manager: Option<& mut ChunkManager>) -> RegionSegment;
	fn unmap(&mut self,segment: RegionSegment);
	//virtual bool haveEfficientRemap(void) const = 0;
}