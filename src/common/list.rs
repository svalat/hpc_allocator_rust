/*****************************************************
             PROJECT  : hpc_allocator_rust
             VERSION  : 0.1.0-dev
             DATE     : 05/2018
             AUTHOR   : Valat Sébastien
             LICENSE  : CeCILL-C
*****************************************************/

///This module implement a double link list by using a list node stored
///into the objects we want to chain. This is to be efficient an use the
///available memory by placing the header inside the memory we want to track.

//import
use common::shared::SharedPtrBox;
use core::marker::PhantomData;

///Basic list node header to be embedded into the object to chain as a list
pub struct ListNode {
	prev: Option<SharedPtrBox<ListNode>>,
	next: Option<SharedPtrBox<ListNode>>,
}

impl ListNode {
	pub fn new() -> Self {
		Self {
			prev: None,
			next: None,
		}
	}

	pub fn init_as_loop(&mut self) {
		self.prev = Some(SharedPtrBox::new_ref_mut(self));
		self.next = Some(SharedPtrBox::new_ref_mut(self));
	}

	pub fn init_as_none(&mut self) {
		self.prev = None;
		self.next = None;
	}

	pub fn is_none(&self) -> bool {
		self.prev.is_none() || self.next.is_none()
	}

	pub fn is_loop(&self) -> bool {
		if self.prev.is_some() && self.next.is_some() {
			let pprev = self.prev.as_ref().unwrap().get_ptr();
			let pnext = self.next.as_ref().unwrap().get_ptr();
			let pself = self as * const ListNode;
			if pprev == pnext && pprev == pself {
				true
			} else {
				false
			}
		} else {
			false
		}
	}
}

pub trait Listable<T> {
	fn get_list_node<'a>(&'a self) -> &'a ListNode;
	fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode;
	fn get_from_list_node<'a>(elmt: &'a ListNode) -> &'a T;
	fn get_from_list_node_mut<'a>(elmt: &'a mut ListNode) -> &'a mut T;
}

pub struct List<T> 
	where T: Listable<T>
{
	root: ListNode,
	phantom: PhantomData<T>,
}

impl <T> List<T> 
	where T: Listable<T>
{
	pub fn new() -> Self {
		Self {
			root: ListNode::new(),
			phantom: PhantomData,
		}
	}

	pub fn is_empty(&self) -> bool {
		self.root.is_loop() || self.root.is_none()
	}

	pub fn push_back(&mut self, item: &mut T) {
		//get node of new item
		let mut item = item.get_list_node_mut();

		//if list is empty
		if self.is_empty() {
			self.root.next = Some(SharedPtrBox::new_ref_mut(&mut self.root));
			self.root.prev = Some(SharedPtrBox::new_ref_mut(&mut self.root));
		}

		//setup prev/next of new item
		item.next = Some(SharedPtrBox::new_ref_mut(&mut self.root));
		item.prev = self.root.prev.clone();

		//insert
		self.root.prev.as_mut().unwrap().get_mut().next = Some(SharedPtrBox::new_ref_mut(&mut item));
		self.root.prev = Some(SharedPtrBox::new_ref_mut(&mut item));
	}

	pub fn push_front(&mut self, item: &mut T) {
		//get node of new item
		let mut item = item.get_list_node_mut();

		//if list is empty
		if self.is_empty() {
			self.root.next = Some(SharedPtrBox::new_ref_mut(&mut self.root));
			self.root.prev = Some(SharedPtrBox::new_ref_mut(&mut self.root));
		}

		//setup prev/next of new item
		item.prev = Some(SharedPtrBox::new_ref_mut(&mut self.root));
		item.next = self.root.next.clone();

		//insert
		self.root.next.as_mut().unwrap().get_mut().prev = Some(SharedPtrBox::new_ref_mut(&mut item));
		self.root.next = Some(SharedPtrBox::new_ref_mut(&mut item));
	}

	pub fn do_hard_checking(&self) {
		if !self.is_empty() {
			let mut cur = &self.root;
			loop {
				//check
				let pcur = cur as * const ListNode;
				let pnext = cur.next.as_ref().unwrap().get_ptr();
				let pprev = cur.prev.as_ref().unwrap().get_ptr();
				assert!(pprev == pcur);
				assert!(pnext == cur);

				//move
				cur = &cur.next.as_ref().unwrap().get();
				//exit loop
				if cur as * const ListNode == &self.root as * const ListNode {
					break;
				}
			}
		}
	}

	pub fn remove(&mut self, item: & mut T) {
		//get node of new item
		let item = item.get_list_node_mut();

		//update prev
		item.prev.as_mut().unwrap().get_mut().next = item.next.clone();
		item.prev.as_mut().unwrap().get_mut().prev = item.prev.clone();

		//loop
		item.init_as_loop();
	}

	pub fn front(&self) -> Option<&T> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.next.as_ref().unwrap().get();
			Some(<T>::get_from_list_node(node))
		}
	}

	pub fn front_mut(&mut self) -> Option<&mut T> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.next.as_mut().unwrap().get_mut();
			//Hum can be cleaner but we want to bypass mutability lifetime....
			Some(<T>::get_from_list_node_mut(unsafe{&mut *(node as * mut ListNode)}))
		}
	}

	pub fn back(&self) -> Option<&T> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.prev.as_ref().unwrap().get();
			Some(<T>::get_from_list_node(node))
		}
	}

	pub fn back_mut(&mut self) -> Option<&mut T> {
		if self.is_empty() {
			None
		} else {
			let node = self.root.prev.as_mut().unwrap().get_mut();
			//Hum can be cleaner but we want to bypass mutability lifetime....
			Some(<T>::get_from_list_node_mut(unsafe{&mut *(node as * mut ListNode)}))
		}
	}

	pub fn pop_front(&mut self) -> Option<&T> {
		/*let ret = self.front_mut();
		match ret {
			Some(x) => {self.remove(x); Some(x)}
			None => None
		}*/
		//TODO
		None
	}

	//TODO 
	//T * popFirst(void);
	//T * popLast(void);
	//Iterator
}

#[cfg(test)]
mod tests
{
	use common::list::*;
	use common::types::*;

	struct Fake {
		node: ListNode,
		pub value: i32,
	}

	impl Fake {
		pub fn new(value:i32) -> Self {
			Self {
				node: ListNode::new(),
				value:value,
			}
		}
	}

	impl Listable<Fake> for Fake {
		fn get_list_node<'a>(&'a self) -> &'a ListNode {
			&self.node
		}

		fn get_list_node_mut<'a>(&'a mut self) -> &'a mut ListNode {
			&mut self.node
		}

		fn get_from_list_node<'a>(elmt: &'a ListNode) -> &'a Fake {
			unsafe{&*(elmt as * const ListNode as Addr as * const Fake)}
		}

		fn get_from_list_node_mut<'a>(elmt: &'a mut ListNode) -> &'a mut Fake {
			unsafe{&mut *(elmt as * mut ListNode as Addr as * mut Fake)}
		}
	}

	#[test]
	fn basic_empty_list_elmnt() {
		let mut el1 = ListNode::new();
		assert_eq!(el1.is_loop(), false);

		el1.init_as_loop();
		assert_eq!(el1.is_loop(), true);
	}

	#[test]
	fn basic_empty_list() {
		let el1: List<Fake> = List::new();
		assert_eq!(el1.is_empty(), true);
	}

	#[test]
	fn push_front() {
		let mut el1: List<Fake> = List::new();
		let mut v1 = Fake::new(10);
		el1.push_front(&mut v1);
		assert_eq!(el1.front().unwrap().value,10);
		assert_eq!(el1.back().unwrap().value,10);

		let mut v2 = Fake::new(11);
		el1.push_front(&mut v2);
		assert_eq!(el1.front().unwrap().value,11);
		assert_eq!(el1.back().unwrap().value,10);
	}

	#[test]
	fn push_back() {
		let mut el1: List<Fake> = List::new();
		let mut v1 = Fake::new(10);
		el1.push_front(&mut v1);
		assert_eq!(el1.front().unwrap().value,10);
		assert_eq!(el1.back().unwrap().value,10);

		let mut v2 = Fake::new(11);
		el1.push_back(&mut v2);
		assert_eq!(el1.front().unwrap().value,10);
		assert_eq!(el1.back().unwrap().value,11);
	}
}