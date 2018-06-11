/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This implement a dummy chunk manager. This is beacuse we cannot easily
///NULL a rust pointer to trait ChunkManager (size not known).
///So the clener solutions seams to push a dummy chunk manager for chunk
///waiting into the memory source.

//import
use common::traits::ChunkManager;
use common::types::{Addr,Size};
use common::shared::SharedPtrBox;

//decl
pub struct DummyChunkManager;

//impl
impl DummyChunkManager {
	pub fn new() -> DummyChunkManager {
		DummyChunkManager {}
	}
}

//impl trait
impl ChunkManager for DummyChunkManager {
	fn free(&mut self,_addr: Addr) {
       //panic!("This is fake implementation, should not be called !");
    }

	fn realloc(&mut self,_ptr: Addr,_size:Size) -> Addr {
        panic!("This is fake implementation, should not be called !");
    }

	fn get_inner_size(&mut self,_ptr: Addr) -> Size {
        panic!("This is fake implementation, should not be called !");
    }
	fn get_total_size(&mut self,_ptr: Addr) -> Size {
        panic!("This is fake implementation, should not be called !");
    }

	fn get_requested_size(&mut self,_ptr: Addr) -> Size {
        panic!("This is fake implementation, should not be called !");
    }
	
    fn hard_checking(&mut self) {
        panic!("This is fake implementation, should not be called !");
    }

	fn is_thread_safe(& self) -> bool {
        panic!("This is fake implementation, should not be called !");
    }

	fn remote_free(&mut self,_ptr: Addr) {
        panic!("This is fake implementation, should not be called !");
    }

    fn set_parent_chunk_manager(&mut self,_parent: Option<SharedPtrBox<ChunkManager>>) {
        panic!("This is fake implementation, should not be called !");
    }

    fn get_parent_chunk_manager(&mut self) -> Option<SharedPtrBox<ChunkManager>> {
        panic!("This is fake implementation, should not be called !");
    }
}
