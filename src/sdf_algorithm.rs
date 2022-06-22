extern crate image;
extern crate num;
extern crate rayon;

use self::image::GrayImage;
use self::image::ImageBuffer;
pub type SDFImage = image::ImageBuffer<image::Luma<f64>, Vec<f64>>;

use std::f32;
use std::sync::Arc;
use std::cmp;
use self::rayon::prelude::*;

use mipmap::*;
use functions::*;
use sdf_task::SdfTask;

use std::cmp::Ord;
use std::cmp::Eq;
use std::cmp::Ordering;
use std::fmt::Debug;

pub trait DstT: Eq + Ord + Copy + Clone + Debug {
	fn get_dst_sqr(&self) -> f64;
	fn new(p1:&UniPoint, p2:&UniPoint) -> Self;
}

#[derive(Clone,Copy,Debug)]
pub struct SimpleDstT {
	dst_sqr: f64,
}

impl PartialEq for SimpleDstT {
	fn eq (&self, other: &Self) -> bool {
		self.get_dst_sqr() == other.get_dst_sqr()
	}
	fn ne (&self, other: &Self) -> bool {
		!self.eq(other)
	}
}

impl Eq for SimpleDstT {}

impl PartialOrd for SimpleDstT {
	fn partial_cmp (&self, other: &Self) -> Option<Ordering> {
		self.get_dst_sqr().partial_cmp(&other.get_dst_sqr())
	}
}

impl Ord for SimpleDstT {
	fn cmp (&self, other: &Self) -> Ordering {
		// order swapped because we want a min-Heap
		other.get_dst_sqr().partial_cmp(&self.get_dst_sqr()).expect("Infinite or NaN distance shouldn't be possible for our use case")
	}
}

impl DstT for SimpleDstT {
	fn get_dst_sqr(&self) -> f64 {
		self.dst_sqr
	}
	fn new(p1:&UniPoint, p2:&UniPoint) -> Self {
		let dx = (p1.x as i32 - p2.x as i32) as f64;
		let dy = (p1.y as i32 - p2.y as i32) as f64;
		let dst_sqr = dx*dx+dy*dy;
		SimpleDstT { dst_sqr }
	}
}

fn min_dst_sqr<T:DstT>(p:&UniPoint, mmx:u32, mmy:u32, mmlevel:u8) -> T {
	let sq = Mipmap::get_pixel_square(mmx,mmy,mmlevel);
	let closest_x = clamp(p.x,sq.topleft.x,sq.topleft.x+sq.length.v);
	let closest_y = clamp(p.y,sq.topleft.y,sq.topleft.y+sq.length.v);
	T::new(p,&UniPoint{x:closest_x,y:closest_y})
}

fn create_task<T:DstT>(x:u32, y:u32, mmx:u32, mmy:u32, mmlevel:u8) -> SdfTask<T> {
	let pxpos = Mipmap::get_center(x,y,0);
	let mindstsqr = min_dst_sqr(&pxpos,mmx,mmy,mmlevel);
	SdfTask{x:mmx,y:mmy,level:mmlevel,best_case_dst_sqr:mindstsqr}
}

pub fn sdf_to_grayscale_image(src: &SDFImage, max_expressable_dst: f64) -> Box<GrayImage> {
	let (width,height) = src.dimensions();
	let fun = |x:u32,y:u32| -> image::Luma<u8> {
			let mut dst : f64 = src.get_pixel(x,y)[0];
			dst = dst / max_expressable_dst * (127 as f64);
			if dst < (-127 as f64) {
				dst = -127 as f64;
			} else if dst > (127 as f64) {
				dst = 127 as f64;
			}
			debug_assert!(dst <= ( 127 as f64));
			debug_assert!(dst >= (-127 as f64));
			let v:u8 = (dst as i32 + 127) as u8;
			image::Luma([v])
		};
	Box::new(ImageBuffer::from_fn(width, height, fun))
}

#[inline]
fn mmget(mm: &Arc<Mipmap>, x:u32, y:u32, level:u8) -> u8 {
	mm.images[level as usize].get_pixel(x,y)[0]
}

#[inline]
fn has_needed(val:u8, needed:u8) -> bool {
	val & needed != 0
}

fn calculate_sdf_at_rec<T:DstT>(mm: &Arc<Mipmap>, x: u32, y:u32, dst_level: u8, best_dst_sqr_input: Option<T>, task: &SdfTask<T>, needed: u8, pxpos: &UniPoint) -> Option<T> {
	debug_assert!(match best_dst_sqr_input { Some(d) => task.best_case_dst_sqr <  d, None => true}); // if not we shouldn't have been called
	debug_assert!(has_needed(mmget(mm,task.x,task.y,task.level), needed));
	let mut best_dst_sqr: Option<T> = best_dst_sqr_input;
	debug_assert!(task.level > 0);
	let children:[(u32,u32);4] = Mipmap::get_children(task.x,task.y);
	let new_level = task.level - 1;
	if new_level == 0 {
		for tup in children.iter() {
			let (cx,cy) = *tup;
			if has_needed(mmget(mm,cx,cy,new_level),needed)  {
				let dstsqr = T::new(pxpos,&Mipmap::get_center(cx,cy,0));
				if best_dst_sqr.is_none() || dstsqr < best_dst_sqr.unwrap() {
					best_dst_sqr = Some(dstsqr);
				}
			}
		}
	} else {
		let mut child_tasks: [Option<SdfTask<T>>; 4] = [None;4];
		let mut child_tasks_idx=0;
		for tup in children.iter() {
			let (cx,cy) = *tup;
			if has_needed(mmget(mm,cx,cy,new_level),needed)  {
				let mindstsqr = min_dst_sqr(pxpos,cx,cy,new_level);
				if best_dst_sqr.is_none() || mindstsqr < best_dst_sqr.unwrap() {
					child_tasks[child_tasks_idx] = Some(SdfTask{x:cx,y:cy,level:new_level,best_case_dst_sqr:mindstsqr});
					child_tasks_idx += 1;
				}
			}
		}
		let child_tasks_slice = &mut child_tasks[0..child_tasks_idx];
		child_tasks_slice.sort_by(|a:&Option<SdfTask<T>>,b:&Option<SdfTask<T>>| -> cmp::Ordering { a.unwrap().best_case_dst_sqr.partial_cmp(&b.unwrap().best_case_dst_sqr).unwrap() });
		for child_task in child_tasks_slice {
			if best_dst_sqr.is_none() || child_task.unwrap().best_case_dst_sqr < best_dst_sqr.unwrap() {
				best_dst_sqr = calculate_sdf_at_rec::<T>(mm, x, y, dst_level, best_dst_sqr, &child_task.unwrap(), needed, pxpos);
			} else {
				// because the tasks are sorted, no other can be relevant at this point
				break;
			}
		}
	}
	//println!("{:?}", best_dst_sqr);
	best_dst_sqr
}

fn calculate_sdf_at<T:DstT>(mm: &Arc<Mipmap>, x: u32, y:u32, dst_level: u8) -> (T,bool) {
	let pxpos = Mipmap::get_center(x,y,dst_level);
	let pxval = mm.get_value(&pxpos);
	let task = create_task(x,y,0,0,mm.get_max_level());
	//let worst_dst = T::new(&UniPoint{x:0,y:0}, &UniPoint{x:u32::MAX,y:u32::MAX});
	(calculate_sdf_at_rec(mm, x, y, dst_level, None, &task, get_needed(pxval), &pxpos).unwrap(), is_black(pxval))
}

fn idx2point <T: num::integer::Integer> (idx: T, width: T) -> (T,T) {
	match num::integer::div_rem(idx,width) {
		(y,x) => (x,y)
	}
}

#[test]
fn idx2pointtest () {
	assert_eq!(idx2point(3,5),(3,0));
	assert_eq!(idx2point(5,5),(0,1));
	assert_eq!(idx2point(22,5),(2,4));
}

pub fn calculate_sdf(mm: Arc<Mipmap>, size: u32) -> Box<SDFImage> {
	{
		let coarsest_val = mm.images.last().expect("Mipmap had no images at all").get_pixel(0,0)[0];
		if !has_black_and_white(coarsest_val) {
			// image has only one color -> everywhere is the maximum distance
			debug_assert!(has_black(coarsest_val) || has_white(coarsest_val), "Mipmap is wrong: Image seems to have neither black nor white pixels");
			let inf = f32::INFINITY as f64;
			let neginf = f32::NEG_INFINITY as f64;
			let dst_val : f64 = if has_black(coarsest_val) { neginf } else { inf };
			return Box::new(SDFImage::from_pixel(size,size,image::Luma([dst_val])))
		}
	}
	let dst_level = mm.get_max_level() - log2(size as u64).expect("destination size must be a power of two");
	let mut results = vec![0f64; (size*size) as usize];
	results.par_iter_mut().enumerate().for_each(|tup| {
	    let (i,result) = tup;
	    let (x,y) = idx2point(i, size as usize);
		let (d,do_invert) = calculate_sdf_at::<SimpleDstT>(&mm, x as u32, y as u32, dst_level);
	    *result = d.get_dst_sqr().sqrt() * if do_invert { -1.0 } else { 1.0 };
	});
	Box::new(SDFImage::from_raw(size,size,results).unwrap())
}
