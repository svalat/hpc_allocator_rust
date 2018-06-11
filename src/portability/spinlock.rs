/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///Import pthread_spinlock from C libray on posix systems
///Need to look if at some point it will appear in crate libc...

//import
extern crate libc;

//import
use core::ops::{Drop, Deref, DerefMut};

//low level spincloks (pthread_spinlock_t) are int (from /usr/include/bits/pthreadtypes.h)
type PthreadSpinLock = libc::c_ulong;
const PTHREAD_PROCESS_PRIVATE: libc::c_int = 0;

//declare extern funcs
extern {
	fn pthread_spin_init(lock: * const PthreadSpinLock, pshared:libc::c_int) -> libc::c_int;
	fn pthread_spin_lock(lock: * const PthreadSpinLock) -> libc::c_int;
	fn pthread_spin_unlock(lock: * const PthreadSpinLock) -> libc::c_int;
	fn pthread_spin_destroy(lock: * const PthreadSpinLock) -> libc::c_int;
}

///built object to hide the lowlevel funcs
pub struct SpinLock<T> {
	lock: PthreadSpinLock,
	data: T,
}

///Implement guard mechanism
pub struct SpinLockGuard<'a, T:'a>
{
    lock: &'a PthreadSpinLock,
    data: &'a mut T,
	unlock: bool,
}

///Implement spinlock
impl <T> SpinLock<T> {
	///Construct the spinlock and embed the content in it
	pub fn new(obj: T) -> Self {
		let ret = Self {
			lock: 0,
			data: obj, 
		};

		let ptr = &ret.lock as * const PthreadSpinLock;
		let status = unsafe{pthread_spin_init(ptr,PTHREAD_PROCESS_PRIVATE)};
		if status != 0 {
			panic!("Fail to init pthread spinlock !");
		}
		
		ret
	}

	///lock
 	pub fn lock(&self) -> SpinLockGuard<T>
    {
		let ptr = &self.lock as * const PthreadSpinLock;
        unsafe{pthread_spin_lock(ptr)};
        SpinLockGuard
        {
            lock: &self.lock,
            data: unsafe{&mut *(&self.data as * const T as * mut T)},
			unlock: true,
        }
    }

	///lock
 	pub fn optional_lock(&self, lock: bool) -> SpinLockGuard<T>
    {
		let ptr = &self.lock as * const PthreadSpinLock;
		if lock {
        	unsafe{pthread_spin_lock(ptr)};
		}

        SpinLockGuard
        {
            lock: &self.lock,
            data: unsafe{&mut *(&self.data as * const T as * mut T)},
			unlock: lock,
        }
    }

	///Special case, consider read unlock for some struct of the allocator which
	///are built and use with this constrain (eg. the region registry).
	pub fn nolock_safe_read<'a>(&'a self) -> &'a T {
		unsafe{& *(&self.data as * const T)}
	}
}

///Implement deref for spin lock guard
impl<'a, T> Deref for SpinLockGuard<'a, T>
{
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T { &*self.data }
}

///Implement deref mutable for spin lock guard
impl<'a, T> DerefMut for SpinLockGuard<'a, T>
{
    fn deref_mut<'b>(&'b mut self) -> &'b mut T { &mut *self.data }
}

///Implement drop for spin lock guard
impl<'a, T> Drop for SpinLockGuard<'a, T>
{
    fn drop(&mut self)
    {
        if self.unlock {
			let ptr = self.lock as * const PthreadSpinLock;
        	unsafe{pthread_spin_unlock(ptr)};
		}
    }
}

#[cfg(test)]
mod tests
{
	extern crate std;

	use portability::spinlock::*;

	#[test]
	fn serial() {
		let spin = SpinLock::new(0);
		*spin.lock() += 1;
		*spin.lock() += 1;
		*spin.lock() += 1;
		assert_eq!(*spin.lock(), 3);
	}

	#[test]
	fn parallel() {
		let spin = std::sync::Arc::new(SpinLock::new(0));
		let mut handlers = std::vec::Vec::new();
		let threads = 32;

		for _ in 0..threads {
			let spin = spin.clone();
			let handler = std::thread::spawn(move|| {
				let mut spin = spin.lock();
				*spin += 1;
				*spin += 1;
			});
			handlers.push(handler);
		}

		for handler in handlers {
			let _ = handler.join();
		}

		let res = spin.lock();
		assert_eq!(*res,2*threads);
	}
}