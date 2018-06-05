/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This implement padding structure to control alignement inside segments.
/// This consist mostly in a header struct with a couple of functions.

//import
use common::types::{Addr,Size};
use common::shared::SharedPtrBox;
use common::consts::*;
use registry::segment::RegionSegment;
use core::mem;

/// Define the header to place before the returned address. This is used
/// to unpack the padding and found the real header to be used by ChunkManager.
pub struct PaddedChunk {
	padding:u16,
	magick:u8,
}

impl PaddedChunk {
	/// Create a new padded header from RegionSegment.
	pub fn new_from_segment(seg: RegionSegment, align: Size, requested_size: Size) -> SharedPtrBox<Self> {
		seg.sanity_check();
		let padding = PaddedChunk::calc_padding_for_segment(seg, align, requested_size);
		Self::new_from_ptr(seg.get_content_addr(),padding,seg.get_inner_size())
	}

	/// Create new padded header from address.
	///
	/// @param addr Base address to padd
	/// @param padding Padding to add.
	/// @param chunk_size To check if we still fit in.
	pub fn new_from_ptr(addr: Addr, padding: Size, chunk_size: Size) -> SharedPtrBox<Self> {
		//errors
		debug_assert!(addr != 0);
		debug_assert!(chunk_size >= padding);
		debug_assert!(padding >= mem::size_of::<Self>());
		//TODO we should find a way to use size of padding directly.
		debug_assert!(padding < (1usize<<(mem::size_of::<u16>()*8)));
		
		//compute base address
		let haddr = (addr + padding) - mem::size_of::<Self>();
		let mut padded_chunk: SharedPtrBox<Self> = SharedPtrBox::new_addr(haddr);
		{
			//setup data
			let header = padded_chunk.get_mut();
			header.magick = PADDED_CHUNK_MAGICK;
			header.padding = padding as u16;
		}
		
		//return
		padded_chunk
	}

	/// Return the content address.
	pub fn get_content_addr(&self) -> Addr {
		(self as * const Self as Addr) + mem::size_of::<Self>()
	}

	/// Caclulate the padding necessary for a given segement.
	pub fn calc_padding_for_segment(segment: RegionSegment, align:Size, request_size: Size) -> Size {
		//errors
		segment.sanity_check();
		
		//calc current align
		let mut delta = segment.get_content_addr() % align;
		if delta != 0 {
			delta = align - delta;
			debug_assert!(delta >= mem::size_of::<Self>());
			if delta < mem::size_of::<Self>() {
				panic!("Cannot handle padding smaller than PaddedChunk size ! (TODO)");
			}
			/*if (delta < sizeof(PaddedChunk))
				delta += align;*/
		}

		//case
		if delta < mem::size_of::<Self>() {
			delta += align;
		}
		
		//check size
		if segment.get_inner_size() < delta + request_size {
			panic!("Segment is too small for the requested padding !");
		}
		
		delta
	}

	/// Build padding info and pad an address.
	pub fn pad(ptr:Addr, padding: Size, chunk_size: Size) -> Addr {
		let header = Self::new_from_ptr(ptr,padding,chunk_size);
		header.get().get_content_addr()
	}

	/// Unpad an address which is padded or not.
	/// Used to reverse the pad operation in all allocator operations.
	#[inline]
	pub fn unpad(ptr: Addr) -> Addr {
		//trivial
		if ptr == 0 {
			return 0;
		}

		//padded
		let header = (ptr - mem::size_of::<Self>()) as * const Self;
		let header = unsafe{&*header};
		if header.magick == PADDED_CHUNK_MAGICK {
			ptr - header.padding as Addr
		} else {
			ptr
		}
	}
}

#[cfg(test)]
mod tests
{
	use chunk::padding::PaddedChunk;
	use portability::osmem;
	use registry::segment::RegionSegment;

	#[test]
	fn unpad_1() {
		let addr = osmem::mmap(0,4096);

		let res = PaddedChunk::unpad(addr+128);
		assert_eq!(addr+128,res);

		osmem::munmap(addr,4096);
	}

	#[test]
	fn unpad_2() {
		let addr = osmem::mmap(0,4096);

		let pad = PaddedChunk::pad(addr,32,4096);
		assert_eq!(pad,addr+32);
		
		let res = PaddedChunk::unpad(pad);
		assert_eq!(addr,res);

		osmem::munmap(addr,4096);
	}

	#[test]
	fn pad_segment() {
		let addr = osmem::mmap(0,4096);
		let seg = RegionSegment::new(addr,4096,None);
		let padded = PaddedChunk::new_from_segment(seg, 64, 1024);

		let pad = padded.get_content_addr();
		assert_eq!(pad%64,0);
		
		let res = PaddedChunk::unpad(pad);
		assert_eq!(seg.get_content_addr(),res);

		osmem::munmap(addr,4096);
	}
 }