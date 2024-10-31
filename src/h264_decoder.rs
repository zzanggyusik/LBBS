use ffmpeg::codec::decoder::Video;
use ffmpeg::format::context::Input;
use ffmpeg::software::scaling::{context::Context as ScalerContext, flag::Flags};
use ffmpeg::util::frame::video::Video as VideoFrame;
use image::{ImageBuffer, Rgba, DynamicImage};
use image::codecs::jpeg::JpegEncoder;
use std::convert::TryInto;
use std::io::Cursor;

pub fn decode_h264_to_jpeg(h264_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // FFmpeg 초기화
    ffmpeg::init().unwrap();

    // H.264 데이터를 입력으로 처리
    let mut ictx = Input::from_bytes(h264_data)?;
    let input = ictx.streams().best(ffmpeg::media::Type::Video).ok_or(ffmpeg::Error::StreamNotFound)?;
    let mut decoder = input.codec().decoder().video()?;

    // 스케일러 설정 (YUV420P -> RGB)
    let mut scaler = ScalerContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )?;

    let mut frame_index = 0;
    let mut output_buffer = Vec::new();

    // 패킷을 읽고 디코딩하여 프레임 추출
    for (stream, packet) in ictx.packets() {
        if stream.index() == input.index() {
            decoder.send_packet(&packet)?;
            let mut decoded = VideoFrame::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgb_frame = VideoFrame::empty();
                scaler.run(&decoded, &mut rgb_frame)?;

                // RGB 데이터를 이미지로 변환
                let buffer: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_raw(
                    rgb_frame.width(),
                    rgb_frame.height(),
                    rgb_frame.data(0).to_owned(),
                ).ok_or("Failed to create image buffer")?;

                // 이미지 버퍼를 JPEG로 저장
                let mut jpeg_buffer = Cursor::new(Vec::new());
                buffer.write_to(&mut jpeg_buffer, image::ImageOutputFormat::Jpeg(80))?;
                output_buffer = jpeg_buffer.into_inner();
                frame_index += 1;

                // 첫 프레임만 처리 (원하는 경우 모든 프레임을 처리하도록 변경 가능)
                break;
            }
        }
    }

    Ok(output_buffer)
}