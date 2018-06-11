/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

/// This file provide a shared box mechanism to spread a same object into several others
/// As inside the allocator we manage our memory ourself we do not manager automatic free
/// inside the container
/// 
/// **CAUTION** This is unsafe, you need to protect content by spinlock or ensure adequate usage.
/// **TODO** Hum, rust have core::ptr::Shared but not enabled by default, keep an eye on it.

//import
use common::types::Addr;
use core::marker::{Sync,Send};
use core::ptr;
use core::ops::{Deref, DerefMut};

/// Define the struct which mostly contain a raw pointer to the data to share.
#[derive(Copy)]
pub struct SharedPtrBox<T: ?Sized> {
	data: * const T,
}

impl <T: Sized> SharedPtrBox<T> {
	/// Setup the container as NULL.
	pub fn new_null() -> Self {
		Self {
			data: ptr::null(),
		}
	}

	/// Make it NULL.
	pub fn set_null(&mut self) {
		self.data = ptr::null();
	}

	/// Return pointer as an address. It panic if it is NULL.
	pub fn get_addr(&self) -> Addr {
		if self.data.is_null() {
			panic!("Try to access NULL address !");
		} else {
			self.data as Addr
		}
	}

	/// Build the container from the Addr value considered as a pointer.
	/// Usefull if need to point memory directly from mmap.
	pub fn new_addr(data: Addr) -> Self {
		Self {
			data: data as * const T,
		}
	}
}

impl <T: ?Sized> SharedPtrBox<T> {
	/// Build the container from a const reference to the data to point.
	/// **Caution**: this is usefull for unit test but in practive your need to ensure
	/// that the object live long enougth as the compiler will not check for you.
	pub fn new_ref(data: & T) -> Self {
		Self {
			data: data as * const T,
		}
	}

	/// Build the container from a const reference to the data to point.
	/// 
	/// Prefer this function to the one with const ref as it is closer to what really to the shared pointer.
	///
	/// **Caution**: this is usefull for unit test but in practive your need to ensure
	/// that the object live long enougth as the compiler will not check for you.
	pub fn new_ref_mut(data: &mut T) -> Self {
		Self {
			data: data as * const T,
		}
	}

	/// Build the container from the const pointer to object.
	/// Also prefer use the mutable version.
	/// You can also use ptr::null() to point nothing but prefer calling new_null() in this case.
	pub fn new_ptr(data: * const T) -> Self {
		Self {
			data: data,
		}
	}

	/// Build the container from the const pointer to object.
	/// You can also use ptr::null() to point nothing but prefer calling new_null() in this case.	
	pub fn new_ptr_mut(data: * mut T) -> Self {
		Self {
			data: data,
		}
	}

	/// Return a mutable refernce to the pointed object.
	/// In debug mode this crash if pointer is Null but not in release mode.
	pub fn get_mut(&mut self) -> &mut T {
		debug_assert!(!self.data.is_null());
		unsafe{&mut *(self.data as * mut T)}
	}

	/// Return a const refernce to the pointed object.
	/// In debug mode this crash if pointer is Null but not in release mode.
	pub fn get(&self) -> &T {
		debug_assert!(!self.data.is_null());
		unsafe{& *self.data}
	}

	/// Return a mutable refernce to the pointed object.
	/// This one make a real check and use Option to safely return the state to the caller if Null.
	pub fn get_safe_mut(&mut self) -> Option<&mut T> {
		if self.data.is_null() {
			None
		} else {
			Some(unsafe{&mut *(self.data as * mut T)})
		}
	}

	/// Return a const refernce to the pointed object.
	/// This one make a real check and use Option to safely return the state to the caller if Null.
	pub fn get_safe(&self) -> Option<&T> {
		if self.data.is_null() {
			None
		} else {
			Some(unsafe{& *self.data})
		}
	}

	/// Check if the container is Null.
	pub fn is_null(&self) -> bool {
		self.data.is_null()
	}

	//check if contain something
	pub fn is_some(&self) -> bool {
		! self.data.is_null()
	}

	/// Return the internal pointer. This panic if it is NULL.
	pub fn get_ptr(&self) -> * const T {
		if self.data.is_null() {
			panic!("Try to access NULL address !");
		} else {
			self.data
		}
	}

	/// Return a mutable pointer. This panic if it is NULL.
	pub fn get_ptr_mut(&mut self) -> * mut T {
		if self.data.is_null() {
			panic!("Try to access NULL address !");
		} else {
			self.data as * mut T
		}
	}
}

impl <T: ?Sized>  PartialEq for SharedPtrBox<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl <T: ?Sized> Eq for SharedPtrBox<T> {}

/// Implement deref for spin lock guard
impl<T: ?Sized> Deref for SharedPtrBox<T>
{
    type Target = T;
    fn deref(& self) -> & T { self.get() }
}

/// Implement deref mutable for spin lock guard
impl<T: ?Sized> DerefMut for SharedPtrBox<T>
{
    fn deref_mut(&mut self) ->  &mut T { self.get_mut()}
}

/// Implement clone.chunk
/// TODO this was to fix some issues but we might use [derive] for this.
impl <T: ?Sized> Clone for SharedPtrBox<T> {
	fn clone(&self) -> Self { 
		Self {
			data: self.data
		}
	}
}

/// Make it usable into threads
unsafe impl <T: ?Sized> Sync for SharedPtrBox<T> {}

/// Make it usable into closures.
unsafe impl <T: ?Sized> Send for SharedPtrBox<T> {}

#[cfg(test)]
mod tests
{
	extern crate std;

	use common::shared::*;
	use portability::spinlock::*;
	use chunk::dummy::DummyChunkManager;
	use core::mem;
	use common::traits::{ChunkManager,ChunkManagerPtr};

	#[test]
	fn basic_1_ref() {
		let a = 10;
		let copy1 = SharedPtrBox::new_ref(&a);
		let mut copy2 = copy1;
		*copy2.get_mut() = 11;
		assert_eq!(a, 11);
	}

	#[test]
	fn basic_1_ptr() {
		let a = 10;
		let copy1 = SharedPtrBox::new_ptr(& a as *const i32);
		let mut copy2 = copy1;
		*copy2.get_mut() = 11;
		assert_eq!(a, 11);
	}

	#[test]
	fn basic_2() {
		let a = 10;
		let copy1 = SharedPtrBox::new_ref(&a);
		let mut copy2 = copy1;
		*copy2.get_safe_mut().unwrap() = 11;
		assert_eq!(a, 11);
	}

	#[test]
	fn threads() {
		let mut a = SpinLock::new(0);
		let spin = SharedPtrBox::new_ref_mut(&mut a);

		let mut handlers = std::vec::Vec::new();
		let threads = 32;

		for _ in 0..threads {
			let spin = spin.clone();
			let handler = std::thread::spawn(move|| {
				let mut spin = spin.get_safe().unwrap().lock();
				*spin += 1;
				*spin += 1;
			});
			handlers.push(handler);
		}

		for handler in handlers {
			let _ = handler.join();
		}

		let res = spin.get().lock();
		assert_eq!(*res,2*threads);
	}

	#[test]
	fn is_null() {
		let a: SharedPtrBox<i32> = SharedPtrBox::new_null();
		assert_eq!(a.is_null(),true);
		let tmp = 10;
		let b = SharedPtrBox::new_ref(&tmp);
		assert_eq!(b.is_null(),false);
	}

	#[test]
	fn deref_const() {
		let a = 10;
		let copy = SharedPtrBox::new_ref(&a);
		assert_eq!(*copy,10);
	}

	#[test]
	fn deref_mut() {
		let a = 10;
		let mut copy = SharedPtrBox::new_ref(&a);
		*copy = 11;
		assert_eq!(*copy,11);
	}

	#[test]
	fn contain_trait() {
		let a = DummyChunkManager::new();
		let mut p = SharedPtrBox::new_ref(&a);
		p.get_mut().free(0);
		assert_eq!(mem::size_of::<ChunkManagerPtr>(), 16);
	}
 }
