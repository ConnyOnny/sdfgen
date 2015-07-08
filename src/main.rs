extern crate image;
extern crate getopts;
extern crate byteorder;
extern crate sdfgen;

use std::fs::File;
use std::io::Write;

use image::GrayImage;

use getopts::Options;

use byteorder::{LittleEndian, WriteBytesExt};

use sdfgen::functions::bw_to_bits;
use sdfgen::functions::bit_compressor;
use sdfgen::sdf_algorithm::calculate_sdf;
use sdfgen::sdf_algorithm::sdf_to_grayscale_image;
use sdfgen::sdf_algorithm::DstT;

fn print_usage(program: &String, opts: &Options) {
	let brief = format!("Usage: {} [options] inputimage.png outputimage.png", program);
	print!("{}", opts.usage(&brief));
}

fn main() {
	let args: Vec<String> = std::env::args().collect();
	let program_name = args[0].clone();

	let mut opts = Options::new();
	opts.optflag("h","help","print help");
	opts.optflag("v","verbose","show what the program is doing");
	opts.optopt ("s","size","size of the output signed distance field image, must be a power of 2. Defaults to input size / 4","OUTPUT_SIZE");
	opts.optopt ( "","maxdst","saturation distance (i.e. 'most far away meaningful distance') in half pixels of the input image. Defaults to input size / 4","SATURATION_DISTANCE");
	opts.optopt ( "","save-mipmaps","save the mipmaps used for accelerated calculation to BASENAMEi.png, where 'i' is the mipmap level","BASENAME");
	opts.optopt ("t","type","One of 'png', 'f32', 'f64'. f32 and f64 are raw floating point formats. Default: png","TYPE");
	if args.len() == 1 {
		print_usage(&program_name, &opts);
		return;
	}
	let parsed_opts = match opts.parse(&args[1..]) {
		Ok(v) => { v }
		Err(e) => { panic!(e.to_string()) }
	};
	if parsed_opts.opt_present("help") || parsed_opts.free.len() != 2 {
		print_usage(&program_name, &opts);
		return;
	}
	let input_image_name = &parsed_opts.free[0];
	let output_image_name = &parsed_opts.free[1];
	let verbose = parsed_opts.opt_present("verbose");

	if verbose {
		println!("Loading input image '{}'.", input_image_name);
	}
	let mut img : GrayImage = image::open(input_image_name).ok().expect("failed to load image").to_luma();
	if verbose {
		let (w,h) = img.dimensions();
		println!("Image is of size {}x{} pixels.",w,h);
	}
	let (input_size,_) = img.dimensions();

	if verbose {
		println!("Converting image to binary.");
	}
	for px in img.pixels_mut() {
		px.data[0] = bw_to_bits(px.data[0]);
	}

	if verbose {
		println!("Calculating Mipmap.");
	}
	let mipmap = sdfgen::mipmap::Mipmap::new(img,bit_compressor);
	if verbose {
		println!("Mipmap has {} levels.", mipmap.get_max_level()+1);
	}

	if parsed_opts.opt_present("save-mipmaps") {
		let basename = parsed_opts.opt_str("save-mipmaps").expect("--save-mipmaps needs exactly one argument.");
		if verbose {
			println!("Saving Mipmaps to {}[0..{}].png", basename, mipmap.get_max_level()+1);
		}
		for i in 0..mipmap.get_max_level()+1 {
			mipmap.images[i as usize].save(format!("/home/c/mipmap {}.png",i)).unwrap();
		}
	}
	let sdf_size = match parsed_opts.opt_str("size") {
		Some(s) => { s.parse::<u32>().unwrap() }
		None	=> { input_size / 4 }
	};
	let sat_dst : DstT = match parsed_opts.opt_str("maxdst") {
		Some(s) => { s.parse::<DstT>().unwrap() }
		None	=> { (input_size / 4) as DstT }
	};
	if verbose {
		println!("Calculating signed distance field of size {} with saturation distance {}.", sdf_size, sat_dst);
	}
	let mipmap_arc = std::sync::Arc::new(mipmap);
	let sdf = calculate_sdf(mipmap_arc.clone(), sdf_size, 4);
	if verbose {
		println!("Doing a final color space conversion.");
	}
	let output_type = match parsed_opts.opt_str("type") {
		Some(s) => s,
		None    => format!("png") // FIXME we don't really need "format!" here
	};
	match output_type.as_ref() {
		"png" => {
			let sdf_u8 = sdf_to_grayscale_image(&(*sdf), sat_dst);
			if verbose {
				println!("Saving signed distance field image in png format as '{}'.", output_image_name);
			}
			let mut outf = File::create(output_image_name).unwrap();
			let mut pngenc = image::png::PNGEncoder::<std::fs::File>::new(&mut outf);
			let (w,h) = sdf_u8.dimensions();
			pngenc.encode(sdf_u8.into_raw().as_ref(), w, h, image::ColorType::Gray(8)).unwrap();
			//sdf_u8.save(output_image_name).unwrap();
		}
		// TODO: remove code duplication here
		"f64" => {
			let mut buf = vec![];
			for px in sdf.into_raw() {
				buf.write_f64::<LittleEndian>(px).unwrap();
			}
			if verbose {
				println!("Saving signed distance field image in f64 raw format as '{}'", output_image_name);
			}
			let mut outf = File::create(output_image_name).unwrap();
			outf.write_all(buf.as_ref()).unwrap();
		}
		"f32" => {
			let mut buf = vec![];
			for px in sdf.into_raw() {
				buf.write_f32::<LittleEndian>(px as f32).unwrap();
			}
			if verbose {
				println!("Saving signed distance field image in f32 raw format as '{}'", output_image_name);
			}
			let mut outf = File::create(output_image_name).unwrap();
			outf.write_all(buf.as_ref()).unwrap();
		}
		_ => {
			panic!("Unknown output format: {}", output_type);
		}
	};

}