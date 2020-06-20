use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use image;

enum ImageTypes {
    Jpeg,
    Png,
    Gif,
    Bmp,
    Unknown,
}

struct ProcessData {
    input_type: ImageTypes,
    compression: u8,
    output_type: ImageTypes,
    output_width: u32,
    output_height: u32,
    crop_pixel_right: u32,
    crop_pixel_down: u32,
    crop_width: u32,
    crop_height: u32,
    ignore_aspect: bool,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3000).into();

    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(handler)) });

    let server = Server::bind(&addr).serve(service);

    println!("Listening on http://{}", addr);

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

async fn handler(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    //Get the type of image we are being sent to make sure it matches what the library guesses

    match req.method() {
        &Method::GET => Ok(Response::new(Body::from("you need to post data"))),
        &Method::POST => {
            let (parts, body) = req.into_parts();
            let whole_body = hyper::body::to_bytes(body).await?;

            let image_config = parse_headers(&parts).unwrap();

            let data = match process_image(&whole_body[..], &image_config) {
                Ok(d) => d,
                Err(e) => {
                    let mut encode_error = Response::default();
                    *encode_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    *encode_error.body_mut() = Body::from(e);
                    return Ok(encode_error);
                }
            };

            //set headers
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "image/jpeg")
                .body(Body::from(data))
                .unwrap())
        }
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

fn process_image(body: &[u8], config: &ProcessData) -> Result<Vec<u8>, String> {
    //Read in the image
    let mut image_decoded = match image::load_from_memory(body) {
        Ok(d) => d,
        Err(e) => {
            //println!("{}", e);
            return Err(e.to_string());
        }
    };

    //We have our image data we should now do some stuff with it.
    //We have to crop the image
    if config.crop_height != 0 && config.crop_width != 0 {
        image_decoded.crop(
            config.crop_pixel_right,
            config.crop_pixel_down,
            config.crop_width,
            config.crop_height,
        );
    }

    //reszie
    if config.output_height != 0 && config.output_width != 0 {
        if config.ignore_aspect == false {
            image_decoded.resize(
                config.output_width,
                config.output_height,
                image::imageops::FilterType::Lanczos3,
            );
        } else {
            image_decoded.resize_exact(
                config.output_width,
                config.output_height,
                image::imageops::FilterType::Lanczos3,
            );
        }
    }

    //Output
    let mut out = Vec::new();
    image_decoded
        .write_to(&mut out, image::ImageOutputFormat::Jpeg(config.compression))
        .unwrap();
    Ok(out)
}

fn parse_headers(req: &http::request::Parts) -> Result<ProcessData, String> {
    let image_type = match req.headers.get("Content-Type") {
        Some(val) => {
            let image_type = match &val
                .to_str()
                .unwrap_or("Invalid header")
                .to_ascii_lowercase()[..]
            {
                "image/jpeg" => ImageTypes::Jpeg,
                "image/png" => ImageTypes::Png,
                "image/gif" => ImageTypes::Gif,
                "image/bmp" => ImageTypes::Bmp,
                _ => ImageTypes::Unknown,
            };
            image_type
        }
        None => ImageTypes::Unknown,
    };

    //Get the compression level
    let compression: u8 = match req.headers.get("X-Compress") {
        Some(val) => {
            if val.is_empty() {
                80;
            }
            let mut com: u8 = val.to_str().unwrap_or("80").trim().parse().unwrap_or(80);
            //Our range for compression is 1 - 100 so if its not in that range set 80
            if com == 0 || com > 100 {
                com = 80;
            }
            com
        }
        None => 80, //We default compression to 80
    };

    //Get the type of image the server wants in response
    let image_response_type = match req.headers.get("Accept") {
        Some(val) => {
            let image_type = match &val
                .to_str()
                .unwrap_or("Invalid header")
                .to_ascii_lowercase()[..]
            {
                "image/jpeg" => ImageTypes::Jpeg,
                "image/png" => ImageTypes::Png,
                "image/gif" => ImageTypes::Gif,
                "image/bmp" => ImageTypes::Bmp,
                _ => ImageTypes::Jpeg,
            };
            image_type
        }
        None => ImageTypes::Jpeg,
    };

    //Get the size the output should be
    let (width, height) = match req.headers.get("X-Size") {
        Some(val) => {
            let sizes = val.to_str().unwrap_or("0x0");
            parse_size(&sizes)
        }
        None => (0u32, 0u32),
    };

    //Should we keep the same aspect ratio
    let ignore_aspect: bool = match req.headers.get("X-ignore-Aspect-Ratio") {
        Some(val) => {
            if val == "true" {
                true;
            }
            false
        }
        None => false,
    };

    //Should we crop the image
    let (x, y, cw, ch) = match req.headers.get("X-Crop") {
        Some(val) => {
            let crops = val.to_str().unwrap_or("0p0p0x0");
            parse_crop(&crops)
        }
        None => (0u32, 0u32, 0u32, 0u32),
    };

    Ok(ProcessData {
        input_type: image_type,
        compression: compression,
        output_type: image_response_type,
        crop_height: ch,
        crop_width: cw,
        crop_pixel_down: x,
        crop_pixel_right: y,
        output_height: height,
        output_width: width,
        ignore_aspect: ignore_aspect,
    })
}

fn parse_size(s: &str) -> (u32, u32) {
    let (mut x, mut y) = (0u32, 0u32);
    //trim the string to remove spaces
    let trimmed = s.trim();
    // expecting input like 800x600

    let mut count = 0;
    for i in trimmed.split("x") {
        if count == 0 {
            x = i.parse().unwrap_or(0);
            count += 1;
        } else if count == 1 {
            y = i.parse().unwrap_or(0);
        }
    }
    (x, y)
}

fn parse_crop(s: &str) -> (u32, u32, u32, u32) {
    let (mut x, mut y, mut w, mut h) = (0u32, 0u32, 0u32, 0u32);

    //split on p's expecting input like 10p10p800x600
    let trimmed = s.trim();
    let mut count = 0;
    for i in trimmed.split("p") {
        if count == 0 {
            x = i.parse().unwrap_or(0);
            count += 1;
        } else if count == 1 {
            y = i.parse().unwrap_or(0);
            count += 1;
        } else if count == 2 {
            let (w1, h1) = parse_size(i);
            w = w1;
            h = h1;
            count += 1;
        }
    }
    (x, y, w, h)
}
