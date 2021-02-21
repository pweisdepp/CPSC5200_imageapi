
#![feature(decl_macro)]

#[macro_use]
extern crate rocket;
extern crate rocket_multipart_form_data;

use rocket::http::{ContentType, Status};
use rocket::{Data, Response, Request, response};

use rocket_multipart_form_data::{mime, RawField, Repetition};
use rocket_multipart_form_data::{
    MultipartFormData,
    MultipartFormDataError,
    MultipartFormDataField,
    MultipartFormDataOptions,
};

#[macro_use]
use rocket_include_static_resources::static_response;
use rocket_include_static_resources::static_resources_initialize;

use rocket_include_static_resources::StaticResponse;
use rocket_raw_response::RawResponse;

use image::*;

use std::io::Cursor;
use std::path::Path;
use image::io::Reader;
use rocket::response::Responder;


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

//TODO: Respond with API command help
#[get("/")]
fn index() -> StaticResponse {
    static_response!("html-image-uploader")
}

#[post("/upload", data = "<data>")]
fn upload(content_type: &ContentType, data: Data) -> Result<RawResponse, &'static str> {
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(
        vec![
        // Image processing parameters have a max length of 10
        MultipartFormDataField::text("params").repetition(Repetition::fixed(10)),

        // Image part of request
        MultipartFormDataField::raw("image")
            .size_limit(32 * 1024 * 1024)
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

    if let Some(text_fields) = params {
        for text_field in text_fields {
            let text = text_field.text;

            let cmd: Vec<&str> = text.split('-').collect();


            // TODO: match on parameters and add to vec - need to deal with rotation degrees too
            match cmd[0].as_str() {
                "fliphori" => {
                    image_parameters.push(ApiCommand::FlipHorizontal);
                }
                "flipvert" => {
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
                _ => {}
            }
        }
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

            let img = raw.raw;

            // TODO: match statement against jpeg and pngs
            // let filename = raw.file_name;
            // let mut ext: Option<&str>= None;
            // if let Some(name) = filename {
            //     let stripped_ext = Some(Path::new(name.as_str())
            //         .extension()
            //         .and_then(|s|s.to_str())
            //         .unwrap());
            // }



            let mut img = image::load_from_memory_with_format(
                img.as_slice(),
                ImageFormat::Jpeg)
                .unwrap();


            // TODO: loop through params, matching on image commands
            // for command in image_parameters {
            //     match command {
            //
            //     }
            //
            // }



            let flipped = img.fliph();





            // write modified image to buffer, send back to client
            let mut buffer = Vec::new();
            flipped.write_to(&mut buffer, ImageFormat::Jpeg);
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