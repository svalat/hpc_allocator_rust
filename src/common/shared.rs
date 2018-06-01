/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This file provide a shared box mechanism to spread a same object into several others
///As inside the allocator we manage our memory ourself we do not manager automatic free
///inside the container
///**CAUTION** This is unsafe, you need to protect content by spinlock or ensure adequate usage.
///**TODO** Hum, rust have core::ptr::Shared but not enabled by default, keep an eye on it.

//import
use common::types::Addr;
use core::marker::{Sync,Send};
use core::ptr;
use core::ops::{Deref, DerefMut};

#[derive(Copy)]
pub struct SharedPtrBox<T> {
	data: * const T,
}

impl <T> SharedPtrBox<T> {
	pub fn new_null() -> Self {
		Self {
			data: ptr::null(),
		}
	}

	pub fn new_ref(data: & T) -> Self {
		Self {
			data: data as * const T,
		}
	}

	pub fn new_ref_mut(data: &mut T) -> Self {
		Self {
			data: data as * const T,
		}
	}

	pub fn new_ptr(data: * const T) -> Self {
		Self {
			data: data,
		}
	}
	
	pub fn new_ptr_mut(data: * mut T) -> Self {
		Self {
			data: data,
		}
	}

	pub fn get_mut(&mut self) -> &mut T {
		debug_assert!(!self.data.is_null());
		unsafe{&mut *(self.data as * mut T)}
	}

	pub fn get(&self) -> &T {
		debug_assert!(!self.data.is_null());
		unsafe{& *self.data}
	}

	pub fn get_safe_mut(&mut self) -> Option<&mut T> {
		if self.data.is_null() {
			None
		} else {
			Some(unsafe{&mut *(self.data as * mut T)})
		}
	}

	pub fn get_safe(&self) -> Option<&T> {
		if self.data.is_null() {
			None
		} else {
			Some(unsafe{& *self.data})
		}
	}

	pub fn is_null(&self) -> bool {
		self.data.is_null()
	}

	pub fn get_ptr(&self) -> * const T {
		if self.data.is_null() {
			panic!("Try to access NULL address !");
		} else {
			self.data
		}
	}

	pub fn get_addr(&self) -> Addr {
		if self.data.is_null() {
			panic!("Try to access NULL address !");
		} else {
			self.data as Addr
		}
	}
}

///Implement deref for spin lock guard
impl<T> Deref for SharedPtrBox<T>
{
    type Target = T;
    fn deref(& self) -> & T { self.get() }
}

///Implement deref mutable for spin lock guard
impl<T> DerefMut for SharedPtrBox<T>
{
    fn deref_mut(&mut self) ->  &mut T { self.get_mut()}
}

impl <T> Clone for SharedPtrBox<T> {
	fn clone(&self) -> Self { 
		Self {
			data: self.data
		}
	}
}

unsafe impl <T> Sync for SharedPtrBox<T> {}
unsafe impl <T> Send for SharedPtrBox<T> {}

#[cfg(test)]
mod tests
{
	extern crate std;

	use common::shared::*;
	use portability::spinlock::*;

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
 }
