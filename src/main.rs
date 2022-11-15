use std::io::{stdout, Cursor, Write};
use std::{fs, path::PathBuf};

use clap::{Arg, Command};
use image::imageops::FilterType;
use rodio::{source::Source, Decoder, OutputStream};

const IMG_CHARS: [char; 10] = ['.', ',', '+', '^', 'o', '*', '&', '0', '#', '@'];
// const IMG_CHARS: [char; 4] = ['.', '!', 'o', 'm'];
// const IMG_CHARS: [char; 4] = ['🐄', '🐼', '🐱', '🐮'];

fn main() {
    // Clap arg parser
    // Subcommands:
    // - play: takes in a file to play and an optional audio file
    // - convert: takes in a folder path (folder of images) and an output file path and if dithering should be used
    let matches = Command::new("Image to Audio")
        .version("0.1")
        .author("Connor Slade <connor@connorcode.com>")
        .about("converts video to text")
        .subcommand(
            Command::new("play")
                .about("plays back an ascii video")
                .arg(Arg::new("file").help("the file to play").required(true))
                .arg(
                    Arg::new("audio")
                        .help("the audio file to play")
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("convert")
                .about("converts a folder of images to an ascii video")
                .arg(
                    Arg::new("folder")
                        .help("the folder of images to convert")
                        .required(true),
                )
                .arg(
                    Arg::new("output")
                        .help("the output file to write to")
                        .required(true),
                )
                .arg(
                    Arg::new("dither")
                        .help("whether to use dithering")
                        .required(false),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("play", matches)) => {
            let file = PathBuf::from(matches.get_one::<String>("file").unwrap());
            let audio = matches.get_one::<String>("audio").map(PathBuf::from);
            play(file, audio);
        }
        Some(("convert", matches)) => {
            let folder = PathBuf::from(matches.get_one::<String>("folder").unwrap());
            let output = PathBuf::from(matches.get_one::<String>("output").unwrap());
            let dither = matches.get_flag("dither");
            convert(folder, output, dither);
        }
        _ => {}
    }
}

// TODO: Binary fileformat
fn play(file: PathBuf, audio: Option<PathBuf>) {
    play_internal(
        fs::read_to_string(file).unwrap(),
        audio.and_then(|x| fs::read(x).ok()),
        15,
    );
}

fn convert(folder: PathBuf, output: PathBuf, dither: bool) {
    let mut out = String::new();
    let mut frames = fs::read_dir(folder)
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

        let size = (200, 200);

        let img = image::open(i.path()).unwrap();
        let img = img.resize(size.0, size.1, FilterType::Triangle).into_rgb8();

        let frame = asciify(im_load(img), dither);
        out.push_str(&frame);
        out.push('\n');
        out.push('\n');
    }

    fs::write(output, out).unwrap();
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
            assert!(per <= 1.0);
            v.push(per);
        }
        image.push(v);
    }
    image
}

fn asciify(mut image: Vec<Vec<f32>>, dither: bool) -> String {
    let dim = (image[0].len(), image.len());

    let mut out = String::new();
    for y in 0..dim.1 as usize {
        for x in 0..dim.0 as usize {
            let mut px = image[y][x];
            if px > 1.0 {
                px = 1.0;
            }

            let index = (px * (IMG_CHARS.len() - 1) as f32).floor();
            let chr = IMG_CHARS[index as usize];
            let err = px - index / IMG_CHARS.len() as f32;

            if dither && x > 1 && x < dim.1 as usize - 1 && y < dim.1 as usize - 1 {
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

fn play_internal(data: String, audio: Option<Vec<u8>>, fps: u16) {
    let fpms = 1000.0 / fps as f32;
    let data = data.replace("\r", "   ");
    let frames = data.split("\n\n");

    if let Some(i) = audio {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let file = Cursor::new(i);
        let source = Decoder::new(file).unwrap();
        stream_handle.play_raw(source.convert_samples()).unwrap();
    }

    for i in frames.skip(15) {
        let start = std::time::Instant::now();
        stdout().write_all("\x1B[H".as_bytes()).unwrap();
        stdout().write_all(i.as_bytes()).unwrap();
        stdout().flush().unwrap();

        while (start.elapsed().as_millis() as f32) < fpms {}
    }
}
