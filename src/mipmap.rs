extern crate image;

use self::image::GrayImage;
use self::image::ImageBuffer;

use functions::log2;

pub struct Mipmap {
	pub images: Vec<GrayImage>
}

// These two types are important so we don't accidentally confuse our u32 indices with u32 positions and sizes
pub struct UniSize {
	pub v: u32,
}
pub struct UniPoint {
	pub x: u32,
	pub y: u32,
}

pub struct UniSquare {
	pub topleft: UniPoint,
	pub length: UniSize,
}

impl Mipmap {
	// returns the top left position of a pixel in unified coordinates
	// in unified coordinates, the center position of the top left pixel in level 0 is (1,1), and the lowe right position (2,2)
	pub fn get_position(x: u32, y: u32, level: u8) -> UniPoint {
		let size = Mipmap::get_pixel_size(level).v;
		UniPoint{x:x*size, y:y*size}
	}
	// calculates how big a pixel at a mipmap level is, in unified coordinates
	pub fn get_pixel_size(level: u8) -> UniSize {
		UniSize{v:2<<level}
	}
	// the center positions of the corner pixels
	pub fn get_corners(x: u32, y: u32, level: u8) -> [UniPoint;4] {
		let tl = Mipmap::get_position(x,y,level); // top left
		let size = Mipmap::get_pixel_size(level).v;
		// TODO this could be simplified with a macro
		[UniPoint{x:tl.x+1,y:tl.y+1}, UniPoint{x:tl.x+size-1,y:tl.y+1}, UniPoint{x:tl.x+1,y:tl.y+size-1}, UniPoint{x:tl.x+size-1,y:tl.y+size-1}]
	}
	pub fn get_pixel_square(x: u32, y: u32, level: u8) -> UniSquare {
		let mut topleft = Mipmap::get_position(x,y,level);
		// we want the center of the corner pixel, not the corner
		topleft.x += 1;
		topleft.y += 1;
		let mut length = Mipmap::get_pixel_size(level);
		length.v -= 2;
		UniSquare{topleft:topleft,length:length}
	}
	pub fn get_center(x: u32, y:u32, level: u8) -> UniPoint {
		let tl = Mipmap::get_position(x,y,level); // top left
		let half_size = Mipmap::get_pixel_size(level).v / 2;
		UniPoint{x:tl.x+half_size, y:tl.y+half_size}
	}
	pub fn lower_right_corner(self: &Mipmap) -> UniPoint {
		let (dimx,dimy) = self.images[0].dimensions();
		Mipmap::get_center(dimx-1,dimy-1,0)
	}
	pub fn get_max_level(self: &Mipmap) -> u8 {
		(self.images.len()-1) as u8
	}
	pub fn get_children(x: u32, y:u32) -> [(u32,u32);4] {
		[(2*x,2*y),(2*x+1,2*y),(2*x,2*y+1),(2*x+1,2*y+1)]
	}
	// lookup in actual level 0 image
	pub fn get_value(self: &Mipmap, position: &UniPoint) -> u8 {
		let x = position.x/2;
		let y = position.y/2;
		self.images[0].get_pixel(x,y).data[0]
	}
	pub fn new<F> (srcimg : GrayImage, compressor : F) -> Mipmap
		where F : Fn(u8,u8,u8,u8) -> u8 {
		let (width,height) = srcimg.dimensions();
		assert_eq!(width,height);
		let sizelog = log2(width as u64).expect("image dimensions must be a power of two");
		let mut ret = Mipmap{images : Vec::with_capacity((sizelog+1) as usize)};
		ret.images.push(srcimg);
		for i in 1..sizelog+1 {
			let smallimg : GrayImage = {
				let bigimg = &ret.images[(i-1) as usize];
				let bigsizelog = sizelog-i+1;
				assert_eq!(bigimg.dimensions(), (1<<bigsizelog as u32, 1<<bigsizelog as u32));
				let smallsizelog = sizelog-i;
				let smallifier = |x:u32, y:u32| -> image::Luma<u8> {
					let a = bigimg.get_pixel(x*2,y*2).data[0];
					let b = bigimg.get_pixel(x*2+1,y*2).data[0];
					let c = bigimg.get_pixel(x*2,y*2+1).data[0];
					let d = bigimg.get_pixel(x*2+1,y*2+1).data[0];
					image::Luma{data:[compressor(a,b,c,d)]}
				};
				ImageBuffer::from_fn(1<<smallsizelog as u32, 1<<smallsizelog as u32,smallifier)
			};
			ret.images.push(smallimg);
		}
		ret
	}
}
