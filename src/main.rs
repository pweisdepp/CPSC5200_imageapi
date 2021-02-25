#![feature(decl_macro)]

#[macro_use]
extern crate rocket;
extern crate rocket_multipart_form_data;

use rocket::{Data, Response, Request, http::ContentType, http::Status};
use rocket_multipart_form_data::*;
use rocket_include_static_resources::{StaticResponse, static_response, static_resources_initialize};
use rocket_raw_response::RawResponse;
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
    Rotate(i16),
}

static HELP_RESPONSE: &str =
    "\n
    CPSC5200 Individual Project - Pete Weisdepp\n
    Process your image using the API:\n
    Params:\n
            'image' - the path to your jpg or png image\n
            'params' - comma-separated commands\n
    Commands:\n
        fliph -> Flip image horizontally\n
        flipv -> Flip image vertically\n
        rotateleft -> Rotate image 90 degrees left\n
        rotateright -> Rotate image 90 degrees right\n
        rotate-N -> Rotate image by N degrees right\n
        grayscale -> Convert the image to grayscale\n
        resize-N -> Resize the image by N percent\n
        thumbnail -> Resize the image to 100x100 thumbnail\n
    Example: \n
    curl -F params='fliph,grayscale' -F image=@/home/pete/test2.jpg --output /home/pete/returntest2.jpg localhost:8000/\n";

#[get("/")]
fn index() -> String {
    String::from(HELP_RESPONSE)
}

#[post("/", data = "<data>")]
fn upload(content_type: &ContentType, data: Data) -> Result<RawResponse, &'static str> {
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(
        vec![
        MultipartFormDataField::text("params"),
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
    //let mut image_parameters = Vec::new();

    let mut parameters = match params {
        Some(text_fields) => {
            let mut image_parameters = Vec::new();
            let textfield = text_fields.get(0).unwrap();
            let text = &textfield.text;

            let commands: Vec<&str> = text.split(',').collect();

            for command in commands {

                // Separate text command from int amount - used for resize(n) and rotate(n)
                let cmd: Vec<&str> = command.split('-').collect();

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
                        image_parameters.push(ApiCommand::Rotate(cmd[1].parse::<i16>().unwrap()));
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
                        return Err("Unrecognized command: GET '/' for allowed commands");
                    }
                }
            }
            Some(image_parameters)

        }
        None => {
            return Err("No parameters specified");
        }
    }.unwrap();

    // Image processing
    let image = multipart_form_data.raw.remove("image");

    match image {
        Some(mut image) => {
            // Get image data from field
            let raw = image.remove(0);

            // TODO: figure out content type and filename
            let content_type = raw.content_type;
            let file_name = raw.file_name;

            let mut ext: Option<&str>= None;

            if let Some(ref name) = file_name {
                ext = Path::new(name.as_str())
                    .extension()
                    .and_then(|s|s.to_str()).clone();
            }

            let mut format: Option<ImageFormat> = None;
            if let Some(format_from_ext) = ext {
                match format_from_ext {
                    "png" => {
                        format = Some(ImageFormat::Png);
                    }

                    "jpg" => {
                        format = Some(ImageFormat::Jpeg);
                    }

                    _ => {
                        return Err("Please upload a .png or .jpg image");
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
            for command in parameters {
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
            img.write_to(&mut buffer, format.unwrap());
            Ok(RawResponse::from_vec(buffer, file_name.clone(), content_type))

        }
        None => Err("Please input a file.")
    }
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index])
        .mount("/", routes![upload])
        .launch();
}