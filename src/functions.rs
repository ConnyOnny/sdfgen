use std::cmp;

static HAS_WHITE_BIT : u8 = 128;
static HAS_BLACK_BIT : u8 =  64;

pub fn optimistic_min <T: PartialOrd> (v1:T, v2:T) -> T {
	let cmpres = v1.partial_cmp(&v2).expect("Could not calculate minimum for some special values. This should not happen.");
	match cmpres {
		cmp::Ordering::Less => v1,
		_ => v2
	}
}

pub fn clamp <T> (v:T, smallest:T, biggest:T) -> T
	where T: Ord {
	cmp::max(cmp::min(v,biggest),smallest)
}

pub fn bw_to_bits (v:u8) -> u8 {
	if v >= 128 {
		HAS_WHITE_BIT
	} else {
		HAS_BLACK_BIT
	}
}

pub fn bit_compressor (a:u8, b:u8, c:u8, d:u8) -> u8 {
	a|b|c|d
}

pub fn has_black_and_white (v:u8) -> bool {
	v & (HAS_WHITE_BIT | HAS_BLACK_BIT) == (HAS_WHITE_BIT | HAS_BLACK_BIT)
}

pub fn has_white (v:u8) -> bool {
	v & HAS_WHITE_BIT != 0
}

pub fn has_black (v:u8) -> bool {
	v & HAS_BLACK_BIT != 0
}

pub fn get_needed (v:u8) -> u8 {
	debug_assert!(is_white(v) || is_black(v));
	if v & HAS_WHITE_BIT != 0 {
		HAS_BLACK_BIT
	} else {
		HAS_WHITE_BIT
	}
}

pub fn is_white (v:u8) -> bool {
	debug_assert!(!has_black_and_white(v));
	v & HAS_WHITE_BIT != 0
}

pub fn is_black (v:u8) -> bool {
	debug_assert!(!has_black_and_white(v));
	v & HAS_BLACK_BIT != 0
}

pub fn log2(x: u64) -> Option<u8> {
	for i in 0..64 {
		let shiftedi : u64 = 1<<i as u64;
		let andedx : u64 = x & shiftedi;
		if andedx == x {
			return Some(i);
		}
	}
	None
}