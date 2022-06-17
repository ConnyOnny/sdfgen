extern crate image;
extern crate num;
extern crate rayon;

use self::image::GrayImage;
use self::image::ImageBuffer;
pub type DstT = f64;
pub type SDFImage = image::ImageBuffer<image::Luma<f64>, Vec<f64>>;

use std::f32;
use std::sync::Arc;
use std::cmp;
use self::rayon::prelude::*;

use mipmap::*;
use functions::*;
use sdf_task::SdfTask;

fn dst_sqr(p1:&UniPoint, p2:&UniPoint) -> DstT {
	let dx = (p1.x as i32 - p2.x as i32) as DstT;
	let dy = (p1.y as i32 - p2.y as i32) as DstT;
	dx*dx+dy*dy
}

fn min_dst_sqr(p:&UniPoint, mmx:u32, mmy:u32, mmlevel:u8) -> DstT {
	let sq = Mipmap::get_pixel_square(mmx,mmy,mmlevel);
	let closest_x = clamp(p.x,sq.topleft.x,sq.topleft.x+sq.length.v);
	let closest_y = clamp(p.y,sq.topleft.y,sq.topleft.y+sq.length.v);
	dst_sqr(p,&UniPoint{x:closest_x,y:closest_y})
}

fn create_task(x:u32, y:u32, mmx:u32, mmy:u32, mmlevel:u8) -> SdfTask {
	let pxpos = Mipmap::get_center(x,y,0);
	let mindstsqr = min_dst_sqr(&pxpos,mmx,mmy,mmlevel);
	SdfTask{x:mmx,y:mmy,level:mmlevel,best_case_dst_sqr:mindstsqr}
}

pub fn sdf_to_grayscale_image(src: &SDFImage, max_expressable_dst: DstT) -> Box<GrayImage> {
	let (width,height) = src.dimensions();
	let fun = |x:u32,y:u32| -> image::Luma<u8> {
			let mut dst : DstT = src.get_pixel(x,y)[0];
			dst = dst / max_expressable_dst * (127 as DstT);
			if dst < (-127 as DstT) {
				dst = -127 as DstT;
			} else if dst > (127 as DstT) {
				dst = 127 as DstT;
			}
			debug_assert!(dst <= ( 127 as DstT));
			debug_assert!(dst >= (-127 as DstT));
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

fn calculate_sdf_at_rec(mm: &Arc<Mipmap>, x: u32, y:u32, dst_level: u8, best_dst_sqr_input: DstT, task: &SdfTask, needed: u8, pxpos: &UniPoint) -> DstT {
	debug_assert!(task.best_case_dst_sqr < best_dst_sqr_input); // if not we shouldn't have been called
	debug_assert!(has_needed(mmget(mm,task.x,task.y,task.level), needed));
	let mut best_dst_sqr = best_dst_sqr_input;
	debug_assert!(task.level > 0);
	let children:[(u32,u32);4] = Mipmap::get_children(task.x,task.y);
	let new_level = task.level - 1;
	if new_level == 0 {
		for tup in children.iter() {
			let (cx,cy) = *tup;
			if has_needed(mmget(mm,cx,cy,new_level),needed)  {
				let dstsqr = dst_sqr(pxpos,&Mipmap::get_center(cx,cy,0));
				if dstsqr < best_dst_sqr {
					best_dst_sqr = dstsqr;
				}
			}
		}
	} else {
		let mut child_tasks = [SdfTask{x:0,y:0,level:0,best_case_dst_sqr:0f64};4];
		let mut child_tasks_idx=0;
		for tup in children.iter() {
			let (cx,cy) = *tup;
			if has_needed(mmget(mm,cx,cy,new_level),needed)  {
				let mindstsqr = min_dst_sqr(pxpos,cx,cy,new_level);
				if mindstsqr < best_dst_sqr {
					child_tasks[child_tasks_idx] = SdfTask{x:cx,y:cy,level:new_level,best_case_dst_sqr:mindstsqr};
					child_tasks_idx += 1;
				}
			}
		}
		let child_tasks_slice = &mut child_tasks[0..child_tasks_idx];
		child_tasks_slice.sort_by(|a:&SdfTask,b:&SdfTask| -> cmp::Ordering { a.best_case_dst_sqr.partial_cmp(&b.best_case_dst_sqr).unwrap() });
		for child_task in child_tasks_slice {
			if child_task.best_case_dst_sqr < best_dst_sqr {
				best_dst_sqr = calculate_sdf_at_rec(mm, x, y, dst_level, best_dst_sqr, &child_task, needed, pxpos);
			} else {
				// because the tasks are sorted, no other can be relevant at this point
				break;
			}
		}
	}
	best_dst_sqr
}

fn calculate_sdf_at(mm: &Arc<Mipmap>, x: u32, y:u32, dst_level: u8) -> DstT {
	let pxpos = Mipmap::get_center(x,y,dst_level);
	let pxval = mm.get_value(&pxpos);
	let task = create_task(x,y,0,0,mm.get_max_level());
	let best_dst_sqr = calculate_sdf_at_rec(mm, x, y, dst_level, f32::INFINITY as DstT, &task, get_needed(pxval), &pxpos);
	// use high precision math here to avoid rounding errors
	let mut best_dst : DstT = (best_dst_sqr as f64).sqrt() as DstT;
	if is_black(pxval) {
		best_dst *= -1 as DstT;
	}
	best_dst
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
			let inf = f32::INFINITY as DstT;
			let neginf = f32::NEG_INFINITY as DstT;
			let dst_val : DstT = if has_black(coarsest_val) { neginf } else { inf };
			return Box::new(SDFImage::from_pixel(size,size,image::Luma([dst_val])))
		}
	}
	let dst_level = mm.get_max_level() - log2(size as u64).expect("destination size must be a power of two");
	let mut results = vec![0f64; (size*size) as usize];
	results.par_iter_mut().enumerate().for_each(|tup| {
	    let (i,result) = tup;
	    let (x,y) = idx2point(i, size as usize);
	    *result = calculate_sdf_at(&mm, x as u32, y as u32, dst_level);
	});
	Box::new(SDFImage::from_raw(size,size,results).unwrap())
}
