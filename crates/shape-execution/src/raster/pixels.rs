//! Portable fixed-point pixel math shared by alpha-aware raster algorithms.

use image::{Rgba, RgbaImage};

const CHANNEL_MAX: u32 = u16::MAX as u32;

// IEC 61966-2-1 sRGB values converted to 0..65535 linear light with
// round-half-up. Keeping this table in the algorithm revision avoids platform
// libm differences in accepted output bytes.
const SRGB_TO_LINEAR: [u16; 256] = [
    0, 20, 40, 60, 80, 99, 119, 139, 159, 179, 199, 219, 241, 264, 288, 313, 340, 367, 396, 427,
    458, 491, 526, 562, 599, 637, 677, 718, 761, 805, 851, 898, 947, 997, 1048, 1101, 1156, 1212,
    1270, 1330, 1391, 1453, 1517, 1583, 1651, 1720, 1790, 1863, 1937, 2013, 2090, 2170, 2250, 2333,
    2418, 2504, 2592, 2681, 2773, 2866, 2961, 3058, 3157, 3258, 3360, 3464, 3570, 3678, 3788, 3900,
    4014, 4129, 4247, 4366, 4488, 4611, 4736, 4864, 4993, 5124, 5257, 5392, 5530, 5669, 5810, 5953,
    6099, 6246, 6395, 6547, 6700, 6856, 7014, 7174, 7335, 7500, 7666, 7834, 8004, 8177, 8352, 8528,
    8708, 8889, 9072, 9258, 9445, 9635, 9828, 10022, 10219, 10417, 10619, 10822, 11028, 11235,
    11446, 11658, 11873, 12090, 12309, 12530, 12754, 12980, 13209, 13440, 13673, 13909, 14146,
    14387, 14629, 14874, 15122, 15371, 15623, 15878, 16135, 16394, 16656, 16920, 17187, 17456,
    17727, 18001, 18277, 18556, 18837, 19121, 19407, 19696, 19987, 20281, 20577, 20876, 21177,
    21481, 21787, 22096, 22407, 22721, 23038, 23357, 23678, 24002, 24329, 24658, 24990, 25325,
    25662, 26001, 26344, 26688, 27036, 27386, 27739, 28094, 28452, 28813, 29176, 29542, 29911,
    30282, 30656, 31033, 31412, 31794, 32179, 32567, 32957, 33350, 33745, 34143, 34544, 34948,
    35355, 35764, 36176, 36591, 37008, 37429, 37852, 38278, 38706, 39138, 39572, 40009, 40449,
    40891, 41337, 41785, 42236, 42690, 43147, 43606, 44069, 44534, 45002, 45473, 45947, 46423,
    46903, 47385, 47871, 48359, 48850, 49344, 49841, 50341, 50844, 51349, 51858, 52369, 52884,
    53401, 53921, 54445, 54971, 55500, 56032, 56567, 57105, 57646, 58190, 58737, 59287, 59840,
    60396, 60955, 61517, 62082, 62650, 63221, 63795, 64372, 64952, 65535,
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct LinearPremultiplied(pub(super) [u16; 4]);

impl LinearPremultiplied {
    pub(super) fn from_straight_srgb(pixel: Rgba<u8>) -> Self {
        let alpha = u32::from(pixel.0[3]) * 257;
        let premultiply = |channel: u8| -> u16 {
            let linear = u32::from(SRGB_TO_LINEAR[usize::from(channel)]);
            u16::try_from((linear * alpha + CHANNEL_MAX / 2) / CHANNEL_MAX)
                .expect("premultiplied channel remains u16")
        };
        Self([
            premultiply(pixel.0[0]),
            premultiply(pixel.0[1]),
            premultiply(pixel.0[2]),
            u16::try_from(alpha).expect("expanded alpha remains u16"),
        ])
    }

    pub(super) fn to_straight_srgb(self) -> Rgba<u8> {
        let alpha = u32::from(self.0[3]);
        if alpha == 0 {
            return Rgba([0, 0, 0, 0]);
        }
        let unpremultiply = |channel: u16| -> u8 {
            let linear = (u32::from(channel) * CHANNEL_MAX + alpha / 2) / alpha;
            linear_to_srgb(u16::try_from(linear.min(CHANNEL_MAX)).expect("clamped linear channel"))
        };
        Rgba([
            unpremultiply(self.0[0]),
            unpremultiply(self.0[1]),
            unpremultiply(self.0[2]),
            u8::try_from((alpha + 128) / 257).expect("expanded alpha contracts to u8"),
        ])
    }

    #[must_use]
    pub(super) const fn alpha(self) -> u16 {
        self.0[3]
    }

    #[must_use]
    pub(super) const fn alpha_only(self) -> Self {
        Self([0, 0, 0, self.0[3]])
    }

    #[must_use]
    pub(super) fn tint(color: Rgba<u8>, coverage: u16) -> Self {
        let color_alpha = u32::from(color.0[3]) * 257;
        let alpha = (u32::from(coverage) * color_alpha + CHANNEL_MAX / 2) / CHANNEL_MAX;
        let premultiply = |channel: u8| -> u16 {
            let linear = u32::from(SRGB_TO_LINEAR[usize::from(channel)]);
            u16::try_from((linear * alpha + CHANNEL_MAX / 2) / CHANNEL_MAX)
                .expect("tinted premultiplied channel remains u16")
        };
        Self([
            premultiply(color.0[0]),
            premultiply(color.0[1]),
            premultiply(color.0[2]),
            u16::try_from(alpha).expect("combined alpha remains u16"),
        ])
    }
}

pub(super) fn linear_premultiplied(image: &RgbaImage) -> Vec<LinearPremultiplied> {
    image
        .pixels()
        .copied()
        .map(LinearPremultiplied::from_straight_srgb)
        .collect()
}

pub(super) fn straight_srgb_image(
    pixels: Vec<LinearPremultiplied>,
    width: u32,
    height: u32,
) -> RgbaImage {
    let mut output = RgbaImage::new(width, height);
    for (target, pixel) in output.pixels_mut().zip(pixels) {
        *target = pixel.to_straight_srgb();
    }
    output
}

pub(super) fn write_straight_srgb(
    image: &mut RgbaImage,
    pixels: impl IntoIterator<Item = LinearPremultiplied>,
) {
    for (target, pixel) in image.pixels_mut().zip(pixels) {
        *target = pixel.to_straight_srgb();
    }
}

/// Runs one exact three-box Gaussian approximation in premultiplied linear light.
pub(super) fn gaussian_blur_clamped(
    pixels: Vec<LinearPremultiplied>,
    width: u32,
    height: u32,
    radius: u16,
) -> Vec<LinearPremultiplied> {
    gaussian_blur(pixels, width, height, radius, EdgeMode::Clamp)
}

pub(super) fn gaussian_blur_transparent(
    pixels: Vec<LinearPremultiplied>,
    width: u32,
    height: u32,
    radius: u16,
) -> Vec<LinearPremultiplied> {
    gaussian_blur(pixels, width, height, radius, EdgeMode::Transparent)
}

#[must_use]
pub(super) fn source_over(
    destination: LinearPremultiplied,
    source: LinearPremultiplied,
) -> LinearPremultiplied {
    let inverse_source_alpha = CHANNEL_MAX - u32::from(source.alpha());
    let mut output = [0_u16; 4];
    for (index, target) in output.iter_mut().enumerate() {
        let value = u32::from(source.0[index])
            + (u32::from(destination.0[index]) * inverse_source_alpha + CHANNEL_MAX / 2)
                / CHANNEL_MAX;
        *target =
            u16::try_from(value.min(CHANNEL_MAX)).expect("source-over channel remains bounded");
    }
    LinearPremultiplied(output)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeMode {
    Clamp,
    Transparent,
}

fn gaussian_blur(
    mut pixels: Vec<LinearPremultiplied>,
    width: u32,
    height: u32,
    radius: u16,
    edge_mode: EdgeMode,
) -> Vec<LinearPremultiplied> {
    if radius == 0 {
        return pixels;
    }
    let width = usize::try_from(width).expect("portable raster width fits usize");
    let height = usize::try_from(height).expect("portable raster height fits usize");
    let radius = usize::from(radius);
    let mut scratch = vec![LinearPremultiplied::default(); pixels.len()];
    for _ in 0..3 {
        box_blur_horizontal(&pixels, &mut scratch, width, height, radius, edge_mode);
        box_blur_vertical(&scratch, &mut pixels, width, height, radius, edge_mode);
    }
    pixels
}

fn box_blur_horizontal(
    source: &[LinearPremultiplied],
    output: &mut [LinearPremultiplied],
    width: usize,
    height: usize,
    radius: usize,
    edge_mode: EdgeMode,
) {
    debug_assert_eq!(source.len(), output.len());
    let window = radius * 2 + 1;
    for y in 0..height {
        let row = y * width;
        let mut sums = [0_u64; 4];
        let radius_signed = isize::try_from(radius).expect("bounded radius fits isize");
        for x in -radius_signed..=radius_signed {
            add(
                &mut sums,
                sample_horizontal(source, row, width, x, edge_mode),
            );
        }
        for x in 0..width {
            output[row + x] = average(sums, window);
            let x = isize::try_from(x).expect("portable x fits isize");
            subtract(
                &mut sums,
                sample_horizontal(source, row, width, x - radius_signed, edge_mode),
            );
            add(
                &mut sums,
                sample_horizontal(source, row, width, x + radius_signed + 1, edge_mode),
            );
        }
    }
}

fn box_blur_vertical(
    source: &[LinearPremultiplied],
    output: &mut [LinearPremultiplied],
    width: usize,
    height: usize,
    radius: usize,
    edge_mode: EdgeMode,
) {
    debug_assert_eq!(source.len(), output.len());
    let window = radius * 2 + 1;
    for x in 0..width {
        let mut sums = [0_u64; 4];
        let radius_signed = isize::try_from(radius).expect("bounded radius fits isize");
        for y in -radius_signed..=radius_signed {
            add(
                &mut sums,
                sample_vertical(source, width, height, x, y, edge_mode),
            );
        }
        for y in 0..height {
            output[y * width + x] = average(sums, window);
            let y = isize::try_from(y).expect("portable y fits isize");
            subtract(
                &mut sums,
                sample_vertical(source, width, height, x, y - radius_signed, edge_mode),
            );
            add(
                &mut sums,
                sample_vertical(source, width, height, x, y + radius_signed + 1, edge_mode),
            );
        }
    }
}

fn sample_horizontal(
    source: &[LinearPremultiplied],
    row: usize,
    width: usize,
    x: isize,
    edge_mode: EdgeMode,
) -> LinearPremultiplied {
    sample_index(x, width, edge_mode).map_or_else(LinearPremultiplied::default, |x| source[row + x])
}

fn sample_vertical(
    source: &[LinearPremultiplied],
    width: usize,
    height: usize,
    x: usize,
    y: isize,
    edge_mode: EdgeMode,
) -> LinearPremultiplied {
    sample_index(y, height, edge_mode)
        .map_or_else(LinearPremultiplied::default, |y| source[y * width + x])
}

fn sample_index(coordinate: isize, length: usize, edge_mode: EdgeMode) -> Option<usize> {
    let length = isize::try_from(length).expect("portable dimension fits isize");
    match edge_mode {
        EdgeMode::Clamp => Some(
            usize::try_from(coordinate.clamp(0, length - 1))
                .expect("clamped coordinate remains nonnegative"),
        ),
        EdgeMode::Transparent => (0..length)
            .contains(&coordinate)
            .then(|| usize::try_from(coordinate).expect("in-range coordinate remains nonnegative")),
    }
}

fn add(sums: &mut [u64; 4], pixel: LinearPremultiplied) {
    for (sum, channel) in sums.iter_mut().zip(pixel.0) {
        *sum += u64::from(channel);
    }
}

fn subtract(sums: &mut [u64; 4], pixel: LinearPremultiplied) {
    for (sum, channel) in sums.iter_mut().zip(pixel.0) {
        *sum -= u64::from(channel);
    }
}

fn average(sums: [u64; 4], window: usize) -> LinearPremultiplied {
    let divisor = u64::try_from(window).expect("bounded blur window fits u64");
    let mut channels = [0_u16; 4];
    for (target, sum) in channels.iter_mut().zip(sums) {
        *target = u16::try_from((sum + divisor / 2) / divisor)
            .expect("average of u16 channels remains u16");
    }
    LinearPremultiplied(channels)
}

fn linear_to_srgb(value: u16) -> u8 {
    match SRGB_TO_LINEAR.binary_search(&value) {
        Ok(index) => u8::try_from(index).expect("sRGB table index remains u8"),
        Err(0) => 0,
        Err(256) => u8::MAX,
        Err(upper) => {
            let lower = upper - 1;
            let lower_distance = value - SRGB_TO_LINEAR[lower];
            let upper_distance = SRGB_TO_LINEAR[upper] - value;
            u8::try_from(if lower_distance < upper_distance {
                lower
            } else {
                upper
            })
            .expect("sRGB table index remains u8")
        }
    }
}

#[cfg(test)]
mod tests;
