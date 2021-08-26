use std::str;

use nom::AsBytes;

pub struct PathSpliter<'a> {
	raw_path: &'a [u8],
	cur_pos: usize,
	segment_buf: Vec<u8>,
}

impl<'a> PathSpliter<'a> {
	pub fn new(p: &'a str) -> Self {
		return Self{
			raw_path: p.as_bytes(),
			cur_pos: 0,
			segment_buf: Vec::new(),
		}
	}
}

impl<'a> Iterator for PathSpliter<'a> {
	type Item = &'a str;
	fn next(&mut self) -> Option<Self::Item> {
		self.segment_buf.clear();

		while self.cur_pos < self.raw_path.len() {
			let c = self.raw_path[self.cur_pos];
			self.cur_pos += 1;
			match c {
				b'/' => {	
					return Some(unsafe{str::from_utf8_unchecked(self.segment_buf.as_slice())})
				},
				b'\\' => {
					if self.cur_pos < self.raw_path.len() {
						let c = self.raw_path[self.cur_pos];
						self.segment_buf.push(c);
						self.cur_pos += 1;
					}
				}
				_ => {
					self.segment_buf.push(c);
				}
			}
		}

		if self.segment_buf.len() == 0 {
			return None
		} else {
			return Some(unsafe{str::from_utf8_unchecked(self.segment_buf.as_bytes())})
		}
	}
}
