extern crate image;
extern crate num;
extern crate simple_parallel;

use self::image::GrayImage;
use self::image::ImageBuffer;
pub type DstT = f64;
pub type SDFImage = image::ImageBuffer<image::Luma<f64>, Vec<f64>>;

use std::f32;
use std::collections::BinaryHeap;
use std::sync::Arc;

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
			let mut dst : DstT = src.get_pixel(x,y).data[0];
			dst = dst / max_expressable_dst * (127 as DstT);
			if dst < (-127 as DstT) {
				dst = -127 as DstT;
			} else if dst > (127 as DstT) {
				dst = 127 as DstT;
			}
			debug_assert!(dst <= ( 127 as DstT));
			debug_assert!(dst >= (-127 as DstT));
			let v:u8 = (dst as i32 + 127) as u8;
			image::Luma{data:[v]}
		};
	Box::new(ImageBuffer::<image::Luma<u8>>::from_fn(width, height, fun))
}

fn calculate_sdf_at(mm: &Arc<Mipmap>, x: u32, y:u32, dst_level: u8) -> f64 {
	let mmget = |x:u32,y:u32,level:u8| -> u8 {
		mm.images[level as usize].get_pixel(x,y).data[0]
	};
	let pxpos = Mipmap::get_center(x,y,dst_level);
	let px_is_white = is_white(mm.get_value(&pxpos));
	let has_needed = |v:u8| -> bool { // this could probably be done more efficiently...
		if px_is_white {
			has_black(v)
		} else {
			has_white(v)
		}
	};
	let mut best_dst_sqr : DstT = f32::INFINITY as DstT;
	let mut tasks = BinaryHeap::<SdfTask>::new();
	tasks.push(create_task(x,y,0,0,mm.get_max_level()));
	while let Some(task) = tasks.pop() {
		if task.best_case_dst_sqr < best_dst_sqr {
			// there could be something valuable in here
			let mmval = mmget(task.x,task.y,task.level);
			debug_assert!(has_needed(mmval));
			if task.level == 0 {
				debug_assert_eq!(task.best_case_dst_sqr, dst_sqr(&pxpos, &Mipmap::get_center(task.x,task.y,0)));
				if task.best_case_dst_sqr < best_dst_sqr {
					best_dst_sqr = task.best_case_dst_sqr;
				}
			} else {
				let children = Mipmap::get_children(task.x,task.y);
				let new_level = task.level - 1;
				for tup in children.into_iter() {
					let (cx,cy) = *tup;
					if has_needed(mmget(cx,cy,new_level))  {
						let mindstsqr = min_dst_sqr(&pxpos,cx,cy,new_level);
						if mindstsqr < best_dst_sqr {
							tasks.push(SdfTask{x:cx,y:cy,level:new_level,best_case_dst_sqr:mindstsqr});
						}
					}
				}
			}
		}
	} // done with tasks
	// use high precision math here to avoid rounding errors
	let mut best_dst : DstT = (best_dst_sqr as f64).sqrt() as DstT;
	if !px_is_white {
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
/*
fn splitnrec<T> (arr: &mut[T], splitlen: usize) -> Vec<&mut[T]> {
	if arr.len() <= splitlen {
		vec![arr]
	} else {
		let (hd,tl) = arr.split_at_mut(splitlen);
		let mut ret = splitnrec(tl, splitlen);
		ret.push(hd);
		ret
	}
}

fn splitn<T> (arr: &mut[T], n: usize) -> Vec<&mut[T]> {
	let splitlen = match num::integer::div_rem(arr.len(),n) {
		(x,0) => x,
		(x,_) => x+1,
	};
	splitnrec(arr, splitlen)
}*/

fn chunkslen (arrlen: usize, n: usize) -> usize {
	match num::integer::div_rem(arrlen,n) {
		(x,0) => x,
		(x,_) => x+1,
	}
}

pub fn calculate_sdf_region(dst: &mut[f64], dstwidth: usize, startidx: usize, mm: Arc<Mipmap>, dst_level: u8) {
	for idx in startidx..startidx+dst.len() {
		let (x,y) = idx2point(idx, dstwidth);
		dst[idx-startidx] = calculate_sdf_at(&mm, x as u32, y as u32, dst_level);
	}
}

struct WorkerArgs<'a> {
	dst: &'a mut[f64],
	dstwidth: usize,
	startidx: usize,
	mm: Arc<Mipmap>,
	dst_level: u8,
}

fn worker (a: WorkerArgs) {
	calculate_sdf_region(a.dst, a.dstwidth, a.startidx, a.mm, a.dst_level)
}

pub fn calculate_sdf(mm: Arc<Mipmap>, size: u32, n_threads: usize) -> Box<SDFImage> {
	assert!(n_threads > 0);
	{
		let coarsest_val = mm.images.last().expect("Mipmap had no images at all").get_pixel(0,0).data[0];
		if !has_black_and_white(coarsest_val) {
			// image has only one color -> everywhere is the maximum distance
			debug_assert!(has_black(coarsest_val) || has_white(coarsest_val), "Mipmap is wrong: Image seems to have neither black nor white pixels");
			let inf = f32::INFINITY as DstT;
			let neginf = f32::NEG_INFINITY as DstT;
			let dst_val : DstT = if has_black(coarsest_val) { neginf } else { inf };
			return Box::new(SDFImage::from_pixel(size,size,image::Luma{data:[dst_val]}))
		}
	}
	let dst_level = mm.get_max_level() - log2(size as u64).expect("destination size must be a power of two");
	let mut results = vec![0f64; (size*size) as usize];
	let chunklen = chunkslen(results.len(), n_threads);
	{
		let result_chunks = results.chunks_mut(chunklen);
		assert_eq!(result_chunks.len(), n_threads);
		let mut args: Vec<WorkerArgs> = vec![];
		let mut currentidx = 0;
		for chunk in result_chunks {
			let chunklen = chunk.len();
			args.push(WorkerArgs { dst: chunk, dstwidth: size as usize, startidx: currentidx, mm:mm.clone(), dst_level:dst_level});
			currentidx += chunklen;
		}
		let mut pool = simple_parallel::Pool::new(n_threads);
		pool.for_(args, worker);
	}
	Box::new(SDFImage::from_raw(size,size,results).unwrap())
	//let mut results = 
	/*let mut results = Box::new(SDFImage::new(size,size));
	for y in 0..size {
		for x in 0..size {
			let best_dst:f64 = calculate_sdf_at(&mm, x, y, dst_level);
			results.put_pixel(x,y,image::Luma{data:[best_dst]});
		}
	}
	results*/
}