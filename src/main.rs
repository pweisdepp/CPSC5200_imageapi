
#![feature(decl_macro)]

#[macro_use]
extern crate rocket;
extern crate rocket_multipart_form_data;

use rocket::http::{ContentType, Status};
use rocket::{Data, Response, Request, response};
use rocket_multipart_form_data::*;

#[macro_use]
use rocket_include_static_resources::{static_response, static_resources_initialize};

use rocket_include_static_resources::StaticResponse;
use rocket_raw_response::RawResponse;
use rocket::response::Responder;
use image::*;
use std::path::Path;

enum ApiCommand {
    FlipHorizontal,
    FlipVertical,
    ConvertToGray,
    Resize(u16),
    Thumbnail,
    RotateLeft,
    RotateRight,
    Rotate(u16),
}

static help_response: &str =
    "Process your image using the API:\
    Params:\
            'image' - the path to your jpg or png image\
            'params' - comma-separated commands\
    Commands:\
        fliph -> Flip image horizontally\
        flipv -> Flip image vertically\
        rotateleft -> Rotate image 90 degrees left\
        rotateright -> Rotate image 90 degrees right\
        rotate-N -> Rotate image by N degrees right\
        grayscale -> Convert the image to grayscale\
        resize-N -> Resize the image by N percent\
        thumbnail -> Resize the image to 100x100 thumbnail\
    Example: \
    curl -F params='fliph,grayscale' -F image=@/home/pete/test2.jpg --output /home/pete/returntest2.jpg localhost:8000/upload";

//TODO: Respond with API command help
#[get("/")]
fn index() -> StaticResponse {
    static_response!("CPSC5200 Individual Assignment by Pete Weisdepp\
    Image processing using RestAPI, built in Rust using Rocket and Image crates")
}

#[post("/upload", data = "<data>")]
fn upload(content_type: &ContentType, data: Data) -> Result<RawResponse, &'static str> {
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(
        vec![
        // Image processing parameters have a max length of 10
        MultipartFormDataField::text("params").repetition(Repetition::fixed(10)),
        // Image has 4MB maximum size
        MultipartFormDataField::raw("image")
            .size_limit(4 * 1024 * 1024)
            .content_type_by_string(Some(mime::IMAGE_STAR))
            .unwrap(),
        ]
    );

    let mut multipart_form_data = match MultipartFormData::parse(content_type, data, options) {
        Ok(multipart_form_data) => multipart_form_data,
        Err(err) => {
            match err {
                MultipartFormDataError::DataTooLargeError(_) => {
                    return Err("The file is too large.");
                }
                MultipartFormDataError::DataTypeError(_) => {
                    return Err("The file is not an image.");
                }
                _ => panic!("{:?}", err),
            }
        }
    };

    let params = multipart_form_data.texts.remove("params");
    let mut image_parameters = Vec::new();

    match params {
        Some(text_fields) => {
            for text_field in text_fields {
                let text = text_field.text;

                // Separate text command from int amount - used for resize(n) and rotate(n)
                let cmd: Vec<&str> = text.split('-').collect();

                match cmd[0] {
                    "fliph" => {
                        image_parameters.push(ApiCommand::FlipHorizontal);
                    }
                    "flipv" => {
                        image_parameters.push(ApiCommand::FlipVertical);
                    }
                    "rotateleft" => {
                        image_parameters.push(ApiCommand::RotateLeft);
                    }
                    "rotateright" => {
                        image_parameters.push(ApiCommand::RotateRight);
                    }
                    "rotate" => {
                        image_parameters.push(ApiCommand::Rotate(cmd[1].parse::<u16>().unwrap()));
                    }
                    "grayscale" => {
                        image_parameters.push(ApiCommand::ConvertToGray);
                    }
                    "resize" => {
                        image_parameters.push(ApiCommand::Resize(cmd[1].parse::<u16>().unwrap()));
                    }
                    "thumbnail" => {
                        image_parameters.push(ApiCommand::Thumbnail);
                    }
                    _ => {
                        Err("Unrecognized command")
                    }
                }
            }
        }
        None => Err("No parameters specified")

    }

    // Image processing
    let image = multipart_form_data.raw.remove("image");

    match image {
        Some(mut image) => {
            // Get image data from field
            let raw = image.remove(0);

            // TODO: figure out content type and filename
            let content_type = raw.content_type;
            let file_name = raw.file_name.unwrap_or("Image".to_string());

            let mut ext: Option<&str>= None;
            if let Some(name) = filename {
                let stripped_ext = Some(Path::new(name.as_str())
                    .extension()
                    .and_then(|s|s.to_str())
                    .unwrap());
                ext = stripped_ext;
            }

            let mut format: Option<ImageFormat>;
            if let Some(format_from_ext) = ext {
                match format_from_ext {
                    "png" => {
                        format = Some(ImageFormat::Png);
                    }

                    "jpg" => {
                        format = Some(ImageFormat::Jpeg);
                    }

                    _ => {
                        Err("Please upload a .png or .jpg image")
                    }
                }
            }

            // Pull out image bytes from request
            let mut img = raw.raw;

            // Convert to DynamicImage in order to do processing
            let mut img = image::load_from_memory_with_format(
                img.as_slice(),
                format.unwrap())
                .unwrap();

            // Loop through and perform image commands
            for command in image_parameters {
                match command {
                    ApiCommand::FlipHorizontal => {
                        img = img.fliph();
                    }
                    ApiCommand::FlipVertical => {
                        img = img.flipv();
                    }
                    ApiCommand::RotateLeft => {
                        img = img.rotate270();
                    }
                    ApiCommand::RotateRight => {
                        img = img.rotate90();
                    }
                    ApiCommand::Rotate(num) => {
                        // TODO: import imageproc to do rotation by arbitrary amount
                    }
                    ApiCommand::ConvertToGray => {
                        img = img.grayscale();
                    }
                    ApiCommand::Resize(num) => {
                        let percent = (num as f32)/100. as f32;
                        let new_width = img.width() * percent as u32;
                        let new_height = img.height() * percent as u32;
                        img = img.resize(new_width, new_height, imageops::FilterType::CatmullRom);
                    }
                    ApiCommand::Thumbnail => {
                        img = img.thumbnail(100, 100);
                    }
                }
            }

            // write DynamicImage to buffer with some sort of format, then send back to client
            let mut buffer = Vec::new();
            img.write_to(&mut buffer, ImageFormat::Jpeg);
            Ok(RawResponse::from_vec(buffer, Some(file_name), content_type))

        }
        None => Err("Please input a file."),
    }
}

fn main() {
    rocket::ignite()
        .attach(StaticResponse::fairing(|resources| {
            static_resources_initialize!(
                resources,
                "html-image-uploader",
                "/home/pete/CLionProjects/CPSC5200_imageapi/src/form.html"
            );
        }))
        .mount("/", routes![index])
        .mount("/", routes![upload])
        .launch();
}