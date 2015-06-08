extern crate image;

use self::image::GrayImage;
use self::image::ImageBuffer;
pub type DstT = f64;
pub type SDFImage = image::ImageBuffer<image::Luma<f64>, Vec<f64>>;

use std::f32;
use std::collections::BinaryHeap;

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
			if dst < (i8::min_value() as DstT) {
				dst = i8::min_value() as DstT;
			} else if dst > (i8::max_value() as DstT) {
				dst = i8::max_value() as DstT;
			}
			debug_assert!(dst <= ( 127 as DstT));
			debug_assert!(dst >= (-127 as DstT));
			let v:u8 = (dst as i32 + 127) as u8;
			image::Luma{data:[v]}
		};
	Box::new(ImageBuffer::<image::Luma<u8>>::from_fn(width, height, fun))
}

pub fn calculate_sdf(mm: &Mipmap, size: u32) -> Box<SDFImage> {
	let mmget = |x:u32,y:u32,level:u8| -> u8 {
		mm.images[level as usize].get_pixel(x,y).data[0]
	};
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
	let mut results = Box::new(SDFImage::new(size,size));
	for y in 0..size {
		for x in 0..size {
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
			let best_dst : DstT = (best_dst_sqr as f64).sqrt() as DstT;
			results.put_pixel(x,y,image::Luma{data:[best_dst]});
		}
	}
	results
}