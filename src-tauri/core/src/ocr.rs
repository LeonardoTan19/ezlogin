#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::imageops::FilterType;
use ndarray::{Array, Array4, ArrayD, Axis};
use ort::{session::Session, value::TensorRef};

use crate::models::OcrResult;

const INPUT_WIDTH: u32 = 320;
const INPUT_HEIGHT: u32 = 48;
const BLANK_INDEX: usize = 0;

#[cfg(test)]
fn load_dict(dict_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(dict_path)
        .with_context(|| format!("failed to read dict file: {}", dict_path.display()))?;
    parse_dict_content(&content)
}

fn parse_dict_content(content: &str) -> Result<Vec<String>> {
    let mut chars = vec![String::new()];
    chars.extend(
        content
            .lines()
            .map(|line| line.trim_end_matches(['\r', '\n']))
            .filter(|line| !line.is_empty())
            .map(ToString::to_string),
    );
    Ok(chars)
}

fn preprocess(image_bytes: &[u8]) -> Result<Array4<f32>> {
    let image = image::load_from_memory(image_bytes).context("invalid captcha image")?;
    let src = image.to_rgb8();
    let (src_w, src_h) = src.dimensions();
    if src_w == 0 || src_h == 0 {
        return Err(anyhow!("captcha image has invalid dimensions"));
    }

    let ratio = src_w as f32 / src_h as f32;
    let target_w = ((INPUT_HEIGHT as f32 * ratio).ceil() as u32).clamp(1, INPUT_WIDTH);

    let resized = image::imageops::resize(&src, target_w, INPUT_HEIGHT, FilterType::Triangle);
    let resized = resized.as_raw();

    let mut data = vec![0.0_f32; (3 * INPUT_WIDTH * INPUT_HEIGHT) as usize];
    let plane_size = (INPUT_WIDTH * INPUT_HEIGHT) as usize;
    let row_stride = (target_w * 3) as usize;
    let scale = 1.0_f32 / 127.5_f32;

    for y in 0..INPUT_HEIGHT {
        let row_start = (y as usize) * row_stride;
        let row = &resized[row_start..row_start + row_stride];

        for (x, pixel) in row.chunks_exact(3).enumerate() {
            let idx = (y * INPUT_WIDTH + x as u32) as usize;
            let base = idx;

            data[base] = f32::from(pixel[0]) * scale - 1.0;
            data[plane_size + base] = f32::from(pixel[1]) * scale - 1.0;
            data[plane_size * 2 + base] = f32::from(pixel[2]) * scale - 1.0;
        }
    }

    let arr = Array::from_shape_vec((1, 3, INPUT_HEIGHT as usize, INPUT_WIDTH as usize), data)
        .context("failed to build input tensor")?;

    Ok(arr)
}

fn decode_ctc(logits: &ArrayD<f32>, dict: &[String]) -> Result<(String, f32)> {
    let shape = logits.shape();
    if shape.len() != 3 {
        return Err(anyhow!("unexpected output shape: {shape:?}"));
    }

    let (time_steps, classes, data_2d) = if shape[0] == 1 {
        let d1 = shape[1];
        let d2 = shape[2];
        let v = logits.index_axis(Axis(0), 0).to_owned();

        if d2 >= d1 {
            (d1, d2, v)
        } else {
            (d2, d1, v.reversed_axes())
        }
    } else if shape[1] == 1 {
        (shape[0], shape[2], logits.index_axis(Axis(1), 0).to_owned())
    } else {
        return Err(anyhow!("unsupported output layout: {shape:?}"));
    };

    if classes == 0 || time_steps == 0 {
        return Err(anyhow!("empty output logits"));
    }

    let mut text = String::new();
    let mut prev_idx = BLANK_INDEX;
    let mut confidence_sum = 0.0_f32;
    let mut confidence_count = 0_u32;

    for t in 0..time_steps {
        let row = data_2d.index_axis(Axis(0), t);

        let mut max_idx = 0_usize;
        let mut max_val = f32::MIN;
        for (idx, val) in row.iter().enumerate() {
            if *val > max_val {
                max_val = *val;
                max_idx = idx;
            }
        }

        if max_idx != BLANK_INDEX && max_idx != prev_idx {
            if let Some(ch) = dict.get(max_idx) {
                text.push_str(ch);
                confidence_sum += max_val;
                confidence_count += 1;
            }
        }

        prev_idx = max_idx;
    }

    let confidence = if confidence_count == 0 {
        0.0
    } else {
        confidence_sum / confidence_count as f32
    };

    Ok((sanitize_captcha(&text), confidence))
}

fn sanitize_captcha(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            if out.len() == 4 {
                break;
            }
        }
    }
    out
}

pub struct OcrEngine {
    session: Session,
    dict: Vec<String>,
}

impl OcrEngine {
    #[cfg(test)]
    pub fn new(model_path: &Path, dict_path: &Path) -> Result<Self> {
        let session = Session::builder()
            .context("failed to create ort session builder")?
            .commit_from_file(model_path)
            .with_context(|| format!("failed to load model: {}", model_path.display()))?;
        let dict = load_dict(dict_path)?;
        Ok(Self { session, dict })
    }

    pub fn from_embedded(model_bytes: &[u8], dict_content: &str) -> Result<Self> {
        let session = Session::builder()
            .context("failed to create ort session builder")?
            .commit_from_memory(model_bytes)
            .context("failed to load embedded rec.onnx from memory")?;
        let dict = parse_dict_content(dict_content)?;
        Ok(Self { session, dict })
    }

    pub fn recognize(&mut self, image_bytes: &[u8]) -> Result<OcrResult> {
        let input = preprocess(image_bytes)?;
        let outputs = self
            .session
            .run(ort::inputs![TensorRef::from_array_view(input.view())?])
            .context("failed to run rec.onnx inference")?;

        let first = &outputs[0];
        let logits = first
            .try_extract_array::<f32>()
            .context("failed to extract output tensor")?
            .to_owned();

        let (text, confidence) = decode_ctc(&logits, &self.dict)?;
        Ok(OcrResult { text, confidence })
    }
}
