use image::{Rgba, RgbaImage};

use super::*;

#[test]
fn srgb_linear_round_trip_is_exact_for_every_channel_value() {
    for value in 0..=u8::MAX {
        let pixel = LinearPremultiplied::from_straight_srgb(Rgba([value, value, value, 255]));
        assert_eq!(pixel.to_straight_srgb(), Rgba([value, value, value, 255]));
    }
}

#[test]
fn transparent_color_is_canonicalized_without_fringe_payload() {
    let pixel = LinearPremultiplied::from_straight_srgb(Rgba([255, 20, 60, 0]));
    assert_eq!(pixel.to_straight_srgb(), Rgba([0, 0, 0, 0]));
}

#[test]
fn clamped_blur_preserves_a_constant_translucent_image() {
    let image = RgbaImage::from_pixel(5, 3, Rgba([127, 64, 20, 113]));
    let blurred = gaussian_blur_clamped(linear_premultiplied(&image), 5, 3, 4);
    let output = straight_srgb_image(blurred, 5, 3);
    assert!(
        output
            .pixels()
            .all(|pixel| *pixel == Rgba([127, 64, 20, 113]))
    );
}

#[test]
fn alpha_aware_blur_does_not_reveal_hidden_transparent_red() {
    let mut image = RgbaImage::from_pixel(3, 1, Rgba([255, 0, 0, 0]));
    image.put_pixel(1, 0, Rgba([0, 0, 255, 255]));
    let blurred = gaussian_blur_clamped(linear_premultiplied(&image), 3, 1, 1);
    let output = straight_srgb_image(blurred, 3, 1);

    assert!(output.pixels().all(|pixel| pixel.0[0] == 0));
    assert!(output.pixels().all(|pixel| pixel.0[2] == 255));
    assert!(output.pixels().all(|pixel| pixel.0[3] > 0));
}

#[test]
fn transparent_edges_preserve_the_exact_three_pass_support() {
    let mut image = RgbaImage::from_pixel(9, 9, Rgba([0, 0, 0, 0]));
    image.put_pixel(4, 4, Rgba([255, 255, 255, 255]));
    let blurred = gaussian_blur_transparent(linear_premultiplied(&image), 9, 9, 1);
    let output = straight_srgb_image(blurred, 9, 9);
    assert!(output.get_pixel(1, 4).0[3] > 0);
    assert_eq!(output.get_pixel(0, 4).0[3], 0);
}

#[test]
fn source_over_uses_premultiplied_linear_alpha() {
    let destination = LinearPremultiplied::from_straight_srgb(Rgba([0, 0, 255, 255]));
    let source = LinearPremultiplied::from_straight_srgb(Rgba([255, 0, 0, 128]));
    let composed = source_over(destination, source).to_straight_srgb();
    assert_eq!(composed.0[3], 255);
    assert!(composed.0[0] >= 187 && composed.0[0] <= 188);
    assert!(composed.0[2] >= 187 && composed.0[2] <= 188);
}
