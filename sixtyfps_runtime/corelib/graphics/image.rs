/* LICENSE BEGIN
    This file is part of the SixtyFPS Project -- https://sixtyfps.io
    Copyright (c) 2021 Olivier Goffart <olivier.goffart@sixtyfps.io>
    Copyright (c) 2021 Simon Hausmann <simon.hausmann@sixtyfps.io>

    SPDX-License-Identifier: GPL-3.0-only
    This file is also available under commercial licensing terms.
    Please contact info@sixtyfps.io for more information.
LICENSE END */

use crate::slice::Slice;
use crate::{SharedString, SharedVector};

/// SharedPixelBuffer is a container for storing image data as pixels. It is
/// internally reference counted and cheap to clone.
///
/// You can construct new a new empty shared pixel buffer with [`SharedPixelBuffer::new`],
/// or you can clone it from an existing contiguous buffer that you might already have, using
/// [`SharedPixelBuffer::clone_from_slice`].
///
/// See the documentation for [`Image`] for examples how to use this type to integrate
/// SixtyFPS with external rendering functions.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct SharedPixelBuffer<Pixel> {
    width: usize,
    height: usize,
    stride: usize,
    data: SharedVector<Pixel>,
}

impl<Pixel> SharedPixelBuffer<Pixel> {
    /// Returns the width of the image in pixels.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the image in pixels.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the number of pixels per line.
    pub fn stride(&self) -> usize {
        self.stride
    }
}

impl<Pixel: Clone> SharedPixelBuffer<Pixel> {
    /// Return a mutable slice to the pixel data. If the SharedPixelBuffer was shared, this will make a copy of the buffer.
    pub fn make_mut_slice(&mut self) -> &mut [Pixel] {
        self.data.make_mut_slice()
    }
}

impl<Pixel: Clone + rgb::Pod> SharedPixelBuffer<Pixel>
where
    [Pixel]: rgb::ComponentBytes<u8>,
{
    /// Returns the pixels interpreted as raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        use rgb::ComponentBytes;
        self.data.as_slice().as_bytes()
    }

    /// Returns the pixels interpreted as raw bytes.
    pub fn make_mut_bytes(&mut self) -> &mut [u8] {
        use rgb::ComponentBytes;
        self.data.make_mut_slice().as_bytes_mut()
    }
}

impl<Pixel> SharedPixelBuffer<Pixel> {
    /// Return a slice to the pixel data.
    pub fn as_slice(&self) -> &[Pixel] {
        self.data.as_slice()
    }
}

impl<Pixel: Clone + Default> SharedPixelBuffer<Pixel> {
    /// Creates a new SharedPixelBuffer with the given width and height. Each pixel will be initialized with the value
    /// that [`Default::default()`] returns for the Pixel type.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            stride: width,
            data: std::iter::repeat(Pixel::default()).take(width * height).collect(),
        }
    }
}

impl<Pixel: Clone> SharedPixelBuffer<Pixel> {
    /// Creates a new SharedPixelBuffer by cloning and converting pixels from an existing
    /// slice. This function is useful when another crate was used to allocate an image
    /// and you would like to convert it for use in SixtyFPS.
    pub fn clone_from_slice<SourcePixelType>(
        pixel_slice: &[SourcePixelType],
        width: usize,
        height: usize,
    ) -> Self
    where
        [SourcePixelType]: rgb::AsPixels<Pixel>,
    {
        use rgb::AsPixels;
        Self {
            width,
            height,
            stride: width,
            data: pixel_slice.as_pixels().iter().cloned().collect(),
        }
    }
}

/// Convenience alias for a pixel with three color channels (red, green and blue), each
/// encoded as u8.
pub type Rgb8Pixel = rgb::RGB8;
/// Convenience alias for a pixel with four color channels (red, green, blue and alpha), each
/// encoded as u8.
pub type Rgba8Pixel = rgb::RGBA8;

/// SharedImageBuffer is a container for images that are stored in CPU accessible memory.
///
/// The SharedImageBuffer's variants represent the different common formats for encoding
/// images in pixels.
#[derive(Clone, Debug)]
#[repr(C)]
pub enum SharedImageBuffer {
    /// This variant holds the data for an image where each pixel has three color channels (red, green,
    /// and blue) and each channel is encoded as unsigned byte.
    RGB8(SharedPixelBuffer<Rgb8Pixel>),
    /// This variant holds the data for an image where each pixel has four color channels (red, green,
    /// blue and alpha) and each channel is encoded as unsigned byte.
    RGBA8(SharedPixelBuffer<Rgba8Pixel>),
    /// This variant holds the data for an image where each pixel has four color channels (red, green,
    /// blue and alpha) and each channel is encoded as unsigned byte. In contrast to [`Self::RGBA8`],
    /// this variant assumes that the alpha channel is also already multiplied to each red, green and blue
    /// component of each pixel.
    /// Only construct this format if you know that your pixels are encoded this way. It is more efficient
    /// for rendering.
    RGBA8Premultiplied(SharedPixelBuffer<Rgba8Pixel>),
}

impl SharedImageBuffer {
    /// Returns the width of the image in pixels.
    #[inline]
    pub fn width(&self) -> usize {
        match self {
            Self::RGB8(buffer) => buffer.width(),
            Self::RGBA8(buffer) => buffer.width(),
            Self::RGBA8Premultiplied(buffer) => buffer.width(),
        }
    }

    /// Returns the height of the image in pixels.
    #[inline]
    pub fn height(&self) -> usize {
        match self {
            Self::RGB8(buffer) => buffer.height(),
            Self::RGBA8(buffer) => buffer.height(),
            Self::RGBA8Premultiplied(buffer) => buffer.height(),
        }
    }
}

impl PartialEq for SharedImageBuffer {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::RGB8(lhs_buffer) => {
                matches!(other, Self::RGB8(rhs_buffer) if lhs_buffer.data.as_ptr().eq(&rhs_buffer.data.as_ptr()))
            }
            Self::RGBA8(lhs_buffer) => {
                matches!(other, Self::RGBA8(rhs_buffer) if lhs_buffer.data.as_ptr().eq(&rhs_buffer.data.as_ptr()))
            }
            Self::RGBA8Premultiplied(lhs_buffer) => {
                matches!(other, Self::RGBA8Premultiplied(rhs_buffer) if lhs_buffer.data.as_ptr().eq(&rhs_buffer.data.as_ptr()))
            }
        }
    }
}

/// A resource is a reference to binary data, for example images. They can be accessible on the file
/// system or embedded in the resulting binary. Or they might be URLs to a web server and a downloaded
/// is necessary before they can be used.
#[derive(Clone, PartialEq, Debug)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum ImageInner {
    /// A resource that does not represent any data.
    None,
    /// A resource that points to a file in the file system
    AbsoluteFilePath(SharedString),
    /// A image file that is embedded in the program as is. The format is the extension
    EmbeddedData {
        data: Slice<'static, u8>,
        format: Slice<'static, u8>,
    },
    EmbeddedImage(SharedImageBuffer),
}

impl Default for ImageInner {
    fn default() -> Self {
        ImageInner::None
    }
}

impl<'a> From<&'a Image> for &'a ImageInner {
    fn from(other: &'a Image) -> Self {
        &other.0
    }
}

/// Error generated if an image cannot be loaded for any reasons.
#[derive(Default, Debug, PartialEq)]
pub struct LoadImageError(());

/// An image type that can be displayed by the Image element. You can construct
/// Image objects from a path to an image file on disk, using [`Self::load_from_path`].
///
/// Another typical use-case is to render the image content with Rust code.
/// For this it's most efficient to create a new SharedPixelBuffer with the known dimensions
/// and pass the the mutable slice to your rendering function. Afterwards you can create an
/// Image.
///
/// The following example creates a 320x200 RGB pixel buffer and calls an external
/// low_level_render() function to draw a shape into it. Finally the result is
/// stored in an Image with [`Self::from_rgb8()`]:
/// ```
/// # use sixtyfps_corelib::graphics::{SharedPixelBuffer, Image, Rgb8Pixel};
///
/// fn low_level_render(width: usize, height: usize, buffer: &mut [u8]) {
///     // render beautiful circle or other shapes here
/// }
///
/// let mut pixel_buffer = SharedPixelBuffer::<Rgb8Pixel>::new(320, 200);
///
/// low_level_render(pixel_buffer.width(), pixel_buffer.height(),
///                  pixel_buffer.make_mut_bytes());
///
/// let image = Image::from_rgb8(pixel_buffer);
/// ```
///
/// Another use-case is to import existing image data into SixtyFPS, by
/// creating a new Image through cloning of another image type.
///
/// The following example uses the popular [image crate](https://docs.rs/image/) to
/// load a `.png` file from disk, apply brightening filter on it and then import
/// it into an [`Image`]:
/// ```no_run
/// # use sixtyfps_corelib::graphics::{SharedPixelBuffer, Image, Rgba8Pixel};
/// let mut cat_image = image::open("cat.png").expect("Error loading cat image").into_rgba8();
///
/// image::imageops::colorops::brighten_in_place(&mut cat_image, 20);
///
/// let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
///     cat_image.as_raw(),
///     cat_image.width() as _,
///     cat_image.height() as _,
/// );
/// let image = Image::from_rgba8(buffer);
/// ```
///
#[repr(transparent)]
#[derive(Default, Clone, Debug, PartialEq, derive_more::From)]
pub struct Image(ImageInner);

impl Image {
    /// Load an Image from a path to a file containing an image
    pub fn load_from_path(path: &std::path::Path) -> Result<Self, LoadImageError> {
        Ok(Image(ImageInner::AbsoluteFilePath(path.to_str().ok_or(LoadImageError(()))?.into())))
    }

    /// Creates a new Image from the specified shared pixel buffer, where each pixel has three color
    /// channels (red, green and blue) encoded as u8.
    pub fn from_rgb8(buffer: SharedPixelBuffer<Rgb8Pixel>) -> Self {
        Image(ImageInner::EmbeddedImage(SharedImageBuffer::RGB8(buffer)))
    }

    /// Creates a new Image from the specified shared pixel buffer, where each pixel has four color
    /// channels (red, green, blue and alpha) encoded as u8.
    pub fn from_rgba8(buffer: SharedPixelBuffer<Rgba8Pixel>) -> Self {
        Image(ImageInner::EmbeddedImage(SharedImageBuffer::RGBA8(buffer)))
    }

    /// Creates a new Image from the specified shared pixel buffer, where each pixel has four color
    /// channels (red, green, blue and alpha) encoded as u8 and, in contrast to [`Self::from_rgba8`],
    /// the alpha channel is also assumed to be multiplied to the red, green and blue channels.
    ///
    /// Only construct an Image with this function if you know that your pixels are encoded this way.
    pub fn from_rgba8_premultiplied(buffer: SharedPixelBuffer<Rgba8Pixel>) -> Self {
        Image(ImageInner::EmbeddedImage(SharedImageBuffer::RGBA8Premultiplied(buffer)))
    }

    /// Returns the size of the Image in pixels.
    pub fn size(&self) -> crate::graphics::Size {
        match &self.0 {
            ImageInner::None => Default::default(),
            ImageInner::AbsoluteFilePath(_) |  ImageInner::EmbeddedData { .. } => {
                match crate::backend::instance() {
                    Some(backend) => backend.image_size(self),
                    None => panic!("sixtyfps::Image::size() called too early (before a graphics backend was chosen). You need to create a component first."),
                }
            },
            ImageInner::EmbeddedImage(buffer) => [buffer.width() as _, buffer.height() as _].into(),
        }
    }
}

#[test]
fn test_image_size_from_buffer_without_backend() {
    {
        assert_eq!(Image::default().size(), Default::default());
    }
    {
        let buffer = SharedPixelBuffer::<Rgb8Pixel>::new(320, 200);
        let image = Image::from_rgb8(buffer);
        assert_eq!(image.size(), [320., 200.].into())
    }
}

#[cfg(feature = "ffi")]
pub(crate) mod ffi {
    #![allow(unsafe_code)]

    use super::super::Size;
    use super::*;

    /// Expand Rgb8Pixel so that cbindgen can see it. (is in fact rgb::RGB<u8>)
    #[cfg(cbindgen)]
    #[repr(C)]
    struct Rgb8Pixel {
        r: u8,
        g: u8,
        b: u8,
    }

    /// Expand Rgba8Pixel so that cbindgen can see it. (is in fact rgb::RGBA<u8>)
    #[cfg(cbindgen)]
    #[repr(C)]
    struct Rgba8Pixel {
        r: u8,
        g: u8,
        b: u8,
    }

    #[no_mangle]
    pub unsafe extern "C" fn sixtyfps_image_size(image: &Image) -> Size {
        image.size()
    }
}
