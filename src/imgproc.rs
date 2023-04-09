use image::imageops::{resize, FilterType};
use image::RgbImage;

fn is_same(img1: &RgbImage, img2: &RgbImage) -> bool {
    let img1 = resize(img1, 4, 4, FilterType::CatmullRom);
    let img2 = resize(img2, 4, 4, FilterType::CatmullRom);

    let zipped = img1.pixels().zip(img2.pixels());
    let mut diffs: Vec<i32> = vec![];
    for (img1_pix, img2_pix) in zipped.into_iter() {
        let r1 = *img1_pix.0.get(0).unwrap() as i32;
        let g1 = *img1_pix.0.get(1).unwrap() as i32;
        let b1 = *img1_pix.0.get(2).unwrap() as i32;

        let r2 = *img2_pix.0.get(0).unwrap() as i32;
        let g2 = *img2_pix.0.get(1).unwrap() as i32;
        let b2 = *img2_pix.0.get(2).unwrap() as i32;
        diffs.append(&mut vec![r1 - r2, g1 - g2, b1 - b2]);
    }

    let score = {
        let mut sum = 0;
        for v in diffs {
            sum += v.abs();
        }
        sum
    };

    // arbitrary similarity score threshold
    score < 500
}
