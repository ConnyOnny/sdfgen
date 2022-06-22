use std::cmp::Ord;
use std::cmp::Eq;
use std::cmp::Ordering;

use sdf_algorithm::DstT;

#[derive(Copy,Clone)]
pub struct SdfTask<T:DstT> {
	pub x: u32,
	pub y: u32,
	pub level: u8,
	pub best_case_dst_sqr: T,
}

impl<T:DstT> PartialEq for SdfTask<T> {
	fn eq (&self, other: &Self) -> bool {
		 self.x == other.x
		&& self.y == other.y
		&& self.level == other.level
		&& self.best_case_dst_sqr.get_dst_sqr() == other.best_case_dst_sqr.get_dst_sqr()
	}
	fn ne (&self, other: &Self) -> bool {
		!self.eq(other)
	}
}

impl<T:DstT> Eq for SdfTask<T> {}

impl<T:DstT> PartialOrd for SdfTask<T> {
	fn partial_cmp (&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<T:DstT> Ord for SdfTask<T> {
	fn cmp (&self, other: &Self) -> Ordering {
		// order swapped because we want a min-Heap
		other.best_case_dst_sqr.get_dst_sqr().partial_cmp(&self.best_case_dst_sqr.get_dst_sqr()).expect("Infinite or NaN distance shouldn't be possible for our use case")
	}
}