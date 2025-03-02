//! jxl-oxide is a JPEG XL decoder written in pure Rust. It's internally organized into a few
//! small crates. This crate acts as a blanket and provides a simple interface made from those
//! crates to decode the actual image.
//!
//! # Decoding an image
//!
//! Decoding a JPEG XL image starts with constructing [`JxlImage`]. First create a builder using
//! [`JxlImage::builder`], and use [`open`][JxlImageBuilder::open] to read a file:
//!
//! ```no_run
//! # use jxl_oxide::JxlImage;
//! let image = JxlImage::builder().open("input.jxl").expect("Failed to read image header");
//! println!("{:?}", image.image_header()); // Prints the image header
//! ```
//!
//! Or, if you're reading from a reader that implements [`Read`][std::io::Read], you can use
//! [`read`][JxlImageBuilder::read]:
//!
//! ```no_run
//! # use jxl_oxide::JxlImage;
//! # let reader = std::io::empty();
//! let image = JxlImage::builder().read(reader).expect("Failed to read image header");
//! println!("{:?}", image.image_header()); // Prints the image header
//! ```
//!
//! In async context, you'll probably want to feed byte buffers directly. In this case, create an
//! image struct with *uninitialized state* using [`build_uninit`][JxlImageBuilder::build_uninit],
//! and call [`feed_bytes`][UninitializedJxlImage::feed_bytes] and
//! [`try_init`][UninitializedJxlImage::try_init]:
//!
//! ```no_run
//! # struct StubReader(&'static [u8]);
//! # impl StubReader {
//! #     fn read(&self) -> StubReaderFuture { StubReaderFuture(self.0) }
//! # }
//! # struct StubReaderFuture(&'static [u8]);
//! # impl std::future::Future for StubReaderFuture {
//! #     type Output = jxl_oxide::Result<&'static [u8]>;
//! #     fn poll(
//! #         self: std::pin::Pin<&mut Self>,
//! #         cx: &mut std::task::Context<'_>,
//! #     ) -> std::task::Poll<Self::Output> {
//! #         std::task::Poll::Ready(Ok(self.0))
//! #     }
//! # }
//! #
//! # use jxl_oxide::{JxlImage, InitializeResult};
//! # async fn run() -> jxl_oxide::Result<()> {
//! # let reader = StubReader(&[
//! #   0xff, 0x0a, 0x30, 0x54, 0x10, 0x09, 0x08, 0x06, 0x01, 0x00, 0x78, 0x00,
//! #   0x4b, 0x38, 0x41, 0x3c, 0xb6, 0x3a, 0x51, 0xfe, 0x00, 0x47, 0x1e, 0xa0,
//! #   0x85, 0xb8, 0x27, 0x1a, 0x48, 0x45, 0x84, 0x1b, 0x71, 0x4f, 0xa8, 0x3e,
//! #   0x8e, 0x30, 0x03, 0x92, 0x84, 0x01,
//! # ]);
//! let mut uninit_image = JxlImage::builder().build_uninit();
//! let image = loop {
//!     uninit_image.feed_bytes(reader.read().await?);
//!     match uninit_image.try_init()? {
//!         InitializeResult::NeedMoreData(uninit) => {
//!             uninit_image = uninit;
//!         }
//!         InitializeResult::Initialized(image) => {
//!             break image;
//!         }
//!     }
//! };
//! println!("{:?}", image.image_header()); // Prints the image header
//! # Ok(())
//! # }
//! ```
//!
//! `JxlImage` parses the image header and embedded ICC profile (if there's any). Use
//! [`JxlImage::render_frame`] to render the image.
//!
//! ```no_run
//! # use jxl_oxide::Render;
//! use jxl_oxide::{JxlImage, RenderResult};
//!
//! # fn present_image(_: Render) {}
//! # fn main() -> jxl_oxide::Result<()> {
//! # let image = JxlImage::builder().open("input.jxl").unwrap();
//! for keyframe_idx in 0..image.num_loaded_keyframes() {
//!     let render = image.render_frame(keyframe_idx)?;
//!     present_image(render);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Color management
//! jxl-oxide has basic color management support, which enables color transformation between
//! well-known color encodings and parsing simple, matrix-based ICC profiles. However, jxl-oxide
//! alone does not support conversion to and from arbitrary ICC profiles, notably CMYK profiles.
//! This includes converting from embedded ICC profiles.
//!
//! Use [`JxlImage::request_color_encoding`] or [`JxlImage::request_icc`] to set color encoding of
//! rendered images. Conversion to and/or from ICC profiles may occur if you do this; in that case,
//! external CMS need to be set using [`JxlImage::set_cms`].
//!
//! ```no_run
//! # use jxl_oxide::{EnumColourEncoding, JxlImage, RenderingIntent};
//! # use jxl_oxide::NullCms as MyCustomCms;
//! # let reader = std::io::empty();
//! let mut image = JxlImage::builder().read(reader).expect("Failed to read image header");
//! image.set_cms(MyCustomCms);
//!
//! let color_encoding = EnumColourEncoding::display_p3(RenderingIntent::Perceptual);
//! image.request_color_encoding(color_encoding);
//! ```
//!
//! External CMS is set to Little CMS 2 by default if `lcms2` feature is enabled. You can
//! explicitly disable this by setting CMS to [`NullCms`].
//!
//! ```no_run
//! # use jxl_oxide::{JxlImage, NullCms};
//! # let reader = std::io::empty();
//! let mut image = JxlImage::builder().read(reader).expect("Failed to read image header");
//! image.set_cms(NullCms);
//! ```
//!
//! ## Not using `set_cms` for color management
//! If implementing `ColorManagementSystem` is difficult for your use case, color management can be
//! done separately using ICC profile of rendered images. [`JxlImage::rendered_icc`] returns ICC
//! profile for further processing.
//!
//! ```no_run
//! # use jxl_oxide::Render;
//! use jxl_oxide::{JxlImage, RenderResult};
//!
//! # fn present_image_with_cms(_: Render, _: &[u8]) {}
//! # fn main() -> jxl_oxide::Result<()> {
//! # let image = JxlImage::builder().open("input.jxl").unwrap();
//! let icc_profile = image.rendered_icc();
//! for keyframe_idx in 0..image.num_loaded_keyframes() {
//!     let render = image.render_frame(keyframe_idx)?;
//!     present_image_with_cms(render, &icc_profile);
//! }
//! # Ok(())
//! # }
//! ```
use std::sync::Arc;

use jxl_bitstream::ContainerDetectingReader;
use jxl_bitstream::Name;
use jxl_bitstream::{Bitstream, Bundle};
use jxl_frame::FrameContext;
use jxl_render::{IndexedFrame, RenderContext};

pub use jxl_color::header as color;
pub use jxl_color::{
    ColorEncodingWithProfile, ColorManagementSystem, EnumColourEncoding, NullCms, RenderingIntent,
};
pub use jxl_frame::header as frame;
pub use jxl_frame::{Frame, FrameHeader};
pub use jxl_grid::{AllocTracker, SimpleGrid};
pub use jxl_image as image;
pub use jxl_image::{ExtraChannelType, ImageHeader};
pub use jxl_threadpool::JxlThreadPool;

mod fb;
#[cfg(feature = "lcms2")]
mod lcms2;

#[cfg(feature = "lcms2")]
pub use self::lcms2::Lcms2;
pub use fb::FrameBuffer;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[cfg(feature = "rayon")]
fn default_pool() -> JxlThreadPool {
    JxlThreadPool::rayon(None)
}

#[cfg(not(feature = "rayon"))]
fn default_pool() -> JxlThreadPool {
    JxlThreadPool::none()
}

/// JPEG XL image decoder builder.
#[derive(Debug, Default)]
pub struct JxlImageBuilder {
    pool: Option<JxlThreadPool>,
    tracker: Option<AllocTracker>,
}

impl JxlImageBuilder {
    /// Sets a custom thread pool.
    pub fn pool(mut self, pool: JxlThreadPool) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Sets an allocation tracker.
    pub fn alloc_tracker(mut self, tracker: AllocTracker) -> Self {
        self.tracker = Some(tracker);
        self
    }

    /// Consumes the builder, and creates an empty, uninitialized JPEG XL image decoder.
    pub fn build_uninit(self) -> UninitializedJxlImage {
        UninitializedJxlImage {
            pool: self.pool.unwrap_or_else(default_pool),
            tracker: self.tracker,
            reader: ContainerDetectingReader::new(),
            buffer: Vec::new(),
        }
    }

    /// Consumes the builder, and creates a JPEG XL image decoder by reading image from the reader.
    pub fn read(self, mut reader: impl std::io::Read) -> Result<JxlImage> {
        let mut uninit = self.build_uninit();
        let mut buf = vec![0u8; 4096];
        let mut image = loop {
            let count = reader.read(&mut buf)?;
            if count == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "reader ended before parsing image header",
                )
                .into());
            }
            let buf = &buf[..count];
            uninit.feed_bytes(buf)?;

            match uninit.try_init()? {
                InitializeResult::NeedMoreData(x) => {
                    uninit = x;
                }
                InitializeResult::Initialized(x) => {
                    break x;
                }
            }
        };

        while !image.end_of_image {
            let count = reader.read(&mut buf)?;
            if count == 0 {
                break;
            }
            let buf = &buf[..count];
            image.feed_bytes(buf)?;
        }

        Ok(image)
    }

    /// Consumes the builder, and creates a JPEG XL image decoder by reading image from the file.
    pub fn open(self, path: impl AsRef<std::path::Path>) -> Result<JxlImage> {
        let file = std::fs::File::open(path)?;
        self.read(file)
    }
}

/// Empty, uninitialized JPEG XL image.
///
/// # Examples
/// ```no_run
/// # fn read_bytes() -> jxl_oxide::Result<&'static [u8]> { Ok(&[]) }
/// # use jxl_oxide::{JxlImage, InitializeResult};
/// # fn main() -> jxl_oxide::Result<()> {
/// let mut uninit_image = JxlImage::builder().build_uninit();
/// let image = loop {
///     let buf = read_bytes()?;
///     uninit_image.feed_bytes(buf)?;
///     match uninit_image.try_init()? {
///         InitializeResult::NeedMoreData(uninit) => {
///             uninit_image = uninit;
///         }
///         InitializeResult::Initialized(image) => {
///             break image;
///         }
///     }
/// };
/// println!("{:?}", image.image_header());
/// # Ok(())
/// # }
/// ```
pub struct UninitializedJxlImage {
    pool: JxlThreadPool,
    tracker: Option<AllocTracker>,
    reader: ContainerDetectingReader,
    buffer: Vec<u8>,
}

impl UninitializedJxlImage {
    /// Feeds more data into the decoder.
    pub fn feed_bytes(&mut self, buf: &[u8]) -> Result<()> {
        self.reader.feed_bytes(buf)?;
        self.buffer.extend(self.reader.take_bytes());
        Ok(())
    }

    /// Returns the internal reader.
    #[inline]
    pub fn reader(&self) -> &ContainerDetectingReader {
        &self.reader
    }

    /// Try to initialize an image with the data fed into so far.
    ///
    /// # Returns
    /// - `Ok(InitializeResult::Initialized(_))` if the initialization was successful,
    /// - `Ok(InitializeResult::NeedMoreData(_))` if the data was not enough, and
    /// - `Err(_)` if there was a decode error during the initialization, meaning invalid bitstream
    ///   was given.
    pub fn try_init(mut self) -> Result<InitializeResult> {
        let mut bitstream = Bitstream::new(&self.buffer);
        let image_header = match ImageHeader::parse(&mut bitstream, ()) {
            Ok(x) => x,
            Err(e) if e.unexpected_eof() => {
                return Ok(InitializeResult::NeedMoreData(self));
            }
            Err(e) => {
                return Err(e.into());
            }
        };

        let embedded_icc = if image_header.metadata.colour_encoding.want_icc() {
            let icc = match jxl_color::icc::read_icc(&mut bitstream) {
                Ok(x) => x,
                Err(e) if e.unexpected_eof() => {
                    return Ok(InitializeResult::NeedMoreData(self));
                }
                Err(e) => {
                    return Err(e.into());
                }
            };
            tracing::debug!("Image has an embedded ICC profile");
            let icc = jxl_color::icc::decode_icc(&icc)?;
            Some(icc)
        } else {
            None
        };
        bitstream.zero_pad_to_byte()?;

        let image_header = Arc::new(image_header);
        let skip_bytes = if image_header.metadata.preview.is_some() {
            let frame = match Frame::parse(
                &mut bitstream,
                FrameContext {
                    image_header: image_header.clone(),
                    tracker: self.tracker.as_ref(),
                    pool: self.pool.clone(),
                },
            ) {
                Ok(x) => x,
                Err(e) if e.unexpected_eof() => {
                    return Ok(InitializeResult::NeedMoreData(self));
                }
                Err(e) => {
                    return Err(e.into());
                }
            };

            let bytes_read = bitstream.num_read_bits() / 8;
            let x = frame.toc().total_byte_size();
            if self.buffer.len() < bytes_read + x {
                return Ok(InitializeResult::NeedMoreData(self));
            }

            x
        } else {
            0usize
        };

        let bytes_read = bitstream.num_read_bits() / 8 + skip_bytes;
        self.buffer.drain(..bytes_read);

        let render_spot_colour = !image_header.metadata.grayscale();

        let mut builder = RenderContext::builder().pool(self.pool.clone());
        if let Some(icc) = embedded_icc {
            builder = builder.embedded_icc(icc);
        }
        if let Some(tracker) = self.tracker {
            builder = builder.alloc_tracker(tracker);
        }
        let mut ctx = builder.build(image_header.clone());
        #[cfg(feature = "lcms2")]
        ctx.set_cms(Lcms2);

        let mut image = JxlImage {
            pool: self.pool.clone(),
            reader: self.reader,
            image_header,
            ctx,
            render_spot_colour,
            end_of_image: false,
            buffer: Vec::new(),
            buffer_offset: bytes_read,
            frame_offsets: Vec::new(),
        };
        image.feed_bytes_inner(&self.buffer)?;

        Ok(InitializeResult::Initialized(image))
    }
}

/// Initialization result from [`UninitializedJxlImage::try_init`].
pub enum InitializeResult {
    /// The data was not enough. Feed more data into the returned image.
    NeedMoreData(UninitializedJxlImage),
    /// The image is successfully initialized.
    Initialized(JxlImage),
}

/// JPEG XL image.
#[derive(Debug)]
pub struct JxlImage {
    pool: JxlThreadPool,
    reader: ContainerDetectingReader,
    image_header: Arc<ImageHeader>,
    ctx: RenderContext,
    render_spot_colour: bool,
    end_of_image: bool,
    buffer: Vec<u8>,
    buffer_offset: usize,
    frame_offsets: Vec<usize>,
}

impl JxlImage {
    /// Creates a decoder builder with default options.
    #[inline]
    pub fn builder() -> JxlImageBuilder {
        JxlImageBuilder::default()
    }

    /// Feeds more data into the decoder.
    pub fn feed_bytes(&mut self, buf: &[u8]) -> Result<()> {
        self.reader.feed_bytes(buf)?;
        let buf = &*self.reader.take_bytes();
        self.feed_bytes_inner(buf)
    }

    fn feed_bytes_inner(&mut self, mut buf: &[u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        if self.end_of_image {
            self.buffer.extend_from_slice(buf);
            return Ok(());
        }

        if let Some(loading_frame) = self.ctx.current_loading_frame() {
            debug_assert!(self.buffer.is_empty());
            let len = buf.len();
            buf = loading_frame.feed_bytes(buf);
            let count = len - buf.len();
            self.buffer_offset += count;

            if loading_frame.is_loading_done() {
                let is_last = loading_frame.header().is_last;
                self.ctx.finalize_current_frame();
                if is_last {
                    self.end_of_image = true;
                    self.buffer = buf.to_vec();
                    return Ok(());
                }
            }
            if buf.is_empty() {
                return Ok(());
            }
        }

        self.buffer.extend_from_slice(buf);
        let mut buf = &*self.buffer;
        while !buf.is_empty() {
            let mut bitstream = Bitstream::new(buf);
            let frame = match self.ctx.load_frame_header(&mut bitstream) {
                Ok(x) => x,
                Err(e) if e.unexpected_eof() => {
                    self.buffer = buf.to_vec();
                    return Ok(());
                }
                Err(e) => {
                    return Err(e.into());
                }
            };
            let frame_index = frame.index();
            assert_eq!(self.frame_offsets.len(), frame_index);
            self.frame_offsets.push(self.buffer_offset);

            let read_bytes = bitstream.num_read_bits() / 8;
            buf = &buf[read_bytes..];
            let len = buf.len();
            buf = frame.feed_bytes(buf);
            let read_bytes = read_bytes + (len - buf.len());
            self.buffer_offset += read_bytes;

            if frame.is_loading_done() {
                let is_last = frame.header().is_last;
                self.ctx.finalize_current_frame();
                if is_last {
                    self.end_of_image = true;
                    self.buffer = buf.to_vec();
                    return Ok(());
                }
            }
        }

        self.buffer.clear();
        Ok(())
    }
}

impl JxlImage {
    /// Returns the image header.
    #[inline]
    pub fn image_header(&self) -> &ImageHeader {
        &self.image_header
    }

    /// Returns the image width with orientation applied.
    #[inline]
    pub fn width(&self) -> u32 {
        self.image_header.width_with_orientation()
    }

    /// Returns the image height with orientation applied.
    #[inline]
    pub fn height(&self) -> u32 {
        self.image_header.height_with_orientation()
    }

    /// Sets color management system implementation to be used by the renderer.
    #[inline]
    pub fn set_cms(&mut self, cms: impl ColorManagementSystem + Send + Sync + 'static) {
        self.ctx.set_cms(cms);
    }

    /// Returns the *original* ICC profile embedded in the image.
    #[inline]
    pub fn original_icc(&self) -> Option<&[u8]> {
        self.ctx.embedded_icc()
    }

    /// Returns the ICC profile that describes rendered images.
    ///
    /// The returned profile will change if different color encoding is specified using
    /// [`request_icc`][Self::request_icc] or
    /// [`request_color_encoding`][Self::request_color_encoding].
    pub fn rendered_icc(&self) -> Vec<u8> {
        let encoding = self.ctx.requested_color_encoding();
        match encoding.encoding() {
            jxl_color::ColourEncoding::Enum(encoding) => {
                jxl_color::icc::colour_encoding_to_icc(encoding)
            }
            jxl_color::ColourEncoding::IccProfile(_) => encoding.icc_profile().to_vec(),
        }
    }

    /// Returns the CICP tag of the color encoding of rendered images, if there's any.
    #[inline]
    pub fn rendered_cicp(&self) -> Option<[u8; 4]> {
        let encoding = self.ctx.requested_color_encoding();
        encoding.encoding().cicp()
    }

    /// Returns the pixel format of the rendered image.
    pub fn pixel_format(&self) -> PixelFormat {
        use jxl_color::{ColourEncoding, ColourSpace};

        let encoding = self.ctx.requested_color_encoding();
        let (is_grayscale, has_black) = match encoding.encoding() {
            ColourEncoding::Enum(EnumColourEncoding {
                colour_space: ColourSpace::Grey,
                ..
            }) => (true, false),
            ColourEncoding::Enum(_) => (false, false),
            ColourEncoding::IccProfile(_) => {
                let profile = encoding.icc_profile();
                if profile.len() < 0x14 {
                    (false, false)
                } else {
                    match &profile[0x10..0x14] {
                        [b'G', b'R', b'A', b'Y'] => (true, false),
                        [b'C', b'M', b'Y', b'K'] => (false, true),
                        _ => (false, false),
                    }
                }
            }
        };
        let mut has_alpha = false;
        for ec_info in &self.image_header.metadata.ec_info {
            if ec_info.is_alpha() {
                has_alpha = true;
            }
        }

        match (is_grayscale, has_black, has_alpha) {
            (false, false, false) => PixelFormat::Rgb,
            (false, false, true) => PixelFormat::Rgba,
            (false, true, false) => PixelFormat::Cmyk,
            (false, true, true) => PixelFormat::Cmyka,
            (true, _, false) => PixelFormat::Gray,
            (true, _, true) => PixelFormat::Graya,
        }
    }

    /// Requests the decoder to render in specific color encoding, described by an ICC profile.
    ///
    /// # Errors
    /// This function will return an error if it cannot parse the ICC profile.
    pub fn request_icc(&mut self, icc_profile: &[u8]) -> Result<()> {
        self.ctx
            .request_color_encoding(ColorEncodingWithProfile::with_icc(icc_profile)?);
        Ok(())
    }

    /// Requests the decoder to render in specific color encoding, described by
    /// `EnumColourEncoding`.
    pub fn request_color_encoding(&mut self, color_encoding: EnumColourEncoding) {
        self.ctx
            .request_color_encoding(ColorEncodingWithProfile::new(color_encoding))
    }

    /// Returns whether the spot color channels will be rendered.
    #[inline]
    pub fn render_spot_colour(&self) -> bool {
        self.render_spot_colour
    }

    /// Sets whether the spot colour channels will be rendered.
    #[inline]
    pub fn set_render_spot_colour(&mut self, render_spot_colour: bool) -> &mut Self {
        if render_spot_colour && self.image_header.metadata.grayscale() {
            tracing::warn!("Spot colour channels are not rendered on grayscale images");
            return self;
        }
        self.render_spot_colour = render_spot_colour;
        self
    }
}

impl JxlImage {
    /// Returns the number of currently loaded keyframes.
    #[inline]
    pub fn num_loaded_keyframes(&self) -> usize {
        self.ctx.loaded_keyframes()
    }

    /// Returns the number of currently loaded frames, including frames that are not displayed
    /// directly.
    #[inline]
    pub fn num_loaded_frames(&self) -> usize {
        self.ctx.loaded_frames()
    }

    /// Returns whether the image is loaded completely, without missing animation keyframes or
    /// partially loaded frames.
    #[inline]
    pub fn is_loading_done(&self) -> bool {
        self.end_of_image
    }

    /// Returns frame data by keyframe index.
    pub fn frame_by_keyframe(&self, keyframe_index: usize) -> Option<&IndexedFrame> {
        self.ctx.keyframe(keyframe_index)
    }

    /// Returns the frame header for the given keyframe index, or `None` if the keyframe does not
    /// exist.
    pub fn frame_header(&self, keyframe_index: usize) -> Option<&FrameHeader> {
        let frame = self.ctx.keyframe(keyframe_index)?;
        Some(frame.header())
    }

    /// Returns frame data by frame index, including frames that are not displayed directly.
    ///
    /// There are some situations where a frame is not displayed directly:
    /// - It may be marked as reference only, and meant to be only used by other frames.
    /// - It may contain LF image (which is 8x downsampled version) of another VarDCT frame.
    /// - Zero duration frame that is not the last frame of image is blended with following frames
    ///   and displayed together.
    pub fn frame(&self, frame_idx: usize) -> Option<&IndexedFrame> {
        self.ctx.frame(frame_idx)
    }

    /// Returns the offset of frame within codestream, in bytes.
    pub fn frame_offset(&self, frame_index: usize) -> Option<usize> {
        self.frame_offsets.get(frame_index).copied()
    }
}

impl JxlImage {
    /// Renders the given keyframe.
    pub fn render_frame(&self, keyframe_index: usize) -> Result<Render> {
        self.render_frame_cropped(keyframe_index, None)
    }

    /// Renders the given keyframe with optional cropping region.
    pub fn render_frame_cropped(
        &self,
        keyframe_index: usize,
        image_region: Option<CropInfo>,
    ) -> Result<Render> {
        let mut grids = self
            .ctx
            .render_keyframe(keyframe_index, image_region.map(From::from))?;
        let grids = grids.take_buffer();
        let (color_channels, extra_channels) = self.process_render(grids)?;

        let frame = self.ctx.keyframe(keyframe_index).unwrap();
        let frame_header = frame.header();
        let result = Render {
            keyframe_index,
            name: frame_header.name.clone(),
            duration: frame_header.duration,
            orientation: self.image_header.metadata.orientation,
            color_channels,
            extra_channels,
        };
        Ok(result)
    }

    /// Renders the currently loading keyframe.
    pub fn render_loading_frame(&mut self) -> Result<Render> {
        self.render_loading_frame_cropped(None)
    }

    /// Renders the currently loading keyframe with optional cropping region.
    pub fn render_loading_frame_cropped(
        &mut self,
        image_region: Option<CropInfo>,
    ) -> Result<Render> {
        let (frame, mut grids) = self
            .ctx
            .render_loading_keyframe(image_region.map(From::from))?;
        let frame_header = frame.header();
        let name = frame_header.name.clone();
        let duration = frame_header.duration;

        let grids = grids.take_buffer();
        let (color_channels, extra_channels) = self.process_render(grids)?;

        let result = Render {
            keyframe_index: self.ctx.loaded_keyframes(),
            name,
            duration,
            orientation: self.image_header.metadata.orientation,
            color_channels,
            extra_channels,
        };
        Ok(result)
    }

    fn process_render(
        &self,
        mut grids: Vec<SimpleGrid<f32>>,
    ) -> Result<(Vec<SimpleGrid<f32>>, Vec<ExtraChannel>)> {
        let pixel_format = self.pixel_format();
        let color_channels = if pixel_format.is_grayscale() { 1 } else { 3 };
        let mut color_channels: Vec<_> = grids.drain(..color_channels).collect();
        let extra_channels: Vec<_> = grids
            .into_iter()
            .zip(&self.image_header.metadata.ec_info)
            .map(|(grid, ec_info)| ExtraChannel {
                ty: ec_info.ty,
                name: ec_info.name.clone(),
                grid,
            })
            .filter(|x| !x.is_black() || pixel_format.has_black()) // filter black channel
            .collect();

        if self.render_spot_colour {
            for ec in &extra_channels {
                if ec.is_spot_colour() {
                    jxl_render::render_spot_color(&mut color_channels, &ec.grid, &ec.ty)?;
                }
            }
        }

        Ok((color_channels, extra_channels))
    }
}

impl JxlImage {
    /// Returns the thread pool used by the renderer.
    #[inline]
    pub fn pool(&self) -> &JxlThreadPool {
        &self.pool
    }

    /// Returns the internal reader.
    pub fn reader(&self) -> &ContainerDetectingReader {
        &self.reader
    }
}

/// Pixel format of the rendered image.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PixelFormat {
    /// Grayscale, single channel
    Gray,
    /// Grayscale with alpha, two channels
    Graya,
    /// RGB, three channels
    Rgb,
    /// RGB with alpha, four channels
    Rgba,
    /// CMYK, four channels
    Cmyk,
    /// CMYK with alpha, five channels
    Cmyka,
}

impl PixelFormat {
    /// Returns the number of channels of the image.
    #[inline]
    pub fn channels(self) -> usize {
        match self {
            PixelFormat::Gray => 1,
            PixelFormat::Graya => 2,
            PixelFormat::Rgb => 3,
            PixelFormat::Rgba => 4,
            PixelFormat::Cmyk => 4,
            PixelFormat::Cmyka => 5,
        }
    }

    /// Returns whether the image is grayscale.
    #[inline]
    pub fn is_grayscale(self) -> bool {
        matches!(self, Self::Gray | Self::Graya)
    }

    /// Returns whether the image has an alpha channel.
    #[inline]
    pub fn has_alpha(self) -> bool {
        matches!(
            self,
            PixelFormat::Graya | PixelFormat::Rgba | PixelFormat::Cmyka
        )
    }

    /// Returns whether the image has a black channel.
    #[inline]
    pub fn has_black(self) -> bool {
        matches!(self, PixelFormat::Cmyk | PixelFormat::Cmyka)
    }
}

/// The result of loading the keyframe.
#[derive(Debug)]
pub enum LoadResult {
    /// The frame is loaded with the given keyframe index.
    Done(usize),
    /// More data is needed to fully load the frame.
    NeedMoreData,
    /// No more frames are present.
    NoMoreFrames,
}

/// The result of loading and rendering the keyframe.
#[derive(Debug)]
pub enum RenderResult {
    /// The frame is rendered.
    Done(Render),
    /// More data is needed to fully render the frame.
    NeedMoreData,
    /// No more frames are present.
    NoMoreFrames,
}

/// The result of rendering a keyframe.
#[derive(Debug)]
pub struct Render {
    keyframe_index: usize,
    name: Name,
    duration: u32,
    orientation: u32,
    color_channels: Vec<SimpleGrid<f32>>,
    extra_channels: Vec<ExtraChannel>,
}

impl Render {
    /// Returns the keyframe index.
    #[inline]
    pub fn keyframe_index(&self) -> usize {
        self.keyframe_index
    }

    /// Returns the name of the frame.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns how many ticks this frame is presented.
    #[inline]
    pub fn duration(&self) -> u32 {
        self.duration
    }

    /// Returns the orientation of the image.
    #[inline]
    pub fn orientation(&self) -> u32 {
        self.orientation
    }

    /// Creates a buffer with interleaved channels, with orientation applied.
    ///
    /// Extra channels other than black and alpha are not included.
    #[inline]
    pub fn image(&self) -> FrameBuffer {
        let mut fb: Vec<_> = self.color_channels.iter().collect();

        // Find black
        for ec in &self.extra_channels {
            if ec.is_black() {
                fb.push(&ec.grid);
                break;
            }
        }
        // Find alpha
        for ec in &self.extra_channels {
            if ec.is_alpha() {
                fb.push(&ec.grid);
                break;
            }
        }

        FrameBuffer::from_grids(&fb, self.orientation)
    }

    /// Creates a buffer with interleaved channels, with orientation applied.
    ///
    /// All extra channels are included.
    #[inline]
    pub fn image_all_channels(&self) -> FrameBuffer {
        let mut fb: Vec<_> = self.color_channels.iter().collect();
        for ec in &self.extra_channels {
            fb.push(&ec.grid);
        }

        FrameBuffer::from_grids(&fb, self.orientation)
    }

    /// Creates a separate buffer by channel, with orientation applied.
    ///
    /// All extra channels are included.
    pub fn image_planar(&self) -> Vec<FrameBuffer> {
        self.color_channels
            .iter()
            .chain(self.extra_channels.iter().map(|x| &x.grid))
            .map(|x| FrameBuffer::from_grids(&[x], self.orientation))
            .collect()
    }

    /// Returns the color channels.
    ///
    /// Orientation is not applied.
    #[inline]
    pub fn color_channels(&self) -> &[SimpleGrid<f32>] {
        &self.color_channels
    }

    /// Returns the mutable slice to the color channels.
    ///
    /// Orientation is not applied.
    #[inline]
    pub fn color_channels_mut(&mut self) -> &mut [SimpleGrid<f32>] {
        &mut self.color_channels
    }

    /// Returns the extra channels, potentially including alpha and black channels.
    ///
    /// Orientation is not applied.
    #[inline]
    pub fn extra_channels(&self) -> &[ExtraChannel] {
        &self.extra_channels
    }

    /// Returns the mutable slice to the extra channels, potentially including alpha and black
    /// channels.
    ///
    /// Orientation is not applied.
    #[inline]
    pub fn extra_channels_mut(&mut self) -> &mut [ExtraChannel] {
        &mut self.extra_channels
    }
}

impl Render {
    /// Creates a stream that writes to borrowed buffer.
    ///
    /// The stream will include black and alpha channels, if exists, in addition to color channels.
    /// Orientation is applied.
    pub fn stream(&self) -> ImageStream {
        let orientation = self.orientation;
        assert!((1..=8).contains(&orientation));
        let mut width = self.color_channels[0].width() as u32;
        let mut height = self.color_channels[0].height() as u32;
        if orientation >= 5 {
            std::mem::swap(&mut width, &mut height);
        }
        let mut grids: Vec<_> = self.color_channels.iter().collect();

        // Find black
        for ec in &self.extra_channels {
            if ec.is_black() {
                grids.push(&ec.grid);
                break;
            }
        }
        // Find alpha
        for ec in &self.extra_channels {
            if ec.is_alpha() {
                grids.push(&ec.grid);
                break;
            }
        }

        ImageStream {
            orientation,
            width,
            height,
            grids,
            y: 0,
            x: 0,
            c: 0,
        }
    }
}

/// Image stream that writes to borrowed buffer.
pub struct ImageStream<'r> {
    orientation: u32,
    width: u32,
    height: u32,
    grids: Vec<&'r SimpleGrid<f32>>,
    y: u32,
    x: u32,
    c: u32,
}

impl ImageStream<'_> {
    /// Returns width of the image.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns height of the image.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the number of channels of the image.
    #[inline]
    pub fn channels(&self) -> u32 {
        self.grids.len() as u32
    }

    /// Writes next samples to the buffer, returning how many samples are written.
    pub fn write_to_buffer(&mut self, buf: &mut [f32]) -> usize {
        let channels = self.grids.len() as u32;
        let mut buf_it = buf.iter_mut();
        let mut count = 0usize;
        'outer: while self.y < self.height {
            while self.x < self.width {
                while self.c < channels {
                    let Some(v) = buf_it.next() else {
                        break 'outer;
                    };
                    let (x, y) = self.to_original_coord(self.x, self.y);
                    *v = *self.grids[self.c as usize]
                        .get(x as usize, y as usize)
                        .unwrap();
                    count += 1;
                    self.c += 1;
                }
                self.c = 0;
                self.x += 1;
            }
            self.x = 0;
            self.y += 1;
        }
        count
    }

    #[inline]
    fn to_original_coord(&self, x: u32, y: u32) -> (u32, u32) {
        let width = self.width;
        let height = self.height;
        match self.orientation {
            1 => (x, y),
            2 => (width - x - 1, y),
            3 => (width - x - 1, height - y - 1),
            4 => (x, height - y - 1),
            5 => (y, x),
            6 => (y, width - x - 1),
            7 => (height - y - 1, width - x - 1),
            8 => (height - y - 1, x),
            _ => unreachable!(),
        }
    }
}

/// Extra channel of the image.
#[derive(Debug)]
pub struct ExtraChannel {
    ty: ExtraChannelType,
    name: Name,
    grid: SimpleGrid<f32>,
}

impl ExtraChannel {
    /// Returns the type of the extra channel.
    #[inline]
    pub fn ty(&self) -> ExtraChannelType {
        self.ty
    }

    /// Returns the name of the channel.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the sample grid of the channel.
    #[inline]
    pub fn grid(&self) -> &SimpleGrid<f32> {
        &self.grid
    }

    /// Returns the mutable sample grid of the channel.
    #[inline]
    pub fn grid_mut(&mut self) -> &mut SimpleGrid<f32> {
        &mut self.grid
    }

    /// Returns `true` if the channel is a black channel of CMYK image.
    #[inline]
    pub fn is_black(&self) -> bool {
        matches!(self.ty, ExtraChannelType::Black)
    }

    /// Returns `true` if the channel is an alpha channel.
    #[inline]
    pub fn is_alpha(&self) -> bool {
        matches!(self.ty, ExtraChannelType::Alpha { .. })
    }

    /// Returns `true` if the channel is a spot colour channel.
    #[inline]
    pub fn is_spot_colour(&self) -> bool {
        matches!(self.ty, ExtraChannelType::SpotColour { .. })
    }
}

/// Cropping region information.
#[derive(Debug, Default, Copy, Clone)]
pub struct CropInfo {
    pub width: u32,
    pub height: u32,
    pub left: u32,
    pub top: u32,
}

impl From<CropInfo> for jxl_render::Region {
    fn from(value: CropInfo) -> Self {
        Self {
            left: value.left as i32,
            top: value.top as i32,
            width: value.width,
            height: value.height,
        }
    }
}
