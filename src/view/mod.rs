#![allow(non_camel_case_types)]

use std::mem;

use vertex;

extern {
  #[link_name = "llvm.convert.from.fp16.f32"]
  fn convert_from_f16_to_f32(a: i16) -> f32;
  
  #[link_name = "llvm.convert.to.fp16.f32"]
  fn convert_from_f32_to_f16(a: f32) -> i16;
}

macro_rules! impl_view {
  ($name:ident, $out:ty, $f:ident) => {
    pub fn len(&self) -> usize {
      return self.count;
    }

    pub fn get(&self, i: usize) -> $out {
      if i > self.len() {
        panic!("Out of bounds view access, index is {:?} but length is just {:?}", i, self.len());
      }

      return $f(self.data[self.stride * i + self.offset]);
    }
  }
}

macro_rules! impl_mut_view {
  ($name:ident, $out:ty, $f:ident) => {
    pub fn set(&mut self, i: usize, value: $out) {
      if i > self.count {
        panic!("Out of bounds view access, index is {:?} but length is just {:?}", i, self.count);
      }

      self.data[self.stride * i + self.offset] = $f(value);
    }
  }
}

macro_rules! impl_view_constructor {
  ($name:ident, $format:ident, $ty:ty, $data:ident, $attribute:ident, $count:ident, $stride:ident) => {
    {
      if $attribute.ty != vertex::Format::$format {
        panic!("Invalid attribute format!");
      }

      if $attribute.elements != 1 {
        panic!("FIXME: Vector formats are going to be deprecated soon :C");
      }

      let stride_unaligned = $stride % mem::size_of::<$ty>() != 0;
      let offset_unaligned = $attribute.offset % mem::size_of::<$ty>() != 0;

      if stride_unaligned || offset_unaligned {
        panic!("Buffer would not be aligned, that could crash the whole application!")
      }

      $name {
        data: unsafe { mem::transmute($data) },
        count: $count,
        stride: $stride / mem::size_of::<$ty>(),
        offset: $attribute.offset / mem::size_of::<$ty>(),
        attribute: $attribute
      }
    }
  }
}

macro_rules! view {
  ($name:ident, $format:ident, $ty:ty, $out:ty, $f:ident) => {
    pub struct $name<'a> {
      data: &'a [$ty],
      count: usize,
      stride: usize,
      offset: usize,
      pub attribute: &'a vertex::Attribute
    }

    impl<'a> $name<'a> {
      pub fn new(data: &'a [u8], attribute: &'a vertex::Attribute, count: usize, stride: usize) -> $name<'a> {
        return impl_view_constructor!($name, $format, $ty, data, attribute, count, stride);
      }

      impl_view!($name, $out, $f);
    }
  }
}

macro_rules! mut_view {
  ($name:ident, $format:ident, $ty:ty, $out:ty, $f_out:ident, $f_in:ident) => {
    pub struct $name<'a> {
      data: &'a mut [$ty],
      count: usize,
      stride: usize,
      offset: usize,
      pub attribute: &'a vertex::Attribute
    }

    impl<'a> $name<'a> {
      pub fn new(data: &'a mut [u8], attribute: &'a vertex::Attribute, count: usize, stride: usize) -> $name<'a> {
        return impl_view_constructor!($name, $format, $ty, data, attribute, count, stride);
      }

      impl_view!($name, $out, $f_out);
      impl_mut_view!($name, $out, $f_in);
    }
  }
}

fn identity<T>(input: T) -> T {
  return input;
}

fn f16_to_f32(input: i16) -> f32 {
  return unsafe { convert_from_f16_to_f32(input) };
}

fn f32_to_f16(input: f32) -> i16 {
  return unsafe { convert_from_f32_to_f16(input) };
}

view!(U8View, u8, u8, u8, identity);
view!(U16View, u16, u16, u16, identity);
view!(U32View, u32, u32, u32, identity);
view!(U64View, u64, u64, u64, identity);

mut_view!(U8MutView, u8, u8, u8, identity, identity);
mut_view!(U1Mut6View, u16, u16, u16, identity, identity);
mut_view!(U3Mut2View, u32, u32, u32, identity, identity);
mut_view!(U6Mut4View, u64, u64, u64, identity, identity);

view!(I8View, i8, i8, i8, identity);
view!(I16View, i16, i16, i16, identity);
view!(I32View, i32, i32, i32, identity);
view!(I64View, i64, i64, i64, identity);

mut_view!(I8MutView, i8, i8, i8, identity, identity);
mut_view!(I1Mut6View, i16, i16, i16, identity, identity);
mut_view!(I3Mut2View, i32, i32, i32, identity, identity);
mut_view!(I6Mut4View, i64, i64, i64, identity, identity);

view!(F16View, f16, i16, f32, f16_to_f32);
view!(F32View, f32, f32, f32, identity);
view!(F64View, f64, f64, f64, identity);

mut_view!(F16MutView, f16, i16, f32, f16_to_f32, f32_to_f16);
mut_view!(F32MutView, f32, f32, f32, identity, identity);
mut_view!(F64MutView, f64, f64, f64, identity, identity);

enum View<'a> {
  f16(F16View<'a>)
}

pub struct F32OmniView<'a> {
  count: usize,
  view: View<'a>
}

impl<'a> F32OmniView<'a> {
  pub fn new(data: &'a [u8], attribute: &'a vertex::Attribute, count: usize, stride: usize) -> F32OmniView<'a> {
    let view = match attribute.ty {
      vertex::Format::f16 => View::f16(F16View::new(data, attribute, count, stride)),
      ty => panic!("Unsupported view type {:?} for omni view", ty)
    };

    return F32OmniView { count: count, view: view };
  }

  pub fn len(&self) -> usize {
    return self.count;
  }

  pub fn get(&self, i: usize) -> f32 {
    match self.view {
      View::f16(ref view) => view.get(i)
    }
  }
}

#[cfg(test)]
mod tests {
  use vertex;

  use super::*;

  #[test]
  fn test_f16_view() {
    let data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF];
    let attribute = vertex::Attribute {
      name: vertex::AttributeName::Position,
      offset: 4,
      ty: vertex::Format::f16,
      elements: 1,
      buffer_index: 0
    };

    let view = F16View::new(data.as_slice(), &attribute, 2, 8);

    assert_eq!(view.get(0), 0.0f32);
    assert_eq!(view.get(1), 0.0f32);
  }

  #[test]
  fn test_f16_omniview() {
    let data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF];
    let attribute = vertex::Attribute {
      name: vertex::AttributeName::Position,
      offset: 4,
      ty: vertex::Format::f16,
      elements: 1,
      buffer_index: 0
    };

    let view = F32OmniView::new(data.as_slice(), &attribute, 2, 8);

    assert_eq!(view.get(0), 0.0f32);
    assert_eq!(view.get(1), 0.0f32);
  }

  #[test]
  #[cfg(target_endian = "little")]
  fn test_f16_mut_view() {
    let mut data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF];
    let attribute = vertex::Attribute {
      name: vertex::AttributeName::Position,
      offset: 4,
      ty: vertex::Format::f16,
      elements: 1,
      buffer_index: 0
    };

    {
      let mut view = F16MutView::new(data.as_mut_slice(), &attribute, 2, 8);

      assert_eq!(view.get(0), 0.0f32);
      assert_eq!(view.get(1), 0.0f32);

      view.set(1, 1.0);

      assert_eq!(view.get(0), 0.0f32);
      assert_eq!(view.get(1), 1.0f32);
    }

    assert_eq!(data, vec![0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0x3C, 0xFF, 0xFF]);
  }

  #[test]
  fn test_f32_view() {
    let data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0];
    let attribute = vertex::Attribute {
      name: vertex::AttributeName::Position,
      offset: 4,
      ty: vertex::Format::f32,
      elements: 1,
      buffer_index: 0
    };

    let view = F32View::new(data.as_slice(), &attribute, 2, 8);

    assert_eq!(view.get(0), 0.0f32);
    assert_eq!(view.get(1), 0.0f32);
  }
}