use std::fs;
use std::io::Write;

use std::fs::File;
use std::io::BufReader;

use rodio::{source::Source, Decoder, OutputStream};

const IMG_CHARS: [char; 10] = ['.', ',', '+', 'o', '^', '*', '0', '&', '#', '@'];

fn main() {
    // play(fs::read_to_string("out.txt").unwrap(), 15);

    let mut out = String::new();
    let mut frames = fs::read_dir("frames")
        .unwrap()
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();

    frames.sort_by(|x, y| {
        let x = x.path();
        let y = y.path();
        let x = x.to_str().unwrap();
        let y = y.to_str().unwrap();

        let x_parts = x.split('.').collect::<Vec<&str>>();
        let y_parts = y.split('.').collect::<Vec<&str>>();

        if x_parts.len() == 3 && y_parts.len() == 3 {
            return x_parts[1]
                .parse::<u32>()
                .unwrap()
                .cmp(&y_parts[1].parse().unwrap());
        }

        x.cmp(y)
    });

    for i in frames {
        println!("Processing `{}`", i.file_name().into_string().unwrap());

        let size = (100, 100);

        let img = image::open(i.path()).unwrap();
        let img = img
            .resize(size.0, size.1, image::imageops::FilterType::Triangle)
            .into_rgb8();

        let frame = asciify(im_load(img));
        out.push_str(&frame);
        out.push('\n');
        out.push('\n');
    }

    fs::write("out.txt", out).unwrap();
}

fn im_load(img: image::RgbImage) -> Vec<Vec<f32>> {
    let mut image = Vec::new();
    let dim = img.dimensions();
    for y in 0..dim.1 {
        let mut v = Vec::new();
        for x in 0..dim.0 {
            let px = img.get_pixel(x, y).0;
            let px = px[0] as u16 + px[1] as u16 + px[2] as u16;
            let per = (px / 3) as f32 / 255.0;
            assert!(per < 1.0);
            v.push(per);
        }
        image.push(v);
    }
    image
}

fn asciify(mut image: Vec<Vec<f32>>) -> String {
    let dim = (image[0].len(), image.len());

    let mut out = String::new();
    for y in 0..dim.1 as usize {
        for x in 0..dim.0 as usize {
            let mut px = image[y][x];
            // todo fix this
            if px > 1.0 {
                px = 0.99;
            }

            let index = (px * IMG_CHARS.len() as f32).floor();
            let chr = IMG_CHARS[index as usize];
            let err = px - index / IMG_CHARS.len() as f32;

            if x > 1 && x < dim.1 as usize - 1 && y < dim.1 as usize - 1 {
                image[y + 0][x + 1] = image[y + 0][x + 1] + err * 7.0 / 16.0;
                image[y + 1][x - 1] = image[y + 1][x - 1] + err * 3.0 / 16.0;
                image[y + 1][x + 0] = image[y + 1][x + 0] + err * 5.0 / 16.0;
                image[y + 1][x + 1] = image[y + 1][x + 1] + err * 1.0 / 16.0;
            }

            out.push(chr);
            out.push(chr);
        }
        out.push('\n');
    }

    out
}

fn play(data: String, fps: u16) {
    let fpms = 1000 / fps as u128;
    let data = data.replace("\r", "   ");
    let frames = data.split("\n\n");

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let file = BufReader::new(File::open("audio.mp3").unwrap());
    let source = Decoder::new(file).unwrap();
    stream_handle.play_raw(source.convert_samples()).unwrap();

    for i in frames {
        let start = std::time::Instant::now();
        std::io::stdout().write_all("\x1B[H".as_bytes()).unwrap();
        std::io::stdout().write_all(i.as_bytes()).unwrap();
        std::io::stdout().flush().unwrap();

        while start.elapsed().as_millis() < fpms {}
    }
}
